// what a pattern clip draws of its pattern, worked out once for each version
// of the pattern rather than for every clip in every frame. the store keeps
// a pattern's object for as long as it's unchanged (keepUnchanged in
// types/sequencer.ts), so a new object is a changed pattern. each drawing is
// kept with the rows it was worked out for: their order and instruments make
// the lanes, and can change while the pattern doesn't
import { NoteBars, Pattern, Row, noteBars, rowNotes } from '../../../types/sequencer'

// a lane of a pattern clip: one row's notes that start inside the pattern,
// in order of their starts. a sample row's are ticks where they start, and
// a synth row's are small bars, each cut at the pattern's end, on `pitches`
// rows (see noteBars)
export type Lane = SampleLane | SynthLane

export interface SampleLane {
  kind: 'sample'
  ticks: number[]
}

export interface SynthLane {
  kind: 'synth'
  bars: NoteBars['bars']
  pitches: number
}

const drawings = new WeakMap<Pattern, { rows: Row[], lanes: Lane[] }>()

// a row's lane in a pattern, or null when it plays nothing there. (a row's
// notes are in order of their starts, so the ones inside the pattern come
// first)
const laneOf = (pattern: Pattern, row: Row): Lane | null => {
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

// a pattern's lanes, with `rows` being the sequencer's rows: one for each
// row that plays notes in it, in the rows' order. the ones kept for it, or
// worked out now if there are none for this pattern and rows
export const patternLanes = (pattern: Pattern, rows: Row[]): Lane[] => {
  const kept = drawings.get(pattern)
  if(kept && kept.rows === rows) return kept.lanes

  const lanes = rows.map(row => laneOf(pattern, row)).filter((lane): lane is Lane => lane !== null)
  drawings.set(pattern, { rows, lanes })
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
