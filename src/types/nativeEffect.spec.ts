import { choiceNormalized, groupParams, normalizedOf, paramKind, plainOf } from './nativeEffect'
import { param } from './nativeEffect.fixture'

const freq = param(0, { unit: 'Hz', min: 20, max: 20000, taper: 'log' })
const gain = param(1, { unit: 'dB', min: -24, max: 24 })
const shape = param(2, { min: 0, max: 6, stepCount: 6, labels: ['a', 'b', 'c', 'd', 'e', 'f', 'g'] })

describe('a parameter’s values', () => {
  it('spreads a log range evenly by ratio, as the engine does', () => {
    expect(plainOf(freq, 0)).toBeCloseTo(20)
    expect(plainOf(freq, 1)).toBeCloseTo(20000)
    // the geometric middle
    expect(plainOf(freq, 0.5)).toBeCloseTo(632.46, 1)
    expect(normalizedOf(freq, 632.46)).toBeCloseTo(0.5)
  })

  it('spreads a linear range evenly', () => {
    expect(plainOf(gain, 0.5)).toBe(0)
    expect(normalizedOf(gain, 12)).toBe(0.75)
    // out of range is brought in
    expect(normalizedOf(gain, 99)).toBe(1)
    expect(plainOf(gain, -1)).toBe(-24)
  })

  it('snaps steps to whole values', () => {
    expect(plainOf(shape, 0.4)).toBe(2)
    expect(choiceNormalized(shape, 3)).toBe(0.5)
    expect(choiceNormalized(shape, 99)).toBe(1)
  })

  it('knows what kind each is', () => {
    expect(paramKind(gain)).toBe('continuous')
    expect(paramKind(param(3, { stepCount: 1 }))).toBe('switch')
    expect(paramKind(shape)).toBe('choice')
  })
})

describe('groups', () => {
  it('keeps the order each group first comes in', () => {
    const params = [
      param(0, { group: 'Band 1' }),
      param(1, { group: 'Band 2' }),
      param(2, { group: 'Band 1' }),
      param(3, { group: 'Output' }),
    ]
    expect(groupParams(params).map(g => [g.name, g.params.map(p => p.id)])).toEqual([
      ['Band 1', [0, 2]],
      ['Band 2', [1]],
      ['Output', [3]],
    ])
  })
})
