import {
  captionOf, echoesOf, FLOOR_DB, levels, MAX_SPAN, nearestNote, noteQuarters, noteSeconds, Settings, sideTimes, spanOf,
} from './echoes'

// the engine's note values (engine/crates/dsp/src/effects/delay.rs)
const NOTES = [
  '1/32 T', '1/32', '1/16 T', '1/32 D', '1/16', '1/8 T', '1/16 D', '1/8', '1/4 T',
  '1/8 D', '1/4', '1/2 T', '1/4 D', '1/2', '1/1 T', '1/2 D', '1/1', '1/1 D',
]

const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: false, note: '1/4 D', timeMs: 250, offset: 0, pingPong: false, feedback: 0.5, mix: 50, ...change,
})

describe('the Delay’s echoes', () => {
  it('reads a note value’s length from its label', () => {
    expect(noteQuarters('1/4')).toBe(1)
    expect(noteQuarters('1/8 D')).toBe(0.75)
    expect(noteQuarters('1/4 T')).toBeCloseTo(2 / 3)
    expect(noteQuarters('1/1 D')).toBe(6)
    // every one of the engine's, shortest first
    const lengths = NOTES.map(noteQuarters)
    expect(lengths.every((q, i) => i === 0 || q > lengths[i - 1])).toBe(true)
    // a quarter at 120 bpm is half a second
    expect(noteSeconds('1/4', 120)).toBe(0.5)
  })

  it('gives each side its time, offset lengthening one, held at 4 s', () => {
    expect(sideTimes(settings(), 120)).toEqual([0.25, 0.25])
    expect(sideTimes(settings({ offset: 20 }), 120)).toEqual([0.25, 0.3])
    expect(sideTimes(settings({ offset: -50 }), 120)).toEqual([0.375, 0.25])
    expect(sideTimes(settings({ sync: true, note: '1/4 D' }), 120)).toEqual([0.75, 0.75])
    expect(sideTimes(settings({ sync: true, note: '1/1 D' }), 40)).toEqual([4, 4])
  })

  it('mixes as the engine does: the dry whole to the middle', () => {
    expect(levels(30)).toEqual({ dry: 1, wet: 0.6 })
    expect(levels(75)).toEqual({ dry: 0.5, wet: 1 })
  })

  it('echoes each side on its own in Stereo, falling by the feedback', () => {
    const echoes = echoesOf(settings({ offset: 20 }), 120, 1)
    const left = echoes.filter(e => e.side === 0)
    expect(left.map(e => e.time)).toEqual([0.25, 0.5, 0.75, 1])
    expect(left.map(e => Math.round(e.db))).toEqual([0, -6, -12, -18])
    expect(echoes.filter(e => e.side === 1).map(e => Number(e.time.toFixed(6)))).toEqual([0.3, 0.6, 0.9])
    expect(echoes.filter(e => e.first).map(e => e.time)).toEqual([0.25, 0.3])
    // in time order
    expect(echoes.every((e, i) => i === 0 || e.time >= echoes[i - 1].time)).toBe(true)
  })

  it('bounces from left to right in Ping-pong, each side’s own time apart', () => {
    const echoes = echoesOf(settings({ pingPong: true, offset: 20 }), 120, 1.2)
    expect(echoes.map(e => [e.side, Number(e.time.toFixed(3))])).toEqual([[0, 0.25], [1, 0.55], [0, 0.8], [1, 1.1]])
    expect(echoes.filter(e => e.first)).toHaveLength(1)
  })

  it('stops at the floor, and has none with nothing wet', () => {
    const echoes = echoesOf(settings({ feedback: 0.5 }), 120, 100)
    expect(echoes.every(e => e.db >= FLOOR_DB)).toBe(true)
    expect(echoes.filter(e => e.side === 0)).toHaveLength(8)
    expect(echoesOf(settings({ mix: 0 }), 120, 100)).toEqual([])
  })

  it('shows until the echoes fade, at least four of the longer time, at most 12 s', () => {
    // the 8th echo (2 s) is the last above the floor
    expect(spanOf(settings(), 120)).toBeCloseTo(2.2)
    expect(spanOf(settings({ feedback: 0 }), 120)).toBe(1)
    expect(spanOf(settings({ timeMs: 2000, feedback: 0.95 }), 120)).toBe(MAX_SPAN)
  })

  it('finds the note value nearest a time', () => {
    expect(NOTES[nearestNote(NOTES, 0.5, 120)]).toBe('1/4')
    expect(NOTES[nearestNote(NOTES, 0.74, 120)]).toBe('1/4 D')
    expect(NOTES[nearestNote(NOTES, 100, 120)]).toBe('1/1 D')
  })

  it('says what the time is', () => {
    expect(captionOf(settings({ sync: true, offset: 15 }), 120)).toBe('1/4 D · 750 ms at 120 bpm · R +15%')
    expect(captionOf(settings({ timeMs: 1250, offset: -5 }), 120)).toBe('1.25 s · L +5%')
    expect(captionOf(settings(), 120)).toBe('250 ms')
  })
})
