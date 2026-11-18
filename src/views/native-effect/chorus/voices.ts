// the Chorus's voices as its settings and readout make them: where each
// one reads from now, and how it's panned. the same rules as the engine's
// (engine/crates/dsp/src/effects/chorus.rs: `layouts`, the voice loop),
// worked out here so the view moves every frame between readouts
import { ParamWire } from '../../../types/nativeEffect'
import { Live, shapeAt, valueOf } from '../modulation/motion'

// the readout's slots (chorus.rs OUT_*)
export const SLOT = {
  PHASE: 0, HZ: 1, OPEN: 2, LEVEL: 3, DELAYS_L: 4, DELAYS_R: 8, GAINS: 12, DRIFT_PHASE: 16, DRIFT_MS: 20, STEREO: 24, SWING: 25,
} as const

export const MOST_VOICES = 4
// Depth at 100% swings the delay this share of itself either way, and the
// voices' delays are spaced this share apart
export const SWING = 0.45
export const SPACING = 0.1

export interface Settings {
  sync: boolean
  rateHz: number
  // 1 to 4
  voices: number
  // 0 to 1
  depth: number
  shape: number
  delayMs: number
  spread: number
  toneHz: number
  mix: number
  bloom: number
  drift: number
  prism: number
}

export const settingsOf = (params: ParamWire[]): Settings | null => {
  if(params.length < 14) return null
  const v = (key: string) => valueOf(params, key)
  return {
    sync: v('sync') >= 0.5,
    rateHz: v('rate'),
    voices: Math.round(v('voices')) + 1,
    depth: v('depth') / 100,
    shape: v('shape'),
    delayMs: v('delay'),
    spread: v('spread') / 100,
    toneHz: v('tone'),
    mix: v('mix') / 100,
    bloom: v('bloom') / 100,
    drift: v('drift') / 100,
    prism: v('prism') / 100,
  }
}

export interface Layout {
  // round the LFO (cycles)
  offset: number
  // -1 left to 1 right
  pan: number
  // its delay, as a share of Delay
  scale: number
}

export const layoutsOf = (n: number, spread: number, prism: number): Layout[] =>
  Array.from({ length: n }, (_, v) => {
    if(n === 1) return { offset: 0, pan: 0, scale: 1 }
    const along = v / (n - 1)
    const width = spread + prism * (1 - spread)
    return { offset: v / n, pan: (2 * along - 1) * width, scale: 1 + SPACING * (along - 0.5) }
  })

// how far round the LFO the right side reads, past the left
export const stereoOf = (n: number, spread: number): number => (n === 1 ? 0.5 * spread : 0.25 * spread)

export interface Voice {
  index: number
  gain: number
  pan: number
  // where it reads from now, ms: left, right
  delays: [number, number]
  // where it'd sit with the LFO at its middle (its own centre), ms
  center: number
}

// each voice now: with a live readout, its gains, stereo offset, Drift and
// Bloom from there, the LFO run on to `live.phase`; without one, as the
// settings place them with the LFO at its start
export const voicesAt = (s: Settings, live: Live | null): Voice[] => {
  const layouts = layoutsOf(s.voices, s.spread, s.prism)
  const slot = (i: number, otherwise: number) => (live ? live.slots[i] ?? otherwise : otherwise)
  const stereo = slot(SLOT.STEREO, stereoOf(s.voices, s.spread))
  // (Bloom has the swing ducked out after a pick, then swelled past Depth)
  const swing = SWING * s.depth * (s.bloom > 0 ? slot(SLOT.SWING, 1) : 1)
  const phase = live?.phase ?? 0
  return layouts.map((layout, v) => {
    const driftPhase = s.drift > 0 ? slot(SLOT.DRIFT_PHASE + v, 0) : 0
    const driftMs = s.drift > 0 ? slot(SLOT.DRIFT_MS + v, 0) : 0
    const center = s.delayMs * layout.scale + driftMs
    const at = (side: number) => Math.max(0, center + s.delayMs * swing * shapeAt(s.shape, phase + layout.offset + driftPhase + side * stereo))
    return { index: v, gain: slot(SLOT.GAINS + v, 1), pan: layout.pan, delays: [at(0), at(1)], center }
  })
}

// how much of a voice each side hears (as the engine pans: 1 each at the
// middle)
export const panGains = (pan: number): [number, number] =>
  [Math.sqrt(Math.max(0, 1 - pan)), Math.sqrt(Math.max(0, 1 + pan))]

// Prism's slice for a voice, 0 (lowest) to 1 (highest): one voice is low on
// the left and high on the right
export const sliceOf = (v: number, n: number, side: number): number =>
  (n === 1 ? 0.15 + 0.7 * side : v / (n - 1))

export const captionOf = (s: Settings, live: Live | null, noteLabel: string): string => {
  const rate = s.sync ? noteLabel : `${(live?.hz ?? s.rateHz).toFixed(2)} Hz`
  const swing = SWING * s.depth * s.delayMs
  return `${s.voices} ${s.voices === 1 ? 'voice' : 'voices'} · ${rate} · ${s.delayMs.toFixed(1)} ms ±${swing.toFixed(1)}`
}
