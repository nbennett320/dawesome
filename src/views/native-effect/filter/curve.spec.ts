import { captionOf, formantsAt, freqAt, freqsAt, gainAt, msAt, qOf, Settings, SLOT, snap, tuneNotes, VOWELS } from './curve'

const settings = (change: Partial<Settings> = {}): Settings => ({
  type: 0, steep: false, cutoff: 1000, resonance: 0, drive: 0, mode: 0, floor: 200, sync: false, rateHz: 1, shape: 0,
  mix: 1, outputDb: 0, vowel: 0, spin: 0, tune: 0, ...change,
})
const live = (phase: number, m = 0) => ({ phase, hz: 1, running: true, slots: Array.from({ length: 32 }, (_, i) => (i === SLOT.M_L ? m : 0)) })
const db = (x: number) => 20 * Math.log10(x)

describe('the Filter’s curve', () => {
  it('maps resonance and the sweep as the engine does', () => {
    expect(qOf(0)).toBeCloseTo(0.707, 6)
    expect(qOf(100)).toBeCloseTo(25, 6)
    expect(freqAt(0, 200, 3200)).toBeCloseTo(200, 6)
    expect(freqAt(0.5, 200, 3200)).toBeCloseTo(800, 6)
    expect(freqAt(1, 3200, 200)).toBeCloseTo(200, 6)
  })

  it('sits at the cutoff while off, and runs the LFO on from its trough', () => {
    expect(freqsAt(settings(), live(0.3))).toEqual([1000, 1000].map(x => expect.closeTo(x, 6)))
    expect(msAt(settings({ mode: 1 }), live(0))[0]).toBeCloseTo(0, 9)
    expect(msAt(settings({ mode: 1 }), live(0.5))[0]).toBeCloseTo(1, 9)
    // square: low then high
    expect(msAt(settings({ mode: 1, shape: 4 }), live(0.6))[0]).toBe(1)
    // S&H, the envelope and the follower are read from the engine
    expect(msAt(settings({ mode: 1, shape: 5 }), live(0.6, 0.42))[0]).toBe(0.42)
    expect(msAt(settings({ mode: 3 }), live(0.6, 0.7))[0]).toBe(0.7)
    // Spin turns the right side the other way
    expect(msAt(settings({ mode: 3, spin: 1 }), live(0, 0.7))[1]).toBeCloseTo(0.3, 9)
  })

  it('steps to notes as Tune says', () => {
    expect(snap(450, 1)).toBeCloseTo(440, 6)
    // C major has no F#: G or F
    const fSharp = 440 * 2 ** (-3 / 12)
    expect(snap(fSharp * 1.001, 2)).toBeCloseTo(440 * 2 ** (-2 / 12), 4)
    expect(tuneNotes(2, 260, 525).length).toBe(8)
    expect(tuneNotes(0, 20, 20000)).toEqual([])
  })

  it('cuts as the engine’s filters do', () => {
    expect(db(gainAt(1000, 1000, settings()))).toBeCloseTo(-3, 0)
    expect(db(gainAt(4000, 1000, settings()))).toBeCloseTo(-24, 0)
    expect(db(gainAt(4000, 1000, settings({ steep: true })))).toBeLessThan(-45)
    expect(db(gainAt(1000, 1000, settings({ type: 2 })))).toBeCloseTo(0, 1)
    // half wet: the dry shows through
    expect(gainAt(20000, 1000, settings({ mix: 0.5 }))).toBeGreaterThan(0.49)
  })

  it('moves through the vowels low to high', () => {
    expect(formantsAt(10).map(f => Math.round(f))).toEqual(VOWELS[0])
    expect(formantsAt(20000).map(f => Math.round(f))).toEqual(VOWELS[4])
  })

  it('says what it is in its caption', () => {
    expect(captionOf(settings(), '1/4')).toBe('Low pass 12 dB · 1.00 kHz · Q 0.7')
    expect(captionOf(settings({ mode: 1, sync: true }), '1/4')).toBe('Low pass 12 dB · 1.00 kHz · Q 0.7 · LFO from 200 Hz at 1/4')
  })
})
