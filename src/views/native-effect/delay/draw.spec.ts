import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { drawDelay, Scene } from './draw'
import { echoesOf, Settings } from './echoes'

// a 2D context that keeps what was asked of it
const fakeContext = () => {
  const calls: Array<[string, unknown[]]> = []
  const styles: string[] = []
  const ctx = new Proxy({} as Record<string, unknown>, {
    get: (_, name: string) => (...args: unknown[]) => { calls.push([name, args]) },
    set: (_, name: string, value) => {
      if(name === 'fillStyle' || name === 'strokeStyle') styles.push(String(value))
      return true
    },
  })
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls, styles }
}

const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: true, note: '1/4', timeMs: 250, offset: 0, pingPong: true, feedback: 0.5, mix: 50, ...change,
})

const scene = (s: Settings, change: Partial<Scene> = {}): Scene => ({
  area: { width: 600, height: 200 },
  span: 2,
  echoes: echoesOf(s, 120, 2),
  dryDb: 0,
  synced: s.sync,
  beat: 0.5,
  barQuarters: 4,
  caption: '1/4 · 500 ms at 120 bpm',
  hot: false,
  ...change,
})

const named = (calls: Array<[string, unknown[]]>, name: string) => calls.filter(([n]) => n === name)

beforeAll(() => applyTheme(slateLightTheme))

describe('drawing the Delay’s echoes', () => {
  it('draws the sound and a bar per echo, the caption, in the theme’s colors', () => {
    const { ctx, calls, styles } = fakeContext()
    const s = scene(settings())
    drawDelay(ctx, s)
    // the background, the sound, and the echoes (500 ms apart, to 2 s)
    expect(s.echoes).toHaveLength(4)
    expect(named(calls, 'fillRect')).toHaveLength(1 + 1 + 4)
    const texts = named(calls, 'fillText').map(([, args]) => args[0])
    expect(texts).toEqual(expect.arrayContaining(['1/4 · 500 ms at 120 bpm', 'L', 'R', '-12']))
    expect(styles.every(c => c.startsWith('rgba('))).toBe(true)
  })

  it('puts left echoes above the middle and right ones below', () => {
    const { ctx, calls } = fakeContext()
    drawDelay(ctx, scene(settings()))
    // (after the background and the sound) left, right, left, right
    const tops = named(calls, 'fillRect').slice(2).map(([, args]) => args[1] as number)
    expect(tops[0]).toBeLessThan(100)
    expect(tops[1]).toBe(100)
    expect(tops[2]).toBeLessThan(100)
  })

  it('numbers the bars when synced, and marks time when not', () => {
    const synced = fakeContext()
    drawDelay(synced.ctx, scene(settings()))
    const bars = named(synced.calls, 'fillText').map(([, args]) => args[0])
    expect(bars).toEqual(expect.arrayContaining(['1', '2']))
    const free = fakeContext()
    drawDelay(free.ctx, scene(settings({ sync: false })))
    expect(named(free.calls, 'fillText').map(([, args]) => args[0])).toEqual(expect.arrayContaining(['500 ms', '1 s', '1.5 s']))
  })

  it('numbers bars as long as the time signature makes them', () => {
    const at = (calls: Array<[string, unknown[]]>, label: string) =>
      named(calls, 'fillText').find(([, args]) => args[0] === label)?.[1][1] as number
    const four = fakeContext()
    drawDelay(four.ctx, scene(settings()))
    const three = fakeContext()
    drawDelay(three.ctx, scene(settings(), { barQuarters: 3 }))
    // bar 2 starts 4 quarters in, or 3 in 3/4 (2 s and 1.5 s across 2 s)
    expect(at(four.calls, '2')).toBeCloseTo(28 + 560 + 3.5)
    expect(at(three.calls, '2')).toBeCloseTo(28 + 420 + 3.5)
  })

  it('rings the first echoes only when they can be taken', () => {
    const still = fakeContext()
    drawDelay(still.ctx, scene(settings({ pingPong: false })))
    expect(named(still.calls, 'strokeRect')).toHaveLength(0)
    const hot = fakeContext()
    drawDelay(hot.ctx, scene(settings({ pingPong: false }), { hot: true }))
    // the first on each side
    expect(named(hot.calls, 'strokeRect')).toHaveLength(2)
  })

  it('draws no sound with nothing dry', () => {
    const { ctx, calls } = fakeContext()
    drawDelay(ctx, scene(settings({ mix: 100 }), { dryDb: -Infinity, echoes: [] }))
    expect(named(calls, 'fillRect')).toHaveLength(1)
  })
})
