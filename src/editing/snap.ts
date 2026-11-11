// what an edge, a cut or a new item snaps to, the same in the playlist and
// the Piano Roll. plain functions, with no canvas

// the lengths the snap control offers, as the engine names them
// (daw::timing::Note)
export type SnapNote =
  | 'WholeNote'
  | 'HalfNote'
  | 'QuarterNote'
  | 'EighthNote'
  | 'SixteenthNote'

// a snap length in beats, a beat being a quarter note (Note::beats in
// engine/src/daw/daw_core/timing.rs)
export const snapNoteBeats = (note: SnapNote): number => ({
  WholeNote: 4,
  HalfNote: 2,
  QuarterNote: 1,
  EighthNote: 0.5,
  SixteenthNote: 0.25,
}[note])

// what the mouse snaps to just now. lengths and places are in the view's
// own unit along time: beats in the playlist, steps in the Piano Roll
export interface Snapping {
  // the view's snap setting
  enabled: boolean
  // a snap line every this many
  every: number
  // the playhead, while it's somewhere to snap to: stopped or paused. null
  // while playing, and in a view with none
  playhead: number | null
  // Option is held: nothing snaps
  bypass: boolean
}

// where an edge or a cut lands, with the mouse at `at`: on the nearest snap
// line, or on the playhead when the mouse is nearer to it than to the snap
// lines either side
export const snapPoint = (at: number, snap: Snapping): number => {
  if(snap.bypass || !snap.enabled || !(snap.every > 0)) return at

  const below = Math.floor(at / snap.every) * snap.every
  const above = below + snap.every
  const head = snap.playhead
  if(head !== null && Math.abs(at - head) < Math.min(at - below, above - at)) return head

  return Math.round(at / snap.every) * snap.every
}

// the snap line at or before `at`: where something new starts when it's
// drawn with the mouse there (the line the mouse is past, not the nearest)
export const snapDown = (at: number, snap: Snapping): number => {
  if(snap.bypass || !snap.enabled || !(snap.every > 0)) return at
  return Math.floor(at / snap.every) * snap.every
}
