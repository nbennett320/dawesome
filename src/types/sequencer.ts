// the pattern sequencer: rows of samples shared by every pattern, and
// patterns that light some of each row's steps. mirrors
// engine/src/daw/daw_core/sequencer.rs

// a step is a 16th note
export const STEPS_PER_BEAT = 4

// the lengths a pattern can be, in bars
export const BAR_CHOICES = [1, 2, 4]

// a row's volume, as a gain: 1 leaves the sample as is
export const ROW_VOLUME_MAX = 2

export interface SequencerRow {
  id: number
  path: string
  volume: number
  muted: boolean
}

export interface Pattern {
  id: number
  // unnamed patterns are shown by their number
  name: string | null
  // in steps
  length: number
  // the steps lit on each row, by row id
  steps: Record<string, number[]>
}

// what every sequencer command returns
export interface SequencerState {
  rows: SequencerRow[]
  patterns: Pattern[]
}

// what a pattern is called: its name, or its number. numbers come from ids,
// so they don't change when another pattern is deleted
export const patternLabel = (pattern: Pick<Pattern, 'id' | 'name'>): string =>
  pattern.name ?? String(pattern.id + 1)

// a sample's file name, without its folders
export const fileName = (path: string): string => path.split(/[\\/]/).pop() ?? path

export const isLit = (pattern: Pattern, rowId: number, step: number): boolean =>
  !!pattern.steps[rowId]?.includes(step)

// a pattern with one step changed (what the engine will make of it)
export const withStep = (pattern: Pattern, rowId: number, step: number, on: boolean): Pattern => {
  const lit = (pattern.steps[rowId] ?? []).filter(s => s !== step)
  if(on) lit.push(step)
  lit.sort((a, b) => a - b)

  const steps = { ...pattern.steps }
  if(lit.length) steps[rowId] = lit
  else delete steps[rowId]
  return { ...pattern, steps }
}

// steps in a bar
export const stepsPerBar = (beatsPerBar: number): number => Math.max(1, beatsPerBar) * STEPS_PER_BEAT

// how many bars long a pattern is, or null when that isn't a whole number
// (the time signature changed after its length was set)
export const patternBars = (pattern: Pattern, beatsPerBar: number): number | null => {
  const bars = pattern.length / stepsPerBar(beatsPerBar)
  return Number.isInteger(bars) ? bars : null
}

// how long a pattern plays for, in seconds, at a tempo
export const patternSeconds = (pattern: Pattern, tempo: number): number =>
  tempo > 0 ? (pattern.length / STEPS_PER_BEAT) * (60 / tempo) : 0

// the step a pattern playing on its own is on, `elapsedMs` after it started
export const stepAt = (elapsedMs: number, pattern: Pattern, beatIntervalMs: number): number | null => {
  const stepMs = beatIntervalMs / STEPS_PER_BEAT
  if(stepMs <= 0 || pattern.length <= 0) return null
  return Math.floor(Math.max(0, elapsedMs) / stepMs) % pattern.length
}

// each row's lit steps in a pattern, in row order: what a pattern clip draws
export const litRows = (pattern: Pattern, rows: SequencerRow[]): number[][] =>
  rows.map(row => (pattern.steps[row.id] ?? []).filter(s => s < pattern.length))
