// the Flanger's comb as its settings and readout make it: where its copy
// reads from now (and how far round Barberpole's turned it), and what that
// does to each frequency. the same rules as the engine's
// (engine/crates/dsp/src/effects/flanger.rs), worked out here so the view
// moves every frame between readouts
import { ParamWire } from '../../../types/nativeEffect'
import { Live, shapeAt, valueOf } from '../modulation/motion'

// the readout's slots (flanger.rs OUT_*)
export const SLOT = { PHASE: 0, HZ: 1, DELAY_L: 2, DELAY_R: 3, CENTER: 4, BARBER: 5, PUSH: 6, ZERO: 7 } as const

// octaves either way of the centre at full Depth
export const SWEEP = 2

export interface Settings {
  sync: boolean
  rateHz: number
  depth: number
  shape: number
  manualMs: number
  // -0.95 to 0.95
  feedback: number
  toneHz: number
  zero: boolean
  // 0 to 1
  spread: number
  mix: number
  // -1 to 1
  barber: number
  // 0 Off, 1 C ... 12 B
  tune: number
  envelope: number
}

export const settingsOf = (params: ParamWire[]): Settings | null => {
  if(params.length < 14) return null
  const v = (key: string) => valueOf(params, key)
  return {
    sync: v('sync') >= 0.5,
    rateHz: v('rate'),
    depth: v('depth') / 100,
    shape: v('shape'),
    manualMs: v('manual'),
    feedback: v('feedback') / 100,
    toneHz: v('tone'),
    zero: v('zero') >= 0.5,
    spread: v('spread') / 100,
    mix: v('mix') / 100,
    barber: v('barberpole') / 100,
    tune: Math.round(v('tune')),
    envelope: v('envelope') / 100,
  }
}

// the note Tune pins the comb to, in Hz (in A 440's octave), or null
export const noteHz = (tune: number): number | null =>
  (tune <= 0 ? null : 440 * 2 ** ((tune - 10) / 12))

// the centre, as a share of Manual: the period of Tune's note nearest
// Manual's (1 when Off)
export const tuned = (tune: number, manualMs: number): number => {
  const note = noteHz(tune)
  if(note === null) return 1
  const manual = 1000 / Math.max(1e-3, manualMs)
  const octaves = Math.round(Math.log2(manual / note))
  return 1000 / (note * 2 ** octaves) / manualMs
}

// one side's copy now
export interface SideTaps {
  // how late it reads, ms
  delayMs: number
  // how far round Barberpole's turned its phase (turns, 0 to 1)
  turn: number
}

const frac = (x: number) => x - Math.floor(x)

export const tapsAt = (s: Settings, live: Live | null): { center: number, sides: [SideTaps, SideTaps] } => {
  const slot = (i: number, otherwise: number) => (live ? live.slots[i] ?? otherwise : otherwise)
  const center = slot(SLOT.CENTER, 0) > 0 ? slot(SLOT.CENTER, 0) : s.manualMs * tuned(s.tune, s.manualMs)
  const push = s.envelope !== 0 ? slot(SLOT.PUSH, 0) : 0
  const phase = live?.phase ?? 0
  // Barberpole turns once a cycle of the LFO at full (either way): it's
  // run on as the LFO is. it takes over from the sweep as it's turned up
  const barber = live ? frac(slot(SLOT.BARBER, 0) + s.barber * (live.phase - frac(live.slots[SLOT.PHASE] ?? 0))) : 0
  const depth = s.depth * (1 - Math.abs(s.barber))
  const side = (n: number): SideTaps => {
    const offset = (n * s.spread) / 2
    const delayMs = center * 2 ** (SWEEP * depth * shapeAt(s.shape, phase + offset) - push)
    return { delayMs, turn: s.barber !== 0 ? frac(barber + offset) : 0 }
  }
  return { center, sides: [side(0), side(1)] }
}

// ---- what the comb does to a frequency

type Complex = [number, number]
const mul = ([a, b]: Complex, [c, d]: Complex): Complex => [a * c - b * d, a * d + b * c]
const div = ([a, b]: Complex, [c, d]: Complex): Complex => {
  const n = c * c + d * d || 1e-30
  return [(a * c + b * d) / n, (b * c - a * d) / n]
}
const late = (ms: number, hz: number): Complex => {
  const w = -2 * Math.PI * hz * (ms / 1000)
  return [Math.cos(w), Math.sin(w)]
}

// the tone filter in the loop (a 12 dB high cut, Q 0.707)
const toneAt = (hz: number, cut: number): Complex => {
  const x = hz / cut
  return div([1, 0], [1 - x * x, x / 0.707])
}

// the level at `hz` (as a gain, 1 is unchanged) of one side's sound
// through the comb: the copy (turned by Barberpole) fed back through the
// tone, mixed with the dry (late by the centre, the copy turned over, for
// Through-zero)
export const gainAt = (hz: number, side: SideTaps, s: Settings, center: number): number => {
  const turn = 2 * Math.PI * side.turn
  const copy = mul(late(side.delayMs, hz), [Math.cos(turn), Math.sin(turn)])
  const loop = mul(toneAt(hz, s.toneHz), copy)
  const wet = div(copy, [1 - s.feedback * loop[0], -s.feedback * loop[1]])
  const turned: Complex = s.zero ? [-wet[0], -wet[1]] : wet
  const dry: Complex = s.zero ? late(center, hz) : [1, 0]
  const out: Complex = [dry[0] + s.mix * (turned[0] - dry[0]), dry[1] + s.mix * (turned[1] - dry[1])]
  return Math.hypot(out[0], out[1])
}

export const captionOf = (s: Settings, live: Live | null, noteLabel: string, center: number): string => {
  const rate = s.sync ? noteLabel : `${(live?.hz ?? s.rateHz).toFixed(2)} Hz`
  const fb = `${s.feedback >= 0 ? '+' : '−'}${Math.round(Math.abs(s.feedback) * 100)}%`
  return `${center.toFixed(2)} ms · ${rate} · feedback ${fb}${s.zero ? ' · through-zero' : ''}`
}

// how near the copy is to the dry, 1 when they meet: Through-zero's flash
export const crossingOf = (delayMs: number, centerMs: number): number =>
  Math.exp(-Math.abs(Math.log2(Math.max(1e-6, delayMs) / Math.max(1e-6, centerMs))) / 0.06)
