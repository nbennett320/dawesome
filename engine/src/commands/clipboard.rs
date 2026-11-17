// copy, cut, paste and repeat clips (see clipboard.rs)
use crate::daw::clipboard::{
  clear_span, copy, copy_span, cut, cut_span, paste, paste_span, repeat, repeat_span, Placed, Span, SpanEdit,
};
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

// the time selection's commands (see clipboard/span.rs). a span is beats
// `start` to `end` on tracks `top` to `bottom`. a clip across one of its ends
// is cut there, and what a paste or a repeat lands on is replaced. each
// change is one step to undo

// copy what's in a span to the clipboard, as a span. returns how many clips
// the clipboard holds
#[tauri::command]
pub fn copy_span_clips(state: TState<'_>, span: Span) -> Result<usize, String> {
  copy_span(&state, span)
}

// copy what's in a span, then take it out, leaving the gap
#[tauri::command]
pub fn cut_span_clips(state: TState<'_>, span: Span) -> Result<SpanEdit, String> {
  cut_span(&state, span)
}

// take what's in a span out, leaving the gap
#[tauri::command]
pub fn clear_span_clips(state: TState<'_>, span: Span) -> Result<SpanEdit, String> {
  clear_span(&state, span)
}

// paste a copied span with its start on `beat` and its top track on
// `track`, replacing what's there. nothing is pasted if a clip would start
// past the playlist's end
#[tauri::command]
pub fn paste_span_clips(state: TState<'_>, beat: f64, track: u32) -> Result<SpanEdit, String> {
  paste_span(&state, beat, track)
}

// put a copy of what's in a span straight after it, replacing what's there
#[tauri::command]
pub fn repeat_span_clips(state: TState<'_>, span: Span) -> Result<SpanEdit, String> {
  repeat_span(&state, span)
}
