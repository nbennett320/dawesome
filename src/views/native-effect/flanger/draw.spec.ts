import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { Settings, tapsAt } from './comb'
import { curveOf, dbToY, drawFlanger, freqToX, Scene } from './draw'

const fakeContext = () => {
  const calls: Array<[string, unknown[]]> = []
  const ctx = new Proxy({} as Record<string, unknown>, {
    get: (_, name: string) => (...args: unknown[]) => { calls.push([name, args]) },
    set: () => true,
  })
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls }
}

const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: false, rateHz: 0.2, depth: 0.7, shape: 0, manualMs: 2, feedback: 0.6, toneHz: 12000, zero: false, spread: 0, mix: 0.5,
  barber: 0, tune: 0, envelope: 0, ...change,
})
const area = { width: 600, height: 200 }

const scene = (s: Settings, change: Partial<Scene> = {}): Scene => {
  const { center, sides } = tapsAt(s, null)
  return { area, settings: s, left: curveOf(sides[0], s, center, area), right: null, trail: [], centerMs: center, crossing: 0, caption: 'comb', hot: false, ...change }
}

beforeAll(() => applyTheme(slateLightTheme))

describe('drawing the Flanger', () => {
  it('places frequencies by octave and levels by dB', () => {
    expect(freqToX(20, area)).toBe(16)
    expect(freqToX(20000, area)).toBe(584)
    expect(dbToY(24, area)).toBe(16)
    expect(dbToY(-100, area)).toBe(184)
  })

  it('draws a curve two points a pixel, its trail, and the right side when given', () => {
    const s = settings()
    const base = fakeContext()
    drawFlanger(base.ctx, scene(s))
    const strokes = base.calls.filter(([n]) => n === 'stroke').length
    const more = fakeContext()
    const curve = scene(s).left
    drawFlanger(more.ctx, scene(s, { trail: [curve, curve], right: curve }))
    expect(more.calls.filter(([n]) => n === 'stroke').length).toBe(strokes + 3)
    expect(curve.length).toBe(1201)
  })

  it('marks Tune’s harmonics', () => {
    const off = fakeContext()
    drawFlanger(off.ctx, scene(settings()))
    const on = fakeContext()
    drawFlanger(on.ctx, scene(settings({ tune: 10 })))
    expect(on.calls.filter(([n]) => n === 'setLineDash').length).toBeGreaterThan(off.calls.filter(([n]) => n === 'setLineDash').length)
  })
})
