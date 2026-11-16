// copy, cut, paste and repeat clips (see clipboard.rs)
use crate::daw::clipboard::{copy, cut, paste, repeat, Placed};
use crate::commands::TState;

// copy clips to the clipboard, replacing what was there. with no clips
// (nothing was selected), the clipboard is left as it is. returns how many
// clips the clipboard holds
#[tauri::command]
pub fn copy_clips(state: TState<'_>, ids: Vec<u64>) -> usize {
  copy(&state, &ids)
}

// copy clips to the clipboard and take them out of the playlist, leaving
// the gap. one step to undo. returns how many clips the clipboard holds
#[tauri::command]
pub fn cut_clips(state: TState<'_>, ids: Vec<u64>) -> Result<usize, String> {
  cut(&state, &ids)
}

// paste the clipboard's clips, the earliest starting at `beat` (which the
// frontend has already snapped) and the rest as long after it as they were.
// each goes on the track it was copied from, or, when `track` is given, the
// topmost copied clip goes on that track and the others as many tracks
// below it as they were. a paste can overlap what's there. one step to undo
//
// if any clip would start past the playlist's end, nothing is pasted
#[tauri::command]
pub fn paste_clips(state: TState<'_>, beat: f64, track: Option<u32>) -> Result<Placed, String> {
  paste(&state, beat, track)
}

// duplicate clips so that the duplicates start exactly where the clips'
// span ends: from the earliest start among them to the latest end. each
// duplicate is on its original's track, and they keep the gaps between
// them. the clipboard isn't changed. one step to undo
//
// if any duplicate would start past the playlist's end, nothing is repeated
#[tauri::command]
pub fn repeat_clips(state: TState<'_>, ids: Vec<u64>) -> Result<Placed, String> {
  repeat(&state, &ids)
}
