use std::sync::atomic::{Ordering};
use std::sync::{Arc};
use std::thread;
use std::time::{
  Duration, 
  SystemTime
};
use tauri;

#[macro_use(lazy_static)]
extern crate lazy_static;

mod app;
mod daw;
mod util;

#[tauri::command]
fn toggle_playlist(state: tauri::State<'_, Arc<daw::InnerState>>) {
  if state.playlist.playing.load(Ordering::SeqCst) {
    daw::pause_playlist(state);
  } else {
    // start playlist
    daw::start_playlist(state);
  }
}

#[tauri::command]
fn get_playlist_playing(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<bool, String> {
  Ok(state.playlist.playing.load(Ordering::SeqCst))
}

#[cfg(target_os = "windows")]
#[tauri::command]
fn get_playlist_start_time(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<u128, String> {
  let elapsed = state
    .playlist
    .started_time
    .lock()
    .unwrap()
    .unwrap()
    .elapsed();
  let now = SystemTime::now()
    .duration_since(SystemTime::Windows)
    .unwrap();
  let start_time = now - elapsed;
  Ok(start_time.as_millis())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
fn get_playlist_start_time(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<u128, String> {
  let elapsed = state
    .playlist
    .started_time
    .lock()
    .unwrap()
    .unwrap()
    .elapsed();
  let now = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .unwrap();
  let start_time = now - elapsed;
  Ok(start_time.as_millis())
}

#[tauri::command]
fn get_playlist_tempo(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<f32, String> {
  Ok(state.tempo())
}

#[tauri::command]
fn set_playlist_tempo(
  state: tauri::State<'_, Arc<daw::InnerState>>,
  val: f32,
) {
  println!("playlist tempo updated: {}", val);
  daw::set_playlist_tempo(state, val);
}

#[tauri::command]
fn toggle_metronome_enabled(
  state: tauri::State<'_, Arc<daw::InnerState>>
) {
  let val = !state.metronome_enabled.load(Ordering::SeqCst);
  state.metronome_enabled.store(val, Ordering::SeqCst);
}

#[tauri::command]
fn get_metronome_enabled(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<bool, String> {
  Ok(state.metronome_enabled.load(Ordering::SeqCst))
}

#[tauri::command]
fn get_playlist_runtime_formatted(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<String, String> {
  let start_time = state.playlist.started_time.lock().unwrap().unwrap();
  let res = util::format_playlist_runtime(start_time);
  Ok(res)
}

#[tauri::command]
fn get_playlist_time_signature(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<(u16, u16), String> {
  let res = state.playlist.time_signature.lock().unwrap();
  Ok((res.numerator, res.denominator))
}

#[tauri::command]
fn set_playlist_time_signature(
  state: tauri::State<'_, Arc<daw::InnerState>>,
  numerator: u16,
  denominator: u16,
) {
  let updated: daw::timing::TimeSignature = daw::timing::TimeSignature {
    numerator,
    denominator,
  };

  *state.playlist.time_signature.lock().unwrap() = updated;
}

#[tauri::command]
fn get_sidebar_samples() -> Result<(
  Vec<String>, 
  Vec<String>, 
  Vec<String>, 
  Vec<String>
), String> {
  let (samples, samples_paths, dirs, dirs_paths) = 
    app::env::get_sample_browser_root();
  Ok((samples, samples_paths, dirs, dirs_paths))
}

#[tauri::command]
fn enumerate_directory(dir_path: String) -> Result<(
  Vec<String>, 
  Vec<String>, 
  Vec<String>, 
  Vec<String>
), String> {
  let (samples, samples_paths, dirs, dirs_paths) = 
    app::env::enumerate_files_in_dir(dir_path);
  Ok((samples, samples_paths, dirs, dirs_paths))
}

#[tauri::command]
fn preview_sample(
  _state: tauri::State<'_, Arc<daw::InnerState>>,
  path: String,
) {
  thread::spawn(move || {
    futures::executor::block_on(daw::play_sample(&path));
  });
}

#[tauri::command]
fn get_audio_drivers() -> Result<Vec<String>, String> {
  Ok(daw::drivers::get_sound_host_names())
}

#[tauri::command]
fn add_audiograph_node(
  state: tauri::State<'_, Arc<daw::InnerState>>,
  sample_path: String,
  track_number: u32,
  drop_x: Option<f32>,
  drop_y: Option<f32>,
) -> Result<u64, String> {
  let min_bound_x = state.playlist.ui.lock().unwrap().viewport.min_bound_x.unwrap();
  let min_bound_y = state.playlist.ui.lock().unwrap().viewport.min_bound_y.unwrap();
  let max_bound_x = state.playlist.ui.lock().unwrap().viewport.max_bound_x.unwrap();
  let max_bound_y = state.playlist.ui.lock().unwrap().viewport.max_bound_y.unwrap();
  let tempo = state.playlist.audiograph.lock().unwrap().tempo();
  let max_beats_displayed = state.playlist.ui.lock().unwrap().max_beats_displayed;
  println!("max_beats_displayed: {}", max_beats_displayed);
  let max_playlist_dur = daw::timing::n_subdivision_duration_from_tempo(
    tempo, 
    max_beats_displayed as u32,
    daw::timing::QuarterNote::new()
  );

  println!("max playlist dur: {}ms", max_playlist_dur.as_millis());

  let start_offset = app::workspaces::playlist::calc_sample_offset(
    drop_x.unwrap_or(min_bound_x),
    drop_y.unwrap_or(min_bound_y),
    min_bound_x,
    min_bound_y,
    max_bound_x,
    max_bound_y,
    max_playlist_dur
  );

  println!("added sample: {}, with offset of: {}ms", sample_path, start_offset.as_millis());

  let id: u64;
  if state.playlist.ui.lock().unwrap().snap_enabled {
    println!("snapping the sample");
    // snap to nearest snap subdivision
    let subdivision = state
      .playlist
      .ui
      .lock()
      .unwrap()
      .snap_subdivision;

    id = state
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .construct_and_add_node_with_snap(
        sample_path, 
        start_offset, 
        track_number,
        daw::timing::SixteenthNote::new());
  } else {
    // don't snap
    id = state
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .construct_and_add_node(sample_path, start_offset, track_number);
  }

  // returns the id of the new node, offset
  Ok(id)
}

#[tauri::command]
fn move_audiograph_node(
  state: tauri::State<'_, Arc<daw::InnerState>>, 
  id: u64,
  track_number: u32,
  drop_x: Option<f32>,
  drop_y: Option<f32>,
) {
  let min_bound_x = state.playlist.ui.lock().unwrap().viewport.min_bound_x.unwrap();
  let min_bound_y = state.playlist.ui.lock().unwrap().viewport.min_bound_y.unwrap();
  let max_bound_x = state.playlist.ui.lock().unwrap().viewport.max_bound_x.unwrap();
  let max_bound_y = state.playlist.ui.lock().unwrap().viewport.max_bound_y.unwrap();
  let tempo = state.playlist.audiograph.lock().unwrap().tempo();
  let max_beats_displayed = state.playlist.ui.lock().unwrap().max_beats_displayed;
  println!("max_beats_displayed: {}", max_beats_displayed);
  let max_playlist_dur = daw::timing::n_subdivision_duration_from_tempo(
    tempo, 
    max_beats_displayed as u32,
    daw::timing::QuarterNote::new()
  );

  let start_offset = app::workspaces::playlist::calc_sample_offset(
    drop_x.unwrap_or(min_bound_x),
    drop_y.unwrap_or(min_bound_y),
    min_bound_x,
    min_bound_y,
    max_bound_x,
    max_bound_y,
    max_playlist_dur
  );


  println!("moved sample with id: {}, with offset of: {}ms", id, start_offset.as_millis());

  if state.playlist.ui.lock().unwrap().snap_enabled {
    println!("snapping the sample");
    // snap to nearest snap subdivision
    let subdivision = state
      .playlist
      .ui
      .lock()
      .unwrap()
      .snap_subdivision;

    state
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .move_node_with_snap(
        id, 
        start_offset, 
        track_number, 
        daw::timing::SixteenthNote::new());
  } else {
    // don't snap
    state
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .move_node(id, start_offset, track_number);
  }
}

#[tauri::command]
fn remove_audiograph_node(
  state: tauri::State<'_, Arc<daw::InnerState>>, 
  id: u64,
) {
  state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .remove_node(id);
}

#[tauri::command]
fn get_node_data(
  state: tauri::State<'_, Arc<daw::InnerState>>,
  id: u64,
) -> Result<(Vec<f32>, f32), String> {
  let playlist = &state.playlist;
  let mut audiograph = playlist
    .audiograph
    .lock()
    .unwrap();
  let node = audiograph.get_mut_node(id).unwrap();
  let waveform = node.get_waveform().clone();
  let dur = node.duration().as_secs_f32();
  let ratio = dur / audiograph.max_beats() as f32;

  println!("waveform dur: {:?}", dur);

  // return waveform data
  Ok((waveform, dur))
}

#[tauri::command]
fn get_playlist_sample_offset(
  state: tauri::State<'_, Arc<daw::InnerState>>,
  drop_x: f32,
  drop_y: f32,
  min_bound_x: f32,
  min_bound_y: f32,
  max_bound_x: f32,
  max_bound_y: f32,
) -> Result<u64, String> {
  // todo: choose a number that isn't arbitrary
  let max_sample_offset = (max_bound_x - min_bound_x).round() as u64 * 5;
  println!("max sample offset: {}", max_sample_offset);
  let res = util::calc_playlist_sample_offset(
    drop_x,
    drop_y,
    min_bound_x,
    min_bound_y,
    max_bound_x,
    max_bound_y,
    max_sample_offset,
  );

  Ok(res)
}

#[tauri::command]
fn get_playlist_data(
  state: tauri::State<'_, Arc<daw::InnerState>>
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
  let track_count = match audiograph.tracks() {
    x if x.len() > 1 => {
      x.len() as u32
    }
    _ => {
      daw::defaults::NUM_OF_TRACKS
    }
  };
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
fn toggle_loop_enabled(
  state: tauri::State<'_, Arc<daw::InnerState>>
) {
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
fn get_loop_enabled(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<bool, String> {
  Ok(state.playlist.loop_enabled.load(Ordering::SeqCst))
}

#[tauri::command]
fn toggle_snap_enabled(
  state: tauri::State<'_, Arc<daw::InnerState>>
) {
  state
    .playlist
    .ui
    .lock()
    .unwrap()
    .toggle_snap_enabled();
}

#[tauri::command]
fn get_snap_enabled(
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<bool, String> {
  Ok(state.playlist.ui.lock().unwrap().snap_enabled)
}

#[tauri::command]
fn get_playlist_max_length (
  state: tauri::State<'_, Arc<daw::InnerState>>
) -> Result<(u64, u64), String> {
  let dur = state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .duration_max();
  let max_beats = state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .max_beats();
  Ok((dur.as_millis().try_into().unwrap(), max_beats))
}

#[tauri::command]
fn init_playlist_workspace(
  state: tauri::State<'_, Arc<daw::InnerState>>,
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
fn toggle_record_input(
  state: tauri::State<'_, Arc<daw::InnerState>>,
) {
  println!("toggling input recording");

  let recording = &state
    .playlist
    .recording;
  daw::input::record_input(recording);
}

fn main() {
  tauri::Builder::default()
    .setup(app::window::setup)
    .menu(app::window::build_menu())
    .manage(Arc::new(daw::InnerState::default()))
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
      enumerate_directory,
      preview_sample,
      get_audio_drivers,
      add_audiograph_node,
      move_audiograph_node,
      remove_audiograph_node,
      get_node_data,
      get_playlist_sample_offset,
      get_playlist_data,
      toggle_loop_enabled,
      get_loop_enabled,
      toggle_snap_enabled,
      get_snap_enabled,
      get_playlist_max_length,
      init_playlist_workspace,
      toggle_record_input
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
