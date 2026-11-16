// what the EQ does at each frequency, for drawing its curve: the same maths
// as the engine's filters (engine/crates/dsp/src/svf.rs, Coefs::new and
// response), so the curve is what's heard. response.spec.ts checks this
// against the engine's own numbers (engine/crates/dsp/fixtures/eq-response.json)
import { ParamWire } from '../../../types/nativeEffect'

// as the shape parameter lists them (dsp's eq::SHAPES)
export enum Shape {
  Bell = 0,
  LowShelf = 1,
  HighShelf = 2,
  LowCut = 3,
  HighCut = 4,
  Notch = 5,
  BandPass = 6,
}

export const BANDS = 8
// each band's parameters, from band * PER_BAND (dsp's eq.rs)
export const PER_BAND = 5
export const ON = 0
export const SHAPE = 1
export const FREQ = 2
export const GAIN = 3
export const Q = 4
export const OUTPUT = BANDS * PER_BAND

// the curve is drawn as at this rate (the engine's may differ: it only
// shows near the top of the range)
export const DRAW_RATE = 48000
// (svf.rs's MAX_SHARE)
const MAX_SHARE = 0.49

export const paramId = (band: number, which: number): number => band * PER_BAND + which

// whether a shape has a gain (the others sit on 0 dB)
export const hasGain = (shape: Shape): boolean =>
  shape === Shape.Bell || shape === Shape.LowShelf || shape === Shape.HighShelf

export interface BandSetting {
  shape: Shape
  freq: number
  gain: number
  q: number
}

interface Coefs {
  g: number
  k: number
  m0: number
  m1: number
  m2: number
}

const coefs = ({ shape, freq, gain, q }: BandSetting, rate: number): Coefs => {
  const f = Math.min(Math.max(freq, 1), rate * MAX_SHARE)
  const qq = Math.max(q, 0.01)
  const g = Math.tan(Math.PI * f / rate)
  const a = 10 ** (gain / 40)
  const k = 1 / qq
  switch(shape) {
    case Shape.Bell: {
      const kb = 1 / (qq * a)
      return { g, k: kb, m0: 1, m1: kb * (a * a - 1), m2: 0 }
    }
    case Shape.LowShelf: return { g: g / Math.sqrt(a), k, m0: 1, m1: k * (a - 1), m2: a * a - 1 }
    case Shape.HighShelf: return { g: g * Math.sqrt(a), k, m0: a * a, m1: k * (1 - a) * a, m2: 1 - a * a }
    case Shape.LowCut: return { g, k, m0: 1, m1: -k, m2: -1 }
    case Shape.HighCut: return { g, k, m0: 0, m1: 0, m2: 1 }
    case Shape.Notch: return { g, k, m0: 1, m1: -k, m2: 0 }
    default: return { g, k, m0: 0, m1: k, m2: 0 }
  }
}

// one band's gain at `freq`, as an amplitude
export const bandResponse = (band: BandSetting, freq: number, rate = DRAW_RATE): number => {
  const c = coefs(band, rate)
  const w = Math.tan(Math.PI * Math.min(freq, rate * 0.4999) / rate) / c.g
  const dr = 1 - w * w
  const di = c.k * w
  const d = dr * dr + di * di
  const [lr, li] = [dr / d, -di / d]
  const [br, bi] = [w * di / d, w * dr / d]
  const re = c.m0 + c.m1 * br + c.m2 * lr
  const im = c.m1 * bi + c.m2 * li
  return Math.sqrt(re * re + im * im)
}

export const toDb = (amplitude: number): number => Math.max(20 * Math.log10(amplitude), -200)

// a band as the EQ's parameters have it
export interface Band extends BandSetting {
  index: number
  on: boolean
}

// the 8 bands, from the EQ's parameters (in id order)
export const bandsOf = (params: ParamWire[]): Band[] =>
  Array.from({ length: BANDS }, (_, index) => {
    const value = (which: number) => params[paramId(index, which)]?.value.value ?? 0
    return {
      index,
      on: value(ON) >= 0.5,
      shape: Math.round(value(SHAPE)) as Shape,
      freq: value(FREQ),
      gain: value(GAIN),
      q: value(Q),
    }
  })

// the whole EQ's gain at `freq`, in dB: every band that's on, and the output
export const totalDb = (bands: Band[], output: number, freq: number): number =>
  bands.filter(b => b.on).reduce((db, b) => db + toDb(bandResponse(b, freq)), output)
