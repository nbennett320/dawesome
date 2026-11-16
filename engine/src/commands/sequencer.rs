// the Pattern Sequencer's commands (see daw/actions/sequencer.rs)
use std::thread;
use crate::daw::actions::sequencer::{
  add_pattern_clip_to, add_pattern_folder_to, add_pattern_of, add_sample, beats_per_bar, duplicate_pattern_of, edit,
  edit_pattern, edit_row, explode, paint, remove_pattern_folder_of, remove_pattern_of, remove_row_of, replace_sample,
  set_pattern_notes_in, set_pattern_step_in, toggle_pattern_in, trim_pattern_in, AddedFolder, AddedPattern, AddedRow,
  Exploded, Painted, RemovedPattern,
};
use crate::daw::{playback, sequencer};
use crate::engine;
use crate::commands::TState;
use crate::daw::actions::synth::sequencer_state;
use crate::daw::project::LoadedPatternClip;

#[tauri::command]
pub fn get_sequencer(state: TState<'_>) -> sequencer::SequencerState {
  sequencer_state(&state)
}

// a row playing the sample at `path`, added at the bottom
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn add_sample_row(state: TState<'_>, path: String) -> Result<AddedRow, String> {
  add_sample(&state, &path)
}

// take a row out, with its notes in every pattern (and a synth row's
// plugin), each pattern they were in trimmed to what's left (see
// Sequencer::trim_pattern)
#[tauri::command]
pub fn remove_row(state: TState<'_>, row_id: u64) -> Result<sequencer::SequencerState, String> {
  remove_row_of(&state, row_id)
}

// move a row to `index` in the list
#[tauri::command]
pub fn move_row(state: TState<'_>, row_id: u64, index: usize) -> Result<sequencer::SequencerState, String> {
  edit(&state, None, |seq| seq.move_row(row_id, index))
}

// label a row ("Snare"). a blank label shows its instrument's name again
#[tauri::command]
pub fn rename_row(state: TState<'_>, row_id: u64, name: String) -> Result<sequencer::RowChange, String> {
  edit_row(&state, None, row_id, |seq| seq.rename_row(row_id, &name))
}

// a row's volume, as a gain: 1 leaves the sound as is
#[tauri::command]
pub fn set_row_volume(
  state: TState<'_>,
  row_id: u64,
  volume: f32,
  gesture: Option<u64>,
) -> Result<sequencer::RowChange, String> {
  edit_row(&state, gesture, row_id, |seq| seq.set_row_volume(row_id, volume))
}

// a row's pan, -1 (left) to 1 (right)
#[tauri::command]
pub fn set_row_pan(
  state: TState<'_>,
  row_id: u64,
  pan: f32,
  gesture: Option<u64>,
) -> Result<sequencer::RowChange, String> {
  edit_row(&state, gesture, row_id, |seq| seq.set_row_pan(row_id, pan))
}

#[tauri::command]
pub fn set_row_muted(
  state: TState<'_>,
  row_id: u64,
  muted: bool,
  gesture: Option<u64>,
) -> Result<sequencer::RowChange, String> {
  edit_row(&state, gesture, row_id, |seq| seq.set_row_muted(row_id, muted))
}

// play the sample at `path` on a sample row instead, keeping its notes and
// everything else about it
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn replace_row_sample(
  state: TState<'_>,
  row_id: u64,
  path: String,
) -> Result<sequencer::RowChange, String> {
  replace_sample(&state, row_id, &path)
}

// play a sample row's sample once, as it is, at its volume and pan
// (whether it's muted or not)
#[tauri::command]
pub fn preview_row(state: TState<'_>, row_id: u64) -> Result<(), String> {
  let instrument = state.playlist.sequencer.lock().unwrap()
    .row(row_id)
    .ok_or_else(|| format!("no row with id {}", row_id))?
    .sample_instrument()
    .ok_or_else(|| format!("row {} plays a synth, which has no sample to preview", row_id))?;

  // (the output may have to be opened first, which takes a moment)
  thread::spawn(move || {
    let previewed = engine::engine().and_then(|engine| engine.preview(instrument.preview(engine.sample_rate())));
    if let Err(e) = previewed {
      println!("could not preview row {}: {}", row_id, e);
    }
  });

  Ok(())
}

// the pitch a row's clicked steps play. the row's notes move with it in
// each pattern where all of them were at the old one (so a row showing
// steps still does)
#[tauri::command]
pub fn set_row_step_pitch(state: TState<'_>, row_id: u64, pitch: u8) -> Result<sequencer::SequencerState, String> {
  edit(&state, None, |seq| seq.set_row_step_pitch(row_id, pitch))
}

// the pitch that plays a sample row's sample unchanged
#[tauri::command]
pub fn set_row_sample_pitch(state: TState<'_>, row_id: u64, pitch: u8) -> Result<sequencer::RowChange, String> {
  edit_row(&state, None, row_id, |seq| seq.set_row_sample_pitch(row_id, pitch))
}

// whether a sample row's notes stop at their ends, or play the sample out
#[tauri::command]
pub fn set_row_gated(state: TState<'_>, row_id: u64, gated: bool) -> Result<sequencer::RowChange, String> {
  edit_row(&state, None, row_id, |seq| seq.set_row_gated(row_id, gated))
}

// a new, empty pattern, `bars` long, at the end
#[tauri::command]
pub fn add_pattern(state: TState<'_>, bars: u32) -> Result<AddedPattern, String> {
  add_pattern_of(&state, bars)
}

// a copy of a pattern, straight after it
#[tauri::command]
pub fn duplicate_pattern(state: TState<'_>, pattern_id: u64) -> Result<AddedPattern, String> {
  duplicate_pattern_of(&state, pattern_id)
}

// delete a pattern, and every clip of it in the playlist
#[tauri::command]
pub fn remove_pattern(state: TState<'_>, pattern_id: u64) -> Result<RemovedPattern, String> {
  remove_pattern_of(&state, pattern_id)
}

// a new, empty folder of patterns in `parent` (none: the top)
#[tauri::command]
pub fn add_pattern_folder(state: TState<'_>, name: String, parent: Option<u64>) -> Result<AddedFolder, String> {
  add_pattern_folder_to(&state, &name, parent)
}

#[tauri::command]
pub fn rename_pattern_folder(state: TState<'_>, folder_id: u64, name: String) -> Result<sequencer::SequencerState, String> {
  edit(&state, None, |seq| seq.rename_folder(folder_id, &name))
}

// move a folder of patterns into another, or (none) to the top
#[tauri::command]
pub fn move_pattern_folder(state: TState<'_>, folder_id: u64, parent: Option<u64>) -> Result<sequencer::SequencerState, String> {
  edit(&state, None, |seq| seq.move_folder(folder_id, parent))
}

// put a pattern in a folder, or (none) at the top
#[tauri::command]
pub fn move_pattern(state: TState<'_>, pattern_id: u64, folder: Option<u64>) -> Result<sequencer::SequencerState, String> {
  edit(&state, None, |seq| seq.move_pattern(pattern_id, folder))
}

// delete a folder of patterns, with its folders and patterns. refused while
// any of its patterns is in the playlist, so no clip goes with it unseen
#[tauri::command]
pub fn remove_pattern_folder(state: TState<'_>, folder_id: u64) -> Result<sequencer::SequencerState, String> {
  remove_pattern_folder_of(&state, folder_id)
}

// name a pattern. a blank name shows its number again
#[tauri::command]
pub fn rename_pattern(state: TState<'_>, pattern_id: u64, name: String) -> Result<sequencer::SequencerState, String> {
  edit(&state, None, |seq| seq.rename_pattern(pattern_id, &name))
}

// make a pattern `bars` long (1 to PATTERN_BARS_MAX), and never trimmed
// shorter. its clips in the playlist keep their own lengths
#[tauri::command]
pub fn set_pattern_bars(state: TState<'_>, pattern_id: u64, bars: u32) -> Result<sequencer::PatternChange, String> {
  let beats = beats_per_bar(&state);
  edit_pattern(&state, None, pattern_id, |seq| seq.set_pattern_bars(pattern_id, bars, beats))
}

// turn one of a row's steps on or off in a pattern, growing it to fit a
// step past its end (see Sequencer::set_pattern_step). `trim` (a step
// cleared; missing is false): the pattern is trimmed after, in the same
// step to undo (see Sequencer::trim_pattern)
#[tauri::command]
pub fn set_pattern_step(
  state: TState<'_>,
  pattern_id: u64,
  row_id: u64,
  step: u32,
  on: bool,
  gesture: Option<u64>,
  trim: Option<bool>,
) -> Result<sequencer::PatternChange, String> {
  set_pattern_step_in(&state, pattern_id, row_id, step, on, gesture, trim.unwrap_or(false))
}

// replace a row's notes in a pattern (the Piano Roll), sorted, growing it to
// fit the notes added (see Sequencer::edit_pattern_notes). one drag of the
// mouse (the same gesture) is one step to undo. `trim` (notes deleted;
// missing is false): the pattern is trimmed after, in the same step (see
// Sequencer::trim_pattern)
#[tauri::command]
pub fn set_pattern_notes(
  state: TState<'_>,
  pattern_id: u64,
  row_id: u64,
  notes: Vec<sequencer::PatternNote>,
  gesture: Option<u64>,
  trim: Option<bool>,
) -> Result<sequencer::PatternChange, String> {
  set_pattern_notes_in(&state, pattern_id, row_id, notes, gesture, trim.unwrap_or(false))
}

// the trim a clear held back while the play cursor was in the bars it takes
// away (the frontend sends it once the cursor has left them), with the
// clear's gesture, so the two are one step to undo while the clear is the
// newest. with nothing to trim (notes went back in those bars), nothing
// changes and it's no step
#[tauri::command]
pub fn trim_pattern(state: TState<'_>, pattern_id: u64, gesture: Option<u64>) -> Result<sequencer::PatternChange, String> {
  trim_pattern_in(&state, pattern_id, gesture)
}

// loop a pattern on its own, from `from_step` (its start if not given),
// stopping whatever was playing. its synth rows' notes play on their
// plugins' master copies
#[tauri::command]
pub fn play_pattern(state: TState<'_>, pattern_id: u64, from_step: Option<f64>) -> Result<(), String> {
  playback::start_pattern(state.inner(), pattern_id, from_step.unwrap_or(0.))
}

// pause whatever is playing, or loop the pattern if nothing is (from
// `from_step`, or its start)
#[tauri::command]
pub fn toggle_pattern(state: TState<'_>, pattern_id: u64, from_step: Option<f64>) -> Result<(), String> {
  toggle_pattern_in(state.inner(), pattern_id, from_step.unwrap_or(0.))
}

// while the pattern plays on its own, carry on from `step` (the Piano Roll's
// ruler). otherwise nothing happens
#[tauri::command]
pub fn seek_pattern(state: TState<'_>, pattern_id: u64, step: f64) -> Result<(), String> {
  playback::seek_pattern(state.inner(), pattern_id, step)
}

// place a pattern in the playlist, dropped at `drop_beat` on `track_number`
// and snapped by the snap setting. the clip stays linked to the pattern, and
// is as long as the pattern is now. returns the clip as it's placed
#[tauri::command]
pub fn add_pattern_clip(
  state: TState<'_>,
  pattern_id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<LoadedPatternClip, String> {
  add_pattern_clip_to(&state, pattern_id, track_number, drop_beat)
}

// split pattern clips into a pattern clip per row, sample or synth, each
// playing a pattern of just that row's notes on a track of its own (see
// AudioGraph::explode_pattern_node). ids that aren't pattern clips, and
// clips of a pattern that plays nothing, are skipped. clips exploded before
// one that fails stay exploded
#[tauri::command]
pub fn explode_pattern_clips(state: TState<'_>, ids: Vec<u64>) -> Result<Exploded, String> {
  explode(&state, &ids)
}

// paint clips of a pattern along a track with the Draw tool, from
// `start_beat` to `end_beat` (snapped by the page, not here): clips of it end
// to end, the last trimmed to stop at the end (see paint_spans). with no
// pattern, a new empty pattern long enough to hold the drag, and one clip
// of it from start to end. one step to undo, the new pattern with it
#[tauri::command]
pub fn paint_pattern_clips(
  state: TState<'_>,
  pattern_id: Option<u64>,
  track_number: u32,
  start_beat: f64,
  end_beat: f64,
) -> Result<Painted, String> {
  paint(&state, pattern_id, track_number, start_beat, end_beat)
}
