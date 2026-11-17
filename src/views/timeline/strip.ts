import { PlaylistTransport } from '../../types/transport'
import { clamp } from '../../util/helpers'
import { SnapNote, snapNoteBeats, snapPoint } from '../../editing/snap'
import { loopMarks } from '../playlist/sketch/loopRegion'
import { barAndBeat, barMs, EMPTY_TAPE_BARS, formatTime } from '../transport/tape'

// the loop as the Timeline shows it, in ms: the same loop the playlist's
// ruler shows. a region is bright while looping and faint while not; with
// no region, looping goes round from the loop's start at the end flag
export interface LoopBand {
  start: number
  end: number
  bright: boolean
}

export const loopBand = (t: PlaylistTransport): LoopBand | null => {
  const { region, flag } = loopMarks(t)
  if(region) {
    return {
      start: region.start * t.beatIntervalMs,
      end: region.end * t.beatIntervalMs,
      bright: t.loopEnabled,
    }
  }
  if(flag === null || flag <= t.loopStartBeats) return null
  return { start: t.loopStartBeats * t.beatIntervalMs, end: flag * t.beatIntervalMs, bright: true }
}

// a clip ending a hair past a bar (float noise) ends on it
const ON_BAR_TOLERANCE_MS = 1

// where the strip ends, in ms: the last clip's end, rounded up to the next
// bar, and far enough to show all of the loop. with no clips, a few bars.
// the same as where the loop ends by default (loop_end_after in
// engine/src/daw/playlist.rs), so a loop that isn't moved fills the strip
export const timelineEndMs = (t: PlaylistTransport): number => {
  const bar = barMs(t)
  if(!(bar > 0)) return 0

  const song = t.songEndMs > 0
    ? (Math.floor(Math.max(0, t.songEndMs - ON_BAR_TOLERANCE_MS) / bar) + 1) * bar
    : bar * EMPTY_TAPE_BARS
  const loop = loopBand(t)
  return Math.max(song, loop ? loop.end : 0)
}

// a position along a strip `width` wide, showing `endMs` of song, and back
export const msToX = (ms: number, endMs: number, width: number): number =>
  (endMs > 0 ? clamp(ms / endMs, 0, 1) * width : 0)

export const xToMs = (x: number, endMs: number, width: number): number =>
  (width > 0 ? clamp(x / width, 0, 1) * endMs : 0)

// the closest bar ticks are drawn, in pixels: closer than this, only every
// 2nd, 4th, 8th… bar gets one
export const MIN_TICK_GAP = 6

// where the bar ticks go, in ms: between the strip's ends, every bar, or
// fewer when bars are narrow
export const barTicks = (t: PlaylistTransport, endMs: number, width: number): number[] => {
  const bar = barMs(t)
  if(!(bar > 0) || !(endMs > 0) || !(width > 0)) return []

  let every = 1
  while(every * bar / endMs * width < MIN_TICK_GAP && every * bar < endMs) every *= 2

  const ticks: number[] = []
  for(let ms = every * bar; ms < endMs - ON_BAR_TOLERANCE_MS; ms += every * bar) ticks.push(ms)
  return ticks
}

// the readout: the time and the bar and beat, e.g. 1:23.4 · bar 33.2
export const readout = (positionMs: number, t: PlaylistTransport): string => {
  const { bar, beat } = barAndBeat(positionMs, t)
  return `${formatTime(positionMs)} · bar ${bar}.${beat}`
}

// the playlist's snap setting
export interface TimelineSnap {
  enabled: boolean
  note: SnapNote
}

// the beat a click or a drag let go at `ms` moves the playhead to: snapped
// as a click in the playlist's ruler is (⌥ makes no difference there either),
// and kept within the playlist, which the engine keeps the playhead in
export const seekBeat = (ms: number, t: PlaylistTransport, snap: TimelineSnap): number => {
  if(!(t.beatIntervalMs > 0)) return 0
  const beat = snapPoint(ms / t.beatIntervalMs, {
    enabled: snap.enabled,
    every: snapNoteBeats(snap.note),
    playhead: null,
    bypass: false,
  })
  return clamp(beat, 0, Math.max(0, t.beatsDisplayed))
}
