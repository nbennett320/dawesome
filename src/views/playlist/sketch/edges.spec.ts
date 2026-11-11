import {
  EDGE_ZONE_PX, Snapping, StretchClip, TrimClip, edgeAt, hiddenBeyond, patternSpan, patternTrimClip, pitchOf,
  sampleTrimClip, snapPoint, stretchAll, stretchSet, stretched, trimAll, trimmed, trimmedWindow, windowOf,
} from './edges'

describe('edgeAt', () => {
  it('is an edge within 6 pixels inside it, at any clip width that has room', () => {
    expect(EDGE_ZONE_PX).toBe(6)
    ;[40, 400, 4_000].forEach(width => {
      expect(edgeAt(0, width)).toBe('left')
      expect(edgeAt(5.9, width)).toBe('left')
      expect(edgeAt(6, width)).toBeNull()
      expect(edgeAt(width / 2, width)).toBeNull()
      expect(edgeAt(width - 6, width)).toBeNull()
      expect(edgeAt(width - 5.9, width)).toBe('right')
      expect(edgeAt(width, width)).toBe('right')
    })
  })

  it('leaves the middle third of a narrow clip for moving it', () => {
    // 12 pixels wide: 4 for each edge, 4 to move it by
    expect(edgeAt(3.9, 12)).toBe('left')
    expect(edgeAt(4, 12)).toBeNull()
    expect(edgeAt(6, 12)).toBeNull()
    expect(edgeAt(8, 12)).toBeNull()
    expect(edgeAt(8.1, 12)).toBe('right')
    // and a sliver still has a middle
    expect(edgeAt(1.5, 3)).toBeNull()
    expect(edgeAt(0.5, 3)).toBe('left')
    expect(edgeAt(2.5, 3)).toBe('right')
  })

  it('is nothing outside the clip', () => {
    expect(edgeAt(-1, 100)).toBeNull()
    expect(edgeAt(101, 100)).toBeNull()
    expect(edgeAt(0, 0)).toBeNull()
  })
})

describe('snapPoint', () => {
  const beats: Snapping = { enabled: true, every: 1, playhead: null, bypass: false }

  it('is the nearest snap step', () => {
    expect(snapPoint(2.4, beats)).toBe(2)
    expect(snapPoint(2.6, beats)).toBe(3)
    expect(snapPoint(2.5, beats)).toBe(3)
    expect(snapPoint(7, beats)).toBe(7)
    expect(snapPoint(1.3, { ...beats, every: 0.25 })).toBe(1.25)
  })

  it('is the playhead when the mouse is nearer to it than to the steps either side', () => {
    const stopped = { ...beats, playhead: 2.3 }
    // the mouse at 2.35: 0.05 from the playhead, 0.35 and 0.65 from the lines
    expect(snapPoint(2.35, stopped)).toBe(2.3)
    // at 2.9 the line at 3 is nearer
    expect(snapPoint(2.9, stopped)).toBe(3)
    // at 2.1: 0.2 from the playhead, 0.1 from the line at 2
    expect(snapPoint(2.1, stopped)).toBe(2)
    // just as near to both: the line
    expect(snapPoint(2.15, { ...beats, playhead: 2.3 })).toBe(2)
    // a playhead in another step, however near the mouse is to a line
    expect(snapPoint(4.4, stopped)).toBe(4)
    // on a line, the line
    expect(snapPoint(2, { ...beats, playhead: 2.001 })).toBe(2)
  })

  it('is not the playhead while playing', () => {
    expect(snapPoint(2.35, { ...beats, playhead: null })).toBe(2)
  })

  it('is where the mouse is with snap off, or Option held', () => {
    expect(snapPoint(2.35, { ...beats, enabled: false, playhead: 2.3 })).toBe(2.35)
    expect(snapPoint(2.35, { ...beats, bypass: true, playhead: 2.3 })).toBe(2.35)
    expect(snapPoint(2.35, { ...beats, every: 0 })).toBe(2.35)
  })
})

// a clip from beat 4 to 12, of a source that runs from beat 2 to 16
const clip: TrimClip = {
  id: 1, start: 4, end: 12, sourceStart: 2, sourceEnd: 16, minLength: 0.02, perBeat: 0.5,
}

describe('trimmed', () => {
  it('moves one edge, and leaves the other where it is', () => {
    expect(trimmed(clip, 'left', 2)).toEqual({ start: 6, end: 12 })
    expect(trimmed(clip, 'left', -1)).toEqual({ start: 3, end: 12 })
    expect(trimmed(clip, 'right', -4)).toEqual({ start: 4, end: 8 })
    expect(trimmed(clip, 'right', 3)).toEqual({ start: 4, end: 15 })
  })

  it('stops at the ends of the source', () => {
    expect(trimmed(clip, 'left', -5).start).toBe(2)
    expect(trimmed(clip, 'right', 9).end).toBe(16)
  })

  it('stops the left edge at the start of the playlist', () => {
    // its source would start a beat before the playlist does
    const early = { ...clip, start: 1, sourceStart: -1 }
    expect(trimmed(early, 'left', -3).start).toBe(0)
  })

  it('lets the right edge go past the end of the playlist, as far as the source goes', () => {
    const late = { ...clip, start: 250, end: 255, sourceStart: 250, sourceEnd: 262 }
    expect(trimmed(late, 'right', 40).end).toBe(262)
  })

  it('never makes a clip shorter than it can be', () => {
    expect(trimmed(clip, 'left', 100).start).toBeCloseTo(11.98, 10)
    expect(trimmed(clip, 'right', -100).end).toBeCloseTo(4.02, 10)
    // a pattern clip: a step (a quarter of a beat)
    const pattern = patternTrimClip(2, 8, 16, null)
    expect(trimmed(pattern, 'right', -100)).toEqual({ start: 8, end: 8.25 })
    expect(trimmed(pattern, 'left', 100)).toEqual({ start: 11.75, end: 12 })
  })
})

describe('trimAll', () => {
  const short: TrimClip = {
    id: 2, start: 0, end: 1.5, sourceStart: 0, sourceEnd: 1.5, minLength: 0.02, perBeat: 0.5,
  }
  const other: TrimClip = {
    id: 3, start: 20, end: 30, sourceStart: 20, sourceEnd: 30, minLength: 0.02, perBeat: 0.5,
  }

  it('moves the same edge of every clip as far as the grabbed clip\'s', () => {
    // the grabbed clip's right edge, from 12 to 11
    expect(trimAll([clip, short, other], 1, 'right', 11)).toEqual([
      { id: 1, start: 4, end: 11 },
      { id: 2, start: 0, end: 0.5 },
      { id: 3, start: 20, end: 29 },
    ])
  })

  it('stops each clip at its own limit while the others carry on', () => {
    // three beats shorter: the short clip is down to its least after 1.48
    const spans = trimAll([clip, short, other], 1, 'right', 9)
    expect(spans[0]).toEqual({ id: 1, start: 4, end: 9 })
    expect(spans[1].end).toBeCloseTo(0.02, 10)
    expect(spans[2]).toEqual({ id: 3, start: 20, end: 27 })

    // and the grabbed clip stopping doesn't stop the others: its left edge
    // can only go back to beat 2, the mouse is at beat 0
    const left = trimAll([clip, { ...other, sourceStart: 10 }], 1, 'left', 0)
    expect(left).toEqual([
      { id: 1, start: 2, end: 12 },
      { id: 3, start: 16, end: 30 },
    ])
  })

  it('is nothing without the grabbed clip', () => {
    expect(trimAll([clip], 9, 'left', 0)).toEqual([])
  })
})

describe('a sample clip', () => {
  // two seconds of sample at 120bpm (half a second to the beat) on beat 4
  const whole = sampleTrimClip(1, 4, 2, 1, null, 0.5)

  it('is as long as its sample plays for, and no shorter than 10ms', () => {
    expect(whole).toEqual({
      id: 1, start: 4, end: 8, sourceStart: 4, sourceEnd: 8, minLength: 0.02, perBeat: 0.5,
    })
    // an octave up, a beat is a second of the sample
    expect(sampleTrimClip(1, 4, 2, 2, null, 0.5).perBeat).toBe(1)
    // an octave up it plays in half the time
    expect(sampleTrimClip(1, 4, 2, 2, null, 0.5).end).toBe(6)
  })

  it('with a window, starts where the window does and has the rest of the sample either side', () => {
    // from 0.5s to 1.5s of the sample
    const windowed = sampleTrimClip(1, 4, 2, 1, { start: 0.5, end: 1.5 }, 0.5)
    expect(windowed).toMatchObject({ start: 4, end: 6, sourceStart: 3, sourceEnd: 7 })
    expect(sampleTrimClip(1, 4, 2, 1, { start: 0.5, end: null }, 0.5).end).toBe(7)
  })

  it('trimmed, plays the part of the sample its edges are at', () => {
    // the left edge to beat 5 (half a second in), the right to beat 7
    expect(windowOf(whole, { start: 5, end: 8 })).toEqual({ start: 0.5, end: null })
    expect(windowOf(whole, { start: 4, end: 7 })).toEqual({ start: 0, end: 1.5 })
    expect(windowOf(whole, { start: 5, end: 7 })).toEqual({ start: 0.5, end: 1.5 })
    // back out to its ends, it's the whole sample
    expect(windowOf(whole, { start: 4, end: 8 })).toEqual({ start: 0, end: null })
    // an octave up, a beat is a second of the sample
    const fast = sampleTrimClip(1, 4, 2, 2, null, 0.5)
    expect(windowOf(fast, { start: 4.5, end: 5.5 })).toEqual({ start: 0.5, end: 1.5 })
    // and an octave down, a quarter of one
    const slow = sampleTrimClip(1, 4, 2, 0.5, null, 0.5)
    expect(slow.end).toBe(12)
    expect(windowOf(slow, { start: 6, end: 10 })).toEqual({ start: 0.5, end: 1.5 })
  })

  it('trimmed again, counts from where the sample starts, not the clip', () => {
    const windowed = sampleTrimClip(1, 4, 2, 1, { start: 0.5, end: 1.5 }, 0.5)
    const span = trimmed(windowed, 'left', -0.5)
    expect(span).toEqual({ start: 3.5, end: 6 })
    expect(windowOf(windowed, span)).toEqual({ start: 0.25, end: 1.5 })
  })
})

describe('a pattern clip', () => {
  it('spans its pattern, or its window of it', () => {
    expect(patternSpan(null, 16)).toEqual([0, 16])
    expect(patternSpan({ start: 4, end: null }, 16)).toEqual([4, 16])
    expect(patternSpan({ start: 4, end: 12 }, 16)).toEqual([4, 12])
  })

  it('ends with a pattern made shorter, and gets its steps back when it is longer again', () => {
    expect(patternSpan({ start: 4, end: 24 }, 16)).toEqual([4, 16])
    expect(patternSpan({ start: 4, end: 24 }, 32)).toEqual([4, 24])
    // past the pattern's end: a step long
    expect(patternSpan({ start: 20, end: null }, 16)).toEqual([20, 21])
  })

  it('is trimmed in steps of its pattern', () => {
    // a bar's pattern on beat 8, playing steps 4 to 12
    const windowed = patternTrimClip(2, 8, 16, { start: 4, end: 12 })
    expect(windowed).toEqual({
      id: 2, start: 8, end: 10, sourceStart: 7, sourceEnd: 11, minLength: 0.25, perBeat: 4,
    })
    // its right edge out to the pattern's end
    const span = trimmed(windowed, 'right', 5)
    expect(span).toEqual({ start: 8, end: 11 })
    expect(windowOf(windowed, span)).toEqual({ start: 4, end: null })
    expect(windowOf(windowed, { start: 8.5, end: 10 })).toEqual({ start: 6, end: 12 })
  })
})

describe('trimmedWindow', () => {
  // a clip whose start beat came back from the engine a little rounded
  const rounded = sampleTrimClip(1, 200.30000305175781, 2, 1, { start: 0.3, end: 1.7 }, 0.5)

  it('keeps the window of the edge that was not dragged, exactly', () => {
    // the right edge in by half a beat: the start is the 0.3 it was, not
    // one worked back out from the rounded beat
    const right = trimmedWindow(rounded, trimmed(rounded, 'right', -0.5), 'right', { start: 0.3, end: 1.7 })
    expect(right.start).toBe(0.3)
    expect(right.end).toBeCloseTo(1.45, 10)

    const left = trimmedWindow(rounded, trimmed(rounded, 'left', 0.5), 'left', { start: 0.3, end: 1.7 })
    expect(left.start).toBeCloseTo(0.55, 10)
    expect(left.end).toBe(1.7)
  })

  it('is the whole of that side for a clip with no window', () => {
    const whole = sampleTrimClip(1, 4, 2, 1, null, 0.5)
    expect(trimmedWindow(whole, { start: 4, end: 7 }, 'right', null)).toEqual({ start: 0, end: 1.5 })
    expect(trimmedWindow(whole, { start: 5, end: 8 }, 'left', undefined)).toEqual({ start: 0.5, end: null })
  })
})

describe('a pattern clip whose window is past its pattern\'s end', () => {
  // steps 20 on, of a pattern now a bar (16 steps) long: a step long
  const past = patternTrimClip(3, 8, 16, { start: 20, end: null })

  it('is a step long, and its right edge has nowhere to go', () => {
    expect(past.end - past.start).toBe(0.25)
    expect(trimmed(past, 'right', 3)).toEqual({ start: 8, end: 8.25 })
    expect(trimmed(past, 'right', -3)).toEqual({ start: 8, end: 8.25 })
  })
})

describe('hiddenBeyond', () => {
  it('marks the edges that hide some of the source', () => {
    expect(hiddenBeyond(0, 2, 2)).toEqual({ left: false, right: false })
    expect(hiddenBeyond(0.5, 2, 2)).toEqual({ left: true, right: false })
    expect(hiddenBeyond(0, 1.5, 2)).toEqual({ left: false, right: true })
    expect(hiddenBeyond(0.5, 1.5, 2)).toEqual({ left: true, right: true })
  })

  it('takes no notice of rounding, or of a source with no length', () => {
    expect(hiddenBeyond(1e-12, 2 - 1e-12, 2)).toEqual({ left: false, right: false })
    // a pattern made shorter than the clip's window: nothing past its end
    expect(hiddenBeyond(0, 16, 16)).toEqual({ left: false, right: false })
    expect(hiddenBeyond(0, 0, 0)).toEqual({ left: false, right: false })
  })
})

describe('stretching', () => {
  // two bars from bar 1 (beats 0 to 8), at its sample's own speed
  const bars: StretchClip = { id: 1, start: 0, end: 8, rate: 1, minLength: 0.02 }

  it('turns a length into a pitch, and back', () => {
    expect(pitchOf(1)).toBe(0)
    expect(pitchOf(2)).toBeCloseTo(12, 10)
    expect(pitchOf(0.5)).toBeCloseTo(-12, 10)
    expect(pitchOf(2 ** (7 / 12))).toBeCloseTo(7, 10)

    // out to bar 5, twice its length: an octave down
    const twice = stretched(bars, 'right', 2)
    expect(twice).toEqual({ start: 0, end: 16, rate: 0.5 })
    expect(pitchOf(twice.rate)).toBeCloseTo(-12, 10)
    // and stretched back by half from there, it's where it started
    const back = stretched({ ...bars, ...twice }, 'right', 0.5)
    expect(back).toEqual({ start: 0, end: 8, rate: 1 })
  })

  it('stretches from where a clip is, whatever its pitch already', () => {
    // a fifth up already, and made a third as long again
    const fifth = { ...bars, rate: 2 ** (7 / 12) }
    const longer = stretched(fifth, 'right', 4 / 3)
    expect(longer.end).toBeCloseTo(32 / 3, 10)
    expect(longer.rate).toBeCloseTo(fifth.rate * 3 / 4, 10)
  })

  it('keeps the end where it is when the left edge is dragged', () => {
    const later = { ...bars, start: 8, end: 16 }
    expect(stretched(later, 'left', 1.5)).toEqual({ start: 4, end: 16, rate: 2 / 3 })
    expect(stretched(later, 'left', 0.5)).toEqual({ start: 12, end: 16, rate: 2 })
  })

  it('stops the edge where the pitch can go no further', () => {
    // two octaves down: four times as long. two up: a quarter
    expect(stretched(bars, 'right', 9)).toEqual({ start: 0, end: 32, rate: 0.25 })
    expect(stretched(bars, 'right', 0.1)).toEqual({ start: 0, end: 2, rate: 4 })
    // already an octave up, it has an octave left that way and three the other
    const up = { ...bars, end: 4, rate: 2 }
    expect(stretched(up, 'right', 0.01).rate).toBe(4)
    expect(stretched(up, 'right', 100)).toEqual({ start: 0, end: 32, rate: 0.25 })
    expect(pitchOf(stretched(up, 'right', 100).rate)).toBe(-24)
  })

  it('stops the left edge at the start of the playlist', () => {
    const near = { ...bars, start: 2, end: 10 }
    expect(stretched(near, 'left', 3)).toEqual({ start: 0, end: 10, rate: 0.8 })
  })

  it('never makes a clip shorter than it can be', () => {
    // a clip so short that a quarter of it would be under the least
    const tiny: StretchClip = { id: 2, start: 0, end: 0.05, rate: 1, minLength: 0.02 }
    const span = stretched(tiny, 'right', 0.01)
    expect(span.end).toBeCloseTo(0.02, 10)
    expect(span.rate).toBeCloseTo(2.5, 10)
  })

  it('stretches every clip of a selection by as much, each from its own other edge', () => {
    const second: StretchClip = { id: 2, start: 20, end: 22, rate: 2, minLength: 0.02 }
    // the grabbed clip's right edge from beat 8 to beat 12: half as long again
    expect(stretchAll([bars, second], 1, 'right', 12)).toEqual([
      { id: 1, start: 0, end: 12, rate: 2 / 3 },
      { id: 2, start: 20, end: 23, rate: 4 / 3 },
    ])
    // and by their left edges: the ends stay
    const moved = stretchAll([{ ...bars, start: 8, end: 16 }, second], 1, 'left', 12)
    expect(moved).toEqual([
      { id: 1, start: 12, end: 16, rate: 2 },
      { id: 2, start: 21, end: 22, rate: 4 },
    ])
  })

  it('stops each clip at its own limit while the others carry on', () => {
    // one already two octaves down can't get longer; the other can
    const slow: StretchClip = { id: 2, start: 20, end: 28, rate: 0.25, minLength: 0.02 }
    const spans = stretchAll([bars, slow], 1, 'right', 16)
    expect(spans[0]).toEqual({ id: 1, start: 0, end: 16, rate: 0.5 })
    expect(spans[1]).toEqual({ id: 2, start: 20, end: 28, rate: 0.25 })
  })

  it('is as short as it goes when the edge is dragged past the other one', () => {
    expect(stretchAll([bars], 1, 'right', -3)).toEqual([{ id: 1, start: 0, end: 2, rate: 4 }])
    expect(stretchAll([bars], 9, 'right', 12)).toEqual([])
  })
})

describe('stretchSet', () => {
  const sample = (id: number) => ({ id, start: id, end: id + 2, rate: 1, minLength: 0.02, pattern: false })
  const pattern = (id: number) => ({ ...sample(id), pattern: true })

  it('is the sample clips of the selection, without the pattern clips', () => {
    const set = stretchSet([sample(1), pattern(2), sample(3)], 1)
    expect(set.map(each => each.id)).toEqual([1, 3])
    expect(set[0]).toEqual({ id: 1, start: 1, end: 3, rate: 1, minLength: 0.02 })
  })

  it('is nothing when the grabbed clip is a pattern clip, whatever is selected', () => {
    expect(stretchSet([pattern(2), sample(1), sample(3)], 2)).toEqual([])
    expect(stretchSet([sample(1)], 9)).toEqual([])
  })
})
