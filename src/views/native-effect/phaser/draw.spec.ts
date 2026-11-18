import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { angleOf, drawPhaser, reachOf, ringOf, Scene } from './draw'
import { Settings, sidesAt } from './notches'

const fakeContext = () => {
  const calls: Array<[string, unknown[]]> = []
  const ctx = new Proxy({} as Record<string, unknown>, {
    get: (_, name: string) => (...args: unknown[]) => { calls.push([name, args]) },
    set: () => true,
  })
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls }
}

const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: false, rateHz: 0.5, depth: 0.8, shape: 0, centerHz: 700, stages: 6, feedback: 0.5, vibe: false, spread: 0.5, mix: 0.5,
  melt: 0, barber: 0, envelope: 0, ...change,
})
const area = { width: 400, height: 300 }

const scene = (s: Settings, change: Partial<Scene> = {}): Scene => {
  const [l] = sidesAt(s, null, 48000)
  return { area, settings: s, left: ringOf(l, s, 48000, area), right: null, trail: [], phase: 0, glow: 0, caption: 'swirl', hot: false, ...change }
}

beforeAll(() => applyTheme(slateLightTheme))

describe('drawing the Phaser', () => {
  it('wraps frequency round from the top, and level outward', () => {
    expect(angleOf(20)).toBe(0)
    expect(angleOf(20000)).toBeCloseTo(2 * Math.PI, 9)
    expect(reachOf(-100)).toBe(0.25)
    expect(reachOf(18)).toBe(1)
  })

  it('draws the ring, its trail, and the right side when given', () => {
    const s = settings()
    const base = fakeContext()
    drawPhaser(base.ctx, scene(s))
    const strokes = base.calls.filter(([n]) => n === 'stroke').length
    const ring = scene(s).left
    const more = fakeContext()
    drawPhaser(more.ctx, scene(s, { trail: [ring, ring], right: ring }))
    expect(more.calls.filter(([n]) => n === 'stroke').length).toBe(strokes + 3)
  })

  it('lights the lamp in Vibe mode', () => {
    const dark = fakeContext()
    drawPhaser(dark.ctx, scene(settings()))
    const lit = fakeContext()
    drawPhaser(lit.ctx, scene(settings({ vibe: true }), { glow: 1 }))
    expect(lit.calls.filter(([n]) => n === 'arc').length).toBe(dark.calls.filter(([n]) => n === 'arc').length + 4)
  })
})
