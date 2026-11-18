import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { brightness, drawChorus, Scene, slideOf, stringAt } from './draw'
import { Settings, voicesAt } from './voices'

const fakeContext = () => {
  const calls: Array<[string, unknown[]]> = []
  const styles: string[] = []
  const ctx = new Proxy({} as Record<string, unknown>, {
    get: (_, name: string) => (...args: unknown[]) => { calls.push([name, args]) },
    set: (_, name: string, value) => {
      if(name === 'strokeStyle') styles.push(String(value))
      return true
    },
  })
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls, styles }
}

const settings = (change: Partial<Settings> = {}): Settings => ({
  sync: false, rateHz: 1, voices: 2, depth: 0.4, shape: 0, delayMs: 10, spread: 0.7, toneHz: 9000, mix: 0.5,
  bloom: 0, drift: 0, prism: 0, ...change,
})

const scene = (s: Settings, change: Partial<Scene> = {}): Scene => ({
  area: { width: 600, height: 200 }, settings: s, voices: voicesAt(s, null), phase: 0, open: 1, caption: '2 voices', hot: false, ...change,
})

beforeAll(() => applyTheme(slateLightTheme))

describe('drawing the Chorus', () => {
  it('draws the dry string on each side, and each voice on each side it’s heard', () => {
    const { ctx, calls } = fakeContext()
    drawChorus(ctx, scene(settings()))
    // two dry strings, two voices on both sides
    expect(calls.filter(([name]) => name === 'stroke').length).toBe(1 + 2 + 4 + 1)
    expect(calls.some(([name, args]) => name === 'fillText' && args[0] === '2 voices')).toBe(true)
  })

  it('leaves a voice out where it isn’t heard (gone, or Bloom has it shut)', () => {
    const shut = fakeContext()
    drawChorus(shut.ctx, scene(settings(), { open: 0 }))
    expect(shut.calls.filter(([name]) => name === 'stroke').length).toBe(1 + 2 + 1)
  })

  it('tints the voices with Prism', () => {
    const plain = fakeContext()
    drawChorus(plain.ctx, scene(settings({ voices: 4 })))
    const prism = fakeContext()
    drawChorus(prism.ctx, scene(settings({ voices: 4, prism: 1 })))
    // (the colours, alpha aside)
    const hues = (styles: string[]) => new Set(styles.map(c => c.replace(/, [\d.]+\)$/, ')'))).size
    expect(hues(prism.styles)).toBeGreaterThan(hues(plain.styles) + 2)
  })

  it('slides a copy a quarter cycle at most for its swing, and brightens with Tone', () => {
    const s = settings({ depth: 1 })
    const [voice] = voicesAt(s, null)
    const quarter = 0.25 / 3.5
    expect(slideOf(voice.center + 4.5, voice, s) - slideOf(voice.center, voice, s)).toBeCloseTo(quarter, 9)
    expect(brightness(1000)).toBe(0)
    expect(brightness(20000)).toBe(1)
    expect(Math.abs(stringAt(0.1, 1))).toBeLessThanOrEqual(1)
  })
})
