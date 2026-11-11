// the Draw tool's clips, on top of the painting rules the playlist shares
// with the Piano Roll (src/editing/paint.ts). kept free of p5 so it can be
// tested on its own
import { Span } from '../../../editing/edges'
import { paintBars, paintSpans } from '../../../editing/paint'
import { MAX_NOTE_BARS, STEPS_PER_BEAT } from '../../../types/sequencer'
import { MIN_PATTERN_CLIP_STEPS } from './edges'

// the clips a paint from `start` to `end` (beats, already snapped) lays
// down, as the engine's paint_pattern_clips lays them
// (engine/src/sequencer_commands.rs): clips of a pattern `steps` long, or
// with no pattern (null) one clip of the new note pattern the paint makes,
// which is enough bars of `beatsPerBar` beats to hold it
export const paintedSpans = (start: number, end: number, steps: number | null, beatsPerBar: number): Span[] => {
  const bar = Math.max(1, beatsPerBar)
  const length = steps === null
    ? paintBars(start, end, bar, MAX_NOTE_BARS) * bar
    : steps / STEPS_PER_BEAT
  return paintSpans(start, end, length, MIN_PATTERN_CLIP_STEPS / STEPS_PER_BEAT)
}
