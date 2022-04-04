use std::sync::atomic;
use std::sync;
use chrono;
use tauri;

mod daw;

#[tauri::command]
fn toggle_playlist(state: tauri::State<'_, sync::Arc<daw::InnerState>>) {
  if state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
    println!("pausing playlist");
    state.playlist_is_playing.store(false, atomic::Ordering::SeqCst);
  } else {
    // start playlist 
    println!("playing playlist");
    state.playlist_is_playing.store(true, atomic::Ordering::SeqCst);
   
    // set playlist start time 
    let now = chrono::offset::Utc::now();
    let timestamp = now.naive_utc().timestamp();
    state.playlist_started_time.store(timestamp, atomic::Ordering::SeqCst);
   
    // toggle metronome if enabled 
    if state.metronome_enabled.load(atomic::Ordering::SeqCst) {
      let state_ref = state.inner();
      daw::run_metronome(state_ref);
    }
  }
}

#[tauri::command]
fn get_playlist_playing(state: tauri::State<'_, sync::Arc<daw::InnerState>>) -> Result<bool, String> {
  Ok(
    state.playlist_is_playing.load(atomic::Ordering::SeqCst)
  )
}

#[tauri::command]
fn get_playlist_start_time(state: tauri::State<'_, sync::Arc<daw::InnerState>>) -> Result<i64, String> {
  Ok(
    state.playlist_started_time.load(atomic::Ordering::SeqCst)
  )
}

#[tauri::command]
fn get_playlist_tempo(state: tauri::State<'_, sync::Arc<daw::InnerState>>) -> Result<f32, String> {
  Ok(
    *state.global_tempo_bpm.lock().unwrap()
  )
}

#[tauri::command]
fn set_playlist_tempo(
  state: tauri::State<'_, sync::Arc<daw::InnerState>>,
  val: f32
) {
  println!("playlist tempo updated: {}", val);
  *state.global_tempo_bpm.lock().unwrap() = val;
}

fn main() {
  tauri::Builder::default()
    .manage(sync::Arc::new(daw::InnerState::default()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      toggle_playlist,
      get_playlist_start_time,
      get_playlist_tempo,
      set_playlist_tempo
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
