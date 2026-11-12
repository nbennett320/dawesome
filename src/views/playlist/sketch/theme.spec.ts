import { patternColors } from './theme'

describe('a pattern clip’s colors', () => {
  const red = [255, 0, 0, 255] as const
  const white = [255, 255, 255, 255] as const
  const black = [0, 0, 0, 255] as const

  it('are its track’s color: a pale body, and stronger label, edge and hits', () => {
    const c = patternColors(red, white, black)
    // red's share: green and blue drop as red grows
    const share = (rgba: readonly number[]) => 1 - rgba[1] / 255
    expect(share(c.card)).toBeCloseTo(0.1, 1)
    expect(share(c.label)).toBeGreaterThan(share(c.card))
    expect(share(c.border)).toBeGreaterThan(share(c.label))
    expect(c.card[0]).toBe(255)
    // the hits are darkened towards the ink
    expect(c.hit[0]).toBeLessThan(255)
    expect(c.hit[1]).toBe(0)
  })
})
