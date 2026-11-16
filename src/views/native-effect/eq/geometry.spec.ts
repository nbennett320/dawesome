import {
  dbToY, dotOf, freeBand, freqToX, hitBand, HIT, newBandShape, PAD, xToFreq, xToFreqNormalized, yToDb,
  yToGainNormalized,
} from './geometry'
import { Band, Shape } from './response'

const area = { width: 800, height: 400 }

const band = (index: number, change: Partial<Band> = {}): Band => ({
  index, on: true, shape: Shape.Bell, freq: 1000, gain: 0, q: 0.707, ...change,
})

describe('the EQ’s curve area', () => {
  it('spreads frequency by octave, as the freq parameter does', () => {
    expect(freqToX(20, area)).toBeCloseTo(0)
    expect(freqToX(20000, area)).toBeCloseTo(800)
    expect(xToFreq(400, area)).toBeCloseTo(632.46, 1)
    expect(xToFreqNormalized(400, area)).toBe(0.5)
    expect(xToFreqNormalized(-50, area)).toBe(0)
  })

  it('spreads gain evenly between ±24 dB, inside a margin', () => {
    expect(dbToY(24, area)).toBe(PAD)
    expect(dbToY(-24, area)).toBe(400 - PAD)
    expect(dbToY(0, area)).toBe(200)
    expect(yToDb(200, area)).toBeCloseTo(0)
    expect(yToGainNormalized(PAD, area)).toBe(1)
    expect(yToGainNormalized(999, area)).toBe(0)
  })

  it('puts a dot at its gain, or on 0 dB for shapes without one', () => {
    expect(dotOf(band(0, { gain: 12 }), area).y).toBe(dbToY(12, area))
    expect(dotOf(band(0, { gain: 12, shape: Shape.LowCut }), area).y).toBe(200)
  })

  it('takes the nearest dot that’s on, within reach', () => {
    const bands = [band(0, { freq: 1000 }), band(1, { freq: 1000, on: false }), band(2, { freq: 1100 })]
    const x = freqToX(1000, area)
    expect(hitBand(bands, x + 1, 200, area, null)).toBe(0)
    expect(hitBand(bands, freqToX(1100, area) - 1, 200, area, null)).toBe(2)
    expect(hitBand(bands, x + HIT + 30, 260, area, null)).toBeNull()
    // two on the same spot: the selected one
    const same = [band(0), band(3)]
    expect(hitBand(same, x, 200, area, 3)).toBe(3)
  })

  it('makes a new band a cut at either end, and a bell between', () => {
    expect(newBandShape(30)).toBe(Shape.LowCut)
    expect(newBandShape(500)).toBe(Shape.Bell)
    expect(newBandShape(17000)).toBe(Shape.HighCut)
  })

  it('finds the first band that’s free', () => {
    expect(freeBand([band(0), band(1, { on: false }), band(2, { on: false })])).toBe(1)
    expect(freeBand([band(0), band(1)])).toBeNull()
  })
})
