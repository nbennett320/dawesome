use std::sync::atomic::{Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime};
use tauri;
use mlua;

use crate::{app, daw, util, STATE};

fn toggle_playlist(state: &Arc<daw::InnerState>) {
  if state.playlist.playing() {
    daw::pause_playlist(state);
  } else {
    // start playlist
    daw::start_playlist(state);
  }
}

#[tauri::command]
pub fn t_toggle_playlist(state: daw::TState<'_>) {
  toggle_playlist(state.inner());
}

#[tauri::command]
pub fn get_playlist_playing(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.playing())
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn get_playlist_start_time(
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
pub fn get_playlist_start_time(
  state: daw::TState<'_>
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
pub fn get_playlist_tempo(
  state: daw::TState<'_>
) -> Result<f32, String> {
  Ok(state.tempo())
}

#[tauri::command]
pub fn set_playlist_tempo(
  state: daw::TState<'_>,
  val: f32,
) {
  println!("playlist tempo updated: {}", val);
  daw::set_playlist_tempo(state, val);
}

#[tauri::command]
pub fn toggle_metronome_enabled(
  state: daw::TState<'_>
) {
  let val = !state.metronome_enabled.load(Ordering::SeqCst);
  state.metronome_enabled.store(val, Ordering::SeqCst);
}

#[tauri::command]
pub fn get_metronome_enabled(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.metronome_enabled.load(Ordering::SeqCst))
}

#[tauri::command]
pub fn get_playlist_runtime_formatted(
  state: daw::TState<'_>
) -> Result<String, String> {
  let start_time = state.playlist.started_time.lock().unwrap().unwrap();
  let res = util::format_playlist_runtime(start_time);
  Ok(res)
}

#[tauri::command]
pub fn get_playlist_beat_count(
  state: daw::TState<'_>
) -> Result<u64, String> {
  let beat_count = state.playlist.total_beats.load(Ordering::SeqCst);
  Ok(beat_count)
}

#[tauri::command]
pub fn get_playlist_time_signature(
  state: daw::TState<'_>
) -> Result<(u16, u16), String> {
  let res = state.playlist.time_signature.lock().unwrap();
  Ok((res.numerator, res.denominator))
}

#[tauri::command]
pub fn set_playlist_time_signature(
  state: daw::TState<'_>,
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
pub fn get_sidebar_samples() -> Result<(
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
pub fn enumerate_directory(dir_path: String) -> Result<(
  Vec<String>, 
  Vec<String>, 
  Vec<String>, 
  Vec<String>
), String> {
  let (samples, samples_paths, dirs, dirs_paths) = 
    app::env::enumerate_files_in_dir(dir_path);
  Ok((samples, samples_paths, dirs, dirs_paths))
}

fn preview_sample(path: String) {
  thread::spawn(move || {
    futures::executor::block_on(daw::play_sample(&path));
  });
}

#[tauri::command]
pub fn t_preview_sample(
  _state: daw::TState<'_>,
  path: String,
) {
  preview_sample(path)
}

#[tauri::command]
pub fn get_audio_drivers() -> Result<Vec<String>, String> {
  Ok(daw::drivers::get_sound_host_names())
}

fn add_audiograph_node(
  state: &daw::InnerState,
  sample_path: String,
  track_number: u32,
  start_offset: std::time::Duration,
  snap_enabled: bool,
) -> u64 {
  // println!("added sample: {}, with offset of: {}ms", sample_path, start_offset.as_millis());

  let id: u64;
  if snap_enabled {
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

  id
}

#[tauri::command]
pub fn t_add_audiograph_node(
  state: daw::TState<'_>,
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

  let snap_enabled = state.playlist.ui.lock().unwrap().snap_enabled;

  // let id = add_audiograph_node(state.inner(), sample_path, track_number, start_offset, snap_enabled);

  Ok(4)
}

#[tauri::command]
pub fn move_audiograph_node(
  state: daw::TState<'_>,
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
pub fn remove_audiograph_node(
  state: daw::TState<'_>,
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
pub fn get_node_data(
  state: daw::TState<'_>,
  id: u64,
) -> Result<(Vec<f32>, f32), String> {
  let playlist = &state.playlist;
  let mut audiograph = playlist
    .audiograph
    .lock()
    .unwrap();
  let node = audiograph.get_mut_node_by_id(id).unwrap();
  println!("offset dur: {:?}", node.start_offset.as_millis());
  let waveform = node.get_waveform().clone();
  let dur = node.duration().as_secs_f32();
  let ratio = dur / audiograph.max_beats() as f32;


  // return waveform data
  Ok((waveform, dur))
}

#[tauri::command]
pub fn get_playlist_sample_offset(
  _state: daw::TState<'_>,
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
pub fn toggle_loop_enabled(
  state: daw::TState<'_>
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
pub fn get_loop_enabled(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.loop_enabled.load(Ordering::SeqCst))
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
pub fn get_playlist_max_length (
  state: daw::TState<'_>
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
pub fn toggle_record_input(
  state: daw::TState<'_>,
) {
  println!("toggling input recording");

  let recording = &state
    .playlist
    .recording();
  daw::input::record_input(*recording);
}

pub fn bind_functions(
  lua: &mlua::Lua,
  // state: &Arc<daw::InnerState>,
) {
  let globals = lua.globals();

  // preview_sample
  let f_preview_sample = 
    lua.create_function(|_, path: String| {
    preview_sample(path);

    Ok(())
  }).unwrap();

  globals.set("preview_sample", f_preview_sample).unwrap();
  
  // add_audiograph_node
  let f_add_audiograph_node = 
    lua.create_function(|_, (path, track_number, start_offset_ms): (String, u32, u64)| {
    let start_offset = std::time::Duration::from_millis(start_offset_ms);

    // add_audiograph_node(state, path, track_number, start_offset, false);

    let id: u64;
    // don't snap
    id = crate::STATE
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .construct_and_add_node(path, start_offset, track_number);

    Ok(id)
  }).unwrap();

  globals.set("add_audiograph_node", f_add_audiograph_node).unwrap();
  
  // add_audiograph_node
  let f_toggle_playlist = 
    lua.create_function(|_, (): ()| {    
    toggle_playlist(&crate::STATE);

    Ok(())
  }).unwrap();

  globals.set("toggle_playlist", f_toggle_playlist).unwrap();

  let f_print_state = 
    lua.create_function(|_, (): ()| {
    println!("playlist.playing: {}", crate::STATE.playlist.playing());
    println!("# nodes: {}", crate::STATE.playlist.audiograph.lock().unwrap().nodes.len());

    Ok(())
  }).unwrap();

  globals.set("print_state", f_print_state).unwrap();

}
