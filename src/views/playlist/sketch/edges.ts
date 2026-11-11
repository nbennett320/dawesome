// clips' edges: how a sample clip and a pattern clip are trimmed and
// stretched, on top of the editing rules the playlist shares with the Piano
// Roll (src/editing). kept free of p5 so it can be tested on its own
import { ClipWindow } from '../../../types/clip'
import { STEPS_PER_BEAT } from '../../../types/sequencer'
import { Edge, Span, TrimItem } from '../../../editing/edges'

export { EDGE_ZONE_PX, edgeAt, trimAll, trimmed } from '../../../editing/edges'
export type { Edge, Span } from '../../../editing/edges'
export { snapPoint } from '../../../editing/snap'
export type { Snapping } from '../../../editing/snap'

// the shortest a sample clip can be trimmed or cut to, in seconds of sound
// (MIN_CLIP in engine/src/daw/daw_core/audiograph.rs)
export const MIN_CLIP_SECONDS = 0.01
// and a pattern clip, in steps
export const MIN_PATTERN_CLIP_STEPS = 1

// a clip, as far as trimming goes. everything is in beats of the playlist
export interface TrimClip extends TrimItem {
  // how much of its source goes by in a beat: seconds of the sample (at
  // the clip's speed), or steps of the pattern
  perBeat: number
}

const clamp = (n: number, low: number, high: number) => Math.min(Math.max(n, low), Math.max(low, high))

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
  const next = clamp(length * ratio, shortest, longest)

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
// engine takes it (see set_clip_windows in engine/src/clip_commands.rs)
export const windowOf = (clip: TrimClip, span: Span): ClipWindow => {
  // (a beat's billionth: less than a nanosecond of sound)
  const close = 1e-9
  const start = (span.start - clip.sourceStart) * clip.perBeat

  return {
    start: span.start - clip.sourceStart < close ? 0 : start,
    end: span.end > clip.sourceEnd - close
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
    minLength: MIN_CLIP_SECONDS / beatSeconds,
    perBeat,
  }
}

// the steps a pattern clip spans, in a pattern `length` steps long: from,
// to. the window is kept as it was set, so a clip of a pattern made shorter
// ends with the pattern, and one whose window is past the pattern's end is
// a step long (PatternNode::span in engine/src/daw/daw_core/audiograph.rs)
export const patternSpan = (window: ClipWindow | null | undefined, length: number): [number, number] => {
  if(!window) return [0, length]

  const end = Math.min(window.end ?? length, length)
  return [window.start, Math.max(end, window.start + MIN_PATTERN_CLIP_STEPS)]
}

// a pattern clip, for trimming
// - length: its pattern's length, in steps
export const patternTrimClip = (
  id: number,
  start: number,
  length: number,
  window: ClipWindow | null | undefined,
): TrimClip => {
  const [from, to] = patternSpan(window, length)
  const sourceStart = start - from / STEPS_PER_BEAT

  return {
    id,
    start,
    end: start + (to - from) / STEPS_PER_BEAT,
    sourceStart,
    sourceEnd: sourceStart + length / STEPS_PER_BEAT,
    minLength: MIN_PATTERN_CLIP_STEPS / STEPS_PER_BEAT,
    perBeat: STEPS_PER_BEAT,
  }
}

// which of a clip's edges hide some of its source beyond them (and get a
// triangle, see PlaylistObject)
// - from, to: the part of its source the clip shows
// - sourceLength: all of it: the sample's seconds, or the pattern's steps
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
