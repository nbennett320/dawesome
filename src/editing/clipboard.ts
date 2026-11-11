// copying, pasting and repeating: where the copies go, the same in the
// playlist and the Piano Roll. (clips are copied by the engine, in
// engine/src/clipboard.rs, by these rules; notes are copied here.) plain
// functions, with no canvas
import { Span } from './edges'

// from the earliest start to the latest end of some items, or null for none
export const spanOf = (items: ReadonlyArray<Span>): Span | null => (items.length
  ? { start: Math.min(...items.map(i => i.start)), end: Math.max(...items.map(i => i.end)) }
  : null)

// how far copies of `items` move for a paste at `at`: the earliest starts
// there, and the rest keep their places after it
export const pasteShift = (items: ReadonlyArray<Span>, at: number): number => {
  const span = spanOf(items)
  return span ? at - span.start : 0
}

// how far copies of `items` move for a repeat: they start where the
// originals' span ends, so repeating again lays them end to end
export const repeatShift = (items: ReadonlyArray<Span>): number => {
  const span = spanOf(items)
  return span ? span.end - span.start : 0
}
