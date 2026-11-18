import { barberChain, gainAt, layoutOf, Settings, sidesAt, SLOT, stageFreq, stagePhase, SWEEP, VIBE_SPREAD } from './notches'

const RATE = 48000
const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: false, rateHz: 1, depth: 0, shape: 0, centerHz: 700, stages: 2, feedback: 0, vibe: false, spread: 0, mix: 0.5,
  melt: 0, barber: 0, envelope: 0, ...change,
})

describe('the Phaser’s notches', () => {
  it('turns a stage’s own frequency a quarter cycle', () => {
    expect(stagePhase(700, 700, RATE)).toBeCloseTo(-Math.PI / 2, 6)
    expect(stagePhase(1, 700, RATE)).toBeCloseTo(0, 2)
  })

  it('places stages as the engine does, an absent one at the top', () => {
    expect(stageFreq(Math.log2(700), 0, 1, RATE)).toBeCloseTo(700, 6)
    expect(stageFreq(Math.log2(700), 1, 1, RATE)).toBeCloseTo(1400, 6)
    expect(stageFreq(Math.log2(700), 0, 0, RATE)).toBeCloseTo(0.49 * RATE, 3)
  })

  it('cuts a notch where two stages turn it half a cycle', () => {
    const s = settings()
    const [side] = sidesAt(s, null, RATE)
    expect(gainAt(700, side, s, RATE)).toBeLessThan(1e-6)
    expect(gainAt(40, side, s, RATE)).toBeGreaterThan(0.95)
  })

  it('sweeps two and a half octaves either way of the centre', () => {
    const s = settings({ depth: 1 })
    const at = (phase: number) => sidesAt(s, { phase, hz: 1, running: true, slots: Array.from({ length: 32 }, (_, i) => (i >= SLOT.PRESENT && i < SLOT.PRESENT + 2 ? 1 : 0)) }, RATE)[0].sweepHz
    expect(at(0)).toBeCloseTo(700 / 2 ** SWEEP, 6)
    expect(at(0.5)).toBeCloseTo(700 * 2 ** SWEEP, 6)
  })

  it('lays out Vibe’s four stages, the rest out', () => {
    const { offsets, present } = layoutOf(settings({ vibe: true, stages: 12 }), null)
    expect(offsets.slice(0, 4)).toEqual(VIBE_SPREAD)
    expect(present).toEqual([1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0])
  })

  it('has Barberpole’s two chains adding up the same wherever they are', () => {
    for(let i = 0; i < 100; i += 1) {
      const u = i / 100
      expect(barberChain(u, 1, true)[1] + barberChain((u + 0.5) % 1, 1, true)[1]).toBeCloseTo(1, 6)
    }
    const [side] = sidesAt(settings({ barber: -0.6 }), { phase: 0.1, hz: 1, running: true, slots: Array.from({ length: 32 }, () => 0) }, RATE)
    expect(side.chains.length).toBe(3)
  })

  it('rings between the notches with feedback, trimmed as the engine trims it', () => {
    const s = settings({ feedback: 0.9, mix: 1 })
    const [side] = sidesAt(s, null, RATE)
    // where the chain turns a whole cycle (or none): the peak
    expect(gainAt(1, side, s, RATE)).toBeCloseTo((1 - 0.45) / (1 - 0.9), 1)
  })
})
