// drawing the EQ's curve area on a 2D canvas, in the theme's colors (the
// `effect` section, and the `track` palette for the bands)
import { painter } from '../../../theme/canvas'
import {
  Area, DB_LINES, dbToY, dotOf, FREQ_LINES, freqLabel, freqToX, MAX_FREQ, MIN_FREQ,
} from './geometry'
import { Band, bandResponse, toDb, totalDb } from './response'

// the bands' colors, in order (band 1 is red)
export const BAND_COLORS = ['red', 'orange', 'yellow', 'green', 'teal', 'blue', 'violet', 'pink'] as const

export const DOT_RADIUS = 9

export interface Scene {
  area: Area
  bands: Band[]
  outputDb: number
  selected: number | null
  // what the selected band is set to, as it's shown ("Bell · 2.50 kHz · …")
  caption: string | null
  // the output's spectrum, in dB from 20 Hz to 20 kHz evenly by octave (see
  // spectrum.ts), drawn behind everything. none: not drawn
  spectrum: number[] | null
}

// the spectrum's own scale, top to bottom of the area (apart from the
// curve's ±24 dB): 0 dB at the top, SPECTRUM_FLOOR at the bottom
export const SPECTRUM_FLOOR = -90

const spectrumY = (db: number, area: Area): number =>
  area.height * Math.min(1, Math.max(0, db / SPECTRUM_FLOOR))

const drawSpectrum = (ctx: CanvasRenderingContext2D, area: Area, levels: number[]) => {
  if(levels.length < 2) return
  ctx.beginPath()
  ctx.moveTo(0, area.height)
  levels.forEach((db, i) => ctx.lineTo((i / (levels.length - 1)) * area.width, spectrumY(db, area)))
  ctx.lineTo(area.width, area.height)
  ctx.closePath()
  ctx.fillStyle = painter('effect').css('spectrum')
  ctx.fill()
}

// a curve across the area: `db` at each frequency
const trace = (ctx: CanvasRenderingContext2D, area: Area, db: (freq: number) => number) => {
  const steps = Math.max(2, Math.ceil(area.width / 2))
  ctx.beginPath()
  for(let i = 0; i <= steps; i += 1) {
    const x = (i / steps) * area.width
    const freq = MIN_FREQ * (MAX_FREQ / MIN_FREQ) ** (i / steps)
    // (deep cuts go off the bottom)
    const y = Math.min(area.height + 2, dbToY(db(freq), area))
    if(i === 0) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  }
}

export const drawEq = (ctx: CanvasRenderingContext2D, scene: Scene) => {
  const { area, bands, outputDb, selected, caption, spectrum } = scene
  const effect = painter('effect')
  const track = painter('track')
  const zero = dbToY(0, area)

  ctx.clearRect(0, 0, area.width, area.height)
  ctx.fillStyle = effect.css('background')
  ctx.fillRect(0, 0, area.width, area.height)

  if(spectrum) drawSpectrum(ctx, area, spectrum)

  // the grid, and its labels
  ctx.lineWidth = 1
  ctx.font = '10px system-ui, sans-serif'
  ctx.fillStyle = effect.css('label')
  ctx.strokeStyle = effect.css('grid')
  ctx.beginPath()
  FREQ_LINES.forEach(freq => {
    const x = Math.round(freqToX(freq, area)) + 0.5
    ctx.moveTo(x, 0)
    ctx.lineTo(x, area.height)
  })
  DB_LINES.forEach(db => {
    const y = Math.round(dbToY(db, area)) + 0.5
    ctx.moveTo(0, y)
    ctx.lineTo(area.width, y)
  })
  ctx.stroke()
  FREQ_LINES.forEach(freq => ctx.fillText(freqLabel(freq), freqToX(freq, area) + 3, area.height - 4))
  DB_LINES.forEach(db => ctx.fillText(`${db > 0 ? '+' : ''}${db}`, 4, dbToY(db, area) - 3))
  ctx.strokeStyle = effect.css('zero')
  ctx.beginPath()
  ctx.moveTo(0, Math.round(zero) + 0.5)
  ctx.lineTo(area.width, Math.round(zero) + 0.5)
  ctx.stroke()

  // the selected band on its own, dashed, over a wash of its color
  const chosen = selected === null ? null : bands[selected]
  if(chosen?.on) {
    const color = BAND_COLORS[chosen.index]
    const own = (freq: number) => toDb(bandResponse(chosen, freq))
    trace(ctx, area, own)
    ctx.lineTo(area.width, zero)
    ctx.lineTo(0, zero)
    ctx.closePath()
    ctx.fillStyle = track.css(color, 0.15)
    ctx.fill()
    trace(ctx, area, own)
    ctx.setLineDash([4, 3])
    ctx.strokeStyle = track.css(color)
    ctx.stroke()
    ctx.setLineDash([])
  }

  // the whole EQ
  const total = (freq: number) => totalDb(bands, outputDb, freq)
  trace(ctx, area, total)
  ctx.lineTo(area.width, zero)
  ctx.lineTo(0, zero)
  ctx.closePath()
  ctx.fillStyle = effect.css('curveFill')
  ctx.fill()
  trace(ctx, area, total)
  ctx.lineWidth = 2.5
  ctx.strokeStyle = effect.css('curve')
  ctx.stroke()

  // the dots: the selected one last, on top
  const on = bands.filter(b => b.on)
  const order = [...on.filter(b => b.index !== selected), ...on.filter(b => b.index === selected)]
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.font = 'bold 10px system-ui, sans-serif'
  order.forEach(band => {
    const { x, y } = dotOf(band, area)
    const isSelected = band.index === selected
    ctx.beginPath()
    ctx.arc(x, y, isSelected ? DOT_RADIUS + 2 : DOT_RADIUS, 0, Math.PI * 2)
    ctx.fillStyle = track.css(BAND_COLORS[band.index])
    ctx.fill()
    ctx.lineWidth = 2
    ctx.strokeStyle = effect.css(isSelected ? 'selectedRing' : 'dotRing')
    ctx.stroke()
    ctx.fillStyle = effect.css('dotText')
    ctx.fillText(String(band.index + 1), x, y + 0.5)
  })

  // what the selected band is set to, beside its dot (kept inside the area)
  if(chosen?.on && caption) {
    const { x, y } = dotOf(chosen, area)
    ctx.font = '11px system-ui, sans-serif'
    const width = ctx.measureText(caption).width
    const left = x + 16 + width > area.width ? x - 16 - width : x + 16
    const top = y - 16 < 10 ? y + 18 : y - 16
    ctx.textAlign = 'left'
    ctx.fillStyle = effect.css('selectedRing')
    ctx.fillText(caption, Math.max(4, left), top)
  }
  ctx.textAlign = 'start'
  ctx.textBaseline = 'alphabetic'
}
