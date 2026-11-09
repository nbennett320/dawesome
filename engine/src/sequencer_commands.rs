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
use crate::event_dispatch::{beat_to_offset, node_beat, PlacedNode};

fn sequencer_state(state: &daw::InnerState) -> daw::SequencerState {
  state.playlist.sequencer.lock().unwrap().state()
}

// run an edit on the sequencer, then return the whole sequencer
fn edit(
  state: &daw::InnerState,
  f: impl FnOnce(&mut daw::Sequencer) -> Result<(), String>,
) -> Result<daw::SequencerState, String> {
  let mut sequencer = state.playlist.sequencer.lock().unwrap();
  f(&mut sequencer)?;
  Ok(sequencer.state())
}

fn beats_per_bar(state: &daw::InnerState) -> u16 {
  state.playlist.time_signature.lock().unwrap().numerator
}

#[tauri::command]
pub fn get_sequencer(state: daw::TState<'_>) -> daw::SequencerState {
  sequencer_state(&state)
}

// a row playing the sample at `path`, added at the bottom
#[tauri::command]
pub fn add_sequencer_row(state: daw::TState<'_>, path: String) -> Result<daw::SequencerState, String> {
  add_row(&state, &path)
}

fn add_row(state: &daw::InnerState, path: &str) -> Result<daw::SequencerState, String> {
  // decoding a sample takes a moment, so it's done before taking the lock
  let sample_rate = state.playlist.audiograph.lock().unwrap().sample_rate();
  edit(state, |seq| seq.add_row(path, sample_rate).map(|_| ()))
}

// take a row out, with its steps in every pattern
#[tauri::command]
pub fn remove_sequencer_row(state: daw::TState<'_>, row_id: u64) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.remove_row(row_id))
}

// a row's volume, as a gain: 1 leaves the sample as is
#[tauri::command]
pub fn set_sequencer_row_volume(state: daw::TState<'_>, row_id: u64, volume: f32) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.set_row_volume(row_id, volume))
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
    let played = rodio::OutputStream::try_default()
      .map_err(|e| e.to_string())
      .and_then(|(_stream, handle)| {
        let sink = rodio::Sink::try_new(&handle).map_err(|e| e.to_string())?;
        sink.append(source);
        sink.sleep_until_end();
        Ok(())
      });

    if let Err(e) = played {
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

// a new, empty pattern `bars` long, at the end
#[tauri::command]
pub fn add_pattern(state: daw::TState<'_>, bars: u32) -> Result<AddedPattern, String> {
  add_pattern_of(&state, bars)
}

fn add_pattern_of(state: &daw::InnerState, bars: u32) -> Result<AddedPattern, String> {
  let beats = beats_per_bar(state);
  let mut sequencer = state.playlist.sequencer.lock().unwrap();
  let id = sequencer.add_pattern(bars, beats)?;
  Ok(AddedPattern { id, sequencer: sequencer.state() })
}

// a copy of a pattern, straight after it
#[tauri::command]
pub fn duplicate_pattern(state: daw::TState<'_>, pattern_id: u64) -> Result<AddedPattern, String> {
  let mut sequencer = state.playlist.sequencer.lock().unwrap();
  let id = sequencer.duplicate_pattern(pattern_id)?;
  Ok(AddedPattern { id, sequencer: sequencer.state() })
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
  // the graph is locked before the sequencer, as everywhere
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let mut sequencer = state.playlist.sequencer.lock().unwrap();
  sequencer.remove_pattern(pattern_id)?;
  let removed_clips = audiograph.remove_pattern_nodes_of(pattern_id);

  Ok(RemovedPattern { removed_clips, sequencer: sequencer.state() })
}

// name a pattern. a blank name shows its number again
#[tauri::command]
pub fn rename_pattern(state: daw::TState<'_>, pattern_id: u64, name: String) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.rename_pattern(pattern_id, &name))
}

// make a pattern `bars` long. its clips in the playlist get longer or shorter
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
) -> Result<daw::SequencerState, String> {
  edit(&state, |seq| seq.set_step(pattern_id, row_id, step, on))
}

// loop a pattern on its own, from its start, stopping whatever was playing
#[tauri::command]
pub fn t_play_pattern(state: daw::TState<'_>, pattern_id: u64) -> Result<(), String> {
  daw::start_pattern(state.inner(), pattern_id)
}

// pause whatever is playing, or loop the pattern if nothing is
#[tauri::command]
pub fn t_toggle_pattern(state: daw::TState<'_>, pattern_id: u64) -> Result<(), String> {
  toggle_pattern(state.inner(), pattern_id)
}

fn toggle_pattern(state: &Arc<daw::InnerState>, pattern_id: u64) -> Result<(), String> {
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

  let id = state.playlist.audiograph.lock().unwrap()
    .add_pattern_node(pattern_id, start_offset, track_number, subdivision)?;
  let start_beat = node_beat(state, id)?;

  Ok(PlacedNode { id, start_beat })
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
    let added = add_pattern_of(&state, 2).unwrap();
    assert_eq!(added.sequencer.patterns[0].length, 24);
    assert_eq!(added.sequencer.patterns[0].id, added.id);
  }

  #[test]
  fn test_pattern_clips_snap_and_report_their_beat() {
    let state = daw::InnerState::default();
    let pattern = add_pattern_of(&state, 1).unwrap().id;

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
    let pattern = add_pattern_of(&state, 1).unwrap().id;
    let other = add_pattern_of(&state, 1).unwrap().id;
    let a = add_pattern_clip_to(&state, pattern, 0, Some(0.)).unwrap().id;
    add_pattern_clip_to(&state, other, 0, Some(4.)).unwrap();

    let removed = remove_pattern_of(&state, pattern).unwrap();
    assert_eq!(removed.removed_clips, vec![a]);
    assert_eq!(removed.sequencer.patterns.len(), 1);
    assert_eq!(state.playlist.audiograph.lock().unwrap().pattern_nodes.len(), 1);
    assert!(remove_pattern_of(&state, pattern).is_err());
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
    let pattern = add_pattern_of(&state, 2).unwrap().id;
    // two bars of 4/4 at 120bpm
    assert_eq!(daw::pattern_len(&state, pattern), Some(Duration::from_millis(4_000)));
  }
}
