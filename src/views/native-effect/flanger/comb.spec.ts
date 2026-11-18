import { barberCopy, BARBER_COPIES, crossingOf, gainAt, noteHz, Settings, SLOT, SWEEP, tapsAt, tuned } from './comb'

const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: false, rateHz: 1, depth: 0, shape: 0, manualMs: 1, feedback: 0, toneHz: 20000, zero: false, spread: 0, mix: 0.5,
  barber: 0, tune: 0, envelope: 0, ...change,
})

const live = (phase: number, slots: Record<number, number> = {}) => {
  const all = Array.from({ length: 32 }, () => 0)
  Object.entries(slots).forEach(([i, v]) => { all[Number(i)] = v })
  return { phase, hz: 1, running: true, slots: all }
}

describe('the Flanger’s comb', () => {
  it('tunes as the engine does: the nearest period of the note to Manual', () => {
    expect(noteHz(10)).toBe(440)
    expect(tuned(10, 2) * 2).toBeCloseTo(1000 / 440, 6)
    expect(tuned(10, 0.5) * 0.5).toBeCloseTo(1000 / 1760, 6)
    expect(tuned(0, 3.7)).toBe(1)
  })

  it('sweeps two octaves either way of the centre', () => {
    const s = settings({ depth: 1, manualMs: 2 })
    expect(tapsAt(s, live(0)).sides[0].delayMs).toBeCloseTo(2 / 2 ** SWEEP, 9)
    expect(tapsAt(s, live(0.5)).sides[0].delayMs).toBeCloseTo(2 * 2 ** SWEEP, 9)
    // the engine's centre, when it says (Tune's)
    expect(tapsAt(settings(), live(0, { [SLOT.CENTER]: 2.27 })).center).toBe(2.27)
  })

  it('cancels half a cycle out and doubles a whole one, with no feedback', () => {
    const s = settings()
    const { center, sides } = tapsAt(s, null)
    expect(gainAt(500, sides[0], s, center)).toBeLessThan(1e-6)
    expect(gainAt(1000, sides[0], s, center)).toBeCloseTo(1, 6)
  })

  it('rings at the harmonics with feedback, the odd ones turned over', () => {
    // (a tone filter that passes everything: even at 20 kHz it nudges the
    // loop's phase, and the peaks sit a touch lower)
    const up = settings({ feedback: 0.9, mix: 1, toneHz: 1e9 })
    const { center, sides } = tapsAt(up, null)
    expect(gainAt(1000, sides[0], up, center)).toBeCloseTo(1 / (1 - 0.9), 3)
    const down = settings({ feedback: -0.9, mix: 1, toneHz: 1e9 })
    expect(gainAt(500, sides[0], down, center)).toBeCloseTo(1 / (1 - 0.9), 3)
  })

  it('cancels outright through zero where the copy meets the dry', () => {
    const s = settings({ zero: true })
    const { center, sides } = tapsAt(s, null)
    // no sweep: the copy's as late as the dry, and turned over
    ;[100, 1000, 7000].forEach(hz => expect(gainAt(hz, sides[0], s, center)).toBeLessThan(1e-6))
    expect(crossingOf(1, 1)).toBe(1)
    expect(crossingOf(2, 1)).toBeLessThan(0.01)
  })

  it('has Barberpole’s copies adding up the same wherever they are', () => {
    for(let i = 0; i < 100; i += 1) {
      const u = i / 100
      const total = Array.from({ length: BARBER_COPIES }, (_, k) => barberCopy((u + k / 3) % 1, 1, true)[1]).reduce((a, b) => a + b)
      expect(total).toBeCloseTo(1.5, 6)
    }
    const { sides } = tapsAt(settings({ barber: 0.5, depth: 0.5 }), live(0.2))
    expect(sides[0].taps.length).toBe(1 + BARBER_COPIES)
    expect(sides[0].taps.reduce((sum, t) => sum + t.level, 0)).toBeCloseTo(1, 6)
  })
})
