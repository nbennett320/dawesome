// where things are on the EQ's curve: frequency across (20 Hz to 20 kHz,
// evenly by octave) and gain up (±24 dB), in the area's own pixels. these
// are the same spreads as the band's freq and gain parameters, so a place on
// the curve is a pair of normalized values
import { Band, hasGain, Shape } from './response'

export const MIN_FREQ = 20
export const MAX_FREQ = 20000
export const MAX_DB = 24
// room above and below ±24 dB, so a dot there isn't cut in half
export const PAD = 12
// how near a press must be to a dot to take it
export const HIT = 12

export interface Area {
  width: number
  height: number
}

// across: 0 to 1 is the freq parameter's normalized value
export const freqToX = (freq: number, area: Area): number =>
  (Math.log(freq / MIN_FREQ) / Math.log(MAX_FREQ / MIN_FREQ)) * area.width

export const xToFreqNormalized = (x: number, area: Area): number =>
  Math.min(1, Math.max(0, x / Math.max(1, area.width)))

export const xToFreq = (x: number, area: Area): number =>
  MIN_FREQ * (MAX_FREQ / MIN_FREQ) ** xToFreqNormalized(x, area)

const span = (area: Area) => Math.max(1, area.height - 2 * PAD)

export const dbToY = (db: number, area: Area): number =>
  PAD + ((MAX_DB - db) / (2 * MAX_DB)) * span(area)

// up: 0 to 1 is the gain parameter's normalized value
export const yToGainNormalized = (y: number, area: Area): number =>
  Math.min(1, Math.max(0, 1 - (y - PAD) / span(area)))

export const yToDb = (y: number, area: Area): number => yToGainNormalized(y, area) * 2 * MAX_DB - MAX_DB

// where a band's dot is: at its frequency, and its gain (0 dB for the
// shapes without one)
export const dotOf = (band: Band, area: Area): { x: number, y: number } => ({
  x: freqToX(band.freq, area),
  y: dbToY(hasGain(band.shape) ? band.gain : 0, area),
})

// the band whose dot a press at (x, y) takes: the nearest of those that are
// on, within HIT. the selected one wins a tie (it's drawn on top)
export const hitBand = (bands: Band[], x: number, y: number, area: Area, selected: number | null): number | null => {
  let best: { index: number, distance: number } | null = null
  bands.filter(b => b.on).forEach(band => {
    const dot = dotOf(band, area)
    const distance = Math.hypot(dot.x - x, dot.y - y)
    if(distance > HIT) return
    const better = !best || distance < best.distance || (distance === best.distance && band.index === selected)
    if(better) best = { index: band.index, distance }
  })
  return (best as { index: number } | null)?.index ?? null
}

// a new band's shape, by where it's added: a cut at either end, a bell
// anywhere else
export const LOW_CUT_BELOW = 40
export const HIGH_CUT_ABOVE = 15000
export const newBandShape = (freq: number): Shape => {
  if(freq < LOW_CUT_BELOW) return Shape.LowCut
  if(freq > HIGH_CUT_ABOVE) return Shape.HighCut
  return Shape.Bell
}

// the first band that's off, for a new one (none when all 8 are on)
export const freeBand = (bands: Band[]): number | null => bands.find(b => !b.on)?.index ?? null

// the frequencies and gains the grid marks
export const FREQ_LINES = [50, 100, 200, 500, 1000, 2000, 5000, 10000]
export const DB_LINES = [-18, -12, -6, 6, 12, 18]

export const freqLabel = (freq: number): string => (freq >= 1000 ? `${freq / 1000}k` : String(freq))
