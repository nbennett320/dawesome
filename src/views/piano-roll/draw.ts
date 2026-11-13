// drawing the Piano Roll: its grid of steps and notes, the time ruler above
// it and the keyboard down its left, each on a plain canvas. only what's in
// view is drawn (the notes in view found by binary search: see
// notesBetween), and only when something changes, at most once a frame (see
// frames.ts), so a pattern with thousands of notes stays smooth. the
// colours come from the theme
import { Edge } from '../../editing/edges'
import { Rect } from '../../editing/lasso'
import { drawCutLine, drawEdgeBar, drawLasso } from '../../editing/look'
import { zoomAround } from '../../editing/view'
import { painter } from '../../theme/canvas'
import { cssVar, currentTheme } from '../../theme/apply'
import { STEPS_PER_BEAT, Note } from '../../types/sequencer'
import { HIGHEST_PITCH, PITCHES, isBlackKey, pitchName } from './notes'

const colors = painter('colors')
const lanes = painter('playlist')
const noteColors = painter('pianoRoll')

// how wide a step is on the grid, in CSS pixels: to start with, and as far
// as zooming goes either way. (the zoom isn't saved)
export const DEFAULT_STEP_WIDTH = 24
export const MIN_STEP_WIDTH = 6
export const MAX_STEP_WIDTH = 96
// how tall piano key row is
export const ROW_HEIGHT = 16
// the keyboard's width, and the ruler's height
export const KEYS_WIDTH = 56
export const RULER_HEIGHT = 20
// narrower steps than this have no line between them, only beats and bars
const STEP_LINES_FROM = 8
// how far the grid runs on, in bars: past the pattern's end, and past the
// furthest the view has reached, so scrolling right finds more
export const RUN_ON_BARS = 4
// how strongly what's past the pattern's end is shaded
const PAST_END_SHADE = 0.5

// a note's corners are rounded like the playlist's clips, less on a note
// too short to take the whole curve
const NOTE_RADIUS = 3
const noteRadius = (w: number) => Math.max(0, Math.min(NOTE_RADIUS, w / 2 - 0.5))
const noteShape = (ctx: CanvasRenderingContext2D, x: number, y: number, w: number) => {
  ctx.roundRect(x + 0.5, y + 1, w, ROW_HEIGHT - 2, noteRadius(w))
}
const fillNote = (ctx: CanvasRenderingContext2D, x: number, y: number, w: number) => {
  ctx.beginPath()
  noteShape(ctx, x, y, w)
  ctx.fill()
}
const strokeNote = (ctx: CanvasRenderingContext2D, x: number, y: number, w: number) => {
  ctx.beginPath()
  ctx.roundRect(x + 1, y + 1.5, w - 1, ROW_HEIGHT - 3, noteRadius(w - 1))
  ctx.stroke()
}

// the grid's whole size, of which a part is in view
export const contentHeight = (): number => PITCHES * ROW_HEIGHT
export const contentWidth = (steps: number, stepWidth: number): number => Math.max(0, steps) * stepWidth

// how many steps the grid runs to (the width it scrolls through):
// RUN_ON_BARS past the pattern's end, or past `seen`, the furthest step the
// view has reached, to the bar. never past where time runs to (`limit`: see
// timelineEnd)
export const gridSteps = (length: number, seen: number, stepsPerBar: number, limit: number): number =>
  Math.min(limit, (Math.ceil(Math.max(length, seen) / stepsPerBar) + RUN_ON_BARS) * stepsPerBar)

// a pitch's row, and the pitch at a height on the grid (both from the top
// of the whole grid, not of what's in view)
export const rowTop = (pitch: number): number => (HIGHEST_PITCH - pitch) * ROW_HEIGHT
export const pitchAtY = (y: number): number => HIGHEST_PITCH - Math.floor(y / ROW_HEIGHT)
export const stepAtX = (x: number, stepWidth: number): number => x / stepWidth
// a height on the grid in rows, from the highest pitch's (the grid's own
// coordinates, which a selection box is in)
export const rowAtY = (y: number): number => y / ROW_HEIGHT

// zoom along time by `factor`, keeping the step at `aroundX` (pixels from
// the grid's left edge in view) where it is. the step width stays between
// MIN_STEP_WIDTH and MAX_STEP_WIDTH; the scroll is where the grid's start
// is scrolled to after, which the scroller may yet hold back at either end
export const zoomGrid = (
  stepWidth: number,
  scrollLeft: number,
  factor: number,
  aroundX: number,
): { stepWidth: number, scrollLeft: number } => {
  const zoom = zoomAround({ scale: stepWidth, pan: -scrollLeft }, factor, aroundX, MIN_STEP_WIDTH, MAX_STEP_WIDTH)
  return { stepWidth: zoom.scale, scrollLeft: Math.max(0, -zoom.pan) }
}

// the part of the grid in view: its size, how far it's scrolled, and how
// wide a step is
export interface Viewport {
  width: number
  height: number
  scrollX: number
  scrollY: number
  stepWidth: number
}

// draw at the screen's pixel density, so lines stay sharp. returns the
// context to draw on, in CSS pixels
export const fitCanvas = (
  canvas: HTMLCanvasElement | null,
  width: number,
  height: number,
): CanvasRenderingContext2D | null => {
  if(!canvas || width <= 0 || height <= 0) return null
  const ctx = canvas.getContext('2d')
  if(!ctx) return null

  const dpr = window.devicePixelRatio || 1
  const w = Math.round(width * dpr)
  const h = Math.round(height * dpr)
  if(canvas.width !== w) canvas.width = w
  if(canvas.height !== h) canvas.height = h
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  return ctx
}

// the pitches with a row in view, highest first
const pitchesInView = ({ height, scrollY }: Pick<Viewport, 'height' | 'scrollY'>): [number, number] => [
  Math.max(0, HIGHEST_PITCH - Math.floor((scrollY + height) / ROW_HEIGHT)),
  Math.min(HIGHEST_PITCH, HIGHEST_PITCH - Math.floor(scrollY / ROW_HEIGHT)),
]

// the longest note in a list, worked out once per list. (the lists are the
// store's, each a new one only when its notes change)
const longest = new WeakMap<Note[], number>()
const longestIn = (notes: Note[]): number => {
  let most = longest.get(notes)
  if(most === undefined) {
    most = notes.reduce((max, note) => Math.max(max, note.length), 0)
    longest.set(notes, most)
  }
  return most
}

// the first place in a list of notes in order (see sortNotes) whose note
// starts at or after `step`
const firstFrom = (notes: Note[], step: number): number => {
  let low = 0
  let high = notes.length
  while(low < high) {
    const mid = (low + high) >> 1
    if(notes[mid].start < step) low = mid + 1
    else high = mid
  }
  return low
}

// the places, from the first up to (not including) the last, of the notes
// in a list in order (see sortNotes) that can be seen between steps `from`
// and `to`: from the first that starts late enough to reach `from` (no note
// is longer than the longest), to the first that starts at `to`. found by
// binary search, so drawing a row costs what's in view, not its every note
export const notesBetween = (notes: Note[], from: number, to: number): [number, number] =>
  [firstFrom(notes, from - longestIn(notes)), firstFrom(notes, to)]

// a crisp 1px line across or down the canvas
const vline = (ctx: CanvasRenderingContext2D, x: number, top: number, bottom: number) => {
  const at = Math.round(x) + 0.5
  ctx.moveTo(at, top)
  ctx.lineTo(at, bottom)
}

const hline = (ctx: CanvasRenderingContext2D, y: number, left: number, right: number) => {
  const at = Math.round(y) + 0.5
  ctx.moveTo(left, at)
  ctx.lineTo(right, at)
}

export interface GridScene {
  // the pattern's length, in steps: shaded past it
  length: number
  // where time runs to (see timelineEnd): nothing past it
  limit: number
  stepsPerBar: number
  // the row's notes
  notes: Note[]
  // whether they're in order (see sortNotes), as the engine keeps them, so
  // those in view are found by binary search. not while a drag has them:
  // it keeps the order it started with, and each one is looked at
  inOrder: boolean
  // every other row's notes in the pattern, a list per row, each in order,
  // drawn faintly behind
  ghosts: Note[][]
  selected: ReadonlySet<number>
  // notes being carried by the mouse (see drawCarried): drawn here as
  // ghosts, where they'll land
  landing: ReadonlySet<number> | null
  // a selection box being dragged, in the grid's own coordinates (steps
  // across, rows down)
  lasso: Rect | null
  // where a click with the Cut tool would cut, and the notes it would
  cut: { ids: ReadonlyArray<number>, step: number } | null
  // an edge with a bar on it: of the note a press would trim, or of those
  // being trimmed
  edgeBar: { ids: ReadonlyArray<number>, edge: Edge } | null
}

// a drawing of the grid: where on the canvas, and what's in view of it,
// the pitches and the steps (with a step's slack to the left)
interface GridFrame {
  ctx: CanvasRenderingContext2D
  view: Viewport
  low: number
  high: number
  from: number
  to: number
  // where time runs to, in pixels across the canvas (or its right edge)
  right: number
}

const inView = ({ low, high, from, to }: GridFrame, note: Note) =>
  note.pitch >= low && note.pitch <= high && note.start + note.length >= from && note.start < to

const place = ({ view }: GridFrame, note: Note) => ({
  x: note.start * view.stepWidth - view.scrollX,
  y: rowTop(note.pitch) - view.scrollY,
  w: Math.max(3, note.length * view.stepWidth - 1),
})

// each note in view, and its place in the list: found by binary search in
// a list in order (see notesBetween), else by looking at each one
const eachInView = (frame: GridFrame, notes: Note[], inOrder: boolean, draw: (note: Note, i: number) => void) => {
  const [first, end] = inOrder ? notesBetween(notes, frame.from, frame.to) : [0, notes.length]
  for(let i = first; i < end; i++) {
    if(inView(frame, notes[i])) draw(notes[i], i)
  }
}

// a row per semitone, the black keys' shaded, up to where time runs to
const drawLanes = ({ ctx, view, low, high, right }: GridFrame) => {
  ctx.fillStyle = lanes.css('background')
  ctx.fillRect(0, 0, right, view.height)
  ctx.fillStyle = colors.css('panel')
  for(let pitch = low; pitch <= high; pitch++) {
    if(isBlackKey(pitch)) ctx.fillRect(0, rowTop(pitch) - view.scrollY, right, ROW_HEIGHT)
  }
}

// a line under each row, stronger between octaves, and a column per step,
// beats and bars stronger
const drawLines = ({ ctx, view, low, high, right }: GridFrame, { limit, stepsPerBar }: GridScene) => {
  const { width, height, scrollX, scrollY, stepWidth } = view
  const rows = (strokeStyle: string, keep: (pitch: number) => boolean) => {
    ctx.beginPath()
    ctx.strokeStyle = strokeStyle
    for(let pitch = low; pitch <= high; pitch++) {
      if(keep(pitch)) hline(ctx, rowTop(pitch) + ROW_HEIGHT - scrollY, 0, right)
    }
    ctx.stroke()
  }
  const first = Math.max(0, Math.floor(scrollX / stepWidth))
  const last = Math.min(limit, Math.ceil((scrollX + width) / stepWidth))
  const columns = (strokeStyle: string, keep: (step: number) => boolean) => {
    ctx.beginPath()
    ctx.strokeStyle = strokeStyle
    for(let step = first; step <= last; step++) {
      if(keep(step)) vline(ctx, step * stepWidth - scrollX, 0, height)
    }
    ctx.stroke()
  }
  const bar = (step: number) => step % stepsPerBar === 0
  const beat = (step: number) => step % STEPS_PER_BEAT === 0
  // (between B and C, an octave apart)
  const octave = (pitch: number) => pitch % 12 === 0

  ctx.lineWidth = 1
  rows(lanes.css('laneLine'), pitch => !octave(pitch))
  // (zoomed far out, lines a step apart would smear together)
  if(stepWidth >= STEP_LINES_FROM) columns(lanes.css('laneLine', 0.6), step => !beat(step) && !bar(step))
  columns(lanes.css('beatLine'), step => beat(step) && !bar(step))
  rows(lanes.css('barLine'), octave)
  columns(lanes.css('barLine'), bar)
}

// past the pattern's end, shaded (a note drawn there grows the pattern),
// with a line at the end in the colour the ruler flags it
const drawPastEnd = ({ ctx, view, right }: GridFrame, length: number) => {
  const end = length * view.stepWidth - view.scrollX
  if(end >= right) return
  const from = Math.max(0, end)
  ctx.fillStyle = colors.css('sunken', PAST_END_SHADE)
  ctx.fillRect(from, 0, right - from, view.height)
  if(end < 0) return
  ctx.beginPath()
  ctx.strokeStyle = lanes.css('loopEnd')
  vline(ctx, end, 0, view.height)
  ctx.stroke()
}

// the other rows' notes, faintly, as one shape (so where they overlap,
// they're no darker)
const drawGhosts = (frame: GridFrame, ghosts: Note[][]) => {
  const { ctx } = frame
  ctx.beginPath()
  ghosts.forEach(notes => eachInView(frame, notes, true, note => {
    const { x, y, w } = place(frame, note)
    noteShape(ctx, x, y, w)
  }))
  ctx.fillStyle = noteColors.css('note', 0.35)
  ctx.fill()
}

// the row's notes, the selected ones in the accent colour, and those being
// carried as an outline, faintly filled, where they'll land
const drawNotes = (frame: GridFrame, { notes, inOrder, selected, landing }: GridScene) => {
  const { ctx } = frame
  const look = {
    fill: noteColors.css('note'),
    border: noteColors.css('noteBorder'),
    selectedFill: noteColors.css('selected'),
    selectedBorder: noteColors.css('selectedBorder'),
    landingFill: noteColors.css('selected', 0.35),
  }
  eachInView(frame, notes, inOrder, (note, i) => {
    const { x, y, w } = place(frame, note)
    const landed = !!landing?.has(i)
    const on = selected.has(i)
    ctx.fillStyle = landed ? look.landingFill : (on ? look.selectedFill : look.fill)
    fillNote(ctx, x, y, w)
    ctx.strokeStyle = landed || on ? look.selectedBorder : look.border
    strokeNote(ctx, x, y, w)
  })
}

// over the notes: the bar on an edge a press would grab (or that's being
// dragged), where a click with the Cut tool would cut, and a selection box
const drawMarks = (frame: GridFrame, { notes, edgeBar, cut, lasso }: GridScene) => {
  const { ctx, view } = frame
  const shown = (i: number) => (notes[i] && inView(frame, notes[i]) ? notes[i] : null)

  edgeBar?.ids.forEach(i => {
    const note = shown(i)
    if(!note) return
    const { x, y, w } = place(frame, note)
    drawEdgeBar(ctx, edgeBar.edge, x + 0.5, x + 0.5 + w, y + 1, ROW_HEIGHT - 2)
  })

  if(cut) {
    const x = Math.round(cut.step * view.stepWidth - view.scrollX) + 0.5
    cut.ids.forEach(i => {
      const note = shown(i)
      if(!note) return
      const y = rowTop(note.pitch) - view.scrollY
      drawCutLine(ctx, x, y - 2, y + ROW_HEIGHT + 2)
    })
  }

  if(lasso) {
    drawLasso(
      ctx,
      Math.round(lasso.left * view.stepWidth - view.scrollX) + 0.5,
      Math.round(lasso.top * ROW_HEIGHT - view.scrollY) + 0.5,
      Math.round(lasso.width * view.stepWidth),
      Math.round(lasso.height * ROW_HEIGHT),
    )
  }
}

export const drawGrid = (ctx: CanvasRenderingContext2D, view: Viewport, scene: GridScene) => {
  const { width, height, scrollX, stepWidth } = view
  const [low, high] = pitchesInView(view)
  const frame: GridFrame = {
    ctx,
    view,
    low,
    high,
    from: scrollX / stepWidth - 1,
    to: (scrollX + width) / stepWidth,
    right: Math.min(width, contentWidth(scene.limit, stepWidth) - scrollX),
  }

  ctx.clearRect(0, 0, width, height)
  // past where time runs to
  ctx.fillStyle = colors.css('sunken')
  ctx.fillRect(0, 0, width, height)
  drawLanes(frame)
  drawLines(frame, scene)
  drawPastEnd(frame, scene.length)
  drawGhosts(frame, scene.ghosts)
  drawNotes(frame, scene)
  drawMarks(frame, scene)
}

// the notes being carried by the mouse, on their own canvas over the grid,
// at the fractional places it has them (see carriedNotes). drawn every frame
// of a drag, so the grid under them is only drawn again when where they'd
// land changes
export const drawCarried = (ctx: CanvasRenderingContext2D, view: Viewport, notes: Note[]) => {
  const { width, height, scrollX, scrollY, stepWidth } = view
  ctx.clearRect(0, 0, width, height)
  ctx.lineWidth = 1
  notes.forEach(note => {
    const x = note.start * stepWidth - scrollX
    const y = rowTop(note.pitch) - scrollY
    const w = Math.max(3, note.length * stepWidth - 1)
    if(x > width || x + w < 0 || y > height || y + ROW_HEIGHT < 0) return
    ctx.fillStyle = noteColors.css('selected')
    fillNote(ctx, x, y, w)
    ctx.strokeStyle = noteColors.css('selectedBorder')
    strokeNote(ctx, x, y, w)
  })
}

// the playhead's colour: as the playlist's, bright while playing or being
// dragged along the ruler, grey while stopped. (before a theme is applied,
// the core colours the base theme gives them)
export const playheadColour = (active: boolean): string => {
  if(!currentTheme()) return cssVar(active ? 'accent' : 'inkFaint')
  return lanes.css(active ? 'playhead' : 'playheadStopped')
}

// the time ruler: a number at each bar and a tick at each beat, as far as
// time runs. past the pattern's end it's shaded, and the end is flagged as
// the playlist flags its loop's: where Play goes round
export const drawRuler = (
  ctx: CanvasRenderingContext2D,
  { width, scrollX, stepWidth }: Pick<Viewport, 'width' | 'scrollX' | 'stepWidth'>,
  { length, limit, stepsPerBar }: Pick<GridScene, 'length' | 'limit' | 'stepsPerBar'>,
) => {
  const height = RULER_HEIGHT
  const end = length * stepWidth - scrollX
  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = lanes.css('ruler')
  ctx.fillRect(0, 0, width, height)
  if(end < width) {
    ctx.fillStyle = colors.css('sunken', PAST_END_SHADE)
    ctx.fillRect(Math.max(0, end), 0, width - Math.max(0, end), height)
  }

  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.strokeStyle = lanes.css('rulerLine')
  hline(ctx, height - 1, 0, width)
  const first = Math.max(0, Math.floor(scrollX / stepWidth / STEPS_PER_BEAT) * STEPS_PER_BEAT)
  for(let step = first; step <= limit && step * stepWidth - scrollX <= width; step += STEPS_PER_BEAT) {
    const bar = step % stepsPerBar === 0
    vline(ctx, step * stepWidth - scrollX, bar ? 2 : height - 6, height)
  }
  ctx.stroke()

  ctx.font = '600 10px system-ui, sans-serif'
  ctx.textBaseline = 'middle'
  ctx.fillStyle = lanes.css('rulerBarText')
  for(let step = Math.floor(first / stepsPerBar) * stepsPerBar; step < limit; step += stepsPerBar) {
    const x = step * stepWidth - scrollX
    if(x > width) break
    if(x >= -40) ctx.fillText(String(step / stepsPerBar + 1), x + 4, height / 2)
  }

  if(end >= 0 && end <= width) {
    // a line and a flag pointing back along the pattern
    ctx.beginPath()
    ctx.lineWidth = 1.5
    ctx.strokeStyle = lanes.css('loopEnd')
    ctx.moveTo(end, 2)
    ctx.lineTo(end, height)
    ctx.stroke()
    ctx.beginPath()
    ctx.fillStyle = lanes.css('loopEnd')
    ctx.moveTo(end, 2)
    ctx.lineTo(end - 7, 5.5)
    ctx.lineTo(end, 9)
    ctx.fill()
  }
}

// the keyboard: white keys the whole width, black keys shorter over them,
// each C named. `pressed` is the key held down, if any
export const drawKeys = (
  ctx: CanvasRenderingContext2D,
  view: Pick<Viewport, 'height' | 'scrollY'>,
  pressed: number | null,
) => {
  const { height, scrollY } = view
  const width = KEYS_WIDTH
  // black keys dark against white ones in either appearance
  const dark = currentTheme()?.appearance === 'dark'
  const [low, high] = pitchesInView(view)

  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = colors.css('raised')
  ctx.fillRect(0, 0, width, height)

  ctx.lineWidth = 1
  for(let pitch = low; pitch <= high; pitch++) {
    const y = rowTop(pitch) - scrollY
    const black = isBlackKey(pitch)

    if(pitch === pressed) {
      ctx.fillStyle = colors.css('accentTintStrong')
      ctx.fillRect(0, y, black ? width * 0.6 : width, ROW_HEIGHT)
    } else if(black) {
      ctx.fillStyle = dark ? colors.css('sunken') : colors.css('inkMid')
      ctx.fillRect(0, y, width * 0.6, ROW_HEIGHT)
    }

    // the line under a white key, where the next one starts: after E and B,
    // which have no black key above them
    const note = pitch % 12
    if(note === 0 || note === 5) {
      ctx.beginPath()
      ctx.strokeStyle = colors.css('lineStrong')
      hline(ctx, y + ROW_HEIGHT - 1, 0, width)
      ctx.stroke()
    }

    if(note === 0) {
      ctx.font = '600 9px system-ui, sans-serif'
      ctx.textBaseline = 'middle'
      ctx.textAlign = 'right'
      ctx.fillStyle = colors.css('inkSoft')
      ctx.fillText(pitchName(pitch), width - 4, y + ROW_HEIGHT / 2)
      ctx.textAlign = 'left'
    }
  }

  ctx.beginPath()
  ctx.strokeStyle = colors.css('lineStrong')
  vline(ctx, width - 1, 0, height)
  ctx.stroke()
}
