// drawing the Piano Roll: its grid of steps and notes, the time ruler above
// it and the keyboard down its left, each on a plain canvas. only what's in
// view is drawn, and only when something changes, so a pattern with
// hundreds of notes stays smooth. the colours come from the theme
import { painter } from '../../theme/canvas'
import { currentTheme } from '../../theme/apply'
import { STEPS_PER_BEAT, Note } from '../../types/sequencer'
import { HIGHEST_PITCH, PITCHES, isBlackKey, pitchName } from './notes'

const colors = painter('colors')
const lanes = painter('playlist')

// how big a step and a semitone are on the grid, in CSS pixels
export const STEP_WIDTH = 24
export const ROW_HEIGHT = 14
// the keyboard's width, and the ruler's height
export const KEYS_WIDTH = 56
export const RULER_HEIGHT = 20
// how much of a note's right end resizes it rather than moving it
export const EDGE_PX = 6

// the grid's whole size, of which a part is in view
export const contentHeight = (): number => PITCHES * ROW_HEIGHT
export const contentWidth = (length: number): number => Math.max(0, length) * STEP_WIDTH

// a pitch's row, and the pitch at a height on the grid (both from the top
// of the whole grid, not of what's in view)
export const rowTop = (pitch: number): number => (HIGHEST_PITCH - pitch) * ROW_HEIGHT
export const pitchAtY = (y: number): number => HIGHEST_PITCH - Math.floor(y / ROW_HEIGHT)
export const stepAtX = (x: number): number => x / STEP_WIDTH

// the part of the grid in view: its size, and how far it's scrolled
export interface Viewport {
  width: number
  height: number
  scrollX: number
  scrollY: number
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
const pitchesInView = ({ height, scrollY }: Viewport): [number, number] => [
  Math.max(0, HIGHEST_PITCH - Math.floor((scrollY + height) / ROW_HEIGHT)),
  Math.min(HIGHEST_PITCH, HIGHEST_PITCH - Math.floor(scrollY / ROW_HEIGHT)),
]

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
  // the pattern's length, in steps
  length: number
  stepsPerBar: number
  notes: Note[]
  selected: Set<number>
  // a rubber band being dragged, from corner to corner on the grid
  box: { x0: number, y0: number, x1: number, y1: number } | null
}

export const drawGrid = (ctx: CanvasRenderingContext2D, view: Viewport, scene: GridScene) => {
  const { width, height, scrollX, scrollY } = view
  const { length, stepsPerBar, notes, selected, box } = scene
  const right = Math.min(width, contentWidth(length) - scrollX)

  ctx.clearRect(0, 0, width, height)
  // past the pattern's end
  ctx.fillStyle = colors.css('sunken')
  ctx.fillRect(0, 0, width, height)
  ctx.fillStyle = lanes.css('background')
  ctx.fillRect(0, 0, right, height)

  // a row per semitone, the black keys' shaded
  const [low, high] = pitchesInView(view)
  ctx.fillStyle = colors.css('panel')
  for(let pitch = low; pitch <= high; pitch++) {
    if(isBlackKey(pitch)) ctx.fillRect(0, rowTop(pitch) - scrollY, right, ROW_HEIGHT)
  }

  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.strokeStyle = lanes.css('laneLine')
  for(let pitch = low; pitch <= high; pitch++) {
    // between B and C, an octave apart, is drawn stronger below
    if(pitch % 12 !== 0) hline(ctx, rowTop(pitch) + ROW_HEIGHT - scrollY, 0, right)
  }
  ctx.stroke()

  // a column per step, beats and bars stronger
  const first = Math.max(0, Math.floor(scrollX / STEP_WIDTH))
  const last = Math.min(length, Math.ceil((scrollX + width) / STEP_WIDTH))
  const columns = (strokeStyle: string, keep: (step: number) => boolean) => {
    ctx.beginPath()
    ctx.strokeStyle = strokeStyle
    for(let step = first; step <= last; step++) {
      if(keep(step)) vline(ctx, step * STEP_WIDTH - scrollX, 0, height)
    }
    ctx.stroke()
  }
  const bar = (step: number) => step % stepsPerBar === 0
  const beat = (step: number) => step % STEPS_PER_BEAT === 0
  columns(lanes.css('laneLine', 0.6), step => !beat(step) && !bar(step))
  columns(lanes.css('beatLine'), step => beat(step) && !bar(step))

  ctx.beginPath()
  ctx.strokeStyle = lanes.css('barLine')
  for(let pitch = low; pitch <= high; pitch++) {
    if(pitch % 12 === 0) hline(ctx, rowTop(pitch) + ROW_HEIGHT - scrollY, 0, right)
  }
  ctx.stroke()
  columns(lanes.css('barLine'), bar)

  // the notes, the selected ones in the accent colour
  const left = scrollX - STEP_WIDTH
  const leftStep = left / STEP_WIDTH
  const rightStep = (scrollX + width) / STEP_WIDTH
  notes.forEach((note, i) => {
    if(note.pitch < low || note.pitch > high) return
    if(note.start + note.length < leftStep || note.start > rightStep) return

    const x = note.start * STEP_WIDTH - scrollX
    const y = rowTop(note.pitch) - scrollY
    const w = Math.max(3, note.length * STEP_WIDTH - 1)
    const on = selected.has(i)

    ctx.fillStyle = on ? lanes.css('selectedBorder') : lanes.css('patternHit')
    ctx.fillRect(x + 0.5, y + 1, w, ROW_HEIGHT - 2)
    ctx.strokeStyle = on ? lanes.css('selectedClip') : lanes.css('patternBorder')
    ctx.strokeRect(x + 1, y + 1.5, w - 1, ROW_HEIGHT - 3)
  })

  if(box) {
    const x = Math.min(box.x0, box.x1) - scrollX
    const y = Math.min(box.y0, box.y1) - scrollY
    const w = Math.abs(box.x1 - box.x0)
    const h = Math.abs(box.y1 - box.y0)
    ctx.fillStyle = lanes.css('lassoFill', 0.12)
    ctx.fillRect(x, y, w, h)
    ctx.strokeStyle = lanes.css('lassoFill')
    ctx.strokeRect(Math.round(x) + 0.5, Math.round(y) + 0.5, Math.round(w), Math.round(h))
  }
}

// the time ruler: a number at each bar, a tick at each beat
export const drawRuler = (
  ctx: CanvasRenderingContext2D,
  { width, scrollX }: Pick<Viewport, 'width' | 'scrollX'>,
  length: number,
  stepsPerBar: number,
) => {
  const height = RULER_HEIGHT
  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = lanes.css('ruler')
  ctx.fillRect(0, 0, width, height)

  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.strokeStyle = lanes.css('rulerLine')
  hline(ctx, height - 1, 0, width)
  const first = Math.max(0, Math.floor(scrollX / STEP_WIDTH / STEPS_PER_BEAT) * STEPS_PER_BEAT)
  for(let step = first; step <= length && step * STEP_WIDTH - scrollX <= width; step += STEPS_PER_BEAT) {
    const bar = step % stepsPerBar === 0
    vline(ctx, step * STEP_WIDTH - scrollX, bar ? 2 : height - 6, height)
  }
  ctx.stroke()

  ctx.font = '600 10px system-ui, sans-serif'
  ctx.textBaseline = 'middle'
  ctx.fillStyle = lanes.css('rulerBarText')
  for(let step = Math.floor(first / stepsPerBar) * stepsPerBar; step < length; step += stepsPerBar) {
    const x = step * STEP_WIDTH - scrollX
    if(x > width) break
    if(x >= -40) ctx.fillText(String(step / stepsPerBar + 1), x + 4, height / 2)
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
  const [low, high] = pitchesInView({ ...view, width, scrollX: 0 })

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
