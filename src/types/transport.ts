// the playlist's length, in bars of its time signature (PLAYLIST_BARS in
// engine/src/daw/state/defaults.rs). the engine works out how many beats
// that is, longer when clips reach further, and reports it as beatsDisplayed
export const PLAYLIST_BARS = 128
// a new project's time signature is 4/4. these stand in until the engine says
export const DEFAULT_BEATS_PER_BAR = 4
export const DEFAULT_PLAYLIST_BEATS = PLAYLIST_BARS * DEFAULT_BEATS_PER_BAR

// mirrors PlaylistTransport in engine/src/event_dispatch.rs
export interface PlaylistTransport {
  playing: boolean
  // engine start time of the current run, ms since the unix epoch (null
  // before the first run). for a pattern playing on its own, when step 0
  // of its time round was (or would have been) heard
  startTimeMs: number | null
  // where on the timeline the current run started
  startOffsetMs: number
  // where the playhead was when this was fetched
  positionMs: number
  beatIntervalMs: number
  // beats played before the engine loops back to the start
  loopBeats: number
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

// a wind (holding rewind or fast forward) in progress: which way, where it
// started, and when (performance.now())
export interface Wind {
  direction: 1 | -1
  fromMs: number
  startedAt: number
}
