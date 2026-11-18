// drawing the Phaser's view, "the swirl": what it does to each frequency,
// wrapped round a circle: frequency is the angle (20 Hz at the top,
// clockwise to 20 kHz), level the distance out. the notches are dents in
// the ring, and they turn as the sweep moves. left and right are two
// rings, apart by Spread. the hue turns with the LFO and recent frames
// fade behind, so it swirls. in Vibe mode a lamp glows in the middle,
// throbbing with its LFO
import { painter } from '../../../theme/canvas'
import { blend, css, spectrumAt } from '../modulation/motion'
import type { Area } from '../modulation/useLiveCanvas'
import { gainAt, Settings, SideNotches } from './notches'

export const LOWEST = 20
export const HIGHEST = 20000
const TOP_DB = 18
const BOTTOM_DB = -36
const POINTS = 360
export const TRAIL = 8

export interface Ring {
  points: Array<[number, number]>
}

// the circle's middle and the largest ring's radius
export const geometry = (area: Area) => {
  const radius = Math.max(10, Math.min(area.width, area.height) / 2 - 14)
  return { cx: area.width / 2, cy: area.height / 2, radius }
}

// where `hz` is round the circle (radians, 0 at the top, clockwise)
export const angleOf = (hz: number): number =>
  (2 * Math.PI * Math.log(hz / LOWEST)) / Math.log(HIGHEST / LOWEST)

// how far out a level is, as a share of the radius (a quarter at the bottom
// of the range, all of it at the top)
export const reachOf = (db: number): number => {
  const clamped = Math.min(TOP_DB, Math.max(BOTTOM_DB, db))
  return 0.25 + (0.75 * (clamped - BOTTOM_DB)) / (TOP_DB - BOTTOM_DB)
}

export const ringOf = (side: SideNotches, s: Settings, rate: number, area: Area): Ring => {
  const { cx, cy, radius } = geometry(area)
  return {
    points: Array.from({ length: POINTS }, (_, i) => {
      const hz = LOWEST * (HIGHEST / LOWEST) ** (i / POINTS)
      const db = 20 * Math.log10(Math.max(1e-6, gainAt(hz, side, s, rate)))
      const angle = angleOf(hz) - Math.PI / 2
      const r = radius * reachOf(db)
      return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)]
    }),
  }
}

export interface Scene {
  area: Area
  settings: Settings
  left: Ring
  right: Ring | null
  trail: Ring[]
  // the LFO's phase (the hue turns with it), and the lamp's brightness (0
  // to 1, Vibe mode)
  phase: number
  glow: number
  caption: string
  hot: boolean
}

const trace = (ctx: CanvasRenderingContext2D, ring: Ring) => {
  ctx.beginPath()
  ring.points.forEach(([x, y], i) => (i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y)))
  ctx.closePath()
}

export const drawPhaser = (ctx: CanvasRenderingContext2D, scene: Scene) => {
  const { area } = scene
  const effect = painter('effect')
  const { cx, cy, radius } = geometry(area)
  ctx.clearRect(0, 0, area.width, area.height)
  ctx.fillStyle = effect.css('background')
  ctx.fillRect(0, 0, area.width, area.height)

  // the lamp, in Vibe mode: rings of light, brighter as it lights
  if(scene.glow > 0.01) {
    for(let i = 4; i >= 1; i -= 1) {
      ctx.fillStyle = effect.css('curve', 0.1 * scene.glow)
      ctx.beginPath()
      ctx.arc(cx, cy, radius * 0.22 * (i / 4) * (0.6 + 0.4 * scene.glow), 0, 2 * Math.PI)
      ctx.fill()
    }
  }

  // 0 dB, and the decades round it
  ctx.lineWidth = 1
  ctx.strokeStyle = effect.css('zero')
  ctx.beginPath()
  ctx.arc(cx, cy, radius * reachOf(0), 0, 2 * Math.PI)
  ctx.stroke()
  ctx.strokeStyle = effect.css('grid')
  ctx.font = '10px system-ui, sans-serif'
  ctx.fillStyle = effect.css('label')
  ctx.beginPath()
  ;[100, 1000, 10000].forEach(hz => {
    const a = angleOf(hz) - Math.PI / 2
    ctx.moveTo(cx + radius * 0.2 * Math.cos(a), cy + radius * 0.2 * Math.sin(a))
    ctx.lineTo(cx + radius * 1.04 * Math.cos(a), cy + radius * 1.04 * Math.sin(a))
  })
  ctx.stroke()
  ;[100, 1000, 10000].forEach(hz => {
    const a = angleOf(hz) - Math.PI / 2
    ctx.fillText(hz >= 1000 ? `${hz / 1000}k` : `${hz}`, cx + radius * 1.08 * Math.cos(a) - 8, cy + radius * 1.08 * Math.sin(a) + 3)
  })

  // the hue, turning with the LFO
  const accent = effect.rgba('curve')
  const hue = blend(accent, spectrumAt(scene.phase), 0.9)

  // the trail: older frames fainter, in the hues they had
  scene.trail.forEach((ring, i) => {
    ctx.strokeStyle = css(blend(accent, spectrumAt(scene.phase - 0.02 * (scene.trail.length - i)), 0.9), (0.3 * (i + 1)) / scene.trail.length)
    trace(ctx, ring)
    ctx.stroke()
  })

  if(scene.right) {
    ctx.lineWidth = 1.25
    ctx.strokeStyle = css(blend(accent, spectrumAt(scene.phase + 0.25), 0.9), 0.55)
    trace(ctx, scene.right)
    ctx.stroke()
  }

  ctx.fillStyle = css(hue, 0.12)
  trace(ctx, scene.left)
  ctx.fill()
  ctx.lineWidth = 2
  ctx.strokeStyle = css(hue)
  ctx.shadowColor = css(hue, 0.6)
  ctx.shadowBlur = 8
  trace(ctx, scene.left)
  ctx.stroke()
  ctx.shadowBlur = 0

  ctx.fillStyle = effect.css(scene.hot ? 'curve' : 'label')
  ctx.fillText(scene.caption, 10, 14)
}
