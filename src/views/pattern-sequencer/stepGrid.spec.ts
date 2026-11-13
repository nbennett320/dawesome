import { describe, expect, it } from 'vitest'
import {
  barX, beatX, gridWidth, stepX, timeX, visibleBars,
} from './stepGrid'

describe('the step grid', () => {
  it('places keys 25px apart in a beat, beats 6px apart, bars 12px', () => {
    expect([0, 1, 3].map(step => stepX(step, 4))).toEqual([0, 25, 75])
    // a beat is 97px of keys
    expect(beatX(1, 4)).toBe(103)
    expect(beatX(4, 4)).toBe(4 * 103 + 6)
    expect(barX(2, 4)).toBe(2 * (4 * 103 + 6))
    expect(stepX(16, 4)).toBe(barX(1, 4))
    // in 3/4
    expect(barX(1, 3)).toBe(3 * 103 + 6)
  })

  it('is as wide as its keys, from the first’s left to the last’s right', () => {
    expect(gridWidth(0, 4)).toBe(0)
    expect(gridWidth(4, 4)).toBe(97)
    expect(gridWidth(16, 4)).toBe(barX(1, 4) - 12)
  })

  it('draws a note across the keys it plays over', () => {
    expect(timeX(0, 4)).toBe(0)
    expect(timeX(1, 4, true)).toBe(22)
    expect(timeX(0.5, 4)).toBe(11)
    // a beat's notes end at its last key, not in the gap after it
    expect(timeX(4, 4, true)).toBe(97)
    expect(timeX(4, 4)).toBe(103)
  })

  it('finds the bars in view, and one more either side', () => {
    const bar = barX(1, 4)
    expect(visibleBars(0, 1200, 4, 128)).toEqual({ from: 0, to: 4 })
    expect(visibleBars(50 * bar, 1200, 4, 128)).toEqual({ from: 49, to: 54 })
    expect(visibleBars(-336, 1200, 4, 2)).toEqual({ from: 0, to: 2 })
    expect(visibleBars(127 * bar, 1200, 4, 128)).toEqual({ from: 126, to: 128 })
  })
})
