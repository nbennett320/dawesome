use std::sync;
use std::sync::atomic;
use tauri;

mod daw;
mod util;
mod app;

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

#[tauri::command]
fn get_playlist_time_signature(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>
) -> Result<(u16, u16), String> {
  let res = state.playlist_time_signature.lock().unwrap();
  Ok((res.numerator, res.denominator))
}

#[tauri::command]
fn set_playlist_time_signature(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>,
  numerator: u16,
  denominator: u16
) {
  let updated: daw::timing::TimeSignature = daw::timing::TimeSignature {
    numerator: numerator,
    denominator: denominator
  };
  *state.playlist_time_signature.lock().unwrap() = updated;
}

#[tauri::command]
fn get_sidebar_samples() -> Result<Vec<String>, String> {
  Ok(app::get_sidebar_samples())
}

#[tauri::command]
fn get_audio_drivers() -> Result<Vec<String>, String> {
  Ok(daw::drivers::get_sound_host_names())
}

#[tauri::command]
fn add_audiograph_node(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>,
  sample_path: String,
  start_offset: u64
) -> Result<u64, String> {
  let id = state.playlist_audiograph.lock().unwrap()
    .construct_and_add_node(sample_path, start_offset);
  
  // returns the id of the new node
  Ok(id)
}

fn main() {
  tauri::Builder::default()
    .setup(app::setup)
    .menu(app::build_menu())
    .manage(sync::Arc::new(daw::InnerState::default()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      toggle_playlist,
      get_playlist_start_time,
      get_playlist_tempo,
      set_playlist_tempo,
      toggle_metronome_enabled,
      get_metronome_enabled,
      get_playlist_runtime_formatted,
      get_playlist_time_signature,
      set_playlist_time_signature,
      get_sidebar_samples,
      get_audio_drivers,
      add_audiograph_node
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
