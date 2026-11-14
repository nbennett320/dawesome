import { Renderer } from './index'
import { paint } from './theme'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { PlaylistTransport } from '../../../types/transport'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

// a p5 that draws nothing, but keeps the ruler's numbers and the lines
// drawn, each with the color it was drawn in. (where they are is as drawn,
// before the zoom: the zoom is p.scale's, which draws nothing either)
const recordingP5 = (width: number, height: number) => {
  const numbers: Array<{ text: string, x: number }> = []
  const lines: Array<{ x: number, color: string }> = []
  const rects: Array<{ x: number, w: number, color: string }> = []
  let stroke = ''
  let fill = ''
  const noop = () => undefined
  const loose = <T extends object>(target: T): T => new Proxy(target, {
    get: (t, key) => (key in t ? t[key as keyof T] : noop),
  })
  const p = loose({
    width,
    height,
    mouseX: -1,
    mouseY: -1,
    drawingContext: loose({
      canvas: { width, height },
      getTransform: () => ({ a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 }),
      measureText: (text: string) => ({ width: text.length * 6 }),
      createLinearGradient: () => loose({}),
    }),
    createCanvas: () => loose({ elt: document.createElement('canvas') }),
    constrain: (n: number, low: number, high: number) => Math.min(Math.max(n, low), high),
    keyIsDown: () => false,
    stroke: (...rgba: number[]) => { stroke = rgba.join() },
    fill: (...rgba: number[]) => { fill = rgba.join() },
    rect: (x: number, _y: number, w: number) => { rects.push({ x, w, color: fill }) },
    line: (x: number) => { lines.push({ x, color: stroke }) },
    text: (text: string, x: number) => { numbers.push({ text, x }) },
  })
  return { p: p as unknown as Parameters<Renderer['sketch']>[0], numbers, lines, rects }
}

const stopped: PlaylistTransport = {
  playing: false, startTimeMs: null, startOffsetMs: 0, positionMs: 0, beatIntervalMs: 500, loopStartBeats: 0, loopEndBeats: 12, loopRegion: false, loopEndPinned: false,
  beatsDisplayed: 384, loopEnabled: false, songEndMs: 0, beatsPerBar: 4, patternId: null, patternLength: 0,
}

describe('the playlist in 3/4', () => {
  beforeEach(() => applyTheme(slateLightTheme))

  // 128 bars of three beats, zoomed so a beat is 40px: the grid splits beats
  // in two, and the ruler numbers every beat
  const draw = () => {
    const width = 960
    const renderer = new Renderer()
    renderer.width = width
    renderer.height = 400
    const { p, numbers, lines } = recordingP5(width, 400)
    renderer.sketch(p)
    p.setup()
    renderer.transport = { ...stopped, beatsPerBar: 3 }
    renderer.beatsDisplayed = 384
    renderer.currentScale = 16
    p.draw()
    return { numbers, lines, beatX: width / 384, beatWidth: width * 16 / 384 }
  }

  it('numbers bars of three beats in the ruler', () => {
    const { numbers, beatWidth } = draw()
    // (a number is drawn just right of its beat)
    const numberAt = (beat: number) => numbers.find(n => Math.abs(n.x - 3 - beat * beatWidth) < 0.01)?.text
    expect([0, 1, 2, 3, 4, 6].map(numberAt)).toEqual(['1', '1.2', '1.3', '2', '2.2', '3'])
  })

  it('draws a bar line every three beats', () => {
    const { lines, beatX } = draw()
    const bar = paint.rgba('barLine').join()
    const barLines = lines.filter(line => line.color === bar).map(line => Math.round(line.x / beatX))
    expect(barLines.slice(0, 5)).toEqual([0, 3, 6, 9, 12])
  })
})

describe('the loop region', () => {
  beforeEach(() => applyTheme(slateLightTheme))

  // 4/4, a beat 10px wide (zoomed so 96 beats fit), a region from beat 8 to 16
  const draw = (t: Partial<PlaylistTransport>) => {
    const width = 960
    const renderer = new Renderer()
    renderer.width = width
    renderer.height = 400
    const { p, lines, rects } = recordingP5(width, 400)
    renderer.sketch(p)
    p.setup()
    renderer.transport = { ...stopped, loopStartBeats: 8, loopEndBeats: 16, loopRegion: true, ...t }
    renderer.beatsDisplayed = 384
    renderer.currentScale = 4
    p.draw()
    const beatX = width / 384
    return {
      // where the lanes are dimmed, in beats
      dimmed: rects.filter(r => r.color === paint.rgba('loopOutside').join())
        .map(r => [Math.round(r.x / beatX), Math.round((r.x + r.w) / beatX)]),
      // the ruler's band, and its flags (drawn unzoomed, a beat 10px)
      band: rects.filter(r => r.color.startsWith(paint.rgba('loopRegion').slice(0, 3).join())).map(r => [r.x / 10, (r.x + r.w) / 10]),
      flags: lines.filter(l => l.color.startsWith(paint.rgba('loopEnd').slice(0, 3).join())).map(l => l.x / 10),
    }
  }

  it('is a band in the ruler with a flag at each end, and dims the lanes outside it', () => {
    const { dimmed, band, flags } = draw({ loopEnabled: true })
    expect(band).toEqual([[8, 16]])
    expect(flags).toEqual([8, 16])
    expect(dimmed).toEqual([[0, 8], [16, 384]])
  })

  it('dims nothing while the playlist doesn’t loop, but still shows', () => {
    const { dimmed, band, flags } = draw({ loopEnabled: false })
    expect(band).toEqual([[8, 16]])
    expect(flags).toEqual([8, 16])
    expect(dimmed).toEqual([])
  })

  it('is only the end flag without one', () => {
    const { dimmed, band, flags } = draw({ loopEnabled: true, loopRegion: false, loopStartBeats: 0, loopEndBeats: 12 })
    expect(band).toEqual([])
    expect(flags).toEqual([12])
    expect(dimmed).toEqual([])
  })
})
