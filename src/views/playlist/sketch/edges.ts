// clips' edges: how a sample clip and a pattern clip are trimmed and
// stretched, on top of the editing rules the playlist shares with the Piano
// Roll (src/editing). kept free of p5 so it can be tested on its own
import { ClipWindow } from '../../../types/clip'
import { STEPS_PER_BEAT, patternStepAt } from '../../../types/sequencer'
import { Edge, Span, TrimItem } from '../../../editing/edges'
import { clampLowWins } from '../../../util/helpers'

export { EDGE_ZONE_PX, edgeAt, trimAll, trimmed } from '../../../editing/edges'
export type { Edge, Span } from '../../../editing/edges'
export { snapPoint } from '../../../editing/snap'
export type { Snapping } from '../../../editing/snap'

// the shortest a sample clip can be trimmed or cut to, in seconds of sound
// (MIN_CLIP in engine/src/daw/daw_core/audiograph.rs)
export const MIN_CLIP_SECONDS = 0.01
// and a pattern clip, in steps
export const MIN_PATTERN_CLIP_STEPS = 1

// a clip, as far as trimming goes. everything is in beats of the playlist,
// but its source's length
export interface TrimClip extends TrimItem {
  // how much of its source goes by in a beat: seconds of the sample (at
  // the clip's speed), or steps of the pattern
  perBeat: number
  // all of its source, in its own unit: the sample's seconds, or the
  // pattern's steps
  sourceLength: number
  // whether it plays its source again past its end (a pattern clip): its
  // right edge goes on past it, and its window's end is always given
  loops: boolean
}


// ---- stretching: Shift+dragging an edge ----

// how far a clip's pitch can go, in semitones either way, and how much
// faster or slower than its sample that plays it (clipSettingRanges.pitch)
export const MAX_PITCH = 24
export const MAX_RATE = 2 ** (MAX_PITCH / 12)

// the pitch, in semitones, that plays a sample `rate` times as fast
export const pitchOf = (rate: number): number => 12 * Math.log2(rate)

// a sample clip, as far as stretching goes. in beats of the playlist
export interface StretchClip {
  id: number
  start: number
  end: number
  // how much faster than its sample it plays now (see playbackRate)
  rate: number
  // the shortest it can be
  minLength: number
}

// a clip stretched to `ratio` times its length, like tape: playing slower
// and lower to be longer, faster and higher to be shorter. the edge that
// isn't dragged stays where it is. it stops where its pitch can go no
// further (two octaves either way), at the shortest a clip can be, and
// with its start at the playlist's
export const stretched = (clip: StretchClip, edge: Edge, ratio: number): Span & { rate: number } => {
  const length = clip.end - clip.start
  if(!(length > 0)) return { start: clip.start, end: clip.end, rate: clip.rate }

  // its length at its sample's own speed
  const plain = length * clip.rate
  const shortest = Math.max(plain / MAX_RATE, clip.minLength)
  // (dragged from the left, no further than the playlist's start)
  const longest = Math.min(plain * MAX_RATE, edge === 'left' ? clip.end : Infinity)
  const next = clampLowWins(length * ratio, shortest, longest)

  return {
    start: edge === 'left' ? clip.end - next : clip.start,
    end: edge === 'left' ? clip.end : clip.start + next,
    rate: plain / next,
  }
}

// the clips a shift+drag on a clip's edge stretches: the grabbed clip and
// the clips trimmed with it (see #startEdgeDrag), less the pattern clips,
// which can't be stretched. none at all when it's a pattern clip's edge
export const stretchSet = (
  clips: ReadonlyArray<StretchClip & { pattern: boolean }>,
  grabbedId: number,
): StretchClip[] => {
  const grabbed = clips.find(clip => clip.id === grabbedId)
  if(!grabbed || grabbed.pattern) return []

  return clips.filter(clip => !clip.pattern).map(({ pattern: _, ...clip }) => clip)
}

// every clip of a stretch: the grabbed clip's edge goes to `target` (where
// the mouse is, after snapping), and each of the others is stretched by as
// much: the same number of times its own length. each stops at its own
// limits, wherever the others are
export const stretchAll = (
  clips: ReadonlyArray<StretchClip>,
  grabbedId: number,
  edge: Edge,
  target: number,
): Array<Span & { id: number, rate: number }> => {
  const grabbed = clips.find(clip => clip.id === grabbedId)
  const length = grabbed ? grabbed.end - grabbed.start : 0
  if(!grabbed || !(length > 0)) return []

  // dragged past the other edge, it's as short as it goes
  const dragged = edge === 'left' ? grabbed.end - target : target - grabbed.start
  const ratio = Math.max(dragged, 0) / length
  return clips.map(clip => ({ id: clip.id, ...stretched(clip, edge, ratio) }))
}

// the part of its source a clip plays once it's trimmed to `span`, as the
// engine takes it (see set_clip_windows in engine/src/commands/clip.rs)
export const windowOf = (clip: TrimClip, span: Span): ClipWindow => {
  // (a beat's billionth: less than a nanosecond of sound)
  const close = 1e-9
  const start = (span.start - clip.sourceStart) * clip.perBeat

  return {
    start: span.start - clip.sourceStart < close ? 0 : start,
    end: !clip.loops && span.end > clip.sourceEnd - close
      ? null
      : (span.end - clip.sourceStart) * clip.perBeat,
  }
}

// the same, after one edge was dragged: the other edge keeps exactly the
// window it had, rather than one worked back out from where the clip is
// drawn (which is only as exact as its start beat is)
export const trimmedWindow = (
  clip: TrimClip,
  span: Span,
  edge: Edge,
  window: ClipWindow | null | undefined,
): ClipWindow => {
  const dragged = windowOf(clip, span)
  return edge === 'left'
    ? { start: dragged.start, end: window?.end ?? null }
    : { start: window?.start ?? 0, end: dragged.end }
}

// a sample clip, for trimming
// - sampleSeconds: the sample's length, before any change of speed
// - rate: how much faster than the sample the clip plays (see playbackRate)
// - beatSeconds: a beat's length
export const sampleTrimClip = (
  id: number,
  start: number,
  sampleSeconds: number,
  rate: number,
  window: ClipWindow | null | undefined,
  beatSeconds: number,
): TrimClip => {
  // seconds of the sample to a beat
  const perBeat = beatSeconds * rate
  const from = window?.start ?? 0
  const to = window?.end ?? sampleSeconds
  const sourceStart = start - from / perBeat

  return {
    id,
    start,
    end: start + (to - from) / perBeat,
    sourceStart,
    sourceEnd: sourceStart + sampleSeconds / perBeat,
    sourceLength: sampleSeconds,
    loops: false,
    minLength: MIN_CLIP_SECONDS / beatSeconds,
    perBeat,
  }
}

// a pattern clip, for trimming. it has a length of its own, and plays its
// pattern over and over when it's longer (see clipPasses in
// types/sequencer.ts), so its left edge goes back as far as its pattern's
// first step (its window's start is never under 0), and its right edge on
// past the pattern's end, as far as the playlist's
// - window: where it starts and ends in its pattern, in steps
// - patternLength: its pattern's steps
// - playlistEnd: the playlist's end, in beats (a clip already past it can
//   be trimmed back, not made longer)
export const patternTrimClip = (
  id: number,
  start: number,
  window: ClipWindow,
  patternLength: number,
  playlistEnd: number,
): TrimClip => {
  const from = window.start
  const end = start + ((window.end ?? from + patternLength) - from) / STEPS_PER_BEAT

  return {
    id,
    start,
    end,
    sourceStart: start - from / STEPS_PER_BEAT,
    sourceEnd: Math.max(playlistEnd, end),
    sourceLength: patternLength,
    loops: true,
    minLength: MIN_PATTERN_CLIP_STEPS / STEPS_PER_BEAT,
    perBeat: STEPS_PER_BEAT,
  }
}

// which of a sample clip's edges hide some of its sample beyond them (and
// get a triangle, see PlaylistObject)
// - from, to: the part of its sample the clip shows
// - sourceLength: all of it, in seconds
export const hiddenBeyond = (
  from: number,
  to: number,
  sourceLength: number,
): { left: boolean, right: boolean } => {
  if(!(sourceLength > 0)) return { left: false, right: false }
  // (a billionth of it is rounding, not trimming)
  const close = sourceLength * 1e-9
  return { left: from > close, right: to < sourceLength - close }
}

// the same for a pattern clip, which can play its pattern more than once:
// the edges that cut into a time through it, rather than being where the
// pattern starts or starts over
// - from, to: where the clip starts and ends in its pattern, in steps (not
//   wrapped: see PatternNode::start in engine/src/daw/daw_core/audiograph.rs)
export const patternHiddenBeyond = (
  from: number,
  to: number,
  patternLength: number,
): { left: boolean, right: boolean } => {
  if(!(patternLength > 0)) return { left: false, right: false }
  const close = patternLength * 1e-9
  const inside = (step: number) => step > close && step < patternLength - close
  return { left: inside(patternStepAt(from, 0, patternLength)), right: inside(patternStepAt(from, to - from, patternLength)) }
}
