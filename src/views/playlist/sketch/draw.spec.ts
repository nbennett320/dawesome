import { invoke } from '@tauri-apps/api/core'
import type { Mock } from 'vitest'
import { Renderer, staticDefaults, trackTop } from './index'
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
  const rects: Array<{ x: number, y: number, w: number, h: number, color: string }> = []
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
      getTransform: () => ({ a: 1, b: 0, c: 0, d: 1, e: 0, f: 0, transformPoint: (point: object) => point }),
      measureText: (text: string) => ({ width: text.length * 6 }),
      createLinearGradient: () => loose({}),
    }),
    createCanvas: () => loose({ elt: document.createElement('canvas') }),
    constrain: (n: number, low: number, high: number) => Math.min(Math.max(n, low), high),
    keyIsDown: () => false,
    stroke: (...rgba: number[]) => { stroke = rgba.join() },
    fill: (...rgba: number[]) => { fill = rgba.join() },
    rect: (x: number, y: number, w: number, h: number) => { rects.push({ x, y, w, h, color: fill }) },
    line: (x: number) => { lines.push({ x, color: stroke }) },
    text: (text: string, x: number) => { numbers.push({ text, x }) },
  })
  return { p: p as unknown as Parameters<Renderer['sketch']>[0], numbers, lines, rects }
}

const stopped: PlaylistTransport = {
  playing: false, startTimeMs: null, startOffsetMs: 0, positionMs: 0, lastPlayStartBeats: null, beatIntervalMs: 500, loopStartBeats: 0, loopEndBeats: 12, loopRegion: false, loopEndPinned: false,
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
      // the ruler's band, and its flags (drawn unzoomed, a beat 10px). the
      // current track's tint and bar are in the accent color too
      band: rects
        .filter(r => r.color.startsWith(paint.rgba('loopRegion').slice(0, 3).join()))
        .filter(r => r.color !== paint.rgba('currentLane').join() && r.color !== paint.rgba('currentTrackEdge').join())
        .map(r => [r.x / 10, (r.x + r.w) / 10]),
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

describe('the current track', () => {
  beforeEach(() => applyTheme(slateLightTheme))

  const draw = (current: number) => {
    const renderer = new Renderer()
    renderer.width = 960
    renderer.height = 400
    const { p, rects } = recordingP5(960, 400)
    renderer.sketch(p)
    p.setup()
    renderer.transport = stopped
    renderer.setCurrentTrack(current)
    p.draw()
    const tops = (key: 'currentLane' | 'currentTrackEdge') =>
      rects.filter(r => r.color === paint.rgba(key).join()).map(r => ({ y: r.y, w: r.w }))
    return { lane: tops('currentLane'), edge: tops('currentTrackEdge') }
  }

  it("tints its lane, and marks the lane's left edge", () => {
    const { lane, edge } = draw(2)
    expect(lane).toEqual([{ y: trackTop(2), w: 960 }])
    expect(edge).toEqual([{ y: trackTop(2) + 1, w: 2 }])
  })

  it('shows the last track while one past it is being made', () => {
    // (4 tracks: one landing on the fifth makes it)
    expect(draw(4).lane).toEqual([{ y: trackTop(3), w: 960 }])
  })
})

describe('the time selection', () => {
  beforeEach(() => applyTheme(slateLightTheme))

  // a beat 10px wide, as in the loop region's tests
  const draw = (sel: Renderer['timeSelection']) => {
    const renderer = new Renderer()
    renderer.width = 960
    renderer.height = 400
    const { p, rects, lines } = recordingP5(960, 400)
    renderer.sketch(p)
    p.setup()
    renderer.transport = stopped
    renderer.beatsDisplayed = 384
    renderer.currentScale = 4
    renderer.timeSelection = sel
    p.draw()
    const beatX = 960 / 384
    return {
      // the wash over the lanes, in beats and track tops
      wash: rects.filter(r => r.color === paint.rgba('timeSelection').join())
        .map(r => ({ from: r.x / beatX, to: (r.x + r.w) / beatX, y: r.y, h: r.h })),
      edges: lines.filter(l => l.color === paint.rgba('timeSelectionEdge').join()).map(l => l.x / beatX),
      // the bar in the ruler (drawn unzoomed, a beat 10px)
      // (above the lanes: the current track's bar is in the accent too)
      mark: rects.filter(r => r.color === paint.rgba('timeSelectionEdge').join() && r.y < trackTop(0))
        .map(r => [r.x / 10, (r.x + r.w) / 10]),
    }
  }

  it('washes its lanes between its ends, and marks its span in the ruler', () => {
    const { wash, edges, mark } = draw({ anchorBeat: 12, edgeBeat: 8, anchorTrack: 1, edgeTrack: 2 })
    // from the top of track 1 to the bottom of track 2
    expect(wash).toEqual([{ from: 8, to: 12, y: trackTop(1), h: trackTop(2) + staticDefaults.trackHeight - trackTop(1) }])
    expect(edges).toEqual([8, 12])
    expect(mark).toEqual([[8, 12]])
  })

  it('is a line while it covers no time', () => {
    const { edges, mark } = draw({ anchorBeat: 8, edgeBeat: 8, anchorTrack: 0, edgeTrack: 1 })
    expect(edges).toEqual([8, 8])
    expect(mark).toEqual([[8, 8.1]])
  })

  it('draws nothing without one', () => {
    const { wash, edges, mark } = draw(null)
    expect([wash, edges, mark]).toEqual([[], [], []])
  })
})

describe('files dragged in from outside, loading', () => {
  beforeEach(() => applyTheme(slateLightTheme))
  afterEach(() => (invoke as Mock).mockReset())

  const files = (n: number) => ({
    kind: 'files' as const,
    files: Array.from({ length: n }, (_, i) => ({ path: `/finder/f${i}.wav`, folder: '' })),
    skipped: [],
  })

  // a playlist with no tracks yet (every file lands on a new one), whose
  // ghosts' sounds are still being read unless `read` answers
  const setup = (read: (path: string) => Promise<[string, number]> = () => new Promise(() => {})) => {
    (invoke as Mock).mockImplementation((_: string, args?: { samplePath?: string }) => read(args?.samplePath ?? ''))
    const renderer = new Renderer()
    renderer.width = 960
    renderer.height = 400
    const recorded = recordingP5(960, 400)
    renderer.sketch(recorded.p)
    recorded.p.setup()
    renderer.transport = stopped
    const draw = () => {
      recorded.numbers.length = 0
      recorded.p.draw()
      return recorded.numbers.map(n => n.text).filter(text => text.endsWith('loading…'))
    }
    return { renderer, draw }
  }

  it('shows a card saying so where each would land, until its sound is read', () => {
    const { renderer, draw } = setup()
    renderer.setSampleDrag(files(3), 100, 120, 0)
    expect(draw()).toEqual(['f0.wav: loading…', 'f1.wav: loading…', 'f2.wav: loading…'])
    renderer.clearSampleDrag()
    expect(draw()).toEqual([])
  })

  it('keeps showing them where they were dropped while they’re placed', () => {
    const { renderer, draw } = setup()
    renderer.setSampleDrag(files(2), 100, 120, 0)
    const hold = renderer.fileDropHolder(100, 120)
    renderer.clearSampleDrag()
    // (shown only once it's called: not while the drop asks first)
    expect(draw()).toEqual([])
    const release = hold()
    expect(draw()).toEqual(['f0.wav: loading…', 'f1.wav: loading…'])
    release()
    expect(draw()).toEqual([])
  })

  it('holds nothing for a sample from the sidebar', () => {
    const { renderer, draw } = setup()
    renderer.setSampleDrag({ kind: 'sample', path: '/samples/kick.wav' }, 100, 120, 0)
    expect(draw()).toEqual(['kick.wav: loading…'])
    const release = renderer.fileDropHolder(100, 120)()
    renderer.clearSampleDrag()
    expect(draw()).toEqual([])
    release()
  })

  it('shows no card for a file whose sound has been read', async () => {
    // (jsdom has none: the clip following the mouse is drawn lifted with one)
    vi.stubGlobal('DOMMatrix', class { translate() { return this } scale() { return this } multiply() { return this } })
    const { renderer, draw } = setup(async path => (path.endsWith('f1.wav') ? new Promise(() => {}) : ['', 2]))
    renderer.setSampleDrag(files(2), 100, 120, 0)
    await new Promise(resolve => setTimeout(resolve, 0))
    // (the first drawn as a clip following the mouse, the second still loading)
    expect(draw()).toEqual(['f1.wav: loading…'])
    vi.unstubAllGlobals()
  })
})
