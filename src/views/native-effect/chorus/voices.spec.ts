import { layoutsOf, panGains, Settings, SLOT, stereoOf, SWING, voicesAt } from './voices'

const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: false, rateHz: 1, voices: 2, depth: 0.4, shape: 0, delayMs: 10, spread: 0.7, toneHz: 9000, mix: 0.5,
  bloom: 0, drift: 0, prism: 0, ...change,
})

describe('the Chorus’s voices', () => {
  it('sit as the engine places them: evenly round the LFO, spaced, fanned by Spread', () => {
    const four = layoutsOf(4, 0.5, 0)
    expect(four.map(l => l.offset)).toEqual([0, 0.25, 0.5, 0.75])
    expect(four.map(l => l.scale)).toEqual([0.95, 1 - 0.1 / 6, 1 + 0.1 / 6, 1.05].map(x => expect.closeTo(x, 9)))
    expect(four[0].pan).toBeCloseTo(-0.5, 9)
    expect(four[3].pan).toBeCloseTo(0.5, 9)
    // Prism fans them all the way across
    expect(layoutsOf(4, 0.5, 1)[3].pan).toBeCloseTo(1, 9)
    expect(layoutsOf(1, 1, 1)).toEqual([{ offset: 0, pan: 0, scale: 1 }])
    expect(stereoOf(1, 1)).toBe(0.5)
    expect(stereoOf(3, 1)).toBe(0.25)
  })

  it('reads each voice’s delay from the LFO, as the engine does', () => {
    // one voice at the LFO's start (a sine's bottom): the delay, swung down
    const [one] = voicesAt(settings({ voices: 1, spread: 1 }), null)
    expect(one.delays[0]).toBeCloseTo(10 * (1 - SWING * 0.4), 9)
    // the right side half a cycle on: swung up
    expect(one.delays[1]).toBeCloseTo(10 * (1 + SWING * 0.4), 9)
  })

  it('takes gains, Drift and the LFO from a live readout', () => {
    const slots = Array.from({ length: 32 }, () => 0)
    slots[SLOT.GAINS] = 1
    slots[SLOT.GAINS + 1] = 0.5
    slots[SLOT.DRIFT_MS] = 2
    const live = { phase: 0.5, hz: 1, running: true, slots }
    const [a, b] = voicesAt(settings({ drift: 1, spread: 0 }), live)
    expect(b.gain).toBe(0.5)
    // at the top of the sine, 2 ms of Drift on
    expect(a.delays[0]).toBeCloseTo(10 * 0.95 + 2 + 10 * SWING * 0.4, 9)
    // (no Drift, its slots are ignored)
    expect(voicesAt(settings({ spread: 0 }), live)[0].delays[0]).toBeCloseTo(10 * 0.95 + 10 * SWING * 0.4, 9)
  })

  it('pans as the engine does: whole at the middle', () => {
    expect(panGains(0)).toEqual([1, 1])
    expect(panGains(1)).toEqual([0, Math.SQRT2])
  })
})
