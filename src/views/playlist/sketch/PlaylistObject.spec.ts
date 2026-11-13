import p5 from 'p5'
import PlaylistObject from './PlaylistObject'
import { Renderer } from './index'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { ClipWindow } from '../../../types/clip'
import { Note, Row } from '../../../types/sequencer'
import { pattern, sampleRow, step, synthRow } from '../../../types/sequencer.fixture'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

// a beat is 10 pixels wide, so a step is 2.5
const BEAT = 10
const STEP = BEAT / 4

type Rect = { x: number, y: number, width: number, height: number }

// a p5 that draws nothing, but keeps the rects and lines drawn, in playlist
// x (before the zoom), each line with the color it was drawn in. the canvas
// shows playlist x from `shownFrom`, `shownWidth` wide
const recordingP5 = (shownFrom = 0, shownWidth = 10_000) => {
  const rects: Rect[] = []
  const lines: Array<{ x: number, color: string }> = []
  let stroke = ''
  const noop = () => undefined
  const loose = <T extends object>(target: T): T => new Proxy(target, {
    get: (t, key) => (key in t ? t[key as keyof T] : noop),
  })
  const p = loose({
    drawingContext: loose({
      canvas: { width: shownWidth, height: 400 },
      getTransform: () => ({ a: 1, b: 0, c: 0, d: 1, e: -shownFrom, f: 0 }),
    }),
    stroke: (...rgba: number[]) => { stroke = rgba.join() },
    line: (x: number) => { lines.push({ x, color: stroke }) },
    rect: (x: number, y: number, width: number, height: number) => { rects.push({ x, y, width, height }) },
  })
  return { p: p as unknown as ConstructorParameters<typeof PlaylistObject>[0], rects, lines }
}

const note = (pitch: number, start: number, length: number): Note => ({ pitch, start, length, velocity: 1 })

// a one bar pattern: a kick (row 0) on steps 0 and 8, and a synth's (row 1)
// note from step 14 to 18 (past the pattern's end), drawn as a clip of it on
// beat 0 that plays `window` of it, with `rows` as the sequencer's rows
const drawn = (
  window: ClipWindow, shownFrom?: number, shownWidth?: number, rows: Row[] = [sampleRow(0), synthRow(1)],
) => {
  const { p, rects, lines } = recordingP5(shownFrom, shownWidth)
  const renderer = new Renderer()
  renderer.width = 1_000
  renderer.beatsDisplayed = 1_000 / BEAT
  renderer.tempo = 120
  renderer.sequencer = {
    rows,
    patterns: [pattern(5, { notes: { 0: [step(0), step(8)], 1: [note(60, 14, 4)] } })],
    folders: [],
  }
  renderer.clipWindows = { 1: window }
  const clip = new PlaylistObject(p, {} as p5.Renderer, renderer, {
    playlistItem: { id: 1, path: '', patternId: 5, startBeat: 0, trackNumber: 0, pixelOffset: {} as never },
    soundData: new Float32Array(0),
    duration: 0,
    trackNumber: 0,
    trackHeight: 100,
    timelineWidth: 1_000,
    timelineHeight: 24,
    currentScale: 1,
  })
  clip.syncView()
  clip.drawPattern()

  const ink = renderer.patternRgba(0)
  const linesIn = (color: readonly number[]) => lines.filter(line => line.color === color.join()).map(line => line.x / STEP)
  // the ticks (narrower than a step), and the steps of the clip each starts
  // on, and each note's bar as (start, end), in steps from the clip's start
  const ticks = rects.filter(rect => rect.width < STEP)
  const barRects = rects.filter(rect => rect.width >= STEP)
  const hits = ticks.map(rect => Math.floor(rect.x / STEP))
  const bars = barRects.map(rect => [rect.x / STEP, (rect.x + rect.width) / STEP])
  return { repeats: linesIn(ink.repeat), beats: linesIn(ink.beat), hits, bars, ticks, barRects }
}

// whether every rect of `lower` is drawn below every rect of `upper`
const below = (lower: Rect[], upper: Rect[]) =>
  Math.min(...lower.map(rect => rect.y)) >= Math.max(...upper.map(rect => rect.y + rect.height))

describe('a pattern clip', () => {
  beforeEach(() => applyTheme(slateLightTheme))

  it('draws its pattern once when it\'s as long as it, with no repeat line', () => {
    const { repeats, beats, hits, bars } = drawn({ start: 0, end: 16 })
    expect(repeats).toEqual([])
    expect(beats).toEqual([4, 8, 12])
    expect(hits).toEqual([0, 8])
    // the synth's note cut at the pattern's end
    expect(bars).toEqual([[14, 16]])
  })

  it('draws its pattern each time round when it\'s longer, with a repeat line where it starts over', () => {
    const { repeats, beats, hits, bars } = drawn({ start: 0, end: 40 })
    expect(repeats).toEqual([16, 32])
    expect(beats).toEqual([4, 8, 12, 20, 24, 28, 36])
    expect(hits.sort((a, b) => a - b)).toEqual([0, 8, 16, 24, 32])
    // cut at the end of each time round, and the third time round ends first
    expect(bars).toEqual([[14, 16], [30, 32]])
  })

  it('draws from where it starts in its pattern, past the pattern\'s end', () => {
    // from step 20 (4 of its second time round) for a bar: the kick on step
    // 8 four steps in, then the one on step 0 twelve steps in
    const { repeats, hits } = drawn({ start: 20, end: 36 })
    expect(repeats).toEqual([12])
    expect(hits.sort((a, b) => a - b)).toEqual([4, 12])
  })

  it('draws a lane for each row that plays notes, in the rows\' order: ticks for a sample, bars for a synth', () => {
    const kickFirst = drawn({ start: 0, end: 16 })
    expect(below(kickFirst.barRects, kickFirst.ticks)).toBe(true)
    // a tick is short, and as tall as its lane (but for a little gap), where
    // a bar is as tall as a row of pitches in its lane
    expect(kickFirst.ticks.every(tick => tick.width <= STEP / 2)).toBe(true)
    expect(kickFirst.ticks[0].height).toBeGreaterThan(kickFirst.barRects[0].height * 6)

    // the synth row moved above the kick, and a snare row with no notes,
    // which has no lane
    const synthFirst = drawn({ start: 0, end: 16 }, undefined, undefined, [synthRow(1), sampleRow(2), sampleRow(0)])
    expect(below(synthFirst.ticks, synthFirst.barRects)).toBe(true)
    expect(synthFirst.ticks.map(t => t.height)).toEqual(kickFirst.ticks.map(t => t.height))
    expect(synthFirst.hits).toEqual([0, 8])
    expect(synthFirst.bars).toEqual([[14, 16]])
  })

  it('draws only the times round that are on the canvas, and only the notes on it', () => {
    // a clip 8 bars long, with bars 3 and 4 on the canvas, and the last step
    // of bar 2: the end of its second time round, where no note starts
    const { repeats, hits } = drawn({ start: 0, end: 128 }, 31 * STEP, 32 * STEP)
    expect(repeats).toEqual([16, 32, 48])
    expect(hits.sort((a, b) => a - b)).toEqual([32, 40, 48, 56])
  })
})
