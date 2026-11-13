// what a pattern clip draws of its pattern, worked out once for each version
// of the pattern rather than for every clip in every frame. the store keeps
// a pattern's object for as long as it's unchanged (keepUnchanged in
// types/sequencer.ts), so a new object is a changed pattern. each drawing is
// kept with the rows it was worked out for: their order and instruments make
// the lanes, and can change while the pattern doesn't
import { NoteBars, Pattern, Row, clipContents, noteBars } from '../../../types/sequencer'

export interface PatternDrawing {
  // a lane for each sample row, in row order: the steps its notes start on
  // inside the pattern, in order
  lanes: number[][]
  // the synth rows' notes as small bars, each cut at the pattern's end, in
  // order of their starts, and how many rows of pitches they're drawn in
  bars: NoteBars['bars']
  rows: number
}

const drawings = new WeakMap<Pattern, { rows: Row[], drawing: PatternDrawing }>()

// the drawing of a pattern, with `rows` being the sequencer's rows: the one
// kept for it, or worked out now if there's none for this pattern and rows
export const patternDrawing = (pattern: Pattern, rows: Row[]): PatternDrawing => {
  const kept = drawings.get(pattern)
  if(kept && kept.rows === rows) return kept.drawing

  const { lanes, notes } = clipContents(pattern, rows)
  const { rows: pitches, bars } = noteBars(notes, pattern.length, 0, pattern.length)
  const drawing = { lanes, bars: bars.sort((a, b) => a.start - b.start), rows: pitches }
  drawings.set(pattern, { rows, drawing })
  return drawing
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
