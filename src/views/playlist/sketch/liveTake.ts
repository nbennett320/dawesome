// the take being recorded, drawn on each of its tracks as a red outlined
// region from where it started to the playhead, so you can see what's been
// recorded so far

// what the engine says about the take in progress (see TakeInProgress in
// engine/src/recording.rs). once a loop recording's loop comes round, it
// says the pass being recorded starts at the loop's start
export interface LiveTake {
  startBeat: number
  tracks: number[]
}

// the beats the region covers: from the take's start to the playhead. once
// the loop has come round (the playhead's back before the start), the pass
// being recorded now, from the loop's start (`loopStartBeat`: a region's, or
// the playlist's)
export const liveTakeSpan = (startBeat: number, playheadBeat: number, loopStartBeat = 0): { start: number, end: number } | null => {
  const start = playheadBeat < startBeat ? Math.min(loopStartBeat, playheadBeat) : startBeat
  return playheadBeat > start ? { start, end: playheadBeat } : null
}
