use std::sync;
use std::sync::atomic;

mod daw;
mod util;

#[tauri::command]
fn toggle_playlist(state: tauri::State<'_, sync::Arc<daw::InnerState>>) {
  if state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
    daw::pause_playlist(state);
  } else {
    // start playlist
    daw::start_playlist(state);
  }
}

#[tauri::command]
fn get_playlist_playing(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>
) -> Result<bool, String> {
  Ok(state.playlist_is_playing.load(atomic::Ordering::SeqCst))
}

#[tauri::command]
fn get_playlist_start_time(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>
) -> Result<i64, String> {
  Ok(state.playlist_started_time.load(atomic::Ordering::SeqCst))
}

#[tauri::command]
fn get_playlist_tempo(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>
) -> Result<f32, String> {
  Ok(*state.global_tempo_bpm.lock().unwrap())
}

#[tauri::command]
fn set_playlist_tempo(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>,
  val: f32,
) {
  println!("playlist tempo updated: {}", val);
  *state.global_tempo_bpm.lock().unwrap() = val;
}

#[tauri::command]
fn toggle_metronome_enabled(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>
) {
  let val = !state.metronome_enabled.load(atomic::Ordering::SeqCst);
  state.metronome_enabled.store(val, atomic::Ordering::SeqCst);
}

#[tauri::command]
fn get_metronome_enabled(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>
) -> Result<bool, String> {
  Ok(state.metronome_enabled.load(atomic::Ordering::SeqCst))
}

#[tauri::command]
fn get_playlist_runtime_formatted(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>
) -> Result<String, String> {
  let res = util::format_playlist_runtime(
    state.playlist_started_time.load(atomic::Ordering::SeqCst),
  );
  Ok(res)
}

fn main() {
  tauri::Builder::default()
    .manage(sync::Arc::new(daw::InnerState::default()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      toggle_playlist,
      get_playlist_start_time,
      get_playlist_tempo,
      set_playlist_tempo,
      toggle_metronome_enabled,
      get_metronome_enabled,
      get_playlist_runtime_formatted
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
