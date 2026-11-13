// what a pattern clip draws of its pattern, worked out once for each version
// of the pattern rather than for every clip in every frame. the store keeps
// a pattern's object for as long as it's unchanged (keepUnchanged in
// types/sequencer.ts), so a new object is a changed pattern. each drawing is
// kept with the rows it was worked out for, as lanes see them: which rows,
// in what order, playing which kind of instrument, at which step pitch and
// gate (which decide whether their notes are plain steps). those can change
// while the pattern doesn't, and a row's other settings (a knob, a mute, a
// name) change nothing that's drawn
import { NoteBars, Pattern, Row, StepRow, noteBars, rowNotes, stepView } from '../../../types/sequencer'

// a lane of a pattern clip: one row's notes that start inside the pattern,
// in order of their starts. when every row's notes in the pattern are plain
// steps (what the Pattern Sequencer shows as steps), each row's lane is its
// lit steps, drawn as cells a step wide. otherwise a sample row's are ticks
// where they start, and a synth row's are small bars, each cut at the
// pattern's end, on `pitches` rows (see noteBars)
export type Lane = StepLane | SampleLane | SynthLane

export interface StepLane {
  kind: 'steps'
  steps: number[]
}

export interface SampleLane {
  kind: 'sample'
  ticks: number[]
}

export interface SynthLane {
  kind: 'synth'
  bars: NoteBars['bars']
  pitches: number
}

// a row as a lane sees it
interface LaneRow extends StepRow {
  id: number
}

// the rows as lanes see them, the same list for as long as that's
// unchanged: every turn of a knob makes a new list of rows
let laneRowsFrom: readonly Row[] = []
let laneRows: readonly LaneRow[] = []
const asLanes = (rows: readonly Row[]): readonly LaneRow[] => {
  if(rows === laneRowsFrom) return laneRows
  laneRowsFrom = rows
  const same = rows.length === laneRows.length && rows.every((row, i) => sameLaneRow(row, laneRows[i]))
  if(!same) laneRows = rows.map(asLane)
  return laneRows
}

const gatedOf = (row: StepRow): boolean => row.instrument.kind === 'sample' && !!row.instrument.gated
const asLane = (row: Row): LaneRow => ({
  id: row.id,
  stepPitch: row.stepPitch,
  instrument: { kind: row.instrument.kind, gated: gatedOf(row) },
})
const sameLaneRow = (row: Row, lane: LaneRow): boolean => row.id === lane.id
  && row.instrument.kind === lane.instrument.kind && row.stepPitch === lane.stepPitch && gatedOf(row) === gatedOf(lane)

const drawings = new WeakMap<Pattern, { rows: readonly LaneRow[], lanes: Lane[] }>()

// a row's lane in a pattern, or null when it plays nothing there. (a row's
// notes are in order of their starts, so the ones inside the pattern come
// first)
const laneOf = (pattern: Pattern, row: LaneRow): Lane | null => {
  const notes = rowNotes(pattern, row.id)
  if(row.instrument.kind === 'synth') {
    const { rows, bars } = noteBars(notes, pattern.length, 0, pattern.length)
    return bars.length ? { kind: 'synth', bars, pitches: rows } : null
  }
  const ticks: number[] = []
  for(const note of notes) {
    if(note.start >= pattern.length) break
    ticks.push(note.start)
  }
  return ticks.length ? { kind: 'sample', ticks } : null
}

// each row's lit steps, when every row's notes in the pattern are plain
// steps, for the rows that light any. null when any row's aren't
const stepLanes = (pattern: Pattern, rows: readonly LaneRow[]): StepLane[] | null => {
  const lanes: StepLane[] = []
  for(const row of rows) {
    const steps = stepView(row, rowNotes(pattern, row.id), pattern.length)
    if(!steps) return null
    if(steps.length) lanes.push({ kind: 'steps', steps })
  }
  return lanes
}

// a pattern's lanes, with `rows` being the sequencer's rows: one for each
// row that plays notes in it, in the rows' order. the ones kept for it, or
// worked out now if there are none for this pattern and rows
export const patternLanes = (pattern: Pattern, rows: readonly Row[]): Lane[] => {
  const lanesRows = asLanes(rows)
  const kept = drawings.get(pattern)
  if(kept && kept.rows === lanesRows) return kept.lanes

  const lanes = stepLanes(pattern, lanesRows)
    ?? lanesRows.map(row => laneOf(pattern, row)).filter((lane): lane is Lane => lane !== null)
  drawings.set(pattern, { rows: lanesRows, lanes })
  return lanes
}

// the first of a list sorted by `start` that starts at or after `step`
export const firstFrom = <T>(sorted: readonly T[], start: (item: T) => number, step: number): number => {
  let low = 0
  let high = sorted.length
  while(low < high) {
    const middle = (low + high) >> 1
    if(start(sorted[middle]) < step) low = middle + 1
    else high = middle
  }
  return low
}
