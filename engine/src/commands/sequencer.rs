// the Pattern Sequencer's commands (see daw/daw_core/sequencer.rs), and
// placing patterns in the playlist
//
// every edit returns what it changed (one row, one pattern, or the whole
// sequencer), so the frontend's copy stays the same as the engine's. edits
// take effect straight away, even while playing
use std::sync::Arc;
use std::thread;
use serde::Serialize;

use crate::daw;
use crate::history;
use crate::commands::playlist::beat_to_offset;
use crate::project::LoadedPatternClip;

use crate::commands::synth::sequencer_state;

// run an edit on the sequencer, as a step that can be undone, then return
// the whole sequencer
// - gesture: the press of the mouse it's part of, if any (a stroke painting
//   steps, a volume knob dragged): one press is one step to undo
pub(crate) fn edit(
  state: &daw::InnerState,
  gesture: Option<u64>,
  f: impl FnOnce(&mut daw::Sequencer) -> Result<(), String>,
) -> Result<daw::SequencerState, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  Ok(sequencer_state(state))
}

// the same for an edit of one row's own settings, returning just that row
pub(crate) fn edit_row(
  state: &daw::InnerState,
  gesture: Option<u64>,
  row_id: u64,
  f: impl FnOnce(&mut daw::Sequencer) -> Result<(), String>,
) -> Result<daw::RowChange, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  state.playlist.sequencer.lock().unwrap().row_change(row_id, &*state.playlist.effects)
}

// and for an edit of one pattern, returning just that pattern
pub(crate) fn edit_pattern(
  state: &daw::InnerState,
  gesture: Option<u64>,
  pattern_id: u64,
  f: impl FnOnce(&mut daw::Sequencer) -> Result<(), String>,
) -> Result<daw::PatternChange, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  state.playlist.sequencer.lock().unwrap().pattern_change(pattern_id)
}

fn beats_per_bar(state: &daw::InnerState) -> u16 {
  state.playlist.time_signature.lock().unwrap().numerator
}

#[tauri::command]
pub fn get_sequencer(state: daw::TState<'_>) -> daw::SequencerState {
  sequencer_state(&state)
}

// a row that was just added, and the sequencer with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddedRow {
  pub row_id: u64,
  pub sequencer: daw::SequencerState,
}

// a row playing the sample at `path`, added at the bottom
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn add_sample_row(state: daw::TState<'_>, path: String) -> Result<AddedRow, String> {
  add_sample(&state, &path)
}

fn add_sample(state: &daw::InnerState, path: &str) -> Result<AddedRow, String> {
  // decoded before taking the sequencer's lock, which playback needs
  let sound = daw::RowSound::load(path)?;
  let row_id = history::sequencer_edit(state, &[], None, || {
    Ok(state.playlist.sequencer.lock().unwrap().add_sample_row(path, sound))
  })?;
  Ok(AddedRow { row_id, sequencer: sequencer_state(state) })
}

// take a row out, with its notes in every pattern (and a synth row's plugin)
#[tauri::command]
pub fn remove_row(state: daw::TState<'_>, row_id: u64) -> Result<daw::SequencerState, String> {
  edit(&state, None, |seq| seq.remove_row(row_id))
}

// move a row to `index` in the list
#[tauri::command]
pub fn move_row(state: daw::TState<'_>, row_id: u64, index: usize) -> Result<daw::SequencerState, String> {
  edit(&state, None, |seq| seq.move_row(row_id, index))
}

// label a row ("Snare"). a blank label shows its instrument's name again
#[tauri::command]
pub fn rename_row(state: daw::TState<'_>, row_id: u64, name: String) -> Result<daw::RowChange, String> {
  edit_row(&state, None, row_id, |seq| seq.rename_row(row_id, &name))
}

// a row's volume, as a gain: 1 leaves the sound as is
#[tauri::command]
pub fn set_row_volume(
  state: daw::TState<'_>,
  row_id: u64,
  volume: f32,
  gesture: Option<u64>,
) -> Result<daw::RowChange, String> {
  edit_row(&state, gesture, row_id, |seq| seq.set_row_volume(row_id, volume))
}

// a row's pan, -1 (left) to 1 (right)
#[tauri::command]
pub fn set_row_pan(
  state: daw::TState<'_>,
  row_id: u64,
  pan: f32,
  gesture: Option<u64>,
) -> Result<daw::RowChange, String> {
  edit_row(&state, gesture, row_id, |seq| seq.set_row_pan(row_id, pan))
}

#[tauri::command]
pub fn set_row_muted(
  state: daw::TState<'_>,
  row_id: u64,
  muted: bool,
  gesture: Option<u64>,
) -> Result<daw::RowChange, String> {
  edit_row(&state, gesture, row_id, |seq| seq.set_row_muted(row_id, muted))
}

// play the sample at `path` on a sample row instead, keeping its notes and
// everything else about it
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn replace_row_sample(
  state: daw::TState<'_>,
  row_id: u64,
  path: String,
) -> Result<daw::RowChange, String> {
  replace_sample(&state, row_id, &path)
}

fn replace_sample(state: &daw::InnerState, row_id: u64, path: &str) -> Result<daw::RowChange, String> {
  // decoded before taking the sequencer's lock, which playback needs
  let sound = daw::RowSound::load(path)?;
  edit_row(state, None, row_id, |seq| seq.replace_row_sample(row_id, path, sound))
}

// play a sample row's sample once, as it is, at its volume and pan
// (whether it's muted or not)
#[tauri::command]
pub fn preview_row(state: daw::TState<'_>, row_id: u64) -> Result<(), String> {
  let instrument = state.playlist.sequencer.lock().unwrap()
    .row(row_id)
    .ok_or_else(|| format!("no row with id {}", row_id))?
    .sample_instrument()
    .ok_or_else(|| format!("row {} plays a synth, which has no sample to preview", row_id))?;

  // (the output may have to be opened first, which takes a moment)
  thread::spawn(move || {
    let previewed = daw::engine().and_then(|engine| engine.preview(instrument.preview(engine.sample_rate())));
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
pub fn set_row_step_pitch(state: daw::TState<'_>, row_id: u64, pitch: u8) -> Result<daw::SequencerState, String> {
  edit(&state, None, |seq| seq.set_row_step_pitch(row_id, pitch))
}

// the pitch that plays a sample row's sample unchanged
#[tauri::command]
pub fn set_row_sample_pitch(state: daw::TState<'_>, row_id: u64, pitch: u8) -> Result<daw::RowChange, String> {
  edit_row(&state, None, row_id, |seq| seq.set_row_sample_pitch(row_id, pitch))
}

// whether a sample row's notes stop at their ends, or play the sample out
#[tauri::command]
pub fn set_row_gated(state: daw::TState<'_>, row_id: u64, gated: bool) -> Result<daw::RowChange, String> {
  edit_row(&state, None, row_id, |seq| seq.set_row_gated(row_id, gated))
}

// a pattern that was just made, and the sequencer with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddedPattern {
  id: u64,
  sequencer: daw::SequencerState,
}

// a new, empty pattern, `bars` long, at the end
#[tauri::command]
pub fn add_pattern(state: daw::TState<'_>, bars: u32) -> Result<AddedPattern, String> {
  add_pattern_of(&state, bars)
}

fn add_pattern_of(state: &daw::InnerState, bars: u32) -> Result<AddedPattern, String> {
  let beats = beats_per_bar(state);
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().add_pattern(bars, beats)
  })?;
  Ok(AddedPattern { id, sequencer: sequencer_state(state) })
}

// a copy of a pattern, straight after it
#[tauri::command]
pub fn duplicate_pattern(state: daw::TState<'_>, pattern_id: u64) -> Result<AddedPattern, String> {
  duplicate_pattern_of(&state, pattern_id)
}

fn duplicate_pattern_of(state: &daw::InnerState, pattern_id: u64) -> Result<AddedPattern, String> {
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().duplicate_pattern(pattern_id)
  })?;
  Ok(AddedPattern { id, sequencer: sequencer_state(state) })
}

// a pattern that was deleted: the sequencer without it, and the playlist
// clips of it that went with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemovedPattern {
  removed_clips: Vec<u64>,
  sequencer: daw::SequencerState,
}

// delete a pattern, and every clip of it in the playlist
#[tauri::command]
pub fn remove_pattern(state: daw::TState<'_>, pattern_id: u64) -> Result<RemovedPattern, String> {
  remove_pattern_of(&state, pattern_id)
}

fn remove_pattern_of(state: &daw::InnerState, pattern_id: u64) -> Result<RemovedPattern, String> {
  // its clips go with it, and come back with it when that's undone
  let clips: Vec<u64> = state.playlist.audiograph.lock().unwrap().pattern_nodes.iter()
    .filter(|node| node.pattern_id == pattern_id)
    .map(|node| node.id)
    .collect();

  let removed_clips = history::sequencer_edit(state, &clips, None, || {
    // the graph is locked before the sequencer, as everywhere
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    sequencer.remove_pattern(pattern_id)?;
    Ok(audiograph.remove_pattern_nodes_of(pattern_id))
  })?;
  Ok(RemovedPattern { removed_clips, sequencer: sequencer_state(state) })
}

// a folder that was just made, and the sequencer with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddedFolder {
  id: u64,
  sequencer: daw::SequencerState,
}

// a new, empty folder of patterns in `parent` (none: the top)
#[tauri::command]
pub fn add_pattern_folder(state: daw::TState<'_>, name: String, parent: Option<u64>) -> Result<AddedFolder, String> {
  add_pattern_folder_to(&state, &name, parent)
}

fn add_pattern_folder_to(state: &daw::InnerState, name: &str, parent: Option<u64>) -> Result<AddedFolder, String> {
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().add_folder(name, parent)
  })?;
  Ok(AddedFolder { id, sequencer: sequencer_state(state) })
}

#[tauri::command]
pub fn rename_pattern_folder(state: daw::TState<'_>, folder_id: u64, name: String) -> Result<daw::SequencerState, String> {
  edit(&state, None, |seq| seq.rename_folder(folder_id, &name))
}

// move a folder of patterns into another, or (none) to the top
#[tauri::command]
pub fn move_pattern_folder(state: daw::TState<'_>, folder_id: u64, parent: Option<u64>) -> Result<daw::SequencerState, String> {
  edit(&state, None, |seq| seq.move_folder(folder_id, parent))
}

// put a pattern in a folder, or (none) at the top
#[tauri::command]
pub fn move_pattern(state: daw::TState<'_>, pattern_id: u64, folder: Option<u64>) -> Result<daw::SequencerState, String> {
  edit(&state, None, |seq| seq.move_pattern(pattern_id, folder))
}

// delete a folder of patterns, with its folders and patterns. refused while
// any of its patterns is in the playlist, so no clip goes with it unseen
#[tauri::command]
pub fn remove_pattern_folder(state: daw::TState<'_>, folder_id: u64) -> Result<daw::SequencerState, String> {
  remove_pattern_folder_of(&state, folder_id)
}

fn remove_pattern_folder_of(state: &daw::InnerState, folder_id: u64) -> Result<daw::SequencerState, String> {
  history::sequencer_edit(state, &[], None, || {
    // the graph is locked before the sequencer, as everywhere
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    let inside = sequencer.patterns_in_folder(folder_id);
    let placed = inside.iter().filter(|id| audiograph.pattern_nodes.iter().any(|n| n.pattern_id == **id)).count();
    if placed > 0 {
      return Err(format!(
        "{} of its patterns {} in the playlist",
        placed, if placed == 1 { "is" } else { "are" },
      ));
    }
    sequencer.remove_folder(folder_id).map(|_| ())
  })?;
  Ok(sequencer_state(state))
}

// name a pattern. a blank name shows its number again
#[tauri::command]
pub fn rename_pattern(state: daw::TState<'_>, pattern_id: u64, name: String) -> Result<daw::SequencerState, String> {
  edit(&state, None, |seq| seq.rename_pattern(pattern_id, &name))
}

// make a pattern `bars` long (1 to PATTERN_BARS_MAX), and never trimmed
// shorter. its clips in the playlist keep their own lengths
#[tauri::command]
pub fn set_pattern_bars(state: daw::TState<'_>, pattern_id: u64, bars: u32) -> Result<daw::PatternChange, String> {
  let beats = beats_per_bar(&state);
  edit_pattern(&state, None, pattern_id, |seq| seq.set_pattern_bars(pattern_id, bars, beats))
}

// turn one of a row's steps on or off in a pattern, growing it to fit a
// step past its end (see Sequencer::set_pattern_step)
#[tauri::command]
pub fn set_pattern_step(
  state: daw::TState<'_>,
  pattern_id: u64,
  row_id: u64,
  step: u32,
  on: bool,
  gesture: Option<u64>,
) -> Result<daw::PatternChange, String> {
  set_step(&state, pattern_id, row_id, step, on, gesture)
}

fn set_step(
  state: &daw::InnerState,
  pattern_id: u64,
  row_id: u64,
  step: u32,
  on: bool,
  gesture: Option<u64>,
) -> Result<daw::PatternChange, String> {
  let beats = beats_per_bar(state);
  edit_pattern(state, gesture, pattern_id, |seq| seq.set_pattern_step(pattern_id, row_id, step, on, beats))
}

// replace a row's notes in a pattern (the Piano Roll), sorted, growing it to
// fit the notes added (see Sequencer::edit_pattern_notes). one drag of the
// mouse (the same gesture) is one step to undo
#[tauri::command]
pub fn set_pattern_notes(
  state: daw::TState<'_>,
  pattern_id: u64,
  row_id: u64,
  notes: Vec<daw::PatternNote>,
  gesture: Option<u64>,
) -> Result<daw::PatternChange, String> {
  set_notes(&state, pattern_id, row_id, notes, gesture)
}

pub(crate) fn set_notes(
  state: &daw::InnerState,
  pattern_id: u64,
  row_id: u64,
  notes: Vec<daw::PatternNote>,
  gesture: Option<u64>,
) -> Result<daw::PatternChange, String> {
  let beats = beats_per_bar(state);
  edit_pattern(state, gesture, pattern_id, |seq| seq.edit_pattern_notes(pattern_id, row_id, notes, beats))
}

// loop a pattern on its own, from `from_step` (its start if not given),
// stopping whatever was playing. its synth rows' notes play on their
// plugins' master copies
#[tauri::command]
pub fn play_pattern(state: daw::TState<'_>, pattern_id: u64, from_step: Option<f64>) -> Result<(), String> {
  daw::start_pattern(state.inner(), pattern_id, from_step.unwrap_or(0.))
}

// pause whatever is playing, or loop the pattern if nothing is (from
// `from_step`, or its start)
#[tauri::command]
pub fn toggle_pattern(state: daw::TState<'_>, pattern_id: u64, from_step: Option<f64>) -> Result<(), String> {
  toggle_pattern_in(state.inner(), pattern_id, from_step.unwrap_or(0.))
}

fn toggle_pattern_in(state: &Arc<daw::InnerState>, pattern_id: u64, from_step: f64) -> Result<(), String> {
  if state.playlist.playing() {
    daw::pause_playlist(state);
    Ok(())
  } else {
    daw::start_pattern(state, pattern_id, from_step)
  }
}

// while the pattern plays on its own, carry on from `step` (the Piano Roll's
// ruler). otherwise nothing happens
#[tauri::command]
pub fn seek_pattern(state: daw::TState<'_>, pattern_id: u64, step: f64) -> Result<(), String> {
  daw::seek_pattern(state.inner(), pattern_id, step)
}

// place a pattern in the playlist, dropped at `drop_beat` on `track_number`
// and snapped by the snap setting. the clip stays linked to the pattern, and
// is as long as the pattern is now. returns the clip as it's placed
#[tauri::command]
pub fn add_pattern_clip(
  state: daw::TState<'_>,
  pattern_id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<LoadedPatternClip, String> {
  add_pattern_clip_to(&state, pattern_id, track_number, drop_beat)
}

fn add_pattern_clip_to(
  state: &daw::InnerState,
  pattern_id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<LoadedPatternClip, String> {
  let start_offset = beat_to_offset(state, drop_beat);
  let subdivision = {
    let ui = state.playlist.ui.lock().unwrap();
    if ui.snap_enabled { Some(ui.snap_subdivision) } else { None }
  };

  // one step to undo
  let id = crate::history::clip_edit(state, &[], None, || state.playlist.audiograph.lock().unwrap()
    .add_pattern_node(pattern_id, start_offset, track_number, subdivision), |&id| vec![id])?;

  let audiograph = state.playlist.audiograph.lock().unwrap();
  let node = audiograph.get_pattern_node_by_id(id).ok_or_else(|| format!("no node with id {}", id))?;
  Ok(LoadedPatternClip::of(node, audiograph.beat_interval()))
}

// pattern clips that were exploded: the ones taken out, the pattern clips
// that replaced them (one per row), the sequencer (with the patterns the
// explode made), and the mixer (whose tracks may have been named)
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Exploded {
  removed: Vec<u64>,
  clips: Vec<LoadedPatternClip>,
  sequencer: daw::SequencerState,
  mixer: daw::MixerSettings,
}

// split pattern clips into a pattern clip per row, sample or synth, each
// playing a pattern of just that row's notes on a track of its own (see
// AudioGraph::explode_pattern_node). ids that aren't pattern clips, and
// clips of a pattern that plays nothing, are skipped. clips exploded before
// one that fails stay exploded
#[tauri::command]
pub fn explode_pattern_clips(state: daw::TState<'_>, ids: Vec<u64>) -> Result<Exploded, String> {
  explode(&state, &ids)
}

fn explode(state: &daw::InnerState, ids: &[u64]) -> Result<Exploded, String> {
  // one step to undo, which also takes back the patterns it made and the
  // names it gave tracks. recorded even if it fails part-way, as the clips
  // exploded by then stay exploded
  let before = history::picture(state, ids);
  let mut touched = ids.to_vec();
  let result = explode_each(state, ids, &mut touched);
  history::record_between(state, None, before, history::picture(state, &touched));
  result
}

// - touched: the clips it made are added to these
fn explode_each(state: &daw::InnerState, ids: &[u64], touched: &mut Vec<u64>) -> Result<Exploded, String> {
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();
  let mut removed = Vec::new();
  let mut clips = Vec::new();

  // one at a time, so a later clip's rows land on the tracks an earlier
  // one's went to
  for &id in ids {
    if !explodable(&audiograph, id) { continue; }
    let made = audiograph.explode_pattern_node(id)?;
    touched.extend(&made);
    removed.push(id);
    for new_id in made {
      let node = audiograph.get_pattern_node_by_id(new_id).expect("a clip that was just made");
      clips.push(LoadedPatternClip::of(node, interval));
    }
  }

  let mixer = state.playlist.mixer.settings(audiograph.track_count());
  drop(audiograph);
  Ok(Exploded { removed, clips, sequencer: sequencer_state(state), mixer })
}

// whether a clip is a pattern clip with notes to explode. (a selection
// often holds an empty clip painted with the Draw tool: refusing it would
// leave the clips exploded before it unseen by the app)
fn explodable(audiograph: &daw::AudioGraph<'static>, id: u64) -> bool {
  let Some(clip) = audiograph.get_pattern_node_by_id(id) else { return false };
  audiograph.sequencer.lock().unwrap().pattern(clip.pattern_id).map_or(false, |p| p.plays_notes())
}

// the clips a drag from `from` to `to` (either way round, already snapped)
// paints, of a pattern `length` beats long, as (start, end) in beats: end to
// end from where the drag starts, the last one cut short where it ends. a
// press let go where it went down paints one whole clip, and an end piece
// shorter than `min_length` is left off. the same rule as paintSpans in
// src/editing/paint.ts, which draws them while the mouse is held
pub(crate) fn paint_spans(from: f64, to: f64, length: f64, min_length: f64) -> Vec<(f64, f64)> {
  if !(length > 0.) { return vec![]; }
  let start = from.min(to).max(0.);
  let end = from.max(to);
  if end - start < min_length { return vec![(start, start + length)]; }

  let mut spans = Vec::new();
  let mut i = 0.;
  while start + i * length < end {
    let at = start + i * length;
    let until = (at + length).min(end);
    if until - at >= min_length { spans.push((at, until)); }
    i += 1.;
  }
  spans
}

// how many bars a new pattern painted over a drag from `from` to `to` is:
// enough to hold it, from 1 to `max_bars`. the same rule as paintBars in
// src/editing/paint.ts
pub(crate) fn paint_bars(from: f64, to: f64, beats_per_bar: f64, max_bars: u32) -> u32 {
  if !(beats_per_bar > 0.) { return 1; }
  let bars = (((to - from).abs() - 1e-9) / beats_per_bar).ceil();
  bars.clamp(1., max_bars.max(1) as f64) as u32
}

// pattern clips painted with the Draw tool: the clips made, in order, the
// pattern they play (made by the paint when none was given), the sequencer,
// the mixer, and how many tracks the playlist has now (a paint below the last
// track makes one)
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Painted {
  clips: Vec<LoadedPatternClip>,
  pattern_id: u64,
  sequencer: daw::SequencerState,
  mixer: daw::MixerSettings,
  track_count: u32,
}

// paint clips of a pattern along a track with the Draw tool, from
// `start_beat` to `end_beat` (snapped by the page, not here): clips of it end
// to end, the last trimmed to stop at the end (see paint_spans). with no
// pattern, a new empty pattern long enough to hold the drag, and one clip
// of it from start to end. one step to undo, the new pattern with it
#[tauri::command]
pub fn paint_pattern_clips(
  state: daw::TState<'_>,
  pattern_id: Option<u64>,
  track_number: u32,
  start_beat: f64,
  end_beat: f64,
) -> Result<Painted, String> {
  paint(&state, pattern_id, track_number, start_beat, end_beat)
}

fn paint(
  state: &daw::InnerState,
  pattern_id: Option<u64>,
  track_number: u32,
  start_beat: f64,
  end_beat: f64,
) -> Result<Painted, String> {
  if !start_beat.is_finite() || !end_beat.is_finite() {
    return Err(format!("{} to {} isn't a stretch of the playlist", start_beat, end_beat));
  }
  // checked first, so a pattern that isn't there changes nothing
  if let Some(id) = pattern_id {
    if state.playlist.sequencer.lock().unwrap().pattern(id).is_none() {
      return Err(format!("no pattern with id {}", id));
    }
  }
  let beats_per_bar = beats_per_bar(state);

  // one step to undo: the pattern, if one's made, and every clip
  let before = history::picture(state, &[]);
  let made = (|| -> Result<(u64, Vec<u64>), String> {
    // the graph is locked before the sequencer, as everywhere. (it locks the
    // sequencer itself to place a clip, so the sequencer's lock is let go
    // before then)
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let (pattern, steps) = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let pattern = match pattern_id {
        Some(id) => id,
        None => {
          let bars = paint_bars(start_beat, end_beat, beats_per_bar as f64, daw::PATTERN_BARS_MAX);
          sequencer.add_pattern(bars, beats_per_bar)?
        }
      };
      let steps = sequencer.pattern(pattern).map(|p| p.length)
        .ok_or_else(|| format!("no pattern with id {}", pattern))?;
      (pattern, steps)
    };

    let per_beat = daw::STEPS_PER_BEAT as f64;
    let length = steps as f64 / per_beat;
    let interval = audiograph.beat_interval();
    let mut clips = Vec::new();
    for (at, until) in paint_spans(start_beat, end_beat, length, daw::MIN_PATTERN_CLIP / per_beat) {
      let id = audiograph.add_pattern_node(pattern, interval.mul_f64(at), track_number, None::<daw::timing::Note>)?;
      clips.push(id);
      // (the last, cut short where the drag ends)
      if until < at + length {
        audiograph.set_pattern_window(id, 0., Some((until - at) * per_beat))?;
      }
    }
    Ok((pattern, clips))
  })();
  let ids: Vec<u64> = made.as_ref().map(|(_, clips)| clips.clone()).unwrap_or_default();
  history::record_between(state, None, before, history::picture(state, &ids));
  let (pattern_id, ids) = made?;

  let audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();
  let clips = ids.iter()
    .filter_map(|&id| audiograph.get_pattern_node_by_id(id))
    .map(|node| LoadedPatternClip::of(node, interval))
    .collect();
  let track_count = audiograph.track_count();
  let mixer = state.playlist.mixer.settings(track_count);
  drop(audiograph);
  Ok(Painted { clips, pattern_id, sequencer: sequencer_state(state), mixer, track_count })
}

// (for other modules' tests)
#[cfg(test)]
pub(crate) fn add_pattern_clip_for_tests(state: &daw::InnerState, pattern_id: u64, track_number: u32, drop_beat: Option<f32>) -> u64 {
  add_pattern_clip_to(state, pattern_id, track_number, drop_beat).unwrap().id
}

#[cfg(test)]
pub(crate) fn remove_pattern_for_tests(state: &daw::InnerState, pattern_id: u64) {
  // without its clips going with it, as the clipboard's copies don't
  state.playlist.sequencer.lock().unwrap().remove_pattern(pattern_id).unwrap();
  state.playlist.audiograph.lock().unwrap().remove_pattern_nodes_of(pattern_id);
}

#[cfg(test)]
mod tests;
