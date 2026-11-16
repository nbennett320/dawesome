// the playlist's commands (see actions/playlist.rs)
use crate::actions::playlist::{add_track_to, snap_setting, SnapSetting};
use crate::daw;

// the playlist's length, and how many tracks it has
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistData {
  // in beats (see daw::playlist_beats)
  beats: u64,
  track_count: u32,
}

#[tauri::command]
pub fn get_playlist_data(state: daw::TState<'_>) -> PlaylistData {
  let track_count = state.playlist.audiograph.lock().unwrap().track_count();
  PlaylistData { beats: daw::playlist_beats(&state), track_count }
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

// the plus below the last track: an empty track under it, which can be
// undone. returns the new track count
#[tauri::command]
pub fn add_track(state: daw::TState<'_>) -> u32 {
  add_track_to(&state)
}
