use std::sync::atomic::{Ordering};
use std::thread;
use tauri;

use crate::{app, daw, history};

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

#[cfg(test)]
pub(crate) fn add_node_at_offset(
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
pub fn add_audiograph_node(
  state: daw::TState<'_>,
  sample_path: String,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<PlacedNode, String> {
  add_clip(state.inner(), sample_path, track_number, drop_beat)
}

// add a clip of a sample where it was dropped, snapped by the snap setting.
// one step to undo
pub(crate) fn add_clip(
  state: &daw::InnerState,
  sample_path: String,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<PlacedNode, String> {
  let snap_enabled = state.playlist.ui.lock().unwrap().snap_enabled;

  let id = history::clip_edit(state, &[], None, || add_audiograph_node_at(
    state, sample_path, track_number, |state| beat_to_offset(state, drop_beat), snap_enabled), |&id| vec![id])?;
  let start_beat = node_beat(state, id)?;

  Ok(PlacedNode { id, start_beat })
}

#[tauri::command]
pub fn move_audiograph_node(
  state: daw::TState<'_>,
  id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<f32, String> {
  move_clip(state.inner(), id, track_number, drop_beat)
}

// move a clip to where it was dropped, snapped by the snap setting. returns
// where it is now, in beats. one step to undo
pub(crate) fn move_clip(
  state: &daw::InnerState,
  id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<f32, String> {
  let start_offset = beat_to_offset(state, drop_beat);

  println!("moved sample with id: {}, with offset of: {}ms", id, start_offset.as_millis());

  // returns where the frontend should draw the node (in beats), after snapping
  history::clip_edit(state, &[id], None, || move_audiograph_node_to(state, id, track_number, start_offset), |_| vec![])?;
  node_beat(state, id)
}

pub(crate) fn move_audiograph_node_to(
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
  move_clips(state.inner(), &ids, anchor_id, drop_beat, track_number)
}

// one step to undo, however many clips
pub(crate) fn move_clips(
  state: &daw::InnerState,
  ids: &[u64],
  anchor_id: u64,
  drop_beat: f32,
  track_number: u32,
) -> Result<Vec<MovedNode>, String> {
  history::clip_edit(
    state, ids, None,
    || move_audiograph_nodes_to(state, ids, anchor_id, drop_beat, track_number),
    |_| vec![])
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

// take several clips out of the playlist (a selection). all of them, or (if
// one isn't there) none
#[tauri::command]
pub fn remove_audiograph_nodes(
  state: daw::TState<'_>,
  ids: Vec<u64>,
) -> Result<(), String> {
  remove_clips(state.inner(), &ids)
}

// take clips of either kind out of the playlist. one step to undo
pub(crate) fn remove_clips(state: &daw::InnerState, ids: &[u64]) -> Result<(), String> {
  // (an id given twice is removed once)
  let mut ids = ids.to_vec();
  ids.sort_unstable();
  ids.dedup();
  let ids = &ids[..];

  history::clip_edit(state, ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.placement(id).is_none()) {
      return Err(format!("no node with id {}", missing));
    }
    for &id in ids {
      audiograph.remove_node(id)?;
    }
    Ok(())
  }, |_| vec![])
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
  // the part of the sample it plays, in seconds of the sample. None: all
  pub(crate) window: Option<ClipWindow>,
}

// the part of its source a clip plays, as the frontend and project files
// have it: seconds of the sample for a sample clip, steps of the pattern
// for a pattern clip
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipWindow {
  pub start: f64,
  // None: to the source's end
  #[serde(default)]
  pub end: Option<f64>,
}

impl ClipWindow {
  pub fn of_sample(window: daw::Window) -> Self {
    ClipWindow { start: window.start.as_secs_f64(), end: window.end.map(|end| end.as_secs_f64()) }
  }

  pub fn of_pattern(window: daw::PatternWindow) -> Self {
    ClipWindow { start: window.start, end: window.end }
  }

  // as lengths of a sample, a number that makes no sense read as the
  // sample's start (or, for the end, its end)
  pub fn sample_times(&self) -> (std::time::Duration, Option<std::time::Duration>) {
    // (a time too far off to count is past any sample's end)
    let time = |secs: f64| match std::time::Duration::try_from_secs_f64(secs) {
      Ok(time) => Some(time),
      Err(_) if secs.is_finite() && secs > 0. => Some(std::time::Duration::MAX),
      Err(_) => None,
    };
    (time(self.start).unwrap_or_default(), self.end.and_then(time))
  }
}

pub(crate) fn clip_info(node: &daw::AudioNode) -> ClipInfo {
  ClipInfo {
    settings: node.settings(),
    sample_duration: node.sample_duration().as_secs_f32(),
    played_duration: node.duration().as_secs_f32(),
    window: node.window().map(ClipWindow::of_sample),
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
// - gesture: the drag (of a knob, or a fade's handle) this change is part
//   of, if any. the changes of one gesture are undone together
#[tauri::command]
pub fn set_clip_settings(
  state: daw::TState<'_>,
  id: u64,
  settings: daw::ClipSettings,
  gesture: Option<u64>,
) -> Result<ClipInfo, String> {
  change_clip_settings(&state, id, settings, gesture)
}

pub(crate) fn change_clip_settings(
  state: &daw::InnerState,
  id: u64,
  settings: daw::ClipSettings,
  gesture: Option<u64>,
) -> Result<ClipInfo, String> {
  history::clip_edit(state, &[id], gesture, || set_clip_settings_of(state, id, settings), |_| vec![])
}

// play a clip on its own, with its settings
#[tauri::command]
pub fn preview_node(
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
  use std::time::Duration;

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
  fn test_add_and_move_snap_to_the_snap_setting() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let ms = |id: u64| state.playlist.audiograph.lock().unwrap()
      .get_node_by_id(id).unwrap().start_offset.as_millis();

    // default snap is one beat (500ms at 120bpm)
    let id = add_node_at_offset(&state, sample.clone(), 0, Duration::from_millis(700), true).unwrap();
    assert_eq!(ms(id), 500);
    let id2 = add_node_at_offset(&state, sample.clone(), 0, Duration::from_millis(800), true).unwrap();
    assert_eq!(ms(id2), 1_000);

    // snap disabled keeps the exact position
    let id3 = add_node_at_offset(&state, sample.clone(), 0, Duration::from_millis(700), false).unwrap();
    assert_eq!(ms(id3), 700);

    // a different snap setting is respected (sixteenth notes = 125ms)
    state.playlist.ui.lock().unwrap().snap_subdivision = daw::timing::Note::SixteenthNote;
    let id4 = add_node_at_offset(&state, sample, 0, Duration::from_millis(700), true).unwrap();
    assert_eq!(ms(id4), 750);
  }

  #[test]
  fn test_clip_settings() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let id = add_node_at_offset(&state, sample, 0, Duration::ZERO, false).unwrap();

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
    let id = add_node_at_offset(&state, sample, 0, beat_to_offset(&state, Some(1.4)), true).unwrap();
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
    let a = add_node_at_offset(&state, sample.clone(), 0, beat_to_offset(&state, Some(2.)), true).unwrap();
    let b = add_node_at_offset(&state, sample.clone(), 1, beat_to_offset(&state, Some(2.25)), false).unwrap();
    let c = add_node_at_offset(&state, sample, 3, beat_to_offset(&state, Some(9.)), true).unwrap();
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
