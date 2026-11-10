// trimming clips by their edges, and where an edge or a cut lands. kept free
// of p5 so it can be tested on its own
import { ClipWindow } from '../../../types/clip'
import { STEPS_PER_BEAT } from '../../../types/sequencer'

export type Edge = 'left' | 'right'

// how wide the zone inside each edge of a clip is, where pressing grabs the
// edge rather than the clip. in screen pixels, whatever the zoom
export const EDGE_ZONE_PX = 6

// the shortest a sample clip can be trimmed or cut to, in seconds of sound
// (MIN_CLIP in engine/src/daw/daw_core/audiograph.rs)
export const MIN_CLIP_SECONDS = 0.01
// and a pattern clip, in steps
export const MIN_PATTERN_CLIP_STEPS = 1

// the edge of a clip the mouse is over, if any
// - x: the mouse, in screen pixels from the clip's left edge
// - width: the clip's width, in screen pixels
// on a narrow clip each zone is a third of it at most, so the middle third
// still moves the clip
export const edgeAt = (x: number, width: number): Edge | null => {
  if(!(width > 0) || x < 0 || x > width) return null

  const zone = Math.min(EDGE_ZONE_PX, width / 3)
  if(x < zone) return 'left'
  if(x > width - zone) return 'right'
  return null
}

// what an edge or a cut snaps to
export interface Snapping {
  // the playlist's snap setting
  enabled: boolean
  // the length of a snap step, in beats
  beats: number
  // the playhead's beat, while it's somewhere to snap to: stopped or paused.
  // null while playing
  playheadBeat: number | null
  // Option is held: nothing snaps
  bypass: boolean
}

// where an edge or a cut lands, with the mouse at `beat`: on the nearest
// snap step, or on the playhead when the mouse is nearer to it than to the
// snap steps either side
export const snapPoint = (beat: number, snap: Snapping): number => {
  if(snap.bypass || !snap.enabled || !(snap.beats > 0)) return beat

  const below = Math.floor(beat / snap.beats) * snap.beats
  const above = below + snap.beats
  const head = snap.playheadBeat
  if(head !== null && Math.abs(beat - head) < Math.min(beat - below, above - beat)) return head

  return Math.round(beat / snap.beats) * snap.beats
}

// a clip, as far as trimming goes. everything is in beats of the playlist
export interface TrimClip {
  id: number
  startBeat: number
  endBeat: number
  // where its sample or pattern would start and end, were all of it
  // played: as far as its edges can go. (the start can be before the
  // playlist's, where no edge can follow it)
  sourceStartBeat: number
  sourceEndBeat: number
  // the shortest it can be
  minBeats: number
}

// where a clip starts and ends
export interface Span {
  startBeat: number
  endBeat: number
}

const clamp = (n: number, low: number, high: number) => Math.min(Math.max(n, low), Math.max(low, high))

// a clip with one edge moved by `delta` beats, stopped where it can go no
// further: the source's ends, the playlist's start, and the other edge
// (less the shortest a clip can be)
export const trimmed = (clip: TrimClip, edge: Edge, delta: number): Span => {
  if(edge === 'left') {
    const earliest = Math.max(clip.sourceStartBeat, 0)
    return {
      startBeat: clamp(clip.startBeat + delta, earliest, clip.endBeat - clip.minBeats),
      endBeat: clip.endBeat,
    }
  }

  const earliest = Math.min(clip.startBeat + clip.minBeats, clip.sourceEndBeat)
  return {
    startBeat: clip.startBeat,
    endBeat: clamp(clip.endBeat + delta, earliest, clip.sourceEndBeat),
  }
}

// every clip of a trim: the grabbed clip's edge goes to `target` (where the
// mouse is, after snapping), and the same edge of each of the others moves
// as far. each stops at its own limits, wherever the others are
export const trimAll = (
  clips: ReadonlyArray<TrimClip>,
  grabbedId: number,
  edge: Edge,
  target: number,
): Array<Span & { id: number }> => {
  const grabbed = clips.find(clip => clip.id === grabbedId)
  if(!grabbed) return []

  const delta = target - (edge === 'left' ? grabbed.startBeat : grabbed.endBeat)
  return clips.map(clip => ({ id: clip.id, ...trimmed(clip, edge, delta) }))
}

// the part of its source a clip plays once it's trimmed to `span`, as the
// engine takes it (see set_clip_windows in engine/src/clip_commands.rs)
// - perBeat: how much of the source goes by in a beat: seconds of the
//   sample (at the clip's speed), or steps of the pattern
export const windowOf = (clip: TrimClip, span: Span, perBeat: number): ClipWindow => {
  // (a beat's billionth: less than a nanosecond of sound)
  const close = 1e-9
  const start = (span.startBeat - clip.sourceStartBeat) * perBeat

  return {
    start: span.startBeat - clip.sourceStartBeat < close ? 0 : start,
    end: span.endBeat > clip.sourceEndBeat - close ? null : (span.endBeat - clip.sourceStartBeat) * perBeat,
  }
}

// a sample clip, for trimming
// - sampleSeconds: the sample's length, before any change of speed
// - rate: how much faster than the sample the clip plays (see playbackRate)
// - beatSeconds: a beat's length
export const sampleTrimClip = (
  id: number,
  startBeat: number,
  sampleSeconds: number,
  rate: number,
  window: ClipWindow | null | undefined,
  beatSeconds: number,
): TrimClip => {
  // seconds of the sample to a beat
  const perBeat = beatSeconds * rate
  const from = window?.start ?? 0
  const to = window?.end ?? sampleSeconds
  const sourceStartBeat = startBeat - from / perBeat

  return {
    id,
    startBeat,
    endBeat: startBeat + (to - from) / perBeat,
    sourceStartBeat,
    sourceEndBeat: sourceStartBeat + sampleSeconds / perBeat,
    minBeats: MIN_CLIP_SECONDS / beatSeconds,
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
  startBeat: number,
  length: number,
  window: ClipWindow | null | undefined,
): TrimClip => {
  const [from, to] = patternSpan(window, length)
  const sourceStartBeat = startBeat - from / STEPS_PER_BEAT

  return {
    id,
    startBeat,
    endBeat: startBeat + (to - from) / STEPS_PER_BEAT,
    sourceStartBeat,
    sourceEndBeat: sourceStartBeat + length / STEPS_PER_BEAT,
    minBeats: MIN_PATTERN_CLIP_STEPS / STEPS_PER_BEAT,
  }
}

// which of a clip's edges hide some of its source beyond them (and get a
// triangle, see PlaylistObject)
// - sourceLength: the sample's seconds, or the pattern's steps
export const hiddenBeyond = (
  window: ClipWindow | null | undefined,
  sourceLength: number,
): { left: boolean, right: boolean } => ({
  left: !!window && window.start > 0,
  right: !!window && window.end !== null && window.end < sourceLength,
})
