import { PlaylistTransport } from '../../types/transport'
import {
  barTicks, loopBand, MIN_TICK_GAP, msToX, readout, seekBeat, timelineEndMs, xToMs,
} from './strip'

// 120bpm 4/4: a beat is 500ms, a bar 2s
const transport = (t: Partial<PlaylistTransport> = {}): PlaylistTransport => ({
  playing: false,
  startTimeMs: null,
  startOffsetMs: 0,
  positionMs: 0,
  lastPlayStartBeats: null,
  beatIntervalMs: 500,
  loopStartBeats: 0, loopEndBeats: 16, loopRegion: false, loopEndPinned: false,
  beatsDisplayed: 256,
  loopEnabled: false,
  songEndMs: 0,
  beatsPerBar: 4,
  patternId: null,
  patternLength: 0,
  ...t,
})

describe('timelineEndMs', () => {
  it('rounds the last clip up to the next bar', () => {
    expect(timelineEndMs(transport({ songEndMs: 5_250 }))).toBe(6_000)
    expect(timelineEndMs(transport({ songEndMs: 100 }))).toBe(2_000)
  })

  it('keeps a song that ends on a bar, or a hair past one', () => {
    expect(timelineEndMs(transport({ songEndMs: 8_000 }))).toBe(8_000)
    expect(timelineEndMs(transport({ songEndMs: 8_000.5 }))).toBe(8_000)
  })

  it('is a few bars with no clips', () => {
    expect(timelineEndMs(transport())).toBe(8_000)
  })

  it('reaches past the song to show all of the loop', () => {
    const region = { loopRegion: true, loopStartBeats: 8, loopEndBeats: 40 }
    expect(timelineEndMs(transport({ songEndMs: 5_250, ...region }))).toBe(20_000)
    // even while not looping, as the region's still shown
    expect(timelineEndMs(transport({ songEndMs: 5_250, ...region, loopEnabled: false }))).toBe(20_000)
  })
})

describe('loopBand', () => {
  it('shows a region bright while looping, and faint while not', () => {
    const region = { loopRegion: true, loopStartBeats: 4, loopEndBeats: 12 }
    expect(loopBand(transport({ ...region, loopEnabled: true }))).toEqual({ start: 2_000, end: 6_000, bright: true })
    expect(loopBand(transport({ ...region, loopEnabled: false }))).toEqual({ start: 2_000, end: 6_000, bright: false })
  })

  it('with no region, shows the loop up to the end flag while looping', () => {
    expect(loopBand(transport({ loopEnabled: true, loopEndBeats: 8 }))).toEqual({ start: 0, end: 4_000, bright: true })
  })

  it('shows nothing with no region while not looping, as the playlist does', () => {
    expect(loopBand(transport({ loopEnabled: false, loopEndPinned: true, loopEndBeats: 8 }))).toBeNull()
  })
})

describe('msToX and xToMs', () => {
  it('go across the strip and back, kept on it', () => {
    expect(msToX(2_000, 8_000, 400)).toBe(100)
    expect(xToMs(100, 8_000, 400)).toBe(2_000)
    expect(msToX(9_000, 8_000, 400)).toBe(400)
    expect(xToMs(-20, 8_000, 400)).toBe(0)
    expect(xToMs(500, 8_000, 400)).toBe(8_000)
  })

  it('stay at the start with nothing to show', () => {
    expect(msToX(2_000, 0, 400)).toBe(0)
    expect(xToMs(100, 8_000, 0)).toBe(0)
  })
})

describe('barTicks', () => {
  it('ticks every bar between the ends', () => {
    expect(barTicks(transport(), 8_000, 800)).toEqual([2_000, 4_000, 6_000])
  })

  it('thins out to every 2nd, 4th… bar when bars get narrow', () => {
    // 64 bars across 200px: 3px a bar, so every 2nd (6px)
    const ticks = barTicks(transport(), 128_000, 200)
    expect(ticks[0]).toBe(4_000)
    expect(ticks[1] - ticks[0]).toBe(4_000)
    // 64 bars across 100px: every 4th
    expect(barTicks(transport(), 128_000, 100)[0]).toBe(8_000)
  })

  it('never puts ticks closer than the gap', () => {
    const width = 333
    const end = 300_000
    const ticks = barTicks(transport(), end, width)
    const gaps = ticks.slice(1).map((ms, i) => msToX(ms, end, width) - msToX(ticks[i], end, width))
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(MIN_TICK_GAP)
  })

  it('has none with nothing to show', () => {
    expect(barTicks(transport(), 0, 400)).toEqual([])
    expect(barTicks(transport(), 8_000, 0)).toEqual([])
  })
})

describe('readout', () => {
  it('gives the time and the bar and beat', () => {
    // 1:23.4 at 120bpm is beat 166 from 0: bar 42, beat 3
    expect(readout(83_400, transport())).toBe('1:23.4 · bar 42.3')
    expect(readout(0, transport())).toBe('0:00.0 · bar 1.1')
  })
})

describe('seekBeat', () => {
  const quarters = { enabled: true, note: 'QuarterNote' as const }

  it('snaps to the nearest snap line, as a click in the ruler does', () => {
    expect(seekBeat(1_200, transport(), quarters)).toBe(2)
    expect(seekBeat(1_300, transport(), quarters)).toBe(3)
    expect(seekBeat(1_300, transport(), { enabled: true, note: 'WholeNote' })).toBe(4)
  })

  it('lands exactly with snap off', () => {
    expect(seekBeat(1_300, transport(), { enabled: false, note: 'QuarterNote' })).toBe(2.6)
  })

  it('stays within the playlist', () => {
    expect(seekBeat(-500, transport(), quarters)).toBe(0)
    expect(seekBeat(200_000, transport({ beatsDisplayed: 64 }), quarters)).toBe(64)
  })
})
