// the Draw tool in the playlist: what a drag along a lane paints. plain
// functions, with no canvas. places are in beats
import { Span } from './edges'
import { clampLowWins } from '../util/helpers'

// the clips a drag from `from` to `to` (either way round, already snapped)
// paints, of a pattern `length` long: end to end from where the drag
// starts, the last one cut short where it ends. a press let go where it
// went down paints one whole clip. nothing shorter than `minLength` is
// painted: an end piece that short is left off
export const paintSpans = (from: number, to: number, length: number, minLength: number): Span[] => {
  if(!(length > 0)) return []
  const start = Math.max(0, Math.min(from, to))
  const end = Math.max(from, to)
  if(end - start < minLength) return [{ start, end: start + length }]

  const spans: Span[] = []
  for(let i = 0; start + i * length < end; i++) {
    const at = start + i * length
    const until = Math.min(at + length, end)
    if(until - at >= minLength) spans.push({ start: at, end: until })
  }
  return spans
}

// how many bars a new pattern drawn over a drag from `from` to `to` is:
// enough to hold it, from 1 to `maxBars`
export const paintBars = (from: number, to: number, beatsPerBar: number, maxBars: number): number => {
  if(!(beatsPerBar > 0)) return 1
  const bars = Math.ceil((Math.abs(to - from) - 1e-9) / beatsPerBar)
  return clampLowWins(bars, 1, maxBars)
}
