// drawing the Filter's view: its curve now (level up, 20 Hz to 20 kHz
// across), over the output's spectrum, with a trail as it moves. the range
// it moves through, Floor to Cutoff, is shaded, with a handle on each; the
// mode shows itself: a flash at each attack (Envelope), a level bar
// (Dynamic), a dial (LFO). Tune's notes are ticks along the bottom
import { painter } from '../../../theme/canvas'
import { blend, css } from '../modulation/motion'
import type { Area } from '../modulation/useLiveCanvas'
import { gainAt, MODE, Settings, tuneNotes } from './curve'

export const LOWEST = 20
export const HIGHEST = 20000
export const TOP_DB = 30
export const BOTTOM_DB = -48
const SPECTRUM_FLOOR = -90
export const PAD = 16
export const TRAIL = 8
// how near a press must be to a handle to take it
export const HIT = 10

export const freqToX = (hz: number, area: Area): number =>
  PAD + (Math.log(Math.max(1, hz) / LOWEST) / Math.log(HIGHEST / LOWEST)) * (area.width - 2 * PAD)

export const xToFreq = (x: number, area: Area): number =>
  LOWEST * (HIGHEST / LOWEST) ** Math.min(1, Math.max(0, (x - PAD) / Math.max(1, area.width - 2 * PAD)))

export const dbToY = (db: number, area: Area): number => {
  const clamped = Math.min(TOP_DB, Math.max(BOTTOM_DB - 6, db))
  return PAD + ((TOP_DB - clamped) / (TOP_DB - BOTTOM_DB)) * (area.height - 2 * PAD)
}

export type Curve = Array<[number, number]>

export const curveOf = (at: number, s: Settings, area: Area): Curve => {
  const steps = Math.max(32, Math.round(area.width / 2))
  return Array.from({ length: steps + 1 }, (_, i) => {
    const hz = LOWEST * (HIGHEST / LOWEST) ** (i / steps)
    return [freqToX(hz, area), dbToY(20 * Math.log10(Math.max(1e-6, gainAt(hz, at, s))), area)]
  })
}

export interface Scene {
  area: Area
  settings: Settings
  left: Curve
  right: Curve | null
  trail: Curve[]
  // where it is now (Hz), for its marker
  freq: number
  // the output's spectrum (dB, 20 Hz to 20 kHz by octave), or none
  spectrum: number[] | null
  // 0 to 1: an attack's flash (Envelope), the playing's level (Dynamic),
  // and the LFO's phase
  flash: number
  level: number
  phase: number
  caption: string
  // the handle the pointer's on or holding
  hot: 'floor' | 'cutoff' | null
}

const stroke = (ctx: CanvasRenderingContext2D, curve: Curve) => {
  ctx.beginPath()
  curve.forEach(([x, y], i) => (i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y)))
  ctx.stroke()
}

export const drawFilter = (ctx: CanvasRenderingContext2D, scene: Scene) => {
  const { area, settings: s } = scene
  const effect = painter('effect')
  const accent = effect.rgba('curve')
  ctx.clearRect(0, 0, area.width, area.height)
  ctx.fillStyle = effect.css('background')
  ctx.fillRect(0, 0, area.width, area.height)

  // an attack's flash
  if(scene.flash > 0.02) {
    ctx.fillStyle = effect.css('curveFill', 3 * scene.flash)
    ctx.fillRect(0, 0, area.width, area.height)
  }

  // the spectrum behind
  if(scene.spectrum && scene.spectrum.length > 1) {
    const levels = scene.spectrum
    ctx.beginPath()
    ctx.moveTo(PAD, area.height)
    levels.forEach((db, i) => {
      const hz = LOWEST * (HIGHEST / LOWEST) ** (i / (levels.length - 1))
      ctx.lineTo(freqToX(hz, area), area.height * Math.min(1, Math.max(0, db / SPECTRUM_FLOOR)))
    })
    ctx.lineTo(area.width - PAD, area.height)
    ctx.closePath()
    ctx.fillStyle = effect.css('spectrum')
    ctx.fill()
  }

  // octaves and 0 dB
  ctx.lineWidth = 1
  ctx.strokeStyle = effect.css('grid')
  ctx.font = '10px system-ui, sans-serif'
  ctx.fillStyle = effect.css('label')
  ctx.beginPath()
  ;[50, 100, 200, 500, 1000, 2000, 5000, 10000].forEach(hz => {
    const x = Math.round(freqToX(hz, area)) + 0.5
    ctx.moveTo(x, 0)
    ctx.lineTo(x, area.height)
  })
  ctx.stroke()
  ;[100, 1000, 10000].forEach(hz => ctx.fillText(hz >= 1000 ? `${hz / 1000}k` : `${hz}`, freqToX(hz, area) + 3, area.height - 4))
  ctx.strokeStyle = effect.css('zero')
  ctx.beginPath()
  const zero = Math.round(dbToY(0, area)) + 0.5
  ctx.moveTo(0, zero)
  ctx.lineTo(area.width, zero)
  ctx.stroke()

  // the range it moves through, and its two ends
  const moving = s.mode !== MODE.OFF
  const [xf, xc] = [freqToX(s.floor, area), freqToX(s.cutoff, area)]
  if(moving) {
    ctx.fillStyle = effect.css('curveFill', 1.5)
    ctx.fillRect(Math.min(xf, xc), 0, Math.abs(xc - xf), area.height)
  }
  const handle = (x: number, which: 'floor' | 'cutoff', dashed: boolean) => {
    ctx.setLineDash(dashed ? [3, 3] : [])
    ctx.strokeStyle = effect.css(scene.hot === which ? 'selectedRing' : 'curve', scene.hot === which ? 1 : 0.6)
    ctx.beginPath()
    ctx.moveTo(Math.round(x) + 0.5, PAD)
    ctx.lineTo(Math.round(x) + 0.5, area.height - PAD)
    ctx.stroke()
    ctx.setLineDash([])
    ctx.fillStyle = effect.css('curve')
    ctx.beginPath()
    ctx.arc(x, PAD, scene.hot === which ? 6 : 5, 0, 2 * Math.PI)
    ctx.fill()
    ctx.fillStyle = effect.css('dotText')
    ctx.fillText(which === 'floor' ? 'F' : 'C', x - 3, PAD + 3.5)
  }
  if(moving) handle(xf, 'floor', true)
  handle(xc, 'cutoff', false)

  // Tune's notes: ticks along the bottom
  const notes = tuneNotes(s.tune, LOWEST, HIGHEST)
  if(notes.length > 0) {
    ctx.strokeStyle = effect.css('curve', 0.4)
    ctx.beginPath()
    notes.forEach(hz => {
      const x = Math.round(freqToX(hz, area)) + 0.5
      ctx.moveTo(x, area.height - PAD)
      ctx.lineTo(x, area.height - PAD - 5)
    })
    ctx.stroke()
  }

  // the trail, the right side (Spin), then the curve
  ctx.lineWidth = 1
  scene.trail.forEach((curve, i) => {
    ctx.strokeStyle = css(blend(effect.rgba('label'), accent, 0.5), (0.3 * (i + 1)) / Math.max(1, scene.trail.length))
    stroke(ctx, curve)
  })
  if(scene.right) {
    ctx.lineWidth = 1.25
    ctx.strokeStyle = css(accent, 0.45)
    stroke(ctx, scene.right)
  }
  ctx.fillStyle = effect.css('curveFill')
  ctx.beginPath()
  ctx.moveTo(scene.left[0][0], area.height)
  scene.left.forEach(([x, y]) => ctx.lineTo(x, y))
  ctx.lineTo(scene.left[scene.left.length - 1][0], area.height)
  ctx.closePath()
  ctx.fill()
  ctx.lineWidth = 2
  ctx.strokeStyle = effect.css('curve')
  ctx.shadowColor = css(accent, 0.5)
  ctx.shadowBlur = 4 + 8 * Math.min(1, s.resonance / 100)
  stroke(ctx, scene.left)
  ctx.shadowBlur = 0

  // where it is now: a dot on the curve's line, at the frequency
  ctx.fillStyle = effect.css('curve')
  ctx.beginPath()
  ctx.arc(freqToX(scene.freq, area), dbToY(20 * Math.log10(Math.max(1e-6, gainAt(scene.freq, scene.freq, s))), area), 3.5, 0, 2 * Math.PI)
  ctx.fill()

  // the mode's own sign: Dynamic's level, the LFO's dial
  if(s.mode === MODE.DYNAMIC) {
    const h = (area.height - 2 * PAD) * Math.min(1, scene.level)
    ctx.fillStyle = effect.css('curve', 0.7)
    ctx.fillRect(area.width - PAD + 4, area.height - PAD - h, 4, h)
  } else if(s.mode === MODE.LFO) {
    const r = 8
    const [cx, cy] = [area.width - r - 8, area.height - r - 18]
    ctx.strokeStyle = effect.css('grid')
    ctx.beginPath()
    ctx.arc(cx, cy, r, 0, 2 * Math.PI)
    ctx.stroke()
    const angle = 2 * Math.PI * scene.phase - Math.PI / 2
    ctx.fillStyle = effect.css('curve')
    ctx.beginPath()
    ctx.arc(cx + r * Math.cos(angle), cy + r * Math.sin(angle), 2.5, 0, 2 * Math.PI)
    ctx.fill()
  }

  ctx.fillStyle = effect.css(scene.hot ? 'curve' : 'label')
  ctx.fillText(scene.caption, PAD, 12)
}
