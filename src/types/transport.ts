// the playlist's length, in bars of its time signature (PLAYLIST_BARS in
// engine/src/daw/defaults.rs). the engine works out how many beats
// that is, longer when clips reach further, and reports it as beatsDisplayed
export const PLAYLIST_BARS = 128
// a new project's time signature is 4/4. these stand in until the engine says
export const DEFAULT_BEATS_PER_BAR = 4
export const DEFAULT_PLAYLIST_BEATS = PLAYLIST_BARS * DEFAULT_BEATS_PER_BAR

// mirrors PlaylistTransport in engine/src/daw/actions/transport.rs
export interface PlaylistTransport {
  playing: boolean
  // engine start time of the current run, ms since the unix epoch (null
  // before the first run). for a pattern playing on its own, when step 0
  // of its time round was (or would have been) heard
  startTimeMs: number | null
  // where on the timeline the current run started
  startOffsetMs: number
  // how fast the song plays (the tape's PITCH knob, with the tape on): the
  // playhead moves this many ms a ms after startTimeMs. 1 when left out
  speed?: number
  // where the playhead was when this was fetched
  positionMs: number
  // where the last play started, in beats (null before anything's played).
  // ⌘← goes back there
  lastPlayStartBeats: number | null
  beatIntervalMs: number
  // where the loop starts and ends, in beats (whether or not looping is
  // on). playback plays in from before its start, and goes round to its
  // start at its end
  loopStartBeats: number
  loopEndBeats: number
  // whether there's a region drawn in the ruler (which is the loop), and
  // whether the end flag was dragged somewhere rather than following the
  // last clip
  loopRegion: boolean
  loopEndPinned: boolean
  // beats spanned by the full playlist width
  beatsDisplayed: number
  loopEnabled: boolean
  // where the last clip finishes (0 with no clips)
  songEndMs: number
  beatsPerBar: number
  // the pattern playing on its own (Play in the Sequencer tab), or null.
  // while one plays, the playlist's playhead stays still
  patternId: number | null
  // how long that pattern is, in steps (0 for none). it loops at this
  // length, counted from startTimeMs, which moves when its length changes:
  // the engine counts its steps afresh from the one it's on then
  patternLength: number
}

// how much of the song has played since a run's start time, at `nowMs`
// (ms since the unix epoch): at another speed (the varispeed) the song
// runs faster or slower than the clock
export const sinceStartMs = (t: { startTimeMs: number, speed?: number }, nowMs: number) =>
  (nowMs - t.startTimeMs) * (t.speed ?? 1)

// a part of the playlist drawn in the ruler, that playback loops while
// looping is on, in beats (start before end). mirrors LoopRegion in
// engine/src/daw/playlist.rs
export interface LoopRegion {
  start: number
  end: number
}

// the loop as the engine has it: the region, where the end flag was dragged
// to (in beats), and whether looping is on. mirrors LoopSettings in
// engine/src/daw/actions/transport.rs
export interface LoopSettings {
  region: LoopRegion | null
  endPin: number | null
  enabled: boolean
}

// a wind (holding rewind or fast forward) in progress: which way, where it
// started, and when (performance.now())
export interface Wind {
  direction: 1 | -1
  fromMs: number
  startedAt: number
}
