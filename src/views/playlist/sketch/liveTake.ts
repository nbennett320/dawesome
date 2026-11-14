// the take being recorded, drawn on each of its tracks as a red outlined
// region from where it started to the playhead, so you can see what's been
// recorded so far

// what the engine says about the take in progress (see TakeInProgress in
// engine/src/recording.rs)
export interface LiveTake {
  startBeat: number
  tracks: number[]
}

// the beats the region covers: from the take's start to the playhead. once
// the loop has come round (the playhead's back before the start), the pass
// being recorded now, from the loop's start
export const liveTakeSpan = (startBeat: number, playheadBeat: number): { start: number, end: number } | null => {
  const start = playheadBeat < startBeat ? 0 : startBeat
  return playheadBeat > start ? { start, end: playheadBeat } : null
}
