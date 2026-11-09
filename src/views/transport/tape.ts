import { PlaylistTransport, Wind } from '../../types/transport'

export type { Wind }

// how much faster than play speed the tape winds
export const WIND_SPEED = 10

// with no clips, the tape is this many bars long, so the reels still move
export const EMPTY_TAPE_BARS = 4

// a bar, in ms
export const barMs = (t: PlaylistTransport) => t.beatIntervalMs * Math.max(1, t.beatsPerBar)

// the loop's length, or 0 when looping is off
export const loopLengthMs = (t: PlaylistTransport) =>
  (t.loopEnabled ? t.beatIntervalMs * t.loopBeats : 0)

// the tape's length: the loop, so the right reel fills just as playback
// jumps back to the start. without looping, to the end of the last clip
export const tapeLengthMs = (t: PlaylistTransport) => {
  const loop = loopLengthMs(t)
  if(loop > 0) return loop
  return t.songEndMs > 0 ? t.songEndMs : barMs(t) * EMPTY_TAPE_BARS
}

// where the playhead is at `nowMs` (ms since the unix epoch), following the
// engine's run while playing. matches daw::state::timeline_position
export const positionAt = (t: PlaylistTransport, nowMs: number): number => {
  if(!t.playing || t.startTimeMs === null) return t.positionMs

  const elapsed = t.startOffsetMs + Math.max(0, nowMs - t.startTimeMs)
  const loopMs = loopLengthMs(t)
  if(loopMs > 0) return elapsed >= loopMs ? elapsed % loopMs : elapsed

  // the engine stops at the end of the song, on the next beat
  return t.songEndMs > 0 ? Math.min(elapsed, t.songEndMs) : elapsed
}

// the bar and beat (from 1) at a position
export const barAndBeat = (positionMs: number, t: PlaylistTransport): { bar: number, beat: number } => {
  const beats = t.beatIntervalMs > 0 ? Math.floor(positionMs / t.beatIntervalMs + 1e-6) : 0
  const perBar = Math.max(1, t.beatsPerBar)
  return { bar: Math.floor(beats / perBar) + 1, beat: (beats % perBar) + 1 }
}

// a position as m:ss.t
export const formatTime = (positionMs: number): string => {
  const tenths = Math.floor(Math.max(0, positionMs) / 100)
  const minutes = Math.floor(tenths / 600)
  const seconds = Math.floor(tenths / 10) % 60
  return `${minutes}:${String(seconds).padStart(2, '0')}.${tenths % 10}`
}

// a little way into a bar still counts as its start, so tapping rewind
// twice quickly goes back two bars rather than sticking at one
const BAR_START_TOLERANCE = 0.25

// where a tap of fast forward (1) or rewind (-1) moves the playhead: to the
// next bar, or back to the start of this bar (the one before, if it's
// already there)
export const tapTarget = (positionMs: number, direction: 1 | -1, t: PlaylistTransport): number => {
  const bar = barMs(t)
  if(bar <= 0) return positionMs

  const into = positionMs / bar
  if(direction > 0) {
    return Math.min((Math.floor(into + 1e-6) + 1) * bar, Math.max(tapeLengthMs(t), positionMs))
  }

  const start = Math.floor(into + 1e-6)
  const target = into - start > BAR_START_TOLERANCE * (t.beatIntervalMs / bar) ? start : start - 1
  return Math.max(0, target * bar)
}


// where a wind has got to at `now` (performance.now()), kept on the tape
export const windPosition = (wind: Wind, now: number, t: PlaylistTransport): number => {
  const moved = wind.direction * WIND_SPEED * Math.max(0, now - wind.startedAt)
  const end = Math.max(tapeLengthMs(t), wind.fromMs)
  return Math.min(Math.max(wind.fromMs + moved, 0), end)
}

// whether a wind has reached the end of the tape it's winding towards
export const windDone = (wind: Wind, positionMs: number, t: PlaylistTransport): boolean =>
  (wind.direction < 0 ? positionMs <= 0 : positionMs >= Math.max(tapeLengthMs(t), wind.fromMs))

// the cassette's largest width and the smallest it shrinks to, in pixels.
// below the smallest, the tab scrolls instead
export const CASSETTE_MAX_WIDTH = 520
export const CASSETTE_MIN_WIDTH = 180

// how wide the cassette can be to fit in the room left beside and above the
// keys (heights and widths in pixels)
export const fitCassetteWidth = (
  roomWidth: number,
  roomHeight: number,
  aspect: number,
): number => {
  const fits = Math.min(roomWidth, roomHeight * aspect, CASSETTE_MAX_WIDTH)
  return Math.max(CASSETTE_MIN_WIDTH, Math.floor(fits))
}
