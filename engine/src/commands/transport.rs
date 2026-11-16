// the transport's commands (see daw/actions/transport.rs)
use std::sync::atomic::Ordering;

use crate::daw::actions::transport::{
  change_loop_end, change_loop_region, change_tempo, loop_settings, playlist_transport, remove_loop_region,
  set_time_signature, unpin_loop_end, LoopSettings, PlaylistTransport,
};
use crate::daw::{playback, timing};
use crate::commands::TState;

#[tauri::command]
pub fn toggle_playlist(state: TState<'_>) {
  if state.playlist.playing() {
    playback::pause_playlist(state.inner());
  } else {
    // start playlist
    playback::start_playlist(state.inner());
  }
}

// play from the playhead (nothing if already playing)
#[tauri::command]
pub fn play_playlist(state: TState<'_>) {
  // a pattern playing on its own is replaced by the playlist
  if !state.playlist.playing_playlist() {
    playback::start_playlist(state.inner());
  }
}

// stop where it is, so play carries on from there
#[tauri::command]
pub fn pause_playlist(state: TState<'_>) {
  if state.playlist.playing() {
    playback::pause_playlist(state.inner());
  }
}

// stop and go back to the start
#[tauri::command]
pub fn stop_playlist(state: TState<'_>) {
  playback::stop_playlist(state.inner());
}

// move the playhead to a beat (kept within the playlist), carrying on playing
// from there if playing. returns the beat it moved to
#[tauri::command]
pub fn seek_playlist(state: TState<'_>, beat: f64) -> f64 {
  playback::seek_playlist(state.inner(), beat)
}

#[tauri::command]
pub fn get_playlist_playing(
  state: TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.playing())
}

#[tauri::command]
pub fn get_playlist_transport(
  state: TState<'_>
) -> PlaylistTransport {
  playlist_transport(state.inner())
}

#[tauri::command]
pub fn set_playlist_tempo(
  state: TState<'_>,
  val: f32,
  gesture: Option<u64>,
) -> Result<f32, String> {
  println!("playlist tempo updated: {}", val);
  change_tempo(state.inner(), val, gesture)
}

#[tauri::command]
pub fn toggle_metronome_enabled(
  state: TState<'_>
) {
  let val = !state.metronome_enabled.load(Ordering::SeqCst);
  state.metronome_enabled.store(val, Ordering::SeqCst);
}

// whether recording counts in a bar first. returns the new setting
#[tauri::command]
pub fn toggle_count_in(state: TState<'_>) -> bool {
  !state.playlist.count_in.fetch_xor(true, Ordering::SeqCst)
}

#[tauri::command]
pub fn get_metronome_enabled(
  state: TState<'_>
) -> Result<bool, String> {
  Ok(state.metronome_enabled.load(Ordering::SeqCst))
}

#[tauri::command]
pub fn set_playlist_time_signature(
  state: TState<'_>,
  numerator: u16,
  denominator: u16,
) {
  set_time_signature(&state, timing::TimeSignature { numerator, denominator });
}

#[tauri::command]
pub fn toggle_loop_enabled(
  state: TState<'_>
) {
  // a recording cuts its passes where the loop comes round: it stays as it
  // is until the recording's over
  if crate::daw::recording::is_recording(&state) { return; }

  let val = !state
    .playlist
    .loop_enabled
    .load(Ordering::SeqCst);

  state
    .playlist
    .loop_enabled
    .store(val, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_loop_region(state: TState<'_>, start: f64, end: f64) -> LoopSettings {
  change_loop_region(&state, start, end)
}

#[tauri::command]
pub fn clear_loop_region(state: TState<'_>) -> LoopSettings {
  remove_loop_region(&state)
}

#[tauri::command]
pub fn set_loop_end(state: TState<'_>, beat: f64) -> LoopSettings {
  change_loop_end(&state, beat)
}

#[tauri::command]
pub fn clear_loop_end(state: TState<'_>) -> LoopSettings {
  unpin_loop_end(&state)
}

#[tauri::command]
pub fn get_loop_settings(state: TState<'_>) -> LoopSettings {
  loop_settings(&state)
}

#[tauri::command]
pub fn get_loop_enabled(
  state: TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.loop_enabled.load(Ordering::SeqCst))
}
