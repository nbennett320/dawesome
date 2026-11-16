import fixture from '../../../../engine/crates/dsp/fixtures/eq-response.json'
import { bandResponse, bandsOf, hasGain, OUTPUT, paramId, Shape, toDb, totalDb, FREQ, GAIN, ON } from './response'
import { param } from '../../../types/nativeEffect.fixture'

describe('the EQ’s curve', () => {
  it('is what the engine’s filters give (its fixture)', () => {
    fixture.cases.forEach(c => {
      fixture.freqs.forEach((f, i) => {
        const db = toDb(bandResponse({ shape: c.shape as Shape, freq: c.freq, gain: c.gain, q: c.q }, f, fixture.rate))
        expect(Math.abs(db - c.db[i]), `shape ${c.shape} at ${c.freq} Hz, heard at ${f} Hz`).toBeLessThan(0.01)
      })
    })
  })

  it('knows which shapes have a gain', () => {
    expect([Shape.Bell, Shape.LowShelf, Shape.HighShelf].every(hasGain)).toBe(true)
    expect([Shape.LowCut, Shape.HighCut, Shape.Notch, Shape.BandPass].some(hasGain)).toBe(false)
  })

  it('reads the bands from the parameters, and adds up those that are on', () => {
    const params = Array.from({ length: OUTPUT + 1 }, (_, id) => param(id))
    const set = (id: number, value: number) => { params[id] = param(id, { value: { id, value, normalized: 0, text: '' } }) }
    set(paramId(2, ON), 1)
    set(paramId(2, FREQ), 1000)
    set(paramId(2, GAIN), 6)
    set(paramId(2, 4), 0.707)
    // band 5 is set, but off
    set(paramId(5, FREQ), 1000)
    set(paramId(5, GAIN), 12)
    const bands = bandsOf(params)
    expect(bands).toHaveLength(8)
    expect(bands[2]).toMatchObject({ index: 2, on: true, shape: Shape.Bell, freq: 1000, gain: 6 })
    expect(bands[5].on).toBe(false)
    expect(totalDb(bands, 0, 1000)).toBeCloseTo(6, 2)
    expect(totalDb(bands, -3, 1000)).toBeCloseTo(3, 2)
  })
})
