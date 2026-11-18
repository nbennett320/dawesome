import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { css, lamp, liveAt, sine, spectrumAt, triangle } from './motion'

const reading = (phase: number, hz: number, running: boolean, at = 1000) =>
  ({ readout: { written: 1, rate: 48000, slots: [phase, hz] }, at, running })

describe('the modulation views’ LFO', () => {
  it('has the engine’s shapes: bottom at 0, top halfway (the lamp sooner)', () => {
    [sine, triangle, lamp].forEach(shape => {
      expect(shape(0)).toBeCloseTo(-1, 6)
      expect(shape(1)).toBeCloseTo(-1, 6)
    })
    expect(sine(0.5)).toBeCloseTo(1, 6)
    expect(triangle(0.5)).toBeCloseTo(1, 6)
    expect(triangle(0.25)).toBeCloseTo(0, 6)
    expect(lamp(0.3)).toBeCloseTo(1, 6)
  })

  it('runs on from a reading at its rate while the effect runs, and holds while it doesn’t', () => {
    expect(liveAt(null, 0)).toBeNull()
    // 2 Hz, 100 ms on: a fifth of a cycle
    expect(liveAt(reading(0.1, 2, true), 1100)!.phase).toBeCloseTo(0.3, 6)
    expect(liveAt(reading(0.9, 2, true), 1100)!.phase).toBeCloseTo(0.1, 6)
    expect(liveAt(reading(0.1, 2, false), 1100)!.phase).toBeCloseTo(0.1, 6)
    // a stalled poll doesn't spin it on for ever
    expect(liveAt(reading(0, 1, true), 1000 + 10_000)!.phase).toBeCloseTo(0.25, 6)
  })

  it('takes a spectrum from the theme’s track colours', () => {
    applyTheme(slateLightTheme)
    const red = spectrumAt(0)
    expect(css(red)).toMatch(/^rgba\(/)
    expect(spectrumAt(1)).toEqual(red)
    expect(spectrumAt(0.5)).not.toEqual(red)
  })
})
