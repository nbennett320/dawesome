use std::sync::atomic::Ordering;

use crate::{app, daw};

#[tauri::command]
pub fn get_playlist_data(
  state: daw::TState<'_>,
) -> Result<(u64, u64, f32, u32), String> {
  let audiograph = state
    .playlist
    .audiograph
    .lock()
    .unwrap();
  let max_playlist_beats = state
    .playlist
    .max_beats
    .load(Ordering::SeqCst);
  let max_beats_displayed = state
    .playlist
    .ui
    .lock()
    .unwrap()
    .max_beats_displayed;
  let track_count = audiograph.track_count();
  let max_playlist_duration = audiograph
    .duration_max()
    .as_secs_f32();

  println!("max dur: {}s, track count: {}", max_playlist_duration, track_count);

  Ok((
    max_playlist_beats,
    max_beats_displayed,
    max_playlist_duration,
    track_count
  ))
}

#[tauri::command]
pub fn init_playlist_workspace(
  state: daw::TState<'_>,
  min_bound_x: f32,
  min_bound_y: f32,
  max_bound_x: f32,
  max_bound_y: f32,
) {
  println!("min_bound_x: {}, max_bound_x: {}", min_bound_x, max_bound_x);
  // update bounding box
  state
    .playlist
    .ui
    .lock()
    .unwrap()
    .viewport
    .set_bounding_box(
      min_bound_x, 
      min_bound_y, 
      max_bound_x, 
      max_bound_y);
}

#[tauri::command]
pub fn toggle_snap_enabled(
  state: daw::TState<'_>
) {
  state
    .playlist
    .ui
    .lock()
    .unwrap()
    .toggle_snap_enabled();
}

#[tauri::command]
pub fn get_snap_enabled(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.ui.lock().unwrap().snap_enabled)
}

// the snap setting, so the frontend can preview where a dragged clip will land
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapSetting {
  enabled: bool,
  // the note value clips snap to
  note: daw::timing::Note,
  // length of one snap step, in beats
  beats: f32,
}

fn snap_setting(state: &daw::InnerState) -> SnapSetting {
  let ui = state.playlist.ui.lock().unwrap();
  let (num, den) = daw::timing::MusicalTiming::ratio(&ui.snap_subdivision);

  SnapSetting {
    enabled: ui.snap_enabled,
    note: ui.snap_subdivision,
    beats: num as f32 / den as f32,
  }
}

#[tauri::command]
pub fn get_snap_setting(
  state: daw::TState<'_>
) -> Result<SnapSetting, String> {
  Ok(snap_setting(state.inner()))
}

// set the note value clips snap to, returns the new snap setting
#[tauri::command]
pub fn set_snap_subdivision(
  state: daw::TState<'_>,
  note: daw::timing::Note,
) -> Result<SnapSetting, String> {
  state.playlist.ui.lock().unwrap().snap_subdivision = note;

  Ok(snap_setting(state.inner()))
}

// the number of beats the playlist shows, and the time they span
// at the current tempo
pub(crate) fn displayed_beats_and_duration(
  state: &daw::InnerState,
) -> (f32, std::time::Duration) {
  let tempo = state.playlist.audiograph.lock().unwrap().tempo();
  let beats = state.playlist.ui.lock().unwrap().max_beats_displayed;
  let dur = daw::timing::n_beat_duration_from_tempo(tempo, beats as u32);

  (beats as f32, dur)
}

// convert a drop position, in beats from the start of the playlist,
// into a start offset
// positions are in beats rather than pixels, so they don't depend on the
// canvas size, zoom, or pan, which the frontend accounts for
pub(crate) fn beat_to_offset(
  state: &daw::InnerState,
  beat: Option<f32>,
) -> std::time::Duration {
  let (beats, dur) = displayed_beats_and_duration(state);

  app::workspaces::playlist::calc_sample_offset(beat.unwrap_or(0.), beats, dur)
}

// the inverse of beat_to_offset: how many beats into the playlist
// a start offset is
pub(crate) fn offset_to_beat(
  state: &daw::InnerState,
  offset: std::time::Duration,
) -> f32 {
  let interval = state.playlist.audiograph.lock().unwrap().beat_interval();

  if interval.is_zero() { return 0.; }

  (offset.as_secs_f64() / interval.as_secs_f64()) as f32
}

// where a node starts, in beats from the start of the playlist, after snapping
pub(crate) fn node_beat(state: &daw::InnerState, id: u64) -> Result<f32, String> {
  let (offset, _) = state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .placement(id)
    .ok_or_else(|| format!("no node with id {}", id))?;

  Ok(offset_to_beat(state, offset))
}

// the plus below the last track: an empty track under it, which can be
// undone. returns the new track count
#[tauri::command]
pub fn add_track(state: daw::TState<'_>) -> u32 {
  add_track_to(&state)
}

pub fn add_track_to(state: &daw::InnerState) -> u32 {
  let (before, count) = {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let before = audiograph.min_tracks;
    (before, audiograph.add_track())
  };
  crate::history::record(state, crate::history::Step::new().with_tracks(before, count));
  count
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_snap_setting() {
    let state = daw::InnerState::default();

    use daw::timing::Note;

    // default: snap on, one beat
    assert_eq!(
      snap_setting(&state),
      SnapSetting { enabled: true, note: Note::QuarterNote, beats: 1. });

    state.playlist.ui.lock().unwrap().snap_subdivision = Note::SixteenthNote;
    state.playlist.ui.lock().unwrap().toggle_snap_enabled();
    assert_eq!(
      snap_setting(&state),
      SnapSetting { enabled: false, note: Note::SixteenthNote, beats: 0.25 });

    // the frontend sends and receives notes by name
    let note: Note = serde_json::from_str("\"EighthNote\"").unwrap();
    assert_eq!(note, Note::EighthNote);
    assert_eq!(serde_json::to_string(&Note::WholeNote).unwrap(), "\"WholeNote\"");
  }

  #[test]
  fn test_beat_to_offset() {
    let state = daw::InnerState::default();

    // default: 120bpm, a playlist of 256 beats (64 bars)
    let ms = |b: Option<f32>| beat_to_offset(&state, b).as_millis();
    assert_eq!(ms(Some(0.)), 0);
    assert_eq!(ms(Some(1.)), 500);
    assert_eq!(ms(Some(8.)), 4_000);
    assert_eq!(ms(Some(1.5)), 750);
    assert_eq!(ms(None), 0);

    // drops outside the playlist are clamped to its start/end
    assert_eq!(ms(Some(-2.)), 0);
    assert_eq!(ms(Some(40.)), 20_000);
    assert_eq!(ms(Some(300.)), 128_000);

    // a beat is longer at a slower tempo
    state.set_tempo(60.);
    assert_eq!(ms(Some(8.)), 8_000);
  }
}

