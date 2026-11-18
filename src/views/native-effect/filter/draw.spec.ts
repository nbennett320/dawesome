import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { Settings } from './curve'
import { curveOf, drawFilter, freqToX, Scene, xToFreq } from './draw'

const fakeContext = () => {
  const calls: Array<[string, unknown[]]> = []
  const ctx = new Proxy({} as Record<string, unknown>, {
    get: (_, name: string) => (...args: unknown[]) => { calls.push([name, args]) },
    set: () => true,
  })
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls }
}

const settings = (change: Partial<Settings> = {}): Settings => ({
  type: 0, steep: true, cutoff: 2000, resonance: 20, drive: 0, mode: 0, floor: 200, sync: true, rateHz: 1, shape: 0,
  mix: 1, outputDb: 0, vowel: 0, spin: 0, tune: 0, ...change,
})
const area = { width: 600, height: 200 }
const scene = (s: Settings, change: Partial<Scene> = {}): Scene => ({
  area, settings: s, left: curveOf(s.cutoff, s, area), right: null, trail: [], freq: s.cutoff, spectrum: null,
  flash: 0, level: 0, phase: 0, caption: 'filter', hot: null, ...change,
})
const count = (calls: Array<[string, unknown[]]>, name: string) => calls.filter(([n]) => n === name).length

beforeAll(() => applyTheme(slateLightTheme))

describe('drawing the Filter', () => {
  it('places frequencies by octave, both ways', () => {
    expect(freqToX(20, area)).toBe(16)
    expect(freqToX(20000, area)).toBe(584)
    expect(xToFreq(freqToX(1234, area), area)).toBeCloseTo(1234, 6)
  })

  it('shows the Floor handle and the range only while something moves it', () => {
    const still = fakeContext()
    drawFilter(still.ctx, scene(settings()))
    const moving = fakeContext()
    drawFilter(moving.ctx, scene(settings({ mode: 1 })))
    expect(moving.calls.some(([n, a]) => n === 'fillText' && a[0] === 'F')).toBe(true)
    expect(still.calls.some(([n, a]) => n === 'fillText' && a[0] === 'F')).toBe(false)
    expect(count(moving.calls, 'fillRect')).toBeGreaterThan(count(still.calls, 'fillRect'))
  })

  it('draws the spectrum, the trail and Spin’s other side when there are any', () => {
    const s = settings({ mode: 1, spin: 1 })
    const base = fakeContext()
    drawFilter(base.ctx, scene(s))
    const more = fakeContext()
    const curve = curveOf(500, s, area)
    drawFilter(more.ctx, scene(s, { trail: [curve, curve], right: curve, spectrum: [-40, -50, -60] }))
    expect(count(more.calls, 'stroke')).toBe(count(base.calls, 'stroke') + 3)
    expect(count(more.calls, 'fill')).toBe(count(base.calls, 'fill') + 1)
  })

  it('shows Dynamic’s level as a bar', () => {
    const dynamic = fakeContext()
    drawFilter(dynamic.ctx, scene(settings({ mode: 3 }), { level: 0.5 }))
    const off = fakeContext()
    drawFilter(off.ctx, scene(settings()))
    expect(count(dynamic.calls, 'fillRect')).toBe(count(off.calls, 'fillRect') + 2)
  })
})
