import { PlaylistTransport } from '../../types/transport'
import {
  barAndBeat,
  CASSETTE_MAX_WIDTH,
  CASSETTE_MIN_WIDTH,
  fitCassetteWidth,
  formatTime,
  loopMoved,
  positionAt,
  tapeLengthMs,
  tapTarget,
  windDone,
  windMovedMs,
  windPosition,
  windSpeedAt,
  WIND_ACCEL,
  WIND_SPEED,
} from './tape'

// 120bpm 4/4: a beat is 500ms, a bar 2s, the 16 beat loop 8s
const transport = (t: Partial<PlaylistTransport> = {}): PlaylistTransport => ({
  playing: false,
  startTimeMs: null,
  startOffsetMs: 0,
  positionMs: 0,
  lastPlayStartBeats: null,
  beatIntervalMs: 500,
  loopStartBeats: 0, loopEndBeats: 16, loopRegion: false, loopEndPinned: false,
  beatsDisplayed: 16,
  loopEnabled: true,
  songEndMs: 0,
  beatsPerBar: 4,
  patternId: null,
  patternLength: 0,
  ...t,
})

describe('tapeLengthMs', () => {
  it('is the loop while looping', () => {
    expect(tapeLengthMs(transport({ songEndMs: 5_250 }))).toBe(8_000)
    expect(tapeLengthMs(transport({ loopEndBeats: 8 }))).toBe(4_000)
  })

  it('runs to the end of the last clip without looping', () => {
    expect(tapeLengthMs(transport({ loopEnabled: false, songEndMs: 5_250 }))).toBe(5_250)
  })

  it('is four bars with no clips and no loop', () => {
    expect(tapeLengthMs(transport({ loopEnabled: false, beatsPerBar: 3 }))).toBe(6_000)
  })
})

describe('positionAt', () => {
  it('is where the engine left it while stopped', () => {
    expect(positionAt(transport({ positionMs: 1_234 }), 99_999)).toBe(1_234)
  })

  it('follows the run from where it started', () => {
    const t = transport({ playing: true, startTimeMs: 10_000, startOffsetMs: 2_000 })
    expect(positionAt(t, 11_500)).toBe(3_500)
  })

  it('runs faster or slower than the clock at another speed (the varispeed)', () => {
    const t = transport({ playing: true, startTimeMs: 10_000, startOffsetMs: 2_000, speed: 1.1 })
    expect(positionAt(t, 11_000)).toBeCloseTo(3_100)
    expect(positionAt({ ...t, speed: 0.88 }, 11_000)).toBeCloseTo(2_880)
  })

  it("stays still while a pattern plays on its own", () => {
    const t = transport({ playing: true, startTimeMs: 10_000, positionMs: 2_000, patternId: 0 })
    expect(positionAt(t, 11_500)).toBe(2_000)
  })

  it('jumps back to the start at the loop end', () => {
    const t = transport({ playing: true, startTimeMs: 10_000, startOffsetMs: 7_000 })
    expect(positionAt(t, 12_000)).toBe(1_000)
  })

  it('plays into a region, then goes round inside it', () => {
    // a region from bar 2 to the middle of bar 3 (2s to 5s)
    const region = { loopStartBeats: 4, loopEndBeats: 10, loopRegion: true }
    const t = transport({ ...region, playing: true, startTimeMs: 10_000, startOffsetMs: 500 })
    expect(positionAt(t, 11_000)).toBe(1_500)
    expect(positionAt(t, 14_400)).toBe(4_900)
    expect(positionAt(t, 14_500)).toBe(2_000)
    expect(positionAt(t, 18_250)).toBe(2_750)
  })

  it('stops at the end of the song without looping', () => {
    const t = transport({
      playing: true, startTimeMs: 10_000, loopEnabled: false, songEndMs: 3_000,
    })
    expect(positionAt(t, 20_000)).toBe(3_000)
  })
})

describe('tapeLengthMs with a region', () => {
  it("runs to the region's end", () => {
    expect(tapeLengthMs(transport({ loopStartBeats: 4, loopEndBeats: 10, loopRegion: true }))).toBe(5_000)
  })
})

describe('loopMoved', () => {
  it('sees either edge move', () => {
    expect(loopMoved(transport(), transport())).toBe(false)
    expect(loopMoved(transport(), transport({ loopEndBeats: 12 }))).toBe(true)
    expect(loopMoved(transport(), transport({ loopStartBeats: 4 }))).toBe(true)
  })
})

describe('barAndBeat', () => {
  it('counts from one', () => {
    expect(barAndBeat(0, transport())).toEqual({ bar: 1, beat: 1 })
    expect(barAndBeat(499, transport())).toEqual({ bar: 1, beat: 1 })
    expect(barAndBeat(2_500, transport())).toEqual({ bar: 2, beat: 2 })
  })

  it('follows the time signature', () => {
    expect(barAndBeat(1_500, transport({ beatsPerBar: 3 }))).toEqual({ bar: 2, beat: 1 })
  })
})

describe('formatTime', () => {
  it('shows minutes, seconds and tenths', () => {
    expect(formatTime(0)).toBe('0:00.0')
    expect(formatTime(61_250)).toBe('1:01.2')
  })
})

describe('tapTarget', () => {
  const t = transport({ loopEnabled: false, songEndMs: 9_000 })

  it('moves forward to the next bar', () => {
    expect(tapTarget(0, 1, t)).toBe(2_000)
    expect(tapTarget(2_500, 1, t)).toBe(4_000)
  })

  it('stops at the end of the tape', () => {
    expect(tapTarget(8_500, 1, t)).toBe(9_000)
  })

  it('goes back to the start of the bar', () => {
    expect(tapTarget(5_000, -1, t)).toBe(4_000)
  })

  it('goes back a bar when already at its start', () => {
    expect(tapTarget(4_000, -1, t)).toBe(2_000)
    expect(tapTarget(4_050, -1, t)).toBe(2_000)
  })

  it('stops at the start', () => {
    expect(tapTarget(0, -1, t)).toBe(0)
  })
})

describe('windPosition', () => {
  const t = transport({ loopEnabled: false, songEndMs: 9_000 })

  it('winds faster than playing, getting up to speed as the engine does', () => {
    // the engine's constants (engine/crates/tape/src/scrub.rs)
    expect(WIND_SPEED).toBe(10)
    expect(WIND_ACCEL).toBe(30)
    // 0.1 s in: 3x, 0.15 s of song gone by
    expect(windSpeedAt(100)).toBeCloseTo(3)
    expect(windMovedMs(100)).toBeCloseTo(150)
    // a second in: a third of a second getting there (1/6 s of song), then 10x
    expect(windSpeedAt(1_000)).toBe(WIND_SPEED)
    expect(windMovedMs(1_000)).toBeCloseTo(10_000 / 6 + 20_000 / 3)
    expect(windPosition({ direction: 1, fromMs: 1_000, startedAt: 0 }, 100, t)).toBeCloseTo(1_150)
    expect(windPosition({ direction: -1, fromMs: 3_000, startedAt: 0 }, 100, t)).toBeCloseTo(2_850)
  })

  it('stops at the loop end while looping', () => {
    const looping = transport({ songEndMs: 3_000 })
    expect(windPosition({ direction: 1, fromMs: 1_000, startedAt: 0 }, 60_000, looping)).toBe(8_000)
  })

  it('stays on the tape', () => {
    const forward = { direction: 1 as const, fromMs: 1_000, startedAt: 0 }
    const back = { direction: -1 as const, fromMs: 1_000, startedAt: 0 }
    expect(windPosition(forward, 60_000, t)).toBe(9_000)
    expect(windPosition(back, 60_000, t)).toBe(0)
    expect(windDone(forward, 9_000, t)).toBe(true)
    expect(windDone(back, 0, t)).toBe(true)
    expect(windDone(back, 10, t)).toBe(false)
  })
})

describe('fitCassetteWidth', () => {
  it('fits the narrower of the room\'s width and height', () => {
    expect(fitCassetteWidth(300, 500, 1.6)).toBe(300)
    expect(fitCassetteWidth(500, 200, 1.6)).toBe(320)
  })

  it('stops growing at its largest', () => {
    expect(fitCassetteWidth(2_000, 2_000, 1.6)).toBe(CASSETTE_MAX_WIDTH)
  })

  it('stops shrinking at its smallest', () => {
    expect(fitCassetteWidth(50, 50, 1.6)).toBe(CASSETTE_MIN_WIDTH)
    expect(fitCassetteWidth(500, -40, 1.6)).toBe(CASSETTE_MIN_WIDTH)
  })
})
