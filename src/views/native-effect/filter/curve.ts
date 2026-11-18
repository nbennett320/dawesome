// the Filter's curve as its settings and readout make it: where its
// frequency is now, and what it does at each frequency. the same rules as
// the engine's (engine/crates/dsp/src/effects/filter.rs), worked out here
// so the view moves every frame between readouts
import { ParamWire } from '../../../types/nativeEffect'
import { bandResponse, Shape } from '../eq/response'
import { Live, sine, triangle, valueOf } from '../modulation/motion'

// the readout's slots (filter.rs OUT_*)
export const SLOT = { PHASE: 0, HZ: 1, FREQ_L: 2, FREQ_R: 3, M_L: 4, M_R: 5, LEVEL: 6, ENVELOPE: 7, TRIGGERS: 8 } as const

export const MODE = { OFF: 0, LFO: 1, ENVELOPE: 2, DYNAMIC: 3 } as const
export const SAMPLE_HOLD = 5
export const Q_LEAST = 0.707
export const Q_MOST = 25
export const Q_24: [number, number] = [0.541, 1.307]
const Q_24_MOST = 40
export const VOWELS = [
  [325, 700, 2530],
  [450, 800, 2830],
  [800, 1150, 2900],
  [400, 1600, 2700],
  [290, 2200, 3000],
]
const FORMANT_LEVELS = [2.5, 1.5, 0.9]
const FORMANT_Q = [6, 9, 12]
export const VOWEL_RANGE: [number, number] = [150, 5000]
const MAJOR = [0, 2, 4, 5, 7, 9, 11]

export interface Settings {
  // 0 low pass, 1 high pass, 2 band pass
  type: number
  steep: boolean
  cutoff: number
  resonance: number
  drive: number
  mode: number
  floor: number
  sync: boolean
  rateHz: number
  shape: number
  mix: number
  outputDb: number
  vowel: number
  spin: number
  // 0 Off, 1 Chromatic, 2.. major keys from C
  tune: number
}

export const settingsOf = (params: ParamWire[]): Settings | null => {
  if(params.length < 19) return null
  const v = (key: string) => valueOf(params, key)
  return {
    type: Math.round(v('type')),
    steep: Math.round(v('slope')) === 1,
    cutoff: v('cutoff'),
    resonance: v('resonance'),
    drive: v('drive'),
    mode: Math.round(v('mode')),
    floor: v('floor'),
    sync: v('sync') >= 0.5,
    rateHz: v('rate'),
    shape: Math.round(v('shape')),
    mix: v('mix') / 100,
    outputDb: v('output'),
    vowel: v('vowel') / 100,
    spin: v('spin') / 100,
    tune: Math.round(v('tune')),
  }
}

export const qOf = (resonance: number): number =>
  Q_LEAST * (Q_MOST / Q_LEAST) ** Math.min(1, Math.max(0, resonance / 100))

// the frequency m puts it at, between Floor and Cutoff
export const freqAt = (m: number, floor: number, cutoff: number): number =>
  2 ** (Math.log2(Math.max(1, floor)) + m * (Math.log2(Math.max(1, cutoff)) - Math.log2(Math.max(1, floor))))

// a frequency stepped to the nearest note Tune allows
export const snap = (hz: number, tune: number): number => {
  if(tune <= 0) return hz
  const midi = 69 + 12 * Math.log2(hz / 440)
  const near = Math.round(midi)
  let note = near
  if(tune > 1) {
    const root = tune - 2
    let best = Infinity
    for(let n = near - 2; n <= near + 2; n += 1) {
      if(MAJOR.includes((((n - root) % 12) + 12) % 12) && Math.abs(n - midi) < best) {
        best = Math.abs(n - midi)
        note = n
      }
    }
  }
  return 440 * 2 ** ((note - 69) / 12)
}

// the notes Tune can land on between `low` and `high` Hz
export const tuneNotes = (tune: number, low: number, high: number): number[] => {
  if(tune <= 0) return []
  const notes: number[] = []
  for(let n = Math.ceil(69 + 12 * Math.log2(low / 440)); 440 * 2 ** ((n - 69) / 12) <= high; n += 1) {
    if(tune === 1 || MAJOR.includes((((n - (tune - 2)) % 12) + 12) % 12)) notes.push(440 * 2 ** ((n - 69) / 12))
  }
  return notes
}

// the LFO's shapes, as the engine's (S&H can't be run on: its value's read)
const lfoAt = (shape: number, phase: number): number | null => {
  const p = phase - Math.floor(phase)
  switch(shape) {
    case 1: return triangle(p)
    case 2: return 2 * p - 1
    case 3: return 1 - 2 * p
    case 4: return p < 0.5 ? -1 : 1
    case SAMPLE_HOLD: return null
    default: return sine(p)
  }
}

// each side's m now: run on with the LFO's own shape, or read from the
// engine (the envelope, the follower, S&H). Off is 1
export const msAt = (s: Settings, live: Live | null): [number, number] => {
  let m = 1
  if(s.mode !== MODE.OFF) {
    const lfo = s.mode === MODE.LFO && live ? lfoAt(s.shape, live.phase) : null
    m = lfo !== null ? 0.5 * (lfo + 1) : live?.slots[SLOT.M_L] ?? 0
  }
  return [m, m + s.spin * (1 - 2 * m)]
}

// each side's frequency now
export const freqsAt = (s: Settings, live: Live | null): [number, number] => {
  const [l, r] = msAt(s, live)
  return [snap(freqAt(l, s.floor, s.cutoff), s.tune), snap(freqAt(r, s.floor, s.cutoff), s.tune)]
}

export const formantsAt = (hz: number): number[] => {
  const [lo, hi] = VOWEL_RANGE
  const along = Math.min(1, Math.max(0, Math.log2(Math.max(1, hz) / lo) / Math.log2(hi / lo))) * (VOWELS.length - 1)
  const i = Math.min(Math.floor(along), VOWELS.length - 2)
  const t = along - i
  return [0, 1, 2].map(k => 2 ** (Math.log2(VOWELS[i][k]) + t * (Math.log2(VOWELS[i + 1][k]) - Math.log2(VOWELS[i][k]))))
}

const SHAPES = [Shape.HighCut, Shape.LowCut, Shape.BandPass]

// the filter's gain at `hz` (an amplitude) with its frequency at `at`. a
// picture: levels mixed by size, not phase (the dry and the filter, the
// filter and the formants), so it's close, not exact, where they overlap
export const gainAt = (hz: number, at: number, s: Settings): number => {
  const shape = SHAPES[s.type] ?? Shape.HighCut
  const q = qOf(s.resonance)
  let wet = s.steep
    ? bandResponse({ shape, freq: at, gain: 0, q: Q_24[0] }, hz) * bandResponse({ shape, freq: at, gain: 0, q: Math.min(Q_24_MOST, (Q_24[1] * q) / Q_LEAST) }, hz)
    : bandResponse({ shape, freq: at, gain: 0, q }, hz)
  if(s.vowel > 0) {
    const voice = formantsAt(at).reduce((sum, f, k) => sum + FORMANT_LEVELS[k] * bandResponse({ shape: Shape.BandPass, freq: f, gain: 0, q: FORMANT_Q[k] }, hz), 0)
    wet += s.vowel * (voice - wet)
  }
  return 10 ** (s.outputDb / 20) * ((1 - s.mix) + s.mix * wet)
}

const MODE_NAMES = ['', 'LFO', 'envelope', 'dynamic']

export const captionOf = (s: Settings, noteLabel: string): string => {
  const hz = (f: number) => (f >= 1000 ? `${(f / 1000).toFixed(2)} kHz` : `${Math.round(f)} Hz`)
  const type = ['Low pass', 'High pass', 'Band pass'][s.type] ?? ''
  const moving = s.mode === MODE.OFF ? '' : ` · ${MODE_NAMES[s.mode]} from ${hz(s.floor)}${s.mode === MODE.LFO ? ` at ${s.sync ? noteLabel : `${s.rateHz.toFixed(2)} Hz`}` : ''}`
  return `${type} ${s.steep ? 24 : 12} dB · ${hz(s.cutoff)} · Q ${qOf(s.resonance).toFixed(1)}${moving}`
}
