import { LIP, fitName, inBox, lipBox, lipWidth } from './lip'

describe('a lane’s lip', () => {
  it('is pinned to the top right, inside its own lane', () => {
    const box = lipBox(124, 100, 800, 60, 24)
    expect(box.x + box.w).toBe(800 - LIP.margin)
    expect(box.w).toBe(60)
    expect(box.y).toBe(124 + LIP.inset)
  })

  it('sticks under the ruler as its lane scrolls up, until the lane pushes it out', () => {
    // the lane's top is 30px up under the ruler: the label stays below it
    expect(lipBox(-6, 100, 800, 60, 24).y).toBe(24 + LIP.inset)
    // the lane's bottom is nearly under the ruler: the label rides up with it
    const box = lipBox(-60, 100, 800, 60, 24)
    expect(box.y + box.h).toBe(40 - LIP.inset)
    expect(box.y).toBeLessThan(24 + LIP.inset)
  })

  it('fits the name, cutting a long one short', () => {
    // a character a pixel
    const ctx = { measureText: (text: string) => ({ width: text.length }) as TextMetrics }
    expect(fitName(ctx, 'Track 1')).toBe('Track 1')
    const long = 'x'.repeat(LIP.maxText + 20)
    const fitted = fitName(ctx, long)
    expect(fitted.endsWith('…')).toBe(true)
    expect(fitted.length).toBeLessThanOrEqual(LIP.maxText)
    expect(lipWidth(1000)).toBe(lipWidth(LIP.maxText))
  })

  it('is hit inside, and not a pixel beside it', () => {
    const box = lipBox(124, 100, 800, 60, 24)
    expect(inBox(box, box.x + 1, box.y + 1)).toBe(true)
    expect(inBox(box, box.x + box.w / 2, box.y + box.h / 2)).toBe(true)
    expect(inBox(box, box.x - 1, box.y + 1)).toBe(false)
    expect(inBox(box, box.x + 1, box.y + box.h + 1)).toBe(false)
  })
})
