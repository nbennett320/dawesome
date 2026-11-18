// the transport's commands (see daw/actions/transport.rs)
use std::sync::atomic::Ordering;

use crate::daw::actions::transport::{
  change_loop_end, change_loop_region, change_tempo, loop_settings, playlist_transport, remove_loop_region,
  set_time_signature, unpin_loop_end, LoopSettings, PlaylistTransport,
};
use crate::daw::{playback, timing};
use crate::commands::TState;

// (the transport's own keys: they stop and start as a tape does, when the
// tape stop and start preference says so)
#[tauri::command]
pub fn toggle_playlist(state: TState<'_>) {
  if state.playlist.playing() {
    playback::pause_playlist_as(state.inner(), playback::Motion::AsTape);
  } else {
    // start playlist
    playback::start_playlist_as(state.inner(), playback::Motion::AsTape);
  }
}

// play from the playhead (nothing if already playing)
#[tauri::command]
pub fn play_playlist(state: TState<'_>) {
  // a pattern playing on its own is replaced by the playlist
  if !state.playlist.playing_playlist() {
    playback::start_playlist_as(state.inner(), playback::Motion::AsTape);
  }
}

// stop where it is, so play carries on from there. `at_once`: not as tape
// (winding, or a pattern let go of), whatever the preference
#[tauri::command]
pub fn pause_playlist(state: TState<'_>, at_once: Option<bool>) {
  if state.playlist.playing() {
    let motion = if at_once.unwrap_or(false) { playback::Motion::AtOnce } else { playback::Motion::AsTape };
    playback::pause_playlist_as(state.inner(), motion);
  }
}

// stop and go back to the start
#[tauri::command]
pub fn stop_playlist(state: TState<'_>) {
  playback::stop_playlist_as(state.inner(), playback::Motion::AsTape);
}

// scrubbing (part S): dragging the playhead or the reels plays the song at
// the drag's speed. it begins where the drag starts, follows it (ms on the
// timeline), and is let go at a beat, which the playhead moves to (playing
// carries on from there if it was playing). see daw/scrub
#[tauri::command]
pub fn scrub_begin(state: TState<'_>, at_ms: f64) -> Result<(), String> {
  crate::daw::scrub::begin(state.inner(), ms(at_ms), None)
}

#[tauri::command]
pub fn scrub_to(at_ms: f64) {
  crate::daw::scrub::drag(ms(at_ms));
}

// returns the beat it moved to
#[tauri::command]
pub fn scrub_end(state: TState<'_>, beat: f64) -> f64 {
  crate::daw::scrub::end(state.inner(), beat)
}

// holding Rewind or Fast forward: the song heard as it winds (T31), from
// `at_ms` at `speed` (multiples of play speed, negative back)
#[tauri::command]
pub fn wind_begin(state: TState<'_>, at_ms: f64, speed: f64) -> Result<(), String> {
  crate::daw::scrub::begin(state.inner(), ms(at_ms), Some(speed))
}

// let go of a wind: the playhead's left where the tape got to (`fallback_ms`
// if the engine doesn't know), kept within the playlist. returns where, in
// ms
#[tauri::command]
pub fn wind_end(state: TState<'_>, fallback_ms: f64) -> f64 {
  let interval = playback::beat_interval(state.inner()).as_secs_f64() * 1000.;
  let at = crate::daw::scrub::wound_to().map_or(fallback_ms, |secs| secs * 1000.).max(0.);
  if interval <= 0. { return 0.; }
  crate::daw::scrub::end(state.inner(), at / interval) * interval
}

fn ms(ms: f64) -> std::time::Duration {
  std::time::Duration::from_secs_f64(if ms.is_finite() { ms.max(0.) / 1000. } else { 0. })
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
