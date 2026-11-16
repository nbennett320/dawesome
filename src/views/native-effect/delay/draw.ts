// drawing the Delay's echo view on a 2D canvas, in the theme's colors (the
// `effect` section): the time grid, the sound and its echoes as bars (left
// above the middle, right below), and what the time is set to
import { painter } from '../../../theme/canvas'
import { Echo, FLOOR_DB } from './echoes'
import { Area, barHeight, LEFT, middle, PAD, timeToX } from './geometry'

export const BAR_WIDTH = 6

export interface Scene {
  area: Area
  // seconds shown across
  span: number
  echoes: Echo[]
  // the dry sound's level, in dB
  dryDb: number
  // the grid in beats (a quarter note lasting `beat` seconds, a bar
  // `barQuarters` of them), or in time
  synced: boolean
  beat: number
  barQuarters: number
  caption: string
  // the first echoes stand out (the pointer's on them, or dragging them)
  hot: boolean
}

// labelled lines this many seconds apart, for a grid in time: the first
// that leaves at least `gap` pixels between them
const STEPS = [0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.25, 0.5, 1, 2]

const timeStep = (span: number, area: Area, gap: number): number =>
  STEPS.find(step => timeToX(step, span, area) - timeToX(0, span, area) >= gap) ?? 5

const timeLabel = (seconds: number): string =>
  (seconds >= 1 ? `${Number(seconds.toFixed(2))} s` : `${Math.round(seconds * 1000)} ms`)

const drawGrid = (ctx: CanvasRenderingContext2D, scene: Scene) => {
  const { area, span, synced, beat, barQuarters } = scene
  const effect = painter('effect')
  const x = (seconds: number) => Math.round(timeToX(seconds, span, area)) + 0.5
  const lines: Array<{ at: number, strong: boolean, label: string | null }> = []
  if(synced && beat > 0) {
    // a line each quarter note (if they're not too close), and each bar
    // stronger, numbered (as many as there's room to)
    const bar = beat * Math.max(0.5, barQuarters)
    if(x(beat) - x(0) >= 6) {
      for(let q = 0; q * beat <= span; q += 1) lines.push({ at: q * beat, strong: false, label: null })
    }
    const labelEvery = Math.max(1, Math.ceil(36 / Math.max(1, x(bar) - x(0))))
    for(let n = 0; n * bar <= span; n += 1) {
      lines.push({ at: n * bar, strong: true, label: n % labelEvery === 0 ? String(n + 1) : null })
    }
  } else {
    // a labelled line at round times far enough apart, and a faint one
    // halfway between
    const step = timeStep(span, area, 72)
    for(let i = 0; (i * step) / 2 <= span; i += 1) {
      const strong = i % 2 === 0
      lines.push({ at: (i * step) / 2, strong, label: strong && i > 0 ? timeLabel((i * step) / 2) : null })
    }
  }

  ctx.lineWidth = 1
  ;[false, true].forEach(strong => {
    ctx.strokeStyle = effect.css('grid', strong ? 1 : 0.5)
    ctx.beginPath()
    lines.filter(l => l.strong === strong).forEach(l => {
      ctx.moveTo(x(l.at), 0)
      ctx.lineTo(x(l.at), area.height)
    })
    ctx.stroke()
  })
  ctx.font = '10px system-ui, sans-serif'
  ctx.fillStyle = effect.css('label')
  lines.forEach(l => { if(l.label) ctx.fillText(l.label, x(l.at) + 3, area.height - 5) })

  // a dashed line at -12, -24 and -36 dB on each side, labelled on the left
  const mid = middle(area)
  ctx.setLineDash([2, 3])
  ctx.strokeStyle = effect.css('grid')
  ctx.beginPath()
  ;[-12, -24, -36].forEach(db => {
    const h = barHeight(db, area)
    ;[mid - h, mid + h].forEach(y => {
      ctx.moveTo(LEFT, Math.round(y) + 0.5)
      ctx.lineTo(area.width, Math.round(y) + 0.5)
    })
    ctx.fillText(String(db), 4, mid - h + 3)
  })
  ctx.stroke()
  ctx.setLineDash([])

  ctx.strokeStyle = effect.css('zero')
  ctx.beginPath()
  ctx.moveTo(0, Math.round(mid) + 0.5)
  ctx.lineTo(area.width, Math.round(mid) + 0.5)
  ctx.stroke()

  ctx.font = 'bold 10px system-ui, sans-serif'
  ctx.fillStyle = effect.css('label')
  ctx.fillText('L', 6, PAD - 4)
  ctx.fillText('R', 6, area.height - PAD + 12)
}

export const drawDelay = (ctx: CanvasRenderingContext2D, scene: Scene) => {
  const { area, span, echoes, dryDb, caption, hot } = scene
  const effect = painter('effect')
  const mid = middle(area)
  const x = (seconds: number) => timeToX(seconds, span, area)

  ctx.clearRect(0, 0, area.width, area.height)
  ctx.fillStyle = effect.css('background')
  ctx.fillRect(0, 0, area.width, area.height)

  drawGrid(ctx, scene)

  // the sound itself, at 0, on both sides
  const dry = barHeight(dryDb, area)
  if(dry > 0) {
    ctx.fillStyle = effect.css('label', 0.6)
    ctx.fillRect(x(0) - 2, mid - dry, 4, dry * 2)
  }

  // how the echoes fade, a faint line over each side's bars
  ctx.setLineDash([3, 3])
  ctx.lineWidth = 1
  ctx.strokeStyle = effect.css('curve', 0.4)
  ;([0, 1] as const).forEach(side => {
    const own = echoes.filter(e => e.side === side)
    if(own.length < 2) return
    ctx.beginPath()
    own.forEach((e, i) => {
      const y = side === 0 ? mid - barHeight(e.db, area) : mid + barHeight(e.db, area)
      if(i === 0) ctx.moveTo(x(e.time), y)
      else ctx.lineTo(x(e.time), y)
    })
    ctx.stroke()
  })
  ctx.setLineDash([])

  // the echoes: fainter as they fade
  echoes.forEach(e => {
    const h = barHeight(e.db, area)
    ctx.fillStyle = effect.css('curve', 0.35 + 0.65 * Math.max(0, 1 - e.db / FLOOR_DB))
    ctx.fillRect(x(e.time) - BAR_WIDTH / 2, e.side === 0 ? mid - h : mid, BAR_WIDTH, h)
  })

  // the first ones, ringed when they can be taken
  if(hot) {
    ctx.lineWidth = 2
    ctx.strokeStyle = effect.css('selectedRing')
    echoes.filter(e => e.first).forEach(e => {
      const h = Math.max(4, barHeight(e.db, area))
      ctx.strokeRect(x(e.time) - BAR_WIDTH / 2 - 2, e.side === 0 ? mid - h - 2 : mid - 2, BAR_WIDTH + 4, h + 4)
    })
  }

  ctx.font = '11px system-ui, sans-serif'
  ctx.textAlign = 'right'
  ctx.fillStyle = effect.css('selectedRing')
  ctx.fillText(caption, area.width - 8, 14)
  ctx.textAlign = 'start'
}
