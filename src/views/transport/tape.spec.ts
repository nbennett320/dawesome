import { PlaylistTransport } from '../../types/transport'
import {
  barAndBeat,
  CASSETTE_MAX_WIDTH,
  CASSETTE_MIN_WIDTH,
  fitCassetteWidth,
  formatTime,
  positionAt,
  tapeLengthMs,
  tapTarget,
  windDone,
  windPosition,
  WIND_SPEED,
} from './tape'

// 120bpm 4/4: a beat is 500ms, a bar 2s, the 16 beat loop 8s
const transport = (t: Partial<PlaylistTransport> = {}): PlaylistTransport => ({
  playing: false,
  startTimeMs: null,
  startOffsetMs: 0,
  positionMs: 0,
  beatIntervalMs: 500,
  loopBeats: 16,
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
    expect(tapeLengthMs(transport({ loopBeats: 8 }))).toBe(4_000)
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

  it("stays still while a pattern plays on its own", () => {
    const t = transport({ playing: true, startTimeMs: 10_000, positionMs: 2_000, patternId: 0 })
    expect(positionAt(t, 11_500)).toBe(2_000)
  })

  it('jumps back to the start at the loop end', () => {
    const t = transport({ playing: true, startTimeMs: 10_000, startOffsetMs: 7_000 })
    expect(positionAt(t, 12_000)).toBe(1_000)
  })

  it('stops at the end of the song without looping', () => {
    const t = transport({
      playing: true, startTimeMs: 10_000, loopEnabled: false, songEndMs: 3_000,
    })
    expect(positionAt(t, 20_000)).toBe(3_000)
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

  it('winds faster than playing', () => {
    expect(windPosition({ direction: 1, fromMs: 1_000, startedAt: 0 }, 100, t))
      .toBe(1_000 + WIND_SPEED * 100)
    expect(windPosition({ direction: -1, fromMs: 3_000, startedAt: 0 }, 100, t))
      .toBe(3_000 - WIND_SPEED * 100)
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
