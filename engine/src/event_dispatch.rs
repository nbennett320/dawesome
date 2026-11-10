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

// play from the playhead (nothing if already playing)
#[tauri::command]
pub fn t_play_playlist(state: daw::TState<'_>) {
  // a pattern playing on its own is replaced by the playlist
  if !state.playlist.playing_playlist() {
    daw::start_playlist(state.inner());
  }
}

// stop where it is, so play carries on from there
#[tauri::command]
pub fn t_pause_playlist(state: daw::TState<'_>) {
  if state.playlist.playing() {
    daw::pause_playlist(state.inner());
  }
}

// stop and go back to the start
#[tauri::command]
pub fn t_stop_playlist(state: daw::TState<'_>) {
  daw::stop_playlist(state.inner());
}

// move the playhead to a beat (kept within the playlist), carrying on playing
// from there if playing. returns the beat it moved to
#[tauri::command]
pub fn t_seek_playlist(state: daw::TState<'_>, beat: f64) -> f64 {
  daw::seek_playlist(state.inner(), beat)
}

// the playhead, in beats
#[tauri::command]
pub fn get_playlist_position(state: daw::TState<'_>) -> f64 {
  let state = state.inner();
  daw::offset_beats(daw::playhead(state), daw::beat_interval(state))
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
  let started = state
    .playlist
    .started_time
    .lock()
    .unwrap()
    .ok_or("playlist has not been started")?;
  Ok(wall_clock_ms(started)?)
}

fn wall_clock_ms(started: Instant) -> Result<f64, String> {
  let now = Instant::now();
  // UNIX_EPOCH is the epoch on every platform, including windows
  let wall = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .map_err(|e| e.to_string())?
    .as_secs_f64() * 1000.;

  // the start time is when the run's first frame is heard, which is just
  // after it starts (by the output's latency)
  Ok(if started >= now {
    wall + started.duration_since(now).as_secs_f64() * 1000.
  } else {
    wall - now.duration_since(started).as_secs_f64() * 1000.
  })
}

// everything the frontend needs to draw the playback cursor in sync with
// the engine's beat loop, without polling every frame
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTransport {
  playing: bool,
  // engine start time of the current run, ms since the unix epoch (None
  // before the first run)
  start_time_ms: Option<f64>,
  // where on the timeline the current run started, in ms. while playing,
  // the playhead is this plus the time since `start_time_ms` (wrapped at the
  // loop's end)
  start_offset_ms: f64,
  // where the playhead is now, in ms
  position_ms: f64,
  // length of one beat tick in the engine loop
  beat_interval_ms: f64,
  // beats played before the engine loops back to the start
  loop_beats: u64,
  // beats spanned by the full playlist width
  beats_displayed: u64,
  loop_enabled: bool,
  // where the last clip finishes, in ms (0 with no clips)
  song_end_ms: f64,
  // the time signature's numerator
  beats_per_bar: u16,
  // the pattern playing on its own (Play in the Sequencer tab), or None
  // when the playlist is playing or nothing is. while a pattern plays, the
  // playlist's playhead stays still
  pattern_id: Option<u64>,
  // how long that pattern is, in ms. it started at `start_time_ms` and
  // loops at this length
  pattern_length_ms: f64,
}

fn playlist_transport(state: &daw::InnerState) -> PlaylistTransport {
  let beat_interval = daw::beat_interval(state);
  // read as a pair: both move when the loop comes round
  let (start_offset, started) = daw::run_anchor(state);
  // (worked out before the struct, whose fields hold other locks as they're read)
  let loop_beats = daw::offset_beats(daw::loop_end(state), beat_interval).round() as u64;

  PlaylistTransport {
    playing: state.playlist.playing(),
    start_time_ms: started.and_then(|started| wall_clock_ms(started).ok()),
    start_offset_ms: start_offset.as_secs_f64() * 1000.,
    position_ms: daw::playhead(state).as_secs_f64() * 1000.,
    beat_interval_ms: beat_interval.as_secs_f64() * 1000.,
    loop_beats,
    beats_displayed: state.playlist.ui.lock().unwrap().max_beats_displayed,
    loop_enabled: state.playlist.loop_enabled.load(Ordering::SeqCst),
    song_end_ms: daw::song_end(state).unwrap_or_default().as_secs_f64() * 1000.,
    beats_per_bar: state.playlist.time_signature.lock().unwrap().numerator,
    pattern_id: state.playlist.playing_pattern(),
    pattern_length_ms: state.playlist.playing_pattern()
      .and_then(|id| daw::pattern_len(state, id))
      .map_or(0., |len| len.as_secs_f64() * 1000.),
  }
}

#[tauri::command]
pub fn get_playlist_transport(
  state: daw::TState<'_>
) -> PlaylistTransport {
  playlist_transport(state.inner())
}

#[tauri::command]
pub fn get_playlist_tempo(
  state: daw::TState<'_>
) -> Result<f32, String> {
  Ok(state.tempo())
}

// change the tempo (see daw::set_playlist_tempo). refused while recording
#[tauri::command]
pub fn set_playlist_tempo(
  state: daw::TState<'_>,
  val: f32,
) -> Result<f32, String> {
  println!("playlist tempo updated: {}", val);
  daw::set_playlist_tempo(state.inner(), val)
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

// whether recording counts in a bar first. returns the new setting
#[tauri::command]
pub fn t_toggle_count_in(state: daw::TState<'_>) -> bool {
  !state.playlist.count_in.fetch_xor(true, Ordering::SeqCst)
}

#[tauri::command]
pub fn get_count_in(state: daw::TState<'_>) -> bool {
  state.playlist.count_in.load(Ordering::SeqCst)
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
  // the playhead's time into the playlist
  Ok(util::format_runtime(daw::playhead(state.inner())))
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

// (async: off the main thread, as listing a folder can take a while)
#[tauri::command(async)]
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

#[tauri::command(async)]
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
  // the first sound opens the output, which takes a moment
  thread::spawn(move || {
    if let Err(e) = daw::play_sample(&path) {
      println!("could not preview sample: {}", e);
    }
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

// the number of beats the playlist shows, and the time they span
// at the current tempo
fn displayed_beats_and_duration(
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

// a node that was just added, and where the frontend should draw it
#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlacedNode {
  pub(crate) id: u64,
  // beats from the start of the playlist, after snapping
  pub(crate) start_beat: f32,
}

fn add_audiograph_node(
  state: &daw::InnerState,
  sample_path: String,
  track_number: u32,
  start_offset: std::time::Duration,
  snap_enabled: bool,
) -> Result<u64, String> {
  add_audiograph_node_at(state, sample_path, track_number, |_| start_offset, snap_enabled)
}

// add a clip where `start_offset` says, asked once its sample has been
// decoded: that takes a while, and the tempo may change meanwhile, which
// would move a time worked out before it
fn add_audiograph_node_at(
  state: &daw::InnerState,
  sample_path: String,
  track_number: u32,
  start_offset: impl FnOnce(&daw::InnerState) -> std::time::Duration,
  snap_enabled: bool,
) -> Result<u64, String> {
  // decoded before taking the graph's lock, which playback (and the
  // playhead, polled from the UI) needs
  let mut node = daw::AudioNode::new(0, sample_path, std::time::Duration::ZERO, track_number)?;
  node.start_offset = start_offset(state);

  let subdivision = state.playlist.ui.lock().unwrap().snap_subdivision;
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  if snap_enabled {
    // snap to nearest snap subdivision
    node.start_offset = audiograph.find_nearest_offset_subdivision(node.start_offset, subdivision);
  }

  Ok(audiograph.insert_node(node))
}

// runs off the main thread (async): decoding the sample takes a while
#[tauri::command(async)]
pub fn t_add_audiograph_node(
  state: daw::TState<'_>,
  sample_path: String,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<PlacedNode, String> {
  let snap_enabled = state.playlist.ui.lock().unwrap().snap_enabled;

  let id = add_audiograph_node_at(
    state.inner(), sample_path, track_number, |state| beat_to_offset(state, drop_beat), snap_enabled)?;
  let start_beat = node_beat(state.inner(), id)?;

  Ok(PlacedNode { id, start_beat })
}

#[tauri::command]
pub fn move_audiograph_node(
  state: daw::TState<'_>,
  id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<f32, String> {
  let start_offset = beat_to_offset(state.inner(), drop_beat);

  println!("moved sample with id: {}, with offset of: {}ms", id, start_offset.as_millis());

  // returns where the frontend should draw the node (in beats), after snapping
  move_audiograph_node_to(state.inner(), id, track_number, start_offset)?;
  node_beat(state.inner(), id)
}

fn move_audiograph_node_to(
  state: &daw::InnerState,
  id: u64,
  track_number: u32,
  start_offset: std::time::Duration,
) -> Result<(), String> {
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

// where a clip in a moved group ended up, for the frontend to draw it
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovedNode {
  id: u64,
  // beats from the start of the playlist, after snapping
  start_beat: f32,
  track_number: u32,
}

// move several clips together (a selection being dragged). the grabbed clip
// goes to `drop_beat` on `track_number`, snapped by the snap setting, and the
// rest move by as much, so their spacing is kept
#[tauri::command]
pub fn move_audiograph_nodes(
  state: daw::TState<'_>,
  ids: Vec<u64>,
  anchor_id: u64,
  drop_beat: f32,
  track_number: u32,
) -> Result<Vec<MovedNode>, String> {
  move_audiograph_nodes_to(state.inner(), &ids, anchor_id, drop_beat, track_number)
}

fn move_audiograph_nodes_to(
  state: &daw::InnerState,
  ids: &[u64],
  anchor_id: u64,
  drop_beat: f32,
  track_number: u32,
) -> Result<Vec<MovedNode>, String> {
  let anchor_offset = beat_to_offset(state, Some(drop_beat));
  // a clip can start as late as the end of the playlist, as with one clip
  let (_, max_offset) = displayed_beats_and_duration(state);
  let subdivision = {
    let ui = state.playlist.ui.lock().unwrap();
    if ui.snap_enabled { Some(ui.snap_subdivision) } else { None }
  };

  let moved = state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .move_nodes(ids, anchor_id, anchor_offset, track_number, max_offset, subdivision)?;

  Ok(moved.into_iter()
    .map(|(id, offset, track_number)| MovedNode {
      id,
      start_beat: offset_to_beat(state, offset),
      track_number,
    })
    .collect())
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
) -> Result<(String, f32), String> {
  let playlist = &state.playlist;
  let mut audiograph = playlist
    .audiograph
    .lock()
    .unwrap();
  let node = audiograph.get_mut_node_by_id(id)
    .ok_or_else(|| format!("no node with id {}", id))?;
  println!("offset dur: {:?}", node.start_offset.as_millis());
  let waveform = node.get_waveform().to_string();
  let dur = node.duration().as_secs_f32();
  let ratio = dur / audiograph.max_beats() as f32;

  // return waveform data
  Ok((waveform, dur))
}

// a clip's playback settings and lengths (seconds), for the Sample tab
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipInfo {
  pub(crate) settings: daw::ClipSettings,
  // the sample's length, before the settings change its speed
  pub(crate) sample_duration: f32,
  // how long the clip plays for
  pub(crate) played_duration: f32,
}

pub(crate) fn clip_info(node: &daw::AudioNode) -> ClipInfo {
  ClipInfo {
    settings: node.settings(),
    sample_duration: node.sample_duration().as_secs_f32(),
    played_duration: node.duration().as_secs_f32(),
  }
}

fn get_clip_of(state: &daw::InnerState, id: u64) -> Result<ClipInfo, String> {
  let audiograph = state.playlist.audiograph.lock().unwrap();
  let node = audiograph.get_node_by_id(id)
    .ok_or_else(|| format!("no node with id {}", id))?;

  Ok(clip_info(node))
}

fn set_clip_settings_of(
  state: &daw::InnerState,
  id: u64,
  settings: daw::ClipSettings,
) -> Result<ClipInfo, String> {
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let node = audiograph.get_mut_node_by_id(id)
    .ok_or_else(|| format!("no node with id {}", id))?;
  node.set_settings(settings);

  Ok(clip_info(node))
}

#[tauri::command]
pub fn get_clip(
  state: daw::TState<'_>,
  id: u64,
) -> Result<ClipInfo, String> {
  get_clip_of(&state, id)
}

// change how a clip plays back. settings are kept in range, and the clip's
// info is returned with the settings that were kept
//
// takes effect the next time the clip starts playing
#[tauri::command]
pub fn set_clip_settings(
  state: daw::TState<'_>,
  id: u64,
  settings: daw::ClipSettings,
) -> Result<ClipInfo, String> {
  set_clip_settings_of(&state, id, settings)
}

// play a clip on its own, with its settings
#[tauri::command]
pub fn t_preview_node(
  state: daw::TState<'_>,
  id: u64,
) -> Result<(), String> {
  let source = state.playlist.audiograph.lock().unwrap()
    .get_node_by_id(id)
    .ok_or_else(|| format!("no node with id {}", id))?
    .source();

  thread::spawn(move || {
    if let Err(e) = daw::engine().and_then(|engine| engine.preview(source)) {
      println!("could not preview clip {}: {}", id, e);
    }
  });

  Ok(())
}

// waveform data and duration (seconds) of a sample that isn't in the
// playlist yet, in the same shape as get_node_data
// used to draw a clip while it's dragged in from the sidebar
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn get_sample_preview(
  state: daw::TState<'_>,
  sample_path: String,
) -> Result<(String, f32), String> {
  let (dur, waveform) = daw::preview_sample(&sample_path)?;

  Ok((waveform, dur.as_secs_f32()))
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::{Duration, Instant};

  #[test]
  fn test_start_time_can_be_just_ahead() {
    let state = daw::InnerState::default();
    let wall = |t: SystemTime| t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() * 1000.;

    // a run that's heard 20ms from now
    *state.playlist.started_time.lock().unwrap() = Some(Instant::now() + Duration::from_millis(20));
    let ahead = get_start_time_ms(&state).unwrap() - wall(SystemTime::now());
    // (the wall clock and Instant are separate clocks, so allow a little)
    assert!(ahead > 15. && ahead < 21., "{}ms ahead", ahead);

    // and one heard 1.5s ago
    *state.playlist.started_time.lock().unwrap() = Some(Instant::now() - Duration::from_millis(1_500));
    let ago = wall(SystemTime::now()) - get_start_time_ms(&state).unwrap();
    assert!(ago > 1_499. && ago < 1_505., "{}ms ago", ago);
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

  #[test]
  fn test_playlist_transport() {
    let state = daw::InnerState::default();

    // before the playlist has been started there's no start time, but the
    // playhead is still reported
    let t = playlist_transport(&state);
    assert_eq!(t.start_time_ms, None);
    assert!(!t.playing);
    assert_eq!(t.position_ms, 0.);
    assert_eq!(t.song_end_ms, 0.);
    assert_eq!(t.beats_per_bar, 4);

    let started = Instant::now() - Duration::from_millis(1_500);
    *state.playlist.started_time.lock().unwrap() = Some(started);

    let t = playlist_transport(&state);
    assert_eq!(t.beat_interval_ms, 500.); // 120bpm
    // nothing in the playlist: the loop is four bars
    assert_eq!(t.loop_beats, 16);
    assert_eq!(t.beats_displayed, 256);
    assert!(t.loop_enabled);

    // start time is reported in wall clock ms, 1.5s in the past
    let now_ms = SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_secs_f64() * 1000.;
    assert!((now_ms - t.start_time_ms.unwrap() - 1_500.).abs() < 50.);

    state.set_tempo(60.);
    assert_eq!(playlist_transport(&state).beat_interval_ms, 1_000.);
  }

  // pretend a run started `ago_ms` back, from `from_ms` into the playlist
  // (starting a real run needs an audio device)
  fn fake_run(state: &daw::InnerState, from_ms: u64, ago_ms: u64) {
    *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(from_ms);
    *state.playlist.started_time.lock().unwrap() = Some(Instant::now() - Duration::from_millis(ago_ms));
    state.playlist.set_playing(true);
  }

  fn playhead_ms(state: &daw::InnerState) -> f64 {
    daw::playhead(state).as_secs_f64() * 1000.
  }

  #[test]
  fn test_playhead_follows_the_run() {
    let state = daw::InnerState::default();

    // stopped: wherever it was left
    *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(750);
    assert_eq!(playhead_ms(&state), 750.);

    // playing from 2s, 1.25s ago
    fake_run(&state, 2_000, 1_250);
    assert!((playhead_ms(&state) - 3_250.).abs() < 50.);
    assert!((playlist_transport(&state).position_ms - 3_250.).abs() < 50.);
    assert_eq!(playlist_transport(&state).start_offset_ms, 2_000.);

    // the 16 beat (8s) loop jumps back to the start
    fake_run(&state, 7_000, 2_000);
    assert!((playhead_ms(&state) - 1_000.).abs() < 50.);

    // without looping it runs on
    state.playlist.loop_enabled.store(false, Ordering::SeqCst);
    assert!((playhead_ms(&state) - 9_000.).abs() < 50.);
  }

  #[test]
  fn test_pause_keeps_the_playhead() {
    let state = Arc::new(daw::InnerState::default());
    fake_run(&state, 2_000, 1_250);

    daw::pause_playlist(&state);
    assert!(!state.playlist.playing());
    assert!((playhead_ms(&state) - 3_250.).abs() < 50.);
    // 3.25s is 6 beats in: the third beat of the second bar
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 6);
    assert_eq!(state.playlist.current_beat.load(Ordering::SeqCst), 2);
  }

  #[test]
  fn test_stop_goes_back_to_the_start() {
    let state = Arc::new(daw::InnerState::default());
    fake_run(&state, 2_000, 1_250);

    daw::stop_playlist(&state);
    assert!(!state.playlist.playing());
    assert_eq!(playhead_ms(&state), 0.);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 0);

    // stopping while stopped also rewinds
    *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(1_000);
    daw::stop_playlist(&state);
    assert_eq!(playhead_ms(&state), 0.);
  }

  #[test]
  fn test_stop_ends_the_run() {
    let state = Arc::new(daw::InnerState::default());
    let run = state.playlist.run_id.load(Ordering::SeqCst);

    // a run's thread checks the id before carrying on, so stopping (or
    // pausing) has to change it
    daw::stop_playlist(&state);
    assert_ne!(state.playlist.run_id.load(Ordering::SeqCst), run);
    let run = state.playlist.run_id.load(Ordering::SeqCst);
    daw::pause_playlist(&state);
    assert_ne!(state.playlist.run_id.load(Ordering::SeqCst), run);
  }

  #[test]
  fn test_seek_while_stopped() {
    let state = Arc::new(daw::InnerState::default());

    assert_eq!(daw::seek_playlist(&state, 4.), 4.);
    assert_eq!(playhead_ms(&state), 2_000.);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 4);
    assert_eq!(state.playlist.current_beat.load(Ordering::SeqCst), 0);
    assert!(!state.playlist.playing());

    // between beats
    assert_eq!(daw::seek_playlist(&state, 2.5), 2.5);
    assert_eq!(playhead_ms(&state), 1_250.);

    // float noise lands on the beat
    assert_eq!(daw::seek_playlist(&state, 3.000_000_1), 3.);

    // kept within the playlist (256 beats)
    assert_eq!(daw::seek_playlist(&state, -3.), 0.);
    assert_eq!(daw::seek_playlist(&state, 40.), 40.);
    assert_eq!(daw::seek_playlist(&state, 400.), 256.);
    assert_eq!(daw::seek_playlist(&state, f64::NAN), 0.);
  }

  #[test]
  fn test_clips_and_the_playhead_keep_their_beat_when_the_tempo_changes() {
    let state = Arc::new(daw::InnerState::default());
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let id = add_audiograph_node(&state, sample, 0, beat_to_offset(&state, Some(8.)), true).unwrap();
    let ms = |id: u64| state.playlist.audiograph.lock().unwrap()
      .get_node_by_id(id).unwrap().start_offset.as_secs_f64() * 1000.;
    let length = |id: u64| state.playlist.audiograph.lock().unwrap()
      .get_node_by_id(id).unwrap().duration();
    let played = length(id);
    daw::seek_playlist(&state, 9.);

    // 120bpm to 90: beat 8 is 5.33s in, rather than 4s
    assert_eq!(daw::set_playlist_tempo(&state, 90.), Ok(90.));
    assert!((node_beat(&state, id).unwrap() - 8.).abs() < 1e-5);
    assert!((ms(id) - 8. * 60_000. / 90.).abs() < 0.01);
    // the playhead is still on beat 9
    assert!((daw::offset_beats(daw::playhead(&state), daw::beat_interval(&state)) - 9.).abs() < 1e-5);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 9);
    // the clip plays the same audio, for as long
    assert_eq!(length(id), played);

    // and back again, without drifting
    daw::set_playlist_tempo(&state, 120.).unwrap();
    assert!((ms(id) - 4_000.).abs() < 0.001);
    assert_eq!(get_playlist_position_of(&state), 9.);

    // tempos that make no sense are refused, and nothing moves
    for bad in [0., 0.5, -30., f32::NAN, f32::INFINITY, 5_000., 1e-20] {
      assert!(daw::set_playlist_tempo(&state, bad).is_err());
    }
    assert_eq!(state.tempo(), 120.);
    assert!((ms(id) - 4_000.).abs() < 0.001);
    assert_eq!(get_playlist_position_of(&state), 9.);
  }

  #[test]
  fn test_playback_carries_on_from_its_beat_at_the_new_tempo() {
    let state = daw::InnerState::default();
    // a clip far enough along that the loop is long
    add_audiograph_node(&state, daw::METRONOME_TICK_PATH.to_string(), 0, beat_to_offset(&state, Some(60.)), true).unwrap();

    // playing from beat 4 (2s), 1.25s ago: beat 6.5
    fake_run(&state, 2_000, 1_250);
    assert_eq!(daw::retime(&state, 60.), Ok(daw::Retimed::Playlist));

    // the run was ended, and the playhead left on the beat it had got to, in
    // the new tempo's time (6.5s), ready to carry on from
    assert!(!state.playlist.playing());
    let beat = daw::offset_beats(daw::playhead(&state), daw::beat_interval(&state));
    assert!((beat - 6.5).abs() < 0.1, "beat {}", beat);
    assert!((playhead_ms(&state) - 6_500.).abs() < 100.);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 6);

    // stopped: nothing to carry on with
    assert_eq!(daw::retime(&state, 90.), Ok(daw::Retimed::Stopped));
    // the same tempo again changes nothing
    assert_eq!(daw::retime(&state, 90.), Ok(daw::Retimed::Stopped));

    // a pattern playing on its own is ended too, to start over. the
    // playlist's playhead stays on its beat
    *state.playlist.mode.lock().unwrap() = daw::PlayMode::Pattern(3);
    state.playlist.set_playing(true);
    assert_eq!(daw::retime(&state, 120.), Ok(daw::Retimed::Pattern(3)));
    assert!(!state.playlist.playing());
    let beat = daw::offset_beats(daw::playhead(&state), daw::beat_interval(&state));
    assert!((beat - 6.5).abs() < 0.1, "beat {}", beat);
  }

  // the playhead, in beats, as get_playlist_position reports it
  fn get_playlist_position_of(state: &daw::InnerState) -> f64 {
    let beats = daw::offset_beats(daw::playhead(state), daw::beat_interval(state));
    (beats * 1e6).round() / 1e6
  }

  #[test]
  fn test_the_loop_ends_with_the_last_clip() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let ms = |d: Duration| d.as_millis();

    // no clips: four bars (8s at 120bpm)
    assert_eq!(ms(daw::loop_end(&state)), 8_000);
    assert_eq!(daw::loop_len(&state).map(ms), Some(8_000));

    // a short clip at beat 9 (the start of bar 3) ends in bar 3
    let id = add_audiograph_node(&state, sample.clone(), 0, beat_to_offset(&state, Some(8.)), true).unwrap();
    assert_eq!(ms(daw::loop_end(&state)), 6_000);
    assert_eq!(playlist_transport(&state).loop_beats, 12);

    // moved into bar 60, the loop follows it
    move_audiograph_node_to(&state, id, 0, beat_to_offset(&state, Some(236.))).unwrap();
    assert_eq!(node_beat(&state, id).unwrap(), 236.);
    assert_eq!(playlist_transport(&state).loop_beats, 240);

    // looping off: no loop length, but where it would end is still known
    state.playlist.loop_enabled.store(false, Ordering::SeqCst);
    assert_eq!(daw::loop_len(&state), None);
    assert_eq!(playlist_transport(&state).loop_beats, 240);
  }

  #[test]
  fn test_the_playhead_wraps_at_the_last_clip() {
    let state = daw::InnerState::default();
    // a clip in bar 2: the loop is two bars (4s)
    add_audiograph_node(&state, daw::METRONOME_TICK_PATH.to_string(), 0, beat_to_offset(&state, Some(5.)), true).unwrap();
    assert_eq!(playlist_transport(&state).loop_beats, 8);

    // 5s into a run from the start: 1s into the second pass
    fake_run(&state, 0, 5_000);
    assert!((playhead_ms(&state) - 1_000.).abs() < 50.);
    let t = playlist_transport(&state);
    assert!((t.position_ms - 1_000.).abs() < 50.);
    assert_eq!(t.start_offset_ms, 0.);
  }

  #[test]
  fn test_song_end_in_transport() {
    let state = daw::InnerState::default();
    let id = state.playlist.audiograph.lock().unwrap()
      .construct_and_add_node(daw::METRONOME_TICK_PATH.to_string(), Duration::from_millis(1_000), 0)
      .unwrap();
    let tick = state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().duration();

    let expected = (Duration::from_millis(1_000) + tick).as_secs_f64() * 1000.;
    assert!((playlist_transport(&state).song_end_ms - expected).abs() < 0.001);
  }

  #[test]
  fn test_runtime_shows_the_playhead() {
    let state = daw::InnerState::default();
    *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(61_250);
    assert_eq!(util::format_runtime(daw::playhead(&state)), "01:01:250");
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

  #[test]
  fn test_clip_settings() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let id = add_audiograph_node(&state, sample, 0, Duration::ZERO, false).unwrap();

    let info = get_clip_of(&state, id).unwrap();
    assert_eq!(info.settings, daw::ClipSettings::default());
    assert_eq!(info.sample_duration, info.played_duration);

    // an octave up halves the played length, and out of range values are kept in range
    let info = set_clip_settings_of(&state, id, daw::ClipSettings {
      pitch: 12.,
      gain_db: 99.,
      ..Default::default()
    }).unwrap();
    assert_eq!(info.settings.pitch, 12.);
    assert_eq!(info.settings.gain_db, 12.);
    assert!((info.played_duration - info.sample_duration / 2.).abs() < 1e-4);
    assert_eq!(get_clip_of(&state, id).unwrap(), info);

    // the frontend's names deserialize
    let settings: daw::ClipSettings = serde_json::from_str(
      r#"{"gainDb":-6,"pan":0.5,"pitch":0,"attackMs":10,"decayMs":20}"#).unwrap();
    assert_eq!(settings.pan, 0.5);

    assert!(get_clip_of(&state, 9999).is_err());
  }

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
  fn test_snapped_nodes_report_their_beat() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();

    // dropped at beat 1.4 snaps to beat 1 (default snap is one beat)
    let id = add_audiograph_node(&state, sample, 0, beat_to_offset(&state, Some(1.4)), true).unwrap();
    assert_eq!(node_beat(&state, id).unwrap(), 1.);

    // moved to beat 3.6 snaps to beat 4
    move_audiograph_node_to(&state, id, 1, beat_to_offset(&state, Some(3.6))).unwrap();
    assert_eq!(node_beat(&state, id).unwrap(), 4.);

    // offset_to_beat undoes beat_to_offset when nothing snaps
    let offset = beat_to_offset(&state, Some(2.46));
    assert!((offset_to_beat(&state, offset) - 2.46).abs() < 0.001);

    assert!(node_beat(&state, 9_999).is_err());
  }

  #[test]
  fn test_move_a_group_of_nodes() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();
    // 120bpm, one-beat snap. b is off the grid, a quarter beat after a
    let a = add_audiograph_node(&state, sample.clone(), 0, beat_to_offset(&state, Some(2.)), true).unwrap();
    let b = add_audiograph_node(&state, sample.clone(), 1, beat_to_offset(&state, Some(2.25)), false).unwrap();
    let c = add_audiograph_node(&state, sample, 3, beat_to_offset(&state, Some(9.)), true).unwrap();
    let beat = |id: u64| node_beat(&state, id).unwrap();

    // a dropped at beat 4.4 on track 1: snaps to 4, b keeps its quarter
    // beat gap, and everything moves down a track
    let moved = move_audiograph_nodes_to(&state, &[a, b], a, 4.4, 1).unwrap();
    assert_eq!(moved, vec![
      MovedNode { id: a, start_beat: 4., track_number: 1 },
      MovedNode { id: b, start_beat: 4.25, track_number: 2 },
    ]);
    // c wasn't in the group
    assert_eq!(beat(c), 9.);

    // well past the first four bars
    move_audiograph_nodes_to(&state, &[a, c], c, 236., 3).unwrap();
    assert_eq!((beat(a), beat(c)), (231., 236.));

    // dragged past the end of the playlist (256 beats): c stops at the end
    move_audiograph_nodes_to(&state, &[a, c], c, 300., 3).unwrap();
    assert_eq!((beat(a), beat(c)), (251., 256.));

    // with snap off the grabbed clip lands exactly where it's dropped
    state.playlist.ui.lock().unwrap().snap_enabled = false;
    move_audiograph_nodes_to(&state, &[a, b], b, 1.5, 2).unwrap();
    assert!((beat(b) - 1.5).abs() < 0.001);
  }
}
