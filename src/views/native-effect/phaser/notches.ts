// the Phaser's notches as its settings and readout make them: where its
// stages sit now, and what the chain does to each frequency. the same
// rules as the engine's (engine/crates/dsp/src/effects/phaser.rs), worked
// out here so the view moves every frame between readouts
import { ParamWire } from '../../../types/nativeEffect'
import { lamp, Live, shapeAt, valueOf } from '../modulation/motion'

// the readout's slots (phaser.rs OUT_*)
export const SLOT = { PHASE: 0, HZ: 1, FREQ_L: 2, FREQ_R: 3, BARBER: 4, PUSH: 5, VIBE: 6, OFFSETS: 7, PRESENT: 19 } as const

export const MOST_STAGES = 12
export const SWEEP = 2.5
export const BARBER_LEAST = 1.5
export const BARBER_CHAINS = 2
export const VIBE_SPREAD = [-3.8, -1.7, 0.5, 2.7]
// (as svf.rs MAX_SHARE, and the lowest a stage goes)
const MAX_SHARE = 0.49
const LOWEST = 10

export interface Settings {
  sync: boolean
  rateHz: number
  depth: number
  shape: number
  centerHz: number
  // 2 to 12
  stages: number
  feedback: number
  vibe: boolean
  spread: number
  mix: number
  melt: number
  barber: number
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
    centerHz: v('center'),
    stages: 2 * (Math.round(v('stages')) + 1),
    feedback: v('feedback') / 100,
    vibe: Math.round(v('mode')) === 1,
    spread: v('spread') / 100,
    mix: v('mix') / 100,
    melt: v('melt') / 100,
    barber: v('barberpole') / 100,
    envelope: v('envelope') / 100,
  }
}

// a stage's frequency: the sweep's `octaves` (log2 Hz) and its own offset,
// then as far toward the top as it's absent
export const stageFreq = (octaves: number, offset: number, present: number, rate: number): number => {
  const top = Math.log2(MAX_SHARE * rate)
  const at = Math.min(top, Math.max(Math.log2(LOWEST), octaves + offset))
  return 2 ** (at + (1 - present) * (top - at))
}

// how far one stage at `stage` Hz turns `hz` (radians, 0 to -pi)
export const stagePhase = (hz: number, stage: number, rate: number): number => {
  const g = Math.tan(Math.PI * Math.min(stage / rate, MAX_SHARE))
  const w = Math.tan(Math.PI * Math.min(hz / rate, 0.4999))
  return -2 * Math.atan(w / g)
}

// the chain's stages now: each one's frequency, per chain (the swept one,
// then Barberpole's two) and its level, for one side
export interface Chain {
  freqs: number[]
  level: number
}

// the stages' layout: from a live readout (Melt, Vibe and any change
// gliding), or as the settings place them
export const layoutOf = (s: Settings, live: Live | null): { offsets: number[], present: number[] } => {
  if(live) {
    return {
      offsets: Array.from({ length: MOST_STAGES }, (_, k) => live.slots[SLOT.OFFSETS + k] ?? 0),
      present: Array.from({ length: MOST_STAGES }, (_, k) => live.slots[SLOT.PRESENT + k] ?? 0),
    }
  }
  const n = s.vibe ? 4 : s.stages
  return {
    offsets: Array.from({ length: MOST_STAGES }, (_, k) => (s.vibe && k < 4 ? VIBE_SPREAD[k] : 0)),
    present: Array.from({ length: MOST_STAGES }, (_, k) => (k < n ? 1 : 0)),
  }
}

const frac = (x: number) => x - Math.floor(x)

// Barberpole's chain at `u` through its slide: octaves from Center, and its
// level (the two always add to 1)
export const barberChain = (u: number, range: number, up: boolean): [number, number] => {
  const along = up ? 2 * u - 1 : 1 - 2 * u
  const s = Math.sin(Math.PI * u)
  return [range * along, s * s]
}

export interface SideNotches {
  chains: Chain[]
  // the sweep's frequency now, Hz
  sweepHz: number
}

export const sidesAt = (s: Settings, live: Live | null, rate: number): [SideNotches, SideNotches] => {
  const { offsets, present } = layoutOf(s, live)
  const push = s.envelope !== 0 && live ? live.slots[SLOT.PUSH] ?? 0 : 0
  const lampShare = live ? live.slots[SLOT.VIBE] ?? 0 : s.vibe ? 1 : 0
  const phase = live?.phase ?? 0
  const barber = live ? frac((live.slots[SLOT.BARBER] ?? 0) + (live.phase - frac(live.slots[SLOT.PHASE] ?? 0))) : 0
  const amount = Math.abs(s.barber)
  const range = Math.max(SWEEP * s.depth, BARBER_LEAST)
  const center = Math.log2(Math.max(1, s.centerHz))
  const freqsAt = (octaves: number) => offsets.map((o, k) => (present[k] > 0 ? stageFreq(octaves, o, present[k], rate) : 0))
  const side = (n: number): SideNotches => {
    const offset = (n * s.spread) / 2
    const shaped = shapeAt(s.shape, phase + offset)
    const lfo = shaped + lampShare * (lamp(phase + offset) - shaped)
    const octaves = center + SWEEP * s.depth * lfo + push
    const chains: Chain[] = [{ freqs: freqsAt(octaves), level: 1 - amount }]
    if(amount > 0) {
      for(let c = 0; c < BARBER_CHAINS; c += 1) {
        const [oct, level] = barberChain(frac(barber + offset + c / BARBER_CHAINS), range, s.barber > 0)
        chains.push({ freqs: freqsAt(center + oct + push), level: amount * level })
      }
    }
    return { chains, sweepHz: 2 ** octaves }
  }
  return [side(0), side(1)]
}

// the level at `hz` (a gain, 1 unchanged) of one side's sound through the
// phaser: the chains (all fed the same, fed back together), mixed with the
// dry
export const gainAt = (hz: number, side: SideNotches, s: Settings, rate: number): number => {
  let re = 0
  let im = 0
  side.chains.forEach(chain => {
    const turn = chain.freqs.reduce((sum, f) => (f > 0 ? sum + stagePhase(hz, f, rate) : sum), 0)
    re += chain.level * Math.cos(turn)
    im += chain.level * Math.sin(turn)
  })
  const trim = 1 - 0.5 * Math.abs(s.feedback)
  // x = trim·in / (1 - fb·W), wet = W·x
  const [dr, di] = [1 - s.feedback * re, -s.feedback * im]
  const n = dr * dr + di * di || 1e-30
  const wr = (trim * (re * dr + im * di)) / n
  const wi = (trim * (im * dr - re * di)) / n
  return Math.hypot(1 + s.mix * (wr - 1), s.mix * wi)
}

export const captionOf = (s: Settings, live: Live | null, noteLabel: string): string => {
  const rate = s.sync ? noteLabel : `${(live?.hz ?? s.rateHz).toFixed(2)} Hz`
  const fb = `${s.feedback >= 0 ? '+' : '−'}${Math.round(Math.abs(s.feedback) * 100)}%`
  const stages = s.vibe ? 'vibe' : `${s.stages} stages`
  return `${stages} · ${rate} · ${Math.round(s.centerHz)} Hz · feedback ${fb}`
}
