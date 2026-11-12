// the Sequencer tab's commands (see daw/daw_core/sequencer.rs), and placing
// patterns in the playlist
//
// every edit returns the whole sequencer, which is small, so the frontend's
// copy stays the same as the engine's. edits take effect straight away,
// even while playing
use std::sync::Arc;
use std::thread;
use serde::Serialize;

use crate::daw;
use crate::history;
use crate::event_dispatch::{beat_to_offset, node_beat, PlacedNode};
use crate::project::LoadedPatternClip;

use crate::synth_commands::sequencer_state;

// run an edit on the sequencer, as a step that can be undone, then return
// the whole sequencer
fn edit(
  state: &daw::InnerState,
  f: impl FnOnce(&mut daw::Sequencer) -> Result<(), String>,
) -> Result<daw::SequencerState, String> {
  edit_in(state, None, f)
}

// - gesture: the press of the mouse it's part of, if any (a stroke painting
//   steps, a volume knob dragged): one press is one step to undo
fn edit_in(
  state: &daw::InnerState,
  gesture: Option<u64>,
  f: impl FnOnce(&mut daw::Sequencer) -> Result<(), String>,
) -> Result<daw::SequencerState, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  Ok(sequencer_state(state))
}

fn beats_per_bar(state: &daw::InnerState) -> u16 {
  state.playlist.time_signature.lock().unwrap().numerator
}

#[tauri::command]
pub fn get_sequencer(state: daw::TState<'_>) -> daw::SequencerState {
  sequencer_state(&state)
}

// a row playing the sample at `path`, added at the bottom
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn add_sequencer_row(state: daw::TState<'_>, path: String) -> Result<daw::SequencerState, String> {
  add_row(&state, &path)
}

fn add_row(state: &daw::InnerState, path: &str) -> Result<daw::SequencerState, String> {
  // decoded before taking the sequencer's lock, which playback needs
  let sample = daw::RowSample::load(path)?;
  edit(state, |seq| seq.add_loaded_row(path, sample).map(|_| ()))
}

// take a row out, with its steps in every pattern
#[tauri::command]
pub fn remove_sequencer_row(state: daw::TState<'_>, row_id: u64) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.remove_row(row_id))
}

// a row's volume, as a gain: 1 leaves the sample as is
#[tauri::command]
pub fn set_sequencer_row_volume(
  state: daw::TState<'_>,
  row_id: u64,
  volume: f32,
  gesture: Option<u64>,
) -> Result<daw::SequencerState, String> {
  set_row_volume(&state, row_id, volume, gesture)
}

fn set_row_volume(state: &daw::InnerState, row_id: u64, volume: f32, gesture: Option<u64>) -> Result<daw::SequencerState, String> {
  edit_in(state, gesture, |seq| seq.set_row_volume(row_id, volume))
}

#[tauri::command]
pub fn set_sequencer_row_muted(state: daw::TState<'_>, row_id: u64, muted: bool) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.set_row_muted(row_id, muted))
}

// play a row's sample once, at its volume
#[tauri::command]
pub fn preview_sequencer_row(state: daw::TState<'_>, row_id: u64) -> Result<(), String> {
  let source = state.playlist.sequencer.lock().unwrap()
    .row(row_id)
    .ok_or_else(|| format!("no sequencer row with id {}", row_id))?
    .source();

  thread::spawn(move || {
    if let Err(e) = daw::engine().and_then(|engine| engine.preview(source)) {
      println!("could not preview sequencer row {}: {}", row_id, e);
    }
  });

  Ok(())
}

// a pattern that was just made, and the sequencer with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddedPattern {
  id: u64,
  sequencer: daw::SequencerState,
}

// a new, empty pattern of `kind`, `bars` long, at the end
#[tauri::command]
pub fn add_pattern(state: daw::TState<'_>, bars: u32, kind: Option<daw::PatternKind>) -> Result<AddedPattern, String> {
  add_pattern_of(&state, bars, kind.unwrap_or_default())
}

fn add_pattern_of(state: &daw::InnerState, bars: u32, kind: daw::PatternKind) -> Result<AddedPattern, String> {
  let beats = beats_per_bar(state);
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().add_pattern(bars, beats, kind)
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

// a new, empty folder of `kind` patterns in `parent` (none: the top)
#[tauri::command]
pub fn add_pattern_folder(state: daw::TState<'_>, name: String, parent: Option<u64>, kind: Option<daw::PatternKind>) -> Result<AddedFolder, String> {
  add_pattern_folder_to(&state, &name, parent, kind.unwrap_or_default())
}

fn add_pattern_folder_to(state: &daw::InnerState, name: &str, parent: Option<u64>, kind: daw::PatternKind) -> Result<AddedFolder, String> {
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().add_folder(name, parent, kind)
  })?;
  Ok(AddedFolder { id, sequencer: sequencer_state(state) })
}

#[tauri::command]
pub fn rename_pattern_folder(state: daw::TState<'_>, folder_id: u64, name: String) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.rename_folder(folder_id, &name))
}

// move a folder of patterns into another, or (none) to the top
#[tauri::command]
pub fn move_pattern_folder(state: daw::TState<'_>, folder_id: u64, parent: Option<u64>) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.move_folder(folder_id, parent))
}

// put a pattern in a folder, or (none) at the top
#[tauri::command]
pub fn move_pattern(state: daw::TState<'_>, pattern_id: u64, folder: Option<u64>) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.move_pattern(pattern_id, folder))
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
  edit(&state, |seq| seq.rename_pattern(pattern_id, &name))
}

// make a pattern `bars` long (1, 2 or 4 for a drum pattern, 1 to 64 for a
// note pattern). its clips in the playlist get longer or shorter
#[tauri::command]
pub fn set_pattern_bars(state: daw::TState<'_>, pattern_id: u64, bars: u32) -> Result<daw::SequencerState, String> {
  let beats = beats_per_bar(&state);
  edit(&state, |seq| seq.set_pattern_bars(pattern_id, bars, beats))
}

// light or clear one of a row's steps in a pattern
#[tauri::command]
pub fn set_pattern_step(
  state: daw::TState<'_>,
  pattern_id: u64,
  row_id: u64,
  step: u32,
  on: bool,
  gesture: Option<u64>,
) -> Result<daw::SequencerState, String> {
  set_step(&state, pattern_id, row_id, step, on, gesture)
}

fn set_step(
  state: &daw::InnerState,
  pattern_id: u64,
  row_id: u64,
  step: u32,
  on: bool,
  gesture: Option<u64>,
) -> Result<daw::SequencerState, String> {
  edit_in(state, gesture, |seq| seq.set_step(pattern_id, row_id, step, on))
}

// replace a synth's notes in a note pattern (the Piano Roll), kept in the
// order given. one drag of the mouse (the same gesture) is one step to undo
#[tauri::command]
pub fn set_pattern_notes(
  state: daw::TState<'_>,
  pattern_id: u64,
  synth_id: u64,
  notes: Vec<daw::PatternNote>,
  gesture: Option<u64>,
) -> Result<daw::SequencerState, String> {
  set_notes(&state, pattern_id, synth_id, notes, gesture)
}

pub(crate) fn set_notes(
  state: &daw::InnerState,
  pattern_id: u64,
  synth_id: u64,
  notes: Vec<daw::PatternNote>,
  gesture: Option<u64>,
) -> Result<daw::SequencerState, String> {
  edit_in(state, gesture, |seq| seq.set_pattern_notes(pattern_id, synth_id, notes))
}

// loop a pattern on its own, from its start, stopping whatever was playing.
// a note pattern's notes play on its synths' master copies
#[tauri::command]
pub fn play_pattern(state: daw::TState<'_>, pattern_id: u64) -> Result<(), String> {
  daw::start_pattern(state.inner(), pattern_id)
}

// pause whatever is playing, or loop the pattern if nothing is
#[tauri::command]
pub fn toggle_pattern(state: daw::TState<'_>, pattern_id: u64) -> Result<(), String> {
  toggle_pattern_in(state.inner(), pattern_id)
}

fn toggle_pattern_in(state: &Arc<daw::InnerState>, pattern_id: u64) -> Result<(), String> {
  if state.playlist.playing() {
    daw::pause_playlist(state);
    Ok(())
  } else {
    daw::start_pattern(state, pattern_id)
  }
}

// place a pattern in the playlist, dropped at `drop_beat` on `track_number`
// and snapped by the snap setting. the clip stays linked to the pattern
#[tauri::command]
pub fn add_pattern_clip(
  state: daw::TState<'_>,
  pattern_id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<PlacedNode, String> {
  add_pattern_clip_to(&state, pattern_id, track_number, drop_beat)
}

fn add_pattern_clip_to(
  state: &daw::InnerState,
  pattern_id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<PlacedNode, String> {
  let start_offset = beat_to_offset(state, drop_beat);
  let subdivision = {
    let ui = state.playlist.ui.lock().unwrap();
    if ui.snap_enabled { Some(ui.snap_subdivision) } else { None }
  };

  // one step to undo
  let id = crate::history::clip_edit(state, &[], None, || state.playlist.audiograph.lock().unwrap()
    .add_pattern_node(pattern_id, start_offset, track_number, subdivision), |&id| vec![id])?;
  let start_beat = node_beat(state, id)?;

  Ok(PlacedNode { id, start_beat })
}

// pattern clips that were exploded: the ones taken out, the pattern clips
// that replaced them (one per sample), the sequencer (with the patterns the
// explode made), and the mixer (whose tracks may have been named)
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Exploded {
  removed: Vec<u64>,
  clips: Vec<LoadedPatternClip>,
  sequencer: daw::SequencerState,
  mixer: daw::MixerSettings,
}

// split pattern clips into a pattern clip per sample, each playing a pattern
// of just that sample's steps on a track of its own (see
// AudioGraph::explode_pattern_node). ids that aren't pattern clips are
// skipped. clips exploded before one that fails stay exploded
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

  // one at a time, so a later clip's samples land on the tracks an earlier
  // one's went to
  for &id in ids {
    if audiograph.get_pattern_node_by_id(id).is_none() { continue; }
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
// pattern, a new empty note pattern long enough to hold the drag, and one
// clip of it from start to end. one step to undo, the new pattern with it
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
          let bars = paint_bars(start_beat, end_beat, beats_per_bar as f64, daw::NOTE_BARS_MAX);
          sequencer.add_pattern(bars, beats_per_bar, daw::PatternKind::Note)?
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
mod tests {
  use super::*;
  use std::time::Duration;

  const KICK: &str = "assets/assets_66-bd-01.wav";

  #[test]
  fn test_a_pattern_follows_the_time_signature() {
    let state = daw::InnerState::default();
    state.playlist.time_signature.lock().unwrap().numerator = 3;
    let added = add_pattern_of(&state, 2, daw::PatternKind::Drum).unwrap();
    assert_eq!(added.sequencer.patterns[0].length, 24);
    assert_eq!(added.sequencer.patterns[0].id, added.id);
  }

  #[test]
  fn test_pattern_clips_snap_and_report_their_beat() {
    let state = daw::InnerState::default();
    let pattern = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap().id;

    // default snap is one beat
    let placed = add_pattern_clip_to(&state, pattern, 1, Some(2.4)).unwrap();
    assert_eq!(placed.start_beat, 2.);
    assert_eq!(
      state.playlist.audiograph.lock().unwrap().placement(placed.id),
      Some((Duration::from_millis(1_000), 1)));

    assert!(add_pattern_clip_to(&state, 99, 0, None).is_err());
  }

  #[test]
  fn test_removing_a_pattern_removes_its_clips() {
    let state = daw::InnerState::default();
    let pattern = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap().id;
    let other = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap().id;
    let a = add_pattern_clip_to(&state, pattern, 0, Some(0.)).unwrap().id;
    add_pattern_clip_to(&state, other, 0, Some(4.)).unwrap();

    let removed = remove_pattern_of(&state, pattern).unwrap();
    assert_eq!(removed.removed_clips, vec![a]);
    assert_eq!(removed.sequencer.patterns.len(), 1);
    assert_eq!(state.playlist.audiograph.lock().unwrap().pattern_nodes.len(), 1);
    assert!(remove_pattern_of(&state, pattern).is_err());
  }

  #[test]
  fn test_a_folder_in_use_isnt_removed() {
    let state = daw::InnerState::default();
    let placed = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap().id;
    let unplaced = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap().id;
    let folder = add_pattern_folder_to(&state, "Drums", None, daw::PatternKind::Drum).unwrap().id;
    {
      let mut seq = state.playlist.sequencer.lock().unwrap();
      seq.move_pattern(placed, Some(folder)).unwrap();
      seq.move_pattern(unplaced, Some(folder)).unwrap();
    }
    add_pattern_clip_to(&state, placed, 0, Some(0.)).unwrap();

    let refused = remove_pattern_folder_of(&state, folder).unwrap_err();
    assert!(refused.contains("1 of its patterns is"), "{}", refused);
    assert_eq!(state.playlist.sequencer.lock().unwrap().patterns().len(), 2);

    // with its clip gone, the folder and both patterns go
    state.playlist.audiograph.lock().unwrap().remove_pattern_nodes_of(placed);
    let seq = remove_pattern_folder_of(&state, folder).unwrap();
    assert!(seq.patterns.is_empty() && seq.folders.is_empty());
  }

  #[test]
  fn test_add_row_returns_the_sequencer() {
    let state = daw::InnerState::default();
    let seq = add_row(&state, KICK).unwrap();
    assert_eq!(seq.rows.len(), 1);
    assert_eq!(seq.rows[0].path, KICK);
    assert!(add_row(&state, "assets/nope.wav").is_err());
  }

  #[test]
  fn test_a_pattern_needs_steps_to_play() {
    let state = Arc::new(daw::InnerState::default());
    assert!(daw::start_pattern(&state, 7).is_err());
    assert!(!state.playlist.playing());
  }

  #[test]
  fn test_pattern_length_in_the_transport_math() {
    let state = daw::InnerState::default();
    let pattern = add_pattern_of(&state, 2, daw::PatternKind::Drum).unwrap().id;
    // two bars of 4/4 at 120bpm
    assert_eq!(daw::pattern_len(&state, pattern), Some(Duration::from_millis(4_000)));
  }

  const SNARE: &str = "assets/assets_66-sd-01.wav";

  // the project as it would be saved: what undo and redo have to get back to
  fn saved(state: &daw::InnerState) -> crate::project::Project {
    crate::project::capture(state, None)
  }

  fn steps(state: &daw::InnerState) -> usize {
    state.history.lock().unwrap().len()
  }

  // a sequencer with two rows, a pattern with steps in a folder, and a clip
  // of the pattern: (state, kick row, snare row, pattern, folder, clip). its
  // plugins are fakes (see effects.rs)
  fn drums() -> (Arc<daw::InnerState>, u64, u64, u64, u64, u64) {
    let state = crate::effects::testing::rig().state;
    let kick = add_row(&state, KICK).unwrap().rows[0].id;
    let snare = add_row(&state, SNARE).unwrap().rows[1].id;
    let pattern = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap().id;
    set_step(&state, pattern, kick, 0, true, None).unwrap();
    set_step(&state, pattern, snare, 4, true, None).unwrap();
    let folder = add_pattern_folder_to(&state, "Drums", None, daw::PatternKind::Drum).unwrap().id;
    edit(&state, |seq| seq.move_pattern(pattern, Some(folder))).unwrap();
    let clip = add_pattern_clip_to(&state, pattern, 1, Some(4.)).unwrap().id;
    (state, kick, snare, pattern, folder, clip)
  }

  // a synth (the fake one) and a one bar note pattern, in `state`
  fn synth_and_pattern(state: &daw::InnerState) -> (u64, u64) {
    let synth = crate::synth_commands::add(state, &crate::effects::testing::synth().id).unwrap().synth_id;
    let pattern = add_pattern_of(state, 1, daw::PatternKind::Note).unwrap().id;
    (synth, pattern)
  }

  #[test]
  fn test_every_sequencer_edit_can_be_undone_and_redone() {
    let (state, kick, snare, pattern, folder, _) = drums();
    let (synth, notes) = synth_and_pattern(&state);
    let s: &daw::InnerState = &state;

    // each edit is made on top of the ones before it
    let edits: Vec<(&str, Box<dyn Fn() -> Result<(), String> + '_>)> = vec![
      ("add a row", Box::new(|| add_row(s, KICK).map(|_| ()))),
      ("row volume", Box::new(|| set_row_volume(s, kick, 0.5, None).map(|_| ()))),
      ("mute a row", Box::new(|| edit(s, |seq| seq.set_row_muted(snare, true)).map(|_| ()))),
      ("light a step", Box::new(|| set_step(s, pattern, kick, 8, true, None).map(|_| ()))),
      ("clear a step", Box::new(|| set_step(s, pattern, kick, 0, false, None).map(|_| ()))),
      ("rename a pattern", Box::new(|| edit(s, |seq| seq.rename_pattern(pattern, "verse")).map(|_| ()))),
      ("set the notes", Box::new(|| set_notes(s, notes, synth, vec![note(60, 1.), note(64, 2.)], None).map(|_| ()))),
      ("note pattern length", Box::new(|| edit(s, |seq| seq.set_pattern_bars(notes, 7, 4)).map(|_| ()))),
      ("synth volume", Box::new(|| crate::synth_commands::set_volume(s, synth, 0.5, None).map(|_| ()))),
      ("mute a synth", Box::new(|| crate::synth_commands::set_muted(s, synth, true).map(|_| ()))),
      ("pattern length", Box::new(|| edit(s, |seq| seq.set_pattern_bars(pattern, 2, 4)).map(|_| ()))),
      ("add a pattern", Box::new(|| add_pattern_of(s, 4, daw::PatternKind::Drum).map(|_| ()))),
      ("duplicate a pattern", Box::new(|| duplicate_pattern_of(s, pattern).map(|_| ()))),
      ("add a folder", Box::new(|| add_pattern_folder_to(s, "Fills", Some(folder), daw::PatternKind::Drum).map(|_| ()))),
      ("add a note folder", Box::new(|| add_pattern_folder_to(s, "Keys", None, daw::PatternKind::Note).map(|_| ()))),
      ("rename a folder", Box::new(|| edit(s, |seq| seq.rename_folder(folder, "Kit")).map(|_| ()))),
      // (the folder just added, out to the top)
      ("move a folder", Box::new(|| edit(s, |seq| seq.move_folder(folder + 1, None)).map(|_| ()))),
      ("move a pattern", Box::new(|| edit(s, |seq| seq.move_pattern(pattern, None)).map(|_| ()))),
      ("remove a folder", Box::new(|| remove_pattern_folder_of(s, folder).map(|_| ()))),
      // with its steps in every pattern
      ("remove a row", Box::new(|| edit(s, |seq| seq.remove_row(snare)).map(|_| ()))),
      // with its clip in the playlist
      ("remove a pattern", Box::new(|| remove_pattern_of(s, pattern).map(|_| ()))),
      // with its notes
      ("remove a synth", Box::new(|| crate::synth_commands::remove(s, synth).map(|_| ()))),
    ];

    let start = saved(&state);
    let had = steps(&state);
    for (what, change) in &edits {
      let count = steps(&state);
      let before = saved(&state);
      change().unwrap_or_else(|e| panic!("could not {}: {}", what, e));
      let after = saved(&state);
      assert_ne!(before, after, "{} changed nothing", what);
      assert_eq!(steps(&state), count + 1, "{} is one step", what);

      assert!(history::undo(&state).unwrap().is_some());
      assert_eq!(saved(&state), before, "{} undone", what);
      assert!(history::redo(&state).unwrap().is_some());
      assert_eq!(saved(&state), after, "{} redone", what);
    }

    // and all the way back
    for _ in &edits { history::undo(&state).unwrap(); }
    assert_eq!(steps(&state), had);
    assert_eq!(saved(&state), start);
  }

  fn note(pitch: u8, start: f64) -> daw::PatternNote {
    daw::PatternNote { pitch, start, length: 1., velocity: 0.7 }
  }

  #[test]
  fn test_a_drag_of_notes_is_one_step() {
    let (state, _, _, _, _, _) = drums();
    let (synth, pattern) = synth_and_pattern(&state);
    let before = saved(&state);
    let had = steps(&state);

    // a note drawn, then dragged along, without letting go of the mouse
    for n in 0..10 {
      let seq = set_notes(&state, pattern, synth, vec![note(60, 0.), note(62, n as f64 * 0.5)], Some(31)).unwrap();
      assert_eq!(seq.pattern(pattern).unwrap().notes[&synth][1].start, n as f64 * 0.5);
    }
    assert_eq!(steps(&state), had + 1);
    let after = saved(&state);

    history::undo(&state).unwrap();
    assert_eq!(saved(&state), before);
    assert!(state.playlist.sequencer.lock().unwrap().pattern(pattern).unwrap().notes.is_empty());
    history::redo(&state).unwrap();
    assert_eq!(saved(&state), after);

    // the next drag is a step of its own; one that changes nothing isn't one
    set_notes(&state, pattern, synth, vec![note(60, 0.)], Some(32)).unwrap();
    set_notes(&state, pattern, synth, vec![note(60, 0.)], Some(33)).unwrap();
    assert_eq!(steps(&state), had + 2);
    assert!(set_notes(&state, 99, synth, vec![], None).is_err());
    assert_eq!(steps(&state), had + 2);
  }

  #[test]
  fn test_placing_a_pattern_is_one_step() {

    let (state, _, _, pattern, _, _) = drums();
    let before = saved(&state);
    let had = steps(&state);

    let placed = add_pattern_clip_to(&state, pattern, 3, Some(12.)).unwrap();
    assert_eq!(steps(&state), had + 1);
    let after = saved(&state);
    assert_ne!(before, after);

    // undone, the clip is gone and its pattern stays. redone, it's the
    // same clip again
    assert_eq!(history::undo(&state).unwrap().unwrap().clips, Some(vec![]));
    assert_eq!(saved(&state), before);
    assert!(state.playlist.sequencer.lock().unwrap().pattern(pattern).is_some());
    assert_eq!(history::redo(&state).unwrap().unwrap().clips, Some(vec![placed.id]));
    assert_eq!(saved(&state), after);
  }

  #[test]
  fn test_an_edit_that_fails_is_no_step() {
    let (state, kick, _, pattern, _, _) = drums();
    let had = steps(&state);
    assert!(set_step(&state, pattern, 99, 0, true, None).is_err());
    assert!(set_step(&state, pattern, kick, 999, true, None).is_err());
    assert!(remove_pattern_of(&state, 99).is_err());
    assert!(add_row(&state, "assets/nope.wav").is_err());
    // nor is one that changes nothing: the step was lit already
    set_step(&state, pattern, kick, 0, true, None).unwrap();
    assert_eq!(steps(&state), had);
  }

  #[test]
  fn test_a_stroke_of_steps_is_one_step() {
    let (state, kick, _, pattern, _, _) = drums();
    let before = saved(&state);
    let had = steps(&state);

    // eight steps painted without letting go of the mouse
    for step in 1..=8 {
      set_step(&state, pattern, kick, step, true, Some(21)).unwrap();
    }
    assert_eq!(steps(&state), had + 1);
    let after = saved(&state);

    history::undo(&state).unwrap();
    assert_eq!(saved(&state), before);
    history::redo(&state).unwrap();
    assert_eq!(saved(&state), after);

    // the next stroke, and a knob's drag, are steps of their own
    set_step(&state, pattern, kick, 12, true, Some(22)).unwrap();
    for i in 1..=20 {
      set_row_volume(&state, kick, 1. - 0.02 * i as f32, Some(23)).unwrap();
    }
    assert_eq!(steps(&state), had + 3);
    history::undo(&state).unwrap();
    assert_eq!(state.playlist.sequencer.lock().unwrap().row(kick).unwrap().volume, 1.);
  }

  #[test]
  fn test_a_removed_row_and_pattern_come_back_as_themselves() {
    let (state, _, snare, pattern, _, clip) = drums();

    edit(&state, |seq| seq.remove_row(snare)).unwrap();
    history::undo(&state).unwrap();
    {
      let seq = state.playlist.sequencer.lock().unwrap();
      assert_eq!(seq.row(snare).unwrap().path, SNARE);
      assert!(seq.pattern(pattern).unwrap().steps[&snare].contains(&4));
    }

    let removed = remove_pattern_of(&state, pattern).unwrap();
    assert_eq!(removed.removed_clips, vec![clip]);
    // its clip comes back with it, on its beat and track, and is selected
    let applied = history::undo(&state).unwrap().unwrap();
    assert_eq!(applied.clips, Some(vec![clip]));
    assert!(state.playlist.sequencer.lock().unwrap().pattern(pattern).is_some());
    assert_eq!(
      state.playlist.audiograph.lock().unwrap().placement(clip),
      Some((Duration::from_millis(2_000), 1)));

    // what's added after an undo never takes the id of something that could
    // still come back by redo
    history::undo(&state).unwrap();
    history::redo(&state).unwrap();
    edit(&state, |seq| seq.remove_row(snare)).unwrap();
    history::undo(&state).unwrap();
    let rows = add_row(&state, KICK).unwrap().rows;
    let ids: std::collections::BTreeSet<u64> = rows.iter().map(|row| row.id).collect();
    assert_eq!(ids.len(), rows.len());
    let added = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap();
    assert_ne!(added.id, pattern);
  }

  #[test]
  fn test_an_undone_pattern_plays_as_it_did() {
    let (state, kick, _, pattern, _, _) = drums();
    // what the pattern's clip sounds like
    let render = || {
      let sounds = {
        let mut audiograph = state.playlist.audiograph.lock().unwrap();
        audiograph.init();
        let sounds = audiograph.run_sounds(Duration::from_secs(5), false, false);
        audiograph.pause();
        sounds
      };
      let (engine, mut renderer) = daw::Engine::new(44_100);
      let voices = sounds
        .into_iter()
        .map(|(delay, source, route)| engine.boxed_voice(source, Some(1), engine.frames_in(delay), route))
        .collect();
      engine.start_run(1, state.playlist.mixer.master(), voices).unwrap();

      let mut out = vec![0f32; 44_100 * 5 * 2];
      renderer.render(&mut out, std::time::Instant::now(), Duration::ZERO);
      out
    };
    let before = render();
    assert!(before.iter().any(|&sample| sample != 0.));

    set_step(&state, pattern, kick, 2, true, None).unwrap();
    assert!(render() != before);
    history::undo(&state).unwrap();
    assert!(render() == before);
  }

  #[test]
  fn test_exploding_is_one_step() {
    let (state, _, _, pattern, _, clip) = drums();
    let other = add_pattern_clip_to(&state, pattern, 2, Some(8.)).unwrap().id;
    let before = saved(&state);
    let had = steps(&state);

    let exploded = explode(&state, &[clip, other]).unwrap();
    assert_eq!(exploded.removed, vec![clip, other]);
    assert_eq!(steps(&state), had + 1);
    let after = saved(&state);
    // it made a pattern per sample, and named tracks after them
    assert!(after.sequencer.patterns.len() > before.sequencer.patterns.len());
    assert_ne!(before.mixer, after.mixer);

    // undone: the clips are pattern clips again, the patterns it made are
    // gone, and so are the names
    let applied = history::undo(&state).unwrap().unwrap();
    assert_eq!(applied.clips, Some(vec![clip, other]));
    assert_eq!(saved(&state), before);

    let made: Vec<u64> = exploded.clips.iter().map(|c| c.id).collect();
    let applied = history::redo(&state).unwrap().unwrap();
    assert_eq!(applied.clips, Some(made));
    assert_eq!(saved(&state), after);
  }

  // (the cases of src/editing/paint.spec.ts, for paintSpans and paintBars,
  // which must agree)
  #[test]
  fn test_the_paint_layout() {
    // end to end over the drag, the last cut short
    assert_eq!(paint_spans(4., 12., 4., 0.25), vec![(4., 8.), (8., 12.)]);
    assert_eq!(paint_spans(0., 10., 4., 0.25), vec![(0., 4.), (4., 8.), (8., 10.)]);
    assert_eq!(paint_spans(2., 3., 4., 0.25), vec![(2., 3.)]);
    // the same dragged backwards
    assert_eq!(paint_spans(10., 0., 4., 0.25), paint_spans(0., 10., 4., 0.25));
    // a click, or a drag shorter than a clip can be: one whole clip
    assert_eq!(paint_spans(6., 6., 4., 0.25), vec![(6., 10.)]);
    assert_eq!(paint_spans(6., 6.1, 4., 0.25), vec![(6., 10.)]);
    // an end piece too short to be a clip is left off
    assert_eq!(paint_spans(0., 8.1, 4., 0.25), vec![(0., 4.), (4., 8.)]);
    // nothing before the playlist's start, and nothing of no length
    assert_eq!(paint_spans(-2., 4., 4., 0.25), vec![(0., 4.)]);
    assert!(paint_spans(0., 8., 0., 0.25).is_empty());

    assert_eq!(paint_bars(0., 4., 4., 64), 1);
    assert_eq!(paint_bars(0., 4.5, 4., 64), 2);
    assert_eq!(paint_bars(8., 0., 4., 64), 2);
    assert_eq!(paint_bars(0., 9., 3., 64), 3);
    assert_eq!(paint_bars(3., 3., 4., 64), 1);
    assert_eq!(paint_bars(0., 1_000., 4., 64), 64);
  }

  // the pattern clips on a track, as (start beat, window), in order
  fn clips_on(state: &daw::InnerState, track: u32) -> Vec<(f32, Option<crate::event_dispatch::ClipWindow>)> {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let interval = audiograph.beat_interval();
    let mut clips: Vec<_> = audiograph.pattern_nodes.iter()
      .filter(|node| node.track_number == track)
      .map(|node| LoadedPatternClip::of(node, interval))
      .map(|clip| (clip.start_beat, clip.window))
      .collect();
    clips.sort_by(|a, b| a.0.total_cmp(&b.0));
    clips
  }

  #[test]
  fn test_painting_lays_clips_end_to_end() {
    let (state, _, _, pattern, _, _) = drums();
    let had = steps(&state);

    // a one bar pattern (four beats), painted over ten
    let painted = paint(&state, Some(pattern), 2, 4., 14.).unwrap();
    assert_eq!(painted.pattern_id, pattern);
    assert_eq!(painted.clips.len(), 3);
    assert!(painted.clips.iter().all(|clip| clip.pattern_id == pattern && clip.track_number == 2));
    let starts: Vec<f32> = painted.clips.iter().map(|clip| clip.start_beat).collect();
    assert_eq!(starts, vec![4., 8., 12.]);
    // the last is trimmed to the drag's end: two beats, eight steps
    assert_eq!(painted.clips[0].window, None);
    assert_eq!(painted.clips[1].window, None);
    assert_eq!(painted.clips[2].window, Some(crate::event_dispatch::ClipWindow { start: 0., end: Some(8.) }));
    assert_eq!(clips_on(&state, 2), painted.clips.iter().map(|c| (c.start_beat, c.window)).collect::<Vec<_>>());
    // no new pattern, and one step
    assert_eq!(painted.sequencer.patterns.len(), 1);
    assert_eq!(steps(&state), had + 1);
  }

  #[test]
  fn test_painting_with_no_pattern_makes_one() {
    let (state, _, _, pattern, _, _) = drums();
    let painted = paint(&state, None, 0, 2., 8.).unwrap();

    // six beats needs two bars: a note pattern of them, played to the end of
    // the drag
    assert_ne!(painted.pattern_id, pattern);
    let made = painted.sequencer.patterns.iter().find(|p| p.id == painted.pattern_id).unwrap();
    assert_eq!(made.kind, daw::PatternKind::Note);
    assert_eq!(made.length, 32);
    assert_eq!(painted.clips.len(), 1);
    assert_eq!(painted.clips[0].start_beat, 2.);
    assert_eq!(painted.clips[0].window, Some(crate::event_dispatch::ClipWindow { start: 0., end: Some(24.) }));

    // a click: a bar, and the clip is all of it
    let clicked = paint(&state, None, 0, 20., 20.).unwrap();
    let made = clicked.sequencer.patterns.iter().find(|p| p.id == clicked.pattern_id).unwrap();
    assert_eq!(made.length, 16);
    assert_eq!(clicked.clips[0].window, None);
  }

  #[test]
  fn test_painting_below_the_last_track_makes_one() {
    let (state, _, _, pattern, _, _) = drums();
    let count = state.playlist.audiograph.lock().unwrap().track_count();
    let painted = paint(&state, Some(pattern), count, 0., 4.).unwrap();
    assert_eq!(painted.track_count, count + 1);
    assert!(painted.mixer.tracks.iter().any(|t| t.track_number == count));
  }

  #[test]
  fn test_a_paint_is_one_step() {
    let (state, _, _, pattern, _, _) = drums();
    for given in [Some(pattern), None] {
      let before = saved(&state);
      let had = steps(&state);

      let painted = paint(&state, given, 3, 0., 10.).unwrap();
      assert_eq!(steps(&state), had + 1);
      let after = saved(&state);
      assert_ne!(before, after);

      // undone, every clip goes (and the pattern it made); redone, they're back
      history::undo(&state).unwrap();
      assert_eq!(saved(&state), before);
      let applied = history::redo(&state).unwrap().unwrap();
      let ids: Vec<u64> = painted.clips.iter().map(|clip| clip.id).collect();
      assert_eq!(applied.clips, Some(ids));
      assert_eq!(saved(&state), after);
    }
  }

  #[test]
  fn test_painting_an_unknown_pattern_changes_nothing() {
    let (state, _, _, _, _, _) = drums();
    let before = saved(&state);
    let had = steps(&state);
    assert!(paint(&state, Some(99), 0, 0., 8.).is_err());
    assert!(paint(&state, None, 0, f64::NAN, 8.).is_err());
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), had);
  }

  #[test]
  fn test_explode_skips_sample_clips() {
    let state = daw::InnerState::default();
    let pattern = add_pattern_of(&state, 1, daw::PatternKind::Drum).unwrap().id;
    let row = add_row(&state, KICK).unwrap().rows[0].id;
    state.playlist.sequencer.lock().unwrap().set_step(pattern, row, 4, true).unwrap();
    let clip = add_pattern_clip_to(&state, pattern, 2, Some(4.)).unwrap().id;
    let sample = state.playlist.audiograph.lock().unwrap()
      .construct_and_add_node(KICK.to_string(), Duration::ZERO, 0).unwrap();

    let exploded = explode(&state, &[sample, clip]).unwrap();
    assert_eq!(exploded.removed, vec![clip]);
    assert_eq!(exploded.clips.len(), 1);
    // where the clip was. track 0 holds only kicks already, so it joins it
    assert_eq!(exploded.clips[0].start_beat, 4.);
    assert_eq!(exploded.clips[0].track_number, 0);
    // a one-row pattern is its own split, so no new pattern is needed
    assert_eq!(exploded.clips[0].pattern_id, pattern);
    assert_eq!(exploded.sequencer.patterns.len(), 1);
    let named = exploded.mixer.tracks.iter().find(|t| t.track_number == 0).unwrap();
    assert_eq!(named.name.as_deref(), Some("assets_66-bd-01"));
  }
}
