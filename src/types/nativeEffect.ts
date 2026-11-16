// Dawesome's own effects' parameters, as the engine describes them. mirrors
// engine/src/native.rs (see context/effects-suite-project/04-api.md). a
// panel is drawn from these: each parameter says its range, how its knob is
// spread over it, and how its value is shown
import { clamp } from '../util/helpers'

export interface ParamValue {
  id: number
  // plain: Hz, dB, a choice's index, 0 or 1
  value: number
  // 0 to 1
  normalized: number
  // as it's shown: "2.50 kHz", "+3.0 dB", "Low shelf", "On"
  text: string
}

export interface ParamWire {
  // its place in the effect's list: never changes
  id: number
  // what projects save it as ("band3.freq")
  key: string
  name: string
  // the controls it's drawn with ("Band 3")
  group: string
  // "Hz", "dB", "%", "ms", "beats" or ""
  unit: string
  min: number
  max: number
  default: number
  defaultNormalized: number
  taper: 'linear' | 'log'
  // 0 continuous, 1 a switch, otherwise a choice of stepCount + 1 labels
  stepCount: number
  labels: string[]
  value: ParamValue
}

export interface EffectParams {
  // which built-in it is ("eq"): which panel draws it
  effect: string
  params: ParamWire[]
}

export type ParamKind = 'continuous' | 'switch' | 'choice'

export const paramKind = (param: Pick<ParamWire, 'stepCount'>): ParamKind => {
  if(param.stepCount === 0) return 'continuous'
  return param.stepCount === 1 ? 'switch' : 'choice'
}

// a normalized value as a plain one, as the engine works it out (so a knob
// being dragged can say where it is before the engine answers)
export const plainOf = (param: Pick<ParamWire, 'min' | 'max' | 'taper' | 'stepCount'>, normalized: number): number => {
  const n = clamp(normalized, 0, 1)
  const plain = param.taper === 'log'
    ? param.min * (param.max / param.min) ** n
    : param.min + n * (param.max - param.min)
  return param.stepCount > 0 ? Math.round(plain) : plain
}

export const normalizedOf = (param: Pick<ParamWire, 'min' | 'max' | 'taper'>, plain: number): number => {
  if(param.max <= param.min) return 0
  const p = clamp(plain, param.min, param.max)
  const n = param.taper === 'log'
    ? Math.log(p / param.min) / Math.log(param.max / param.min)
    : (p - param.min) / (param.max - param.min)
  return clamp(n, 0, 1)
}

// a choice's normalized value for its `index`th label
export const choiceNormalized = (param: Pick<ParamWire, 'stepCount'>, index: number): number =>
  (param.stepCount > 0 ? clamp(index, 0, param.stepCount) / param.stepCount : 0)

export interface ParamGroup {
  name: string
  params: ParamWire[]
}

// parameters by their group, in the order each group first comes
export const groupParams = (params: ParamWire[]): ParamGroup[] => {
  const groups: ParamGroup[] = []
  params.forEach(param => {
    const group = groups.find(g => g.name === param.group)
    if(group) group.params.push(param)
    else groups.push({ name: param.group, params: [param] })
  })
  return groups
}

// a built-in effect's output as a spectrum (get_effect_spectrum)
export interface EffectSpectrum {
  // frames it's put out so far: not going up means it isn't being run
  written: number
  rate: number
  // dB (a full-scale sine is 0, silence -120), from 20 Hz to 20 kHz evenly
  // by octave
  levels: number[]
}
