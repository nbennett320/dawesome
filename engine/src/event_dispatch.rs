use std::sync::atomic::{Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant, SystemTime};
use tauri;

use crate::daw::{InnerState};
use crate::{app, daw, util, STATE};

pub fn toggle_playlist(state: &Arc<daw::InnerState>) {
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

#[tauri::command]
pub fn get_playlist_start_time(
  state: daw::TState<'_>
) -> Result<u128, String> {
  Ok(get_start_time_ms(state.inner())? as u128)
}

// convert the engine's monotonic start time into wall clock ms
// (since the unix epoch), which the frontend can compare to Date.now()
fn get_start_time_ms(state: &daw::InnerState) -> Result<f64, String> {
  let elapsed = state
    .playlist
    .started_time
    .lock()
    .unwrap()
    .ok_or("playlist has not been started")?
    .elapsed();
  // UNIX_EPOCH is the epoch on every platform, including windows
  let now = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .map_err(|e| e.to_string())?;
  Ok((now - elapsed).as_secs_f64() * 1000.)
}

// everything the frontend needs to draw the playback cursor in sync with
// the engine's beat loop, without polling every frame
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTransport {
  // engine start time of the current run, ms since the unix epoch
  start_time_ms: f64,
  // length of one beat tick in the engine loop
  beat_interval_ms: f64,
  // beats played before the engine loops back to the start
  loop_beats: u64,
  // beats spanned by the full playlist width
  beats_displayed: u64,
  loop_enabled: bool,
}

fn playlist_transport(state: &daw::InnerState) -> Result<PlaylistTransport, String> {
  let start_time_ms = get_start_time_ms(state)?;
  let beat_interval = state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .interval_of_subdivision(daw::timing::QuarterNote::new());

  Ok(PlaylistTransport {
    start_time_ms,
    beat_interval_ms: beat_interval.as_secs_f64() * 1000.,
    loop_beats: state.playlist.max_beats.load(Ordering::SeqCst),
    beats_displayed: state.playlist.ui.lock().unwrap().max_beats_displayed,
    loop_enabled: state.playlist.loop_enabled.load(Ordering::SeqCst),
  })
}

#[tauri::command]
pub fn get_playlist_transport(
  state: daw::TState<'_>
) -> Result<PlaylistTransport, String> {
  playlist_transport(state.inner())
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

pub fn toggle_metronome_enabled(
  state: &Arc<daw::InnerState>
) {
  let val = !state.metronome_enabled.load(Ordering::SeqCst);
  state.metronome_enabled.store(val, Ordering::SeqCst);
}

#[tauri::command]
pub fn t_toggle_metronome_enabled(
  state: daw::TState<'_>
) {
  toggle_metronome_enabled(state.inner());
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
  // before the first play there is no start time; show a zero runtime
  let start_time = state
    .playlist
    .started_time
    .lock()
    .unwrap()
    .unwrap_or_else(Instant::now);
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

pub fn preview_sample(path: String) {
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

// read the playlist viewport bounds, set by init_playlist_workspace
// returns (min_x, min_y, max_x, max_y)
fn playlist_bounds(
  state: &daw::InnerState,
) -> Result<(f32, f32, f32, f32), String> {
  let ui = state.playlist.ui.lock().unwrap();
  let vp = &ui.viewport;

  match (vp.min_bound_x, vp.min_bound_y, vp.max_bound_x, vp.max_bound_y) {
    (Some(min_x), Some(min_y), Some(max_x), Some(max_y)) =>
      Ok((min_x, min_y, max_x, max_y)),
    _ => Err("playlist workspace has not been initialized".to_string()),
  }
}

// convert a drop position (pixels from the playlist's left edge)
// into a start offset; the playlist width spans max_beats_displayed beats
fn drop_x_to_offset(
  state: &daw::InnerState,
  drop_x: Option<f32>,
) -> Result<std::time::Duration, String> {
  let (min_bound_x, _, max_bound_x, _) = playlist_bounds(state)?;
  let tempo = state.playlist.audiograph.lock().unwrap().tempo();
  let max_beats_displayed = state.playlist.ui.lock().unwrap().max_beats_displayed;
  let max_playlist_dur = daw::timing::n_beat_duration_from_tempo(
    tempo,
    max_beats_displayed as u32,
  );

  Ok(app::workspaces::playlist::calc_sample_offset(
    drop_x.unwrap_or(0.),
    max_bound_x - min_bound_x,
    max_playlist_dur,
  ))
}

fn add_audiograph_node(
  state: &daw::InnerState,
  sample_path: String,
  track_number: u32,
  start_offset: std::time::Duration,
  snap_enabled: bool,
) -> Result<u64, String> {
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
        subdivision)?;
  } else {
    // don't snap
    id = state
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .construct_and_add_node(sample_path, start_offset, track_number)?;
  }

  Ok(id)
}

#[tauri::command]
pub fn t_add_audiograph_node(
  state: daw::TState<'_>,
  sample_path: String,
  track_number: u32,
  drop_x: Option<f32>,
  drop_y: Option<f32>,
) -> Result<u64, String> {
  // vertical position is already resolved into track_number by the frontend
  let _ = drop_y;
  let start_offset = drop_x_to_offset(state.inner(), drop_x)?;

  let snap_enabled = state.playlist.ui.lock().unwrap().snap_enabled;

  add_audiograph_node(state.inner(), sample_path, track_number, start_offset, snap_enabled)
}

#[tauri::command]
pub fn move_audiograph_node(
  state: daw::TState<'_>,
  id: u64,
  track_number: u32,
  drop_x: Option<f32>,
  drop_y: Option<f32>,
) -> Result<(), String> {
  // vertical position is already resolved into track_number by the frontend
  let _ = drop_y;
  let start_offset = drop_x_to_offset(state.inner(), drop_x)?;

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
        subdivision)
  } else {
    // don't snap
    state
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .move_node(id, start_offset, track_number)
  }
}

#[tauri::command]
pub fn remove_audiograph_node(
  state: daw::TState<'_>,
  id: u64,
) -> Result<(), String> {
  state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .remove_node(id)?;

  Ok(())
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
  let node = audiograph.get_mut_node_by_id(id)
    .ok_or_else(|| format!("no node with id {}", id))?;
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

pub fn toggle_loop_enabled(
  state: &Arc<InnerState>
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
pub fn t_toggle_loop_enabled(
  state: daw::TState<'_>
) {
  toggle_loop_enabled(state.inner());
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
  let dur_ms: u64 = dur.as_millis().try_into().map_err(|_| "playlist too long")?;
  Ok((dur_ms, max_beats))
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;

  #[test]
  fn test_playlist_bounds_before_and_after_init() {
    let state = daw::InnerState::default();

    // before the frontend calls init_playlist_workspace
    assert!(playlist_bounds(&state).is_err());

    state
      .playlist
      .ui
      .lock()
      .unwrap()
      .viewport
      .set_bounding_box(0., 10., 800., 400.);

    assert_eq!(playlist_bounds(&state).unwrap(), (0., 10., 800., 400.));
  }

  #[test]
  fn test_drop_x_to_offset() {
    let state = daw::InnerState::default();

    // before init_playlist_workspace there are no bounds
    assert!(drop_x_to_offset(&state, Some(100.)).is_err());

    // playlist from screen x=300 to x=1100 (800px wide)
    state.playlist.ui.lock().unwrap()
      .viewport.set_bounding_box(300., 0., 1_100., 400.);

    // default: 120bpm, 16 beats displayed -> width spans 8s
    let ms = |x: Option<f32>| drop_x_to_offset(&state, x).unwrap().as_millis();
    assert_eq!(ms(Some(0.)), 0);
    assert_eq!(ms(Some(400.)), 4_000); // middle of the playlist -> beat 9
    assert_eq!(ms(Some(50.)), 500);    // one beat in
    assert_eq!(ms(None), 0);

    // at 60bpm the same width spans 16s
    state.set_tempo(60.);
    assert_eq!(ms(Some(400.)), 8_000);
  }

  #[test]
  fn test_playlist_transport() {
    let state = daw::InnerState::default();

    // nothing to sync to before the playlist has been started
    assert!(playlist_transport(&state).is_err());

    let started = Instant::now() - std::time::Duration::from_millis(1_500);
    *state.playlist.started_time.lock().unwrap() = Some(started);

    let t = playlist_transport(&state).unwrap();
    assert_eq!(t.beat_interval_ms, 500.); // 120bpm
    assert_eq!(t.loop_beats, 16);
    assert_eq!(t.beats_displayed, 16);
    assert!(t.loop_enabled);

    // start time is reported in wall clock ms, 1.5s in the past
    let now_ms = SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_secs_f64() * 1000.;
    assert!((now_ms - t.start_time_ms - 1_500.).abs() < 50.);

    state.set_tempo(60.);
    assert_eq!(playlist_transport(&state).unwrap().beat_interval_ms, 1_000.);
  }

  #[test]
  fn test_add_and_move_snap_to_the_snap_setting() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let ms = |id: u64| state.playlist.audiograph.lock().unwrap()
      .get_node_by_id(id).unwrap().start_offset.as_millis();

    // default snap is one beat (500ms at 120bpm)
    let id = add_audiograph_node(&state, sample.clone(), 0, Duration::from_millis(700), true).unwrap();
    assert_eq!(ms(id), 500);
    let id2 = add_audiograph_node(&state, sample.clone(), 0, Duration::from_millis(800), true).unwrap();
    assert_eq!(ms(id2), 1_000);

    // snap disabled keeps the exact position
    let id3 = add_audiograph_node(&state, sample.clone(), 0, Duration::from_millis(700), false).unwrap();
    assert_eq!(ms(id3), 700);

    // a different snap setting is respected (sixteenth notes = 125ms)
    state.playlist.ui.lock().unwrap().snap_subdivision = daw::timing::Note::SixteenthNote;
    let id4 = add_audiograph_node(&state, sample, 0, Duration::from_millis(700), true).unwrap();
    assert_eq!(ms(id4), 750);
  }
}
