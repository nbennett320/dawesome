import { decodeLevels } from './waveform'

describe('decodeLevels', () => {
  it('reads the levels the engine encodes', () => {
    // encode_levels(&[0., 1., 0.5]) in the engine's tests
    const levels = decodeLevels('AAD//wCA')
    expect(levels).toHaveLength(3)
    expect(levels[0]).toBe(0)
    expect(levels[1]).toBe(1)
    expect(levels[2]).toBeCloseTo(0.5, 4)
  })

  it('reads no levels', () => {
    expect(decodeLevels('')).toHaveLength(0)
  })
})
