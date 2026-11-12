import { clampLowWins } from '../util/helpers'

// moving a group of items together. plain functions, with no canvas

// an item's place in its view: where it starts along time (beats in the
// playlist, steps in the Piano Roll) and which lane it's on (a track, a
// pitch)
export interface Placement {
  id: number
  beat: number
  track: number
  // how long it is, where its end has to stay inside too (a note in its
  // pattern). left out, only where it starts is kept in range (a clip)
  length?: number
}

// where a group of items lands when one of them, the anchor, is dropped at
// `beat` on `track`. the same rule the engine uses for clips (group_move in
// audiograph.rs): every item moves by the anchor's change, cut short so
// none starts before 0 or ends after `maxBeat` (starts after it, for items
// with no length), or goes above lane 0 or below `maxTrack` (in the
// playlist, the new-track area). a group that already reaches past
// `maxBeat` goes no further, but isn't pulled back. `beat` should already
// be snapped
export const groupLanding = <T extends Placement>(
  placed: ReadonlyArray<T>,
  anchorId: number,
  beat: number,
  track: number,
  maxBeat: number,
  maxTrack: number,
): T[] => {
  const anchor = placed.find(c => c.id === anchorId)
  if(!anchor) return []

  const beats = placed.map(c => c.beat)
  const ends = placed.map(c => c.beat + (c.length ?? 0))
  const tracks = placed.map(c => c.track)
  const shift = clampLowWins(beat - anchor.beat, -Math.min(...beats), Math.max(0, maxBeat - Math.max(...ends)))
  const down = clampLowWins(track - anchor.track, -Math.min(...tracks), maxTrack - Math.max(...tracks))

  return placed.map(c => ({ ...c, beat: c.beat + shift, track: c.track + down }))
}
