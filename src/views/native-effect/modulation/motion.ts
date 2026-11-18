// what the modulation effects' views (Chorus, Flanger, Phaser) share: their
// LFO's shapes, as the engine has them (engine/crates/dsp/src/lfo.rs), and
// where the LFO is now, run on from the last readout at its rate so a view
// moves smoothly between polls
import { mixRgba, RGBA } from '../../../theme/color'
import { painter } from '../../../theme/canvas'
import { ParamWire } from '../../../types/nativeEffect'
import type { Reading } from '../useEffectReadout'

// a parameter's plain value, by its saved key (0 if there's none)
export const valueOf = (params: ParamWire[], key: string): number =>
  params.find(p => p.key === key)?.value.value ?? 0

export const paramOf = (params: ParamWire[], key: string): ParamWire | undefined =>
  params.find(p => p.key === key)

// ---- the shapes: each -1 at phase 0, 1 halfway (the lamp sooner)

const LAMP_RISE = 0.3

const frac = (x: number) => x - Math.floor(x)

export const sine = (phase: number): number => -Math.cos(2 * Math.PI * frac(phase))

export const triangle = (phase: number): number => {
  const p = frac(phase)
  return p < 0.5 ? 4 * p - 1 : 3 - 4 * p
}

export const lamp = (phase: number): number => {
  const p = frac(phase)
  return p < LAMP_RISE
    ? (0.5 - 0.5 * Math.cos((Math.PI * p) / LAMP_RISE)) * 2 - 1
    : (0.5 + 0.5 * Math.cos((Math.PI * (p - LAMP_RISE)) / (1 - LAMP_RISE))) * 2 - 1
}

// Shape's choice: 0 sine, 1 triangle
export const shapeAt = (shape: number, phase: number): number =>
  (Math.round(shape) === 1 ? triangle(phase) : sine(phase))

// ---- the live LFO

export interface Live {
  // 0 to 1
  phase: number
  hz: number
  // the effect's being run (not resting or bypassed)
  running: boolean
  slots: number[]
}

// where a reading's LFO is at `now` (performance.now(), ms): run on at its
// rate while the effect's running, held while it isn't. its phase is in
// slot `phaseSlot`, its rate in `hzSlot`
export const liveAt = (reading: Reading | null, now: number, phaseSlot = 0, hzSlot = 1): Live | null => {
  if(!reading) return null
  const { slots } = reading.readout
  const hz = slots[hzSlot] ?? 0
  const ran = reading.running ? Math.max(0, now - reading.at) / 1000 : 0
  // (no further than a few polls on: a stalled poll shouldn't spin it)
  return { phase: frac((slots[phaseSlot] ?? 0) + hz * Math.min(ran, 0.25)), hz, running: reading.running, slots }
}

// ---- colours

// a spectrum from the theme's track colours, red to violet, for slices
// (Prism) and hues that turn (the Phaser)
const SPECTRUM = ['red', 'orange', 'yellow', 'green', 'teal', 'blue', 'violet'] as const

// the spectrum's colour `along` of the way (0 to 1, wrapping)
export const spectrumAt = (along: number): RGBA => {
  const track = painter('track')
  const at = frac(along) * SPECTRUM.length
  const i = Math.floor(at)
  return blend(track.rgba(SPECTRUM[i]), track.rgba(SPECTRUM[(i + 1) % SPECTRUM.length]), at - i)
}

export const css = ([r, g, b, a]: Readonly<RGBA>, alpha = 1): string =>
  `rgba(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)}, ${+((a / 255) * alpha).toFixed(3)})`

// `t` of the way from `a` to `b` (theme/color's mixRgba takes `t` of a)
export const blend = (a: Readonly<RGBA>, b: Readonly<RGBA>, t: number): RGBA => mixRgba(b, a, t)
