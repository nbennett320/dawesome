// the Delay's echoes as its settings make them: when each comes, on which
// side, and how loud. the same rules as the engine's
// (engine/crates/dsp/src/effects/delay.rs: `times`, `levels`), worked out
// here so the echo view follows a knob at once, with no audio to read.
// a note value's length is read from its label ("1/8 D"), which the engine
// keeps as its formula
import { ParamWire } from '../../../types/nativeEffect'

// the parameters' ids (their places in the engine's list)
export const SYNC = 0
export const NOTE = 1
export const TIME = 2
export const OFFSET = 3
export const MODE = 4
export const FEEDBACK = 5
export const LOW_CUT = 6
export const HIGH_CUT = 7
export const MIX = 8
export const PARAM_COUNT = 9

export const PING_PONG = 1
// the longest a side can be, in seconds (the engine holds longer at this)
export const LONGEST = 4
// echoes quieter than this aren't drawn
export const FLOOR_DB = -48
// the longest stretch the view shows, in seconds
export const MAX_SPAN = 12

export interface Settings {
  sync: boolean
  // the note value's label ("1/4 D")
  note: string
  timeMs: number
  // -50 to 50 (%): over 0, the right side's longer
  offset: number
  pingPong: boolean
  // 0 to 0.95
  feedback: number
  // 0 to 100
  mix: number
}

// what the Delay's parameters are set to (none until they've all come)
export const settingsOf = (params: ParamWire[]): Settings | null => {
  if(params.length < PARAM_COUNT) return null
  const value = (id: number) => params[id].value.value
  return {
    sync: value(SYNC) >= 0.5,
    note: params[NOTE].labels[Math.round(value(NOTE))] ?? '1/4',
    timeMs: value(TIME),
    offset: value(OFFSET),
    pingPong: Math.round(value(MODE)) === PING_PONG,
    feedback: value(FEEDBACK) / 100,
    mix: value(MIX),
  }
}

// a note value's length in quarter notes, from its label: 1/n of a whole
// note, × 1.5 dotted (D), × 2/3 triplet (T)
export const noteQuarters = (label: string): number => {
  const [fraction, kind] = label.split(' ')
  const n = Number(fraction.split('/')[1]) || 4
  const times = kind === 'D' ? 1.5 : kind === 'T' ? 2 / 3 : 1
  return (4 / n) * times
}

// seconds a note value lasts at `tempo` (quarter notes a minute)
export const noteSeconds = (label: string, tempo: number): number =>
  (noteQuarters(label) * 60) / (tempo > 0 ? tempo : 120)

// the time before offset, in seconds
export const baseSeconds = (s: Settings, tempo: number): number =>
  (s.sync ? noteSeconds(s.note, tempo) : s.timeMs / 1000)

// which side offset lengthens: the right over 0, the left under
export const longerSide = (s: Settings): 0 | 1 => (s.offset >= 0 ? 1 : 0)

// how much longer than the base the longer side is
export const offsetFactor = (s: Settings): number => 1 + Math.abs(s.offset) / 100

// each side's time, in seconds [left, right]
export const sideTimes = (s: Settings, tempo: number): [number, number] => {
  const base = baseSeconds(s, tempo)
  const longer = base * offsetFactor(s)
  const [l, r] = longerSide(s) === 1 ? [base, longer] : [longer, base]
  return [Math.min(l, LONGEST), Math.min(r, LONGEST)]
}

// the dry sound's and the echoes' levels at `mix` (0 to 100): the dry stays
// whole to the middle, then falls; the echoes are whole from there
export const levels = (mix: number): { dry: number, wet: number } => {
  const m = Math.min(1, Math.max(0, mix / 100))
  return { dry: Math.min(1, 2 * (1 - m)), wet: Math.min(1, 2 * m) }
}

export interface Echo {
  // 0 left, 1 right
  side: 0 | 1
  // seconds after the sound
  time: number
  // its level (the cuts' loss isn't counted)
  db: number
  // the first on its side: the one that's dragged
  first: boolean
}

const toDb = (level: number) => (level > 0 ? 20 * Math.log10(level) : -Infinity)

// every echo up to `until` seconds that's louder than FLOOR_DB, in time
// order, from a sound in the middle
export const echoesOf = (s: Settings, tempo: number, until: number): Echo[] => {
  const times = sideTimes(s, tempo)
  const { wet } = levels(s.mix)
  const echoes: Echo[] = []
  const audible = (level: number) => toDb(level) >= FLOOR_DB
  if(s.pingPong) {
    // into the left, then bouncing
    let [time, level, side] = [0, wet, 0 as 0 | 1]
    for(let n = 0; audible(level) && n < 1000; n += 1) {
      time += times[side]
      if(time > until) break
      echoes.push({ side, time, db: toDb(level), first: n === 0 })
      level *= s.feedback
      side = side === 0 ? 1 : 0
    }
  } else {
    ([0, 1] as const).forEach(side => {
      let level = wet
      for(let n = 1; audible(level) && n * times[side] <= until && n < 1000; n += 1) {
        echoes.push({ side, time: n * times[side], db: toDb(level), first: n === 1 })
        level *= s.feedback
      }
    })
    echoes.sort((a, b) => a.time - b.time || a.side - b.side)
  }
  return echoes
}

// how many seconds the view shows: until the echoes fade (and a little
// after), at least four times the longer side's time, at most MAX_SPAN
export const spanOf = (s: Settings, tempo: number): number => {
  const longest = Math.max(...sideTimes(s, tempo))
  const echoes = echoesOf(s, tempo, MAX_SPAN)
  const last = echoes.length ? echoes[echoes.length - 1].time : 0
  return Math.min(MAX_SPAN, Math.max(last * 1.1, longest * 4))
}

// the note value nearest `seconds` at `tempo`: its index in `labels`
export const nearestNote = (labels: string[], seconds: number, tempo: number): number => {
  let best = 0
  labels.forEach((label, i) => {
    const off = (index: number) => Math.abs(Math.log(noteSeconds(labels[index], tempo) / seconds))
    if(off(i) < off(best)) best = i
  })
  return best
}

// what's shown above the echoes: "1/4 D · 750 ms at 120 bpm · R +15%"
export const captionOf = (s: Settings, tempo: number): string => {
  const ms = (seconds: number) => (seconds >= 1 ? `${seconds.toFixed(2)} s` : `${Math.round(seconds * 1000)} ms`)
  const base = baseSeconds(s, tempo)
  const time = s.sync ? `${s.note} · ${ms(base)} at ${Math.round(tempo)} bpm` : ms(base)
  const offset = Math.round(s.offset)
  return offset === 0 ? time : `${time} · ${offset > 0 ? 'R' : 'L'} +${Math.abs(offset)}%`
}
