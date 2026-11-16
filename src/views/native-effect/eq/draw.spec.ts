import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { drawEq } from './draw'
import { Band, Shape } from './response'

// a 2D context that keeps what was asked of it
const fakeContext = () => {
  const calls: Array<[string, unknown[]]> = []
  const styles: string[] = []
  const ctx = new Proxy({} as Record<string, unknown>, {
    get: (_, name: string) => {
      if(name === 'measureText') return (text: string) => ({ width: text.length * 6 })
      return (...args: unknown[]) => { calls.push([name, args]) }
    },
    set: (_, name: string, value) => {
      if(name === 'fillStyle' || name === 'strokeStyle') styles.push(String(value))
      return true
    },
  })
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls, styles }
}

const band = (index: number, change: Partial<Band> = {}): Band => ({
  index, on: false, shape: Shape.Bell, freq: 1000, gain: 0, q: 0.707, ...change,
})

beforeAll(() => applyTheme(slateLightTheme))

describe('drawing the EQ', () => {
  it('draws the grid, the curve, a dot per band that’s on, and the selected band’s caption, in the theme’s colors', () => {
    const { ctx, calls, styles } = fakeContext()
    const bands = Array.from({ length: 8 }, (_, i) => band(i))
    bands[0] = band(0, { on: true, shape: Shape.LowCut, freq: 80 })
    bands[3] = band(3, { on: true, freq: 2500, gain: 4.5 })
    drawEq(ctx, { area: { width: 800, height: 400 }, bands, outputDb: 0, selected: 3, caption: 'Bell · 2.50 kHz' })

    expect(calls.filter(([name]) => name === 'arc')).toHaveLength(2)
    const texts = calls.filter(([name]) => name === 'fillText').map(([, args]) => args[0])
    expect(texts).toEqual(expect.arrayContaining(['1k', '+6', '-18', '1', '4', 'Bell · 2.50 kHz']))
    // the selected band's own curve is dashed
    expect(calls.some(([name, args]) => name === 'setLineDash' && (args[0] as number[]).length > 0)).toBe(true)
    // every color is from the theme
    expect(styles.every(s => s.startsWith('rgba('))).toBe(true)
  })

  it('draws no caption or dashed curve with nothing selected', () => {
    const { ctx, calls } = fakeContext()
    const bands = Array.from({ length: 8 }, (_, i) => band(i))
    drawEq(ctx, { area: { width: 300, height: 150 }, bands, outputDb: -6, selected: null, caption: null })
    expect(calls.some(([name, args]) => name === 'setLineDash' && (args[0] as number[]).length > 0)).toBe(false)
    expect(calls.filter(([name]) => name === 'arc')).toHaveLength(0)
  })
})
