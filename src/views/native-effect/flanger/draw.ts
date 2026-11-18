// drawing the Flanger's view, "the comb": what it does to each frequency
// (20 Hz to 20 kHz across, by octave; level up and down), from where its
// copies read now. as the sweep moves, the teeth glide; with feedback
// they grow into sharp ringing peaks. recent frames fade behind it, a
// trail: the swoosh. Tune marks its note's harmonics; Through-zero
// flashes as the copy passes the dry
import { painter } from '../../../theme/canvas'
import { blend, css } from '../modulation/motion'
import type { Area } from '../modulation/useLiveCanvas'
import { gainAt, noteHz, Settings, SideTaps } from './comb'

export const LOWEST = 20
export const HIGHEST = 20000
// level shown, dB
export const TOP_DB = 24
export const BOTTOM_DB = -30
const PAD = 16
// points across a curve, a pixel: the teeth get close together up high
const PER_PIXEL = 2
// frames kept for the trail
export const TRAIL = 10

export const freqToX = (hz: number, area: Area): number =>
  PAD + (Math.log(hz / LOWEST) / Math.log(HIGHEST / LOWEST)) * (area.width - 2 * PAD)

export const dbToY = (db: number, area: Area): number => {
  const clamped = Math.min(TOP_DB, Math.max(BOTTOM_DB, db))
  return PAD + ((TOP_DB - clamped) / (TOP_DB - BOTTOM_DB)) * (area.height - 2 * PAD)
}

// a curve: [x, y] points across the area
export type Curve = Array<[number, number]>

export const curveOf = (side: SideTaps, s: Settings, center: number, area: Area): Curve => {
  const points = Math.max(64, Math.round(area.width * PER_PIXEL))
  return Array.from({ length: points + 1 }, (_, i) => {
    const hz = LOWEST * (HIGHEST / LOWEST) ** (i / points)
    const db = 20 * Math.log10(Math.max(1e-6, gainAt(hz, side, s, center)))
    return [freqToX(hz, area), dbToY(db, area)]
  })
}

export interface Scene {
  area: Area
  settings: Settings
  // the left side's curve now, the right's (if Spread puts it elsewhere),
  // and earlier frames' left curves, oldest first
  left: Curve
  right: Curve | null
  trail: Curve[]
  // the sweep's centre, ms (Tune's period)
  centerMs: number
  // how near the copy is to passing the dry (1 at it), for Through-zero
  crossing: number
  caption: string
  hot: boolean
}

const stroke = (ctx: CanvasRenderingContext2D, curve: Curve) => {
  ctx.beginPath()
  curve.forEach(([x, y], i) => (i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y)))
  ctx.stroke()
}

export const drawFlanger = (ctx: CanvasRenderingContext2D, scene: Scene) => {
  const { area, settings: s } = scene
  const effect = painter('effect')
  ctx.clearRect(0, 0, area.width, area.height)
  ctx.fillStyle = effect.css('background')
  ctx.fillRect(0, 0, area.width, area.height)

  // Through-zero's flash, as the copy passes the dry
  if(s.zero && scene.crossing > 0.02) {
    ctx.fillStyle = effect.css('curveFill', 4 * scene.crossing)
    ctx.fillRect(0, 0, area.width, area.height)
  }

  // octaves, and 0 dB
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

  // Tune: the comb's fundamental (1/centre, its note) and harmonics, faint,
  // where the peaks ring
  if(noteHz(s.tune) !== null && scene.centerMs > 0) {
    const fundamental = 1000 / scene.centerMs
    ctx.setLineDash([2, 3])
    ctx.strokeStyle = effect.css('curve', 0.35)
    ctx.beginPath()
    for(let n = 1; n * fundamental <= HIGHEST && n <= 64; n += 1) {
      const x = Math.round(freqToX(n * fundamental, area)) + 0.5
      ctx.moveTo(x, 0)
      ctx.lineTo(x, area.height)
    }
    ctx.stroke()
    ctx.setLineDash([])
  }

  // the trail: older frames fainter
  const accent = effect.rgba('curve')
  const label = effect.rgba('label')
  ctx.lineWidth = 1
  scene.trail.forEach((curve, i) => {
    ctx.strokeStyle = css(blend(label, accent, 0.5), (0.25 * (i + 1)) / Math.max(1, scene.trail.length))
    stroke(ctx, curve)
  })

  // the right side, if it's elsewhere
  if(scene.right) {
    ctx.lineWidth = 1.25
    ctx.strokeStyle = css(accent, 0.45)
    stroke(ctx, scene.right)
  }

  // the left: filled under, a bright line
  ctx.fillStyle = effect.css('curveFill')
  ctx.beginPath()
  ctx.moveTo(scene.left[0][0], area.height)
  scene.left.forEach(([x, y]) => ctx.lineTo(x, y))
  ctx.lineTo(scene.left[scene.left.length - 1][0], area.height)
  ctx.closePath()
  ctx.fill()
  ctx.lineWidth = 1.75
  ctx.strokeStyle = effect.css('curve')
  ctx.shadowColor = css(accent, 0.5)
  ctx.shadowBlur = 4 + 6 * Math.abs(s.feedback)
  stroke(ctx, scene.left)
  ctx.shadowBlur = 0

  ctx.fillStyle = effect.css(scene.hot ? 'curve' : 'label')
  ctx.fillText(scene.caption, PAD, 12)
}
