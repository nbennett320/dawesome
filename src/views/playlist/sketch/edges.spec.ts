import {
  EDGE_ZONE_PX, Snapping, TrimClip, edgeAt, hiddenBeyond, patternSpan, patternTrimClip, sampleTrimClip,
  snapPoint, trimAll, trimmed, windowOf,
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
  const beats: Snapping = { enabled: true, beats: 1, playheadBeat: null, bypass: false }

  it('is the nearest snap step', () => {
    expect(snapPoint(2.4, beats)).toBe(2)
    expect(snapPoint(2.6, beats)).toBe(3)
    expect(snapPoint(2.5, beats)).toBe(3)
    expect(snapPoint(7, beats)).toBe(7)
    expect(snapPoint(1.3, { ...beats, beats: 0.25 })).toBe(1.25)
  })

  it('is the playhead when the mouse is nearer to it than to the steps either side', () => {
    const stopped = { ...beats, playheadBeat: 2.3 }
    // the mouse at 2.35: 0.05 from the playhead, 0.35 and 0.65 from the lines
    expect(snapPoint(2.35, stopped)).toBe(2.3)
    // at 2.9 the line at 3 is nearer
    expect(snapPoint(2.9, stopped)).toBe(3)
    // at 2.1: 0.2 from the playhead, 0.1 from the line at 2
    expect(snapPoint(2.1, stopped)).toBe(2)
    // just as near to both: the line
    expect(snapPoint(2.15, { ...beats, playheadBeat: 2.3 })).toBe(2)
    // a playhead in another step, however near the mouse is to a line
    expect(snapPoint(4.4, stopped)).toBe(4)
    // on a line, the line
    expect(snapPoint(2, { ...beats, playheadBeat: 2.001 })).toBe(2)
  })

  it('is not the playhead while playing', () => {
    expect(snapPoint(2.35, { ...beats, playheadBeat: null })).toBe(2)
  })

  it('is where the mouse is with snap off, or Option held', () => {
    expect(snapPoint(2.35, { ...beats, enabled: false, playheadBeat: 2.3 })).toBe(2.35)
    expect(snapPoint(2.35, { ...beats, bypass: true, playheadBeat: 2.3 })).toBe(2.35)
    expect(snapPoint(2.35, { ...beats, beats: 0 })).toBe(2.35)
  })
})

// a clip from beat 4 to 12, of a source that runs from beat 2 to 16
const clip: TrimClip = { id: 1, startBeat: 4, endBeat: 12, sourceStartBeat: 2, sourceEndBeat: 16, minBeats: 0.02 }

describe('trimmed', () => {
  it('moves one edge, and leaves the other where it is', () => {
    expect(trimmed(clip, 'left', 2)).toEqual({ startBeat: 6, endBeat: 12 })
    expect(trimmed(clip, 'left', -1)).toEqual({ startBeat: 3, endBeat: 12 })
    expect(trimmed(clip, 'right', -4)).toEqual({ startBeat: 4, endBeat: 8 })
    expect(trimmed(clip, 'right', 3)).toEqual({ startBeat: 4, endBeat: 15 })
  })

  it('stops at the ends of the source', () => {
    expect(trimmed(clip, 'left', -5).startBeat).toBe(2)
    expect(trimmed(clip, 'right', 9).endBeat).toBe(16)
  })

  it('stops the left edge at the start of the playlist', () => {
    // its source would start a beat before the playlist does
    const early = { ...clip, startBeat: 1, sourceStartBeat: -1 }
    expect(trimmed(early, 'left', -3).startBeat).toBe(0)
  })

  it('lets the right edge go past the end of the playlist, as far as the source goes', () => {
    const late = { ...clip, startBeat: 250, endBeat: 255, sourceStartBeat: 250, sourceEndBeat: 262 }
    expect(trimmed(late, 'right', 40).endBeat).toBe(262)
  })

  it('never makes a clip shorter than it can be', () => {
    expect(trimmed(clip, 'left', 100).startBeat).toBeCloseTo(11.98, 10)
    expect(trimmed(clip, 'right', -100).endBeat).toBeCloseTo(4.02, 10)
    // a pattern clip: a step (a quarter of a beat)
    const pattern = patternTrimClip(2, 8, 16, null)
    expect(trimmed(pattern, 'right', -100)).toEqual({ startBeat: 8, endBeat: 8.25 })
    expect(trimmed(pattern, 'left', 100)).toEqual({ startBeat: 11.75, endBeat: 12 })
  })
})

describe('trimAll', () => {
  const short: TrimClip = { id: 2, startBeat: 0, endBeat: 1.5, sourceStartBeat: 0, sourceEndBeat: 1.5, minBeats: 0.02 }
  const other: TrimClip = { id: 3, startBeat: 20, endBeat: 30, sourceStartBeat: 20, sourceEndBeat: 30, minBeats: 0.02 }

  it('moves the same edge of every clip as far as the grabbed clip\'s', () => {
    // the grabbed clip's right edge, from 12 to 11
    expect(trimAll([clip, short, other], 1, 'right', 11)).toEqual([
      { id: 1, startBeat: 4, endBeat: 11 },
      { id: 2, startBeat: 0, endBeat: 0.5 },
      { id: 3, startBeat: 20, endBeat: 29 },
    ])
  })

  it('stops each clip at its own limit while the others carry on', () => {
    // three beats shorter: the short clip is down to its least after 1.48
    const spans = trimAll([clip, short, other], 1, 'right', 9)
    expect(spans[0]).toEqual({ id: 1, startBeat: 4, endBeat: 9 })
    expect(spans[1].endBeat).toBeCloseTo(0.02, 10)
    expect(spans[2]).toEqual({ id: 3, startBeat: 20, endBeat: 27 })

    // and the grabbed clip stopping doesn't stop the others: its left edge
    // can only go back to beat 2, the mouse is at beat 0
    const left = trimAll([clip, { ...other, sourceStartBeat: 10 }], 1, 'left', 0)
    expect(left).toEqual([
      { id: 1, startBeat: 2, endBeat: 12 },
      { id: 3, startBeat: 16, endBeat: 30 },
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
    expect(whole).toEqual({ id: 1, startBeat: 4, endBeat: 8, sourceStartBeat: 4, sourceEndBeat: 8, minBeats: 0.02 })
    // an octave up it plays in half the time
    expect(sampleTrimClip(1, 4, 2, 2, null, 0.5).endBeat).toBe(6)
  })

  it('with a window, starts where the window does and has the rest of the sample either side', () => {
    // from 0.5s to 1.5s of the sample
    const windowed = sampleTrimClip(1, 4, 2, 1, { start: 0.5, end: 1.5 }, 0.5)
    expect(windowed).toMatchObject({ startBeat: 4, endBeat: 6, sourceStartBeat: 3, sourceEndBeat: 7 })
    expect(sampleTrimClip(1, 4, 2, 1, { start: 0.5, end: null }, 0.5).endBeat).toBe(7)
  })

  it('trimmed, plays the part of the sample its edges are at', () => {
    // the left edge to beat 5 (half a second in), the right to beat 7
    expect(windowOf(whole, { startBeat: 5, endBeat: 8 }, 0.5)).toEqual({ start: 0.5, end: null })
    expect(windowOf(whole, { startBeat: 4, endBeat: 7 }, 0.5)).toEqual({ start: 0, end: 1.5 })
    expect(windowOf(whole, { startBeat: 5, endBeat: 7 }, 0.5)).toEqual({ start: 0.5, end: 1.5 })
    // back out to its ends, it's the whole sample
    expect(windowOf(whole, { startBeat: 4, endBeat: 8 }, 0.5)).toEqual({ start: 0, end: null })
    // an octave up, a beat is a second of the sample
    const fast = sampleTrimClip(1, 4, 2, 2, null, 0.5)
    expect(windowOf(fast, { startBeat: 4.5, endBeat: 5.5 }, 1)).toEqual({ start: 0.5, end: 1.5 })
  })

  it('trimmed again, counts from where the sample starts, not the clip', () => {
    const windowed = sampleTrimClip(1, 4, 2, 1, { start: 0.5, end: 1.5 }, 0.5)
    const span = trimmed(windowed, 'left', -0.5)
    expect(span).toEqual({ startBeat: 3.5, endBeat: 6 })
    expect(windowOf(windowed, span, 0.5)).toEqual({ start: 0.25, end: 1.5 })
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
    expect(windowed).toEqual({ id: 2, startBeat: 8, endBeat: 10, sourceStartBeat: 7, sourceEndBeat: 11, minBeats: 0.25 })
    // its right edge out to the pattern's end
    const span = trimmed(windowed, 'right', 5)
    expect(span).toEqual({ startBeat: 8, endBeat: 11 })
    expect(windowOf(windowed, span, 4)).toEqual({ start: 4, end: null })
    expect(windowOf(windowed, { startBeat: 8.5, endBeat: 10 }, 4)).toEqual({ start: 6, end: 12 })
  })
})

describe('hiddenBeyond', () => {
  it('marks the edges that hide some of the source', () => {
    expect(hiddenBeyond(null, 2)).toEqual({ left: false, right: false })
    expect(hiddenBeyond({ start: 0.5, end: null }, 2)).toEqual({ left: true, right: false })
    expect(hiddenBeyond({ start: 0, end: 1.5 }, 2)).toEqual({ left: false, right: true })
    expect(hiddenBeyond({ start: 0.5, end: 1.5 }, 2)).toEqual({ left: true, right: true })
    // a pattern made shorter than the window: nothing is hidden past its end
    expect(hiddenBeyond({ start: 0, end: 24 }, 16)).toEqual({ left: false, right: false })
  })
})
