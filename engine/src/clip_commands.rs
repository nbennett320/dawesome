// trimming and cutting clips in the playlist (see the windows in
// daw/daw_core/audiograph.rs and clip_reader.rs)
//
// both commands return every clip they changed or made, as it is now, for
// the frontend to show
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::daw;
use crate::history;
use crate::playlist_commands::{beat_to_offset, displayed_beats_and_duration, node_beat, offset_to_beat};
use crate::project::{LoadedClip, LoadedPatternClip};

// clips as they are after a change, of either kind
#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChangedClips {
  pub clips: Vec<LoadedClip>,
  pub pattern_clips: Vec<LoadedPatternClip>,
}

// the clips with these ids, as they are now (ids that name no clip are
// left out)
pub fn changed_clips(state: &daw::InnerState, ids: &[u64]) -> ChangedClips {
  let audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();

  ChangedClips {
    clips: ids.iter()
      .filter_map(|&id| audiograph.get_node_by_id(id))
      .map(|node| LoadedClip::of(node, interval))
      .collect(),
    pattern_clips: ids.iter()
      .filter_map(|&id| audiograph.get_pattern_node_by_id(id))
      .map(|node| LoadedPatternClip::of(node, interval))
      .collect(),
  }
}

// the part of its source a clip is to play: seconds of the sample for a
// sample clip, steps of the pattern for a pattern clip
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct WindowEdit {
  pub id: u64,
  #[serde(flatten)]
  pub window: ClipWindow,
}

// trim clips: set the part of its sample or pattern each one plays. the
// sound stays where it is in the playlist, so a clip whose start is
// trimmed starts later. every window is kept within its source and no
// shorter than a clip can be. all the clips are trimmed, as one step to
// undo, or (when an id names no clip) none are
#[tauri::command]
pub fn set_clip_windows(state: daw::TState<'_>, edits: Vec<WindowEdit>) -> Result<ChangedClips, String> {
  set_windows(&state, &edits)
}

pub(crate) fn set_windows(state: &daw::InnerState, edits: &[WindowEdit]) -> Result<ChangedClips, String> {
  let ids: Vec<u64> = edits.iter().map(|edit| edit.id).collect();

  history::clip_edit(state, &ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.placement(id).is_none()) {
      return Err(format!("no node with id {}", missing));
    }

    for edit in edits {
      if audiograph.get_node_by_id(edit.id).is_some() {
        let (start, end) = edit.window.sample_times();
        audiograph.set_window(edit.id, start, end)?;
      } else {
        audiograph.set_pattern_window(edit.id, edit.window.start, edit.window.end)?;
      }
    }
    Ok(())
  }, |_| vec![])?;

  Ok(changed_clips(state, &ids))
}

// cut clips in two at a beat of the playlist. each clip keeps what's
// before the cut, and a new clip plays what's after it (see
// AudioGraph::split_node). clips the beat isn't inside, or is too near an
// edge of, are left as they are. one step to undo, however many are cut
//
// returns both halves of every clip that was cut
#[tauri::command]
pub fn split_clips(state: daw::TState<'_>, ids: Vec<u64>, beat: f64) -> Result<ChangedClips, String> {
  split(&state, &ids, beat)
}

pub(crate) fn split(state: &daw::InnerState, ids: &[u64], beat: f64) -> Result<ChangedClips, String> {
  if !beat.is_finite() || beat < 0. {
    return Err(format!("{} isn't a beat of the playlist", beat));
  }

  // each clip that was cut, and the half that was made
  let cut = history::clip_edit(state, ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.placement(id).is_none()) {
      return Err(format!("no node with id {}", missing));
    }

    let at: Duration = audiograph.beat_interval().mul_f64(beat);
    let mut cut = Vec::new();
    for &id in ids {
      if let Some(made) = audiograph.split_node(id, at)? {
        cut.push((id, made));
      }
    }
    Ok(cut)
  }, |cut| cut.iter().map(|&(_, made)| made).collect())?;

  let touched: Vec<u64> = cut.iter().flat_map(|&(id, made)| [id, made]).collect();
  Ok(changed_clips(state, &touched))
}

// which edge of a clip stays where it is while it's stretched
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Keep {
  Start,
  End,
}

// a sample clip to stretch: the pitch that makes it the length it was
// dragged to, and the edge that wasn't dragged
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct StretchEdit {
  pub id: u64,
  pub pitch: f32,
  pub keep: Keep,
}

// stretch sample clips, like tape: each is given a pitch, which changes
// how fast and so how long it plays, with one of its edges staying where it
// is in the playlist (see AudioGraph::stretch_node). the pitch is kept in
// its range. all the clips are stretched, as one step to undo, or (when an
// id doesn't name a sample clip) none are
#[tauri::command]
pub fn stretch_clips(state: daw::TState<'_>, edits: Vec<StretchEdit>) -> Result<ChangedClips, String> {
  stretch(&state, &edits)
}

pub(crate) fn stretch(state: &daw::InnerState, edits: &[StretchEdit]) -> Result<ChangedClips, String> {
  let ids: Vec<u64> = edits.iter().map(|edit| edit.id).collect();

  history::clip_edit(state, &ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.get_node_by_id(id).is_none()) {
      return Err(format!("no sample clip with id {}", missing));
    }

    for edit in edits {
      audiograph.stretch_node(edit.id, edit.pitch, edit.keep == Keep::End)?;
    }
    Ok(())
  }, |_| vec![])?;

  Ok(changed_clips(state, &ids))
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;
  use crate::clip_commands::{add_clip, change_clip_settings};
  use crate::playlist_commands::node_beat;
  use crate::project;

  const KICK: &str = "assets/assets_66-bd-01.wav";

  fn state() -> Arc<daw::InnerState> {
    Arc::new(daw::InnerState::default())
  }

  fn saved(state: &daw::InnerState) -> project::Project {
    project::capture(state, None)
  }

  fn steps(state: &daw::InnerState) -> usize {
    state.history.lock().unwrap().len()
  }

  fn edit(id: u64, start: f64, end: Option<f64>) -> WindowEdit {
    WindowEdit { id, window: ClipWindow { start, end } }
  }

  // a sample clip a few hundred milliseconds long on beat 4, and a clip of
  // a one bar pattern on beat 8: (state, sample clip, pattern clip)
  fn clips() -> (Arc<daw::InnerState>, u64, u64) {
    let state = state();
    let sample = add_clip(&state, KICK.to_string(), 0, Some(4.)).unwrap().id;
    let pattern = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let row = sequencer.add_row(KICK).unwrap();
      let pattern = sequencer.add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
      sequencer.set_step(pattern, row, 0, true).unwrap();
      sequencer.set_step(pattern, row, 10, true).unwrap();
      pattern
    };
    let clip = state.playlist.audiograph.lock().unwrap()
      .add_pattern_node::<daw::timing::Note>(pattern, Duration::from_secs(4), 1, None).unwrap();
    state.history.lock().unwrap().clear();
    (state, sample, clip)
  }

  fn sample_length(state: &daw::InnerState, id: u64) -> f64 {
    state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().sample_duration().as_secs_f64()
  }

  #[test]
  fn test_trimming_clips_of_both_kinds_is_one_step() {
    let (state, sample, pattern) = clips();
    let length = sample_length(&state, sample);
    assert!(length > 0.1, "the kick is too short for this test");
    let before = saved(&state);

    let changed = set_windows(&state, &[edit(sample, 0.02, Some(0.08)), edit(pattern, 4., Some(12.))]).unwrap();
    assert_eq!(steps(&state), 1);
    let after = saved(&state);
    assert_ne!(before, after);

    // what comes back is each clip as it is now
    assert_eq!(changed.clips.len(), 1);
    assert_eq!(changed.clips[0].id, sample);
    assert_eq!(changed.clips[0].info.window, Some(ClipWindow { start: 0.02, end: Some(0.08) }));
    assert!((changed.clips[0].info.played_duration - 0.06).abs() < 1e-6);
    // 20ms later than beat 4, at 120bpm
    assert!((changed.clips[0].start_beat - 4.04).abs() < 1e-5);
    assert_eq!(changed.pattern_clips.len(), 1);
    assert_eq!(changed.pattern_clips[0].window, Some(ClipWindow { start: 4., end: Some(12.) }));
    // a step later for each of the four steps hidden: a beat
    assert_eq!(changed.pattern_clips[0].start_beat, 9.);

    assert!(history::undo(&state).unwrap().is_some());
    assert_eq!(saved(&state), before);
    assert_eq!(node_beat(&state, sample), Ok(4.));
    assert_eq!(state.playlist.audiograph.lock().unwrap().get_node_by_id(sample).unwrap().window(), None);
    assert!(history::redo(&state).unwrap().is_some());
    assert_eq!(saved(&state), after);

    // a window that's the whole source again is no window
    let whole = set_windows(&state, &[edit(sample, 0., None), edit(pattern, 0., Some(16.))]).unwrap();
    assert_eq!(whole.clips[0].info.window, None);
    assert_eq!(whole.pattern_clips[0].window, None);
    assert_eq!(saved(&state), before);
  }

  #[test]
  fn test_trimming_is_all_or_nothing() {
    let (state, sample, _) = clips();
    let before = saved(&state);

    assert!(set_windows(&state, &[edit(sample, 0.02, None), edit(9_999, 0., None)]).is_err());
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);

    // a trim that changes nothing is no step
    set_windows(&state, &[edit(sample, 0., None)]).unwrap();
    assert_eq!(steps(&state), 0);
    // numbers that make no sense are the source's ends
    set_windows(&state, &[edit(sample, f64::NAN, Some(f64::INFINITY))]).unwrap();
    set_windows(&state, &[edit(sample, -4., Some(-1.))]).unwrap();
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);
    // and an end before the start leaves the least a clip can be
    set_windows(&state, &[edit(sample, 0.05, Some(0.01))]).unwrap();
    let node = state.playlist.audiograph.lock().unwrap().get_node_by_id(sample).unwrap().clone();
    assert_eq!(node.window().unwrap().start, Duration::from_millis(50));
    assert_eq!(node.duration(), daw::MIN_CLIP);
  }

  #[test]
  fn test_cutting_a_selection_is_one_step() {
    let (state, sample, pattern) = clips();
    // a second sample clip, which the cut isn't inside
    let elsewhere = add_clip(&state, KICK.to_string(), 2, Some(20.)).unwrap().id;
    // (the pattern clip is moved under the cut: onto beat 3)
    state.playlist.audiograph.lock().unwrap().move_node(pattern, Duration::from_millis(1_500), 1).unwrap();
    change_clip_settings(&state, sample, daw::ClipSettings { attack_ms: 20., decay_ms: 30., ..Default::default() }, None).unwrap();
    state.history.lock().unwrap().clear();
    let before = saved(&state);

    // beat 4.1: 50ms into the sample clip, and 4.4 steps into the pattern clip
    let changed = split(&state, &[sample, pattern, elsewhere], 4.1).unwrap();
    assert_eq!(steps(&state), 1);
    let after = saved(&state);

    // both halves of each clip that was cut, and nothing of the one that wasn't
    assert_eq!(changed.clips.len(), 2);
    assert_eq!(changed.pattern_clips.len(), 2);
    let (left, right) = (&changed.clips[0], &changed.clips[1]);
    assert_eq!(left.id, sample);
    assert!(right.id != sample && right.id != pattern && right.id != elsewhere);
    assert_eq!(left.start_beat, 4.);
    assert!((right.start_beat - 4.1).abs() < 1e-5);
    assert_eq!(left.info.window, Some(ClipWindow { start: 0., end: right.info.window.map(|w| w.start) }));
    assert!((right.info.window.unwrap().start - 0.05).abs() < 1e-8);
    assert_eq!(right.info.window.unwrap().end, None);
    assert_eq!((left.path.as_str(), right.track_number), (KICK, 0));
    // the fade in stays with the first half, the fade out goes with the second
    assert_eq!((left.info.settings.attack_ms, left.info.settings.decay_ms), (20., 0.));
    assert_eq!((right.info.settings.attack_ms, right.info.settings.decay_ms), (0., 30.));
    let (first, second) = (&changed.pattern_clips[0], &changed.pattern_clips[1]);
    assert_eq!(first.id, pattern);
    assert_eq!(first.window, Some(ClipWindow { start: 0., end: Some(4.4) }));
    assert_eq!(second.window, Some(ClipWindow { start: 4.4, end: None }));
    assert_eq!((first.start_beat, second.track_number), (3., 1));
    assert!((second.start_beat - 4.1).abs() < 1e-5);

    // one undo puts every clip back as it was, and takes the new ones away
    let undone = history::undo(&state).unwrap().unwrap();
    assert_eq!(undone.clips, Some(vec![sample, pattern]));
    assert_eq!(saved(&state), before);
    {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      assert_eq!((audiograph.nodes.len(), audiograph.pattern_nodes.len()), (2, 1));
    }
    // redone, the halves are the same clips again
    let redone = history::redo(&state).unwrap().unwrap();
    assert_eq!(redone.clips.unwrap().len(), 4);
    assert_eq!(saved(&state), after);
    assert_eq!(changed_clips(&state, &[right.id]).clips[0], *right);
  }

  #[test]
  fn test_stretching_clips_is_one_step() {
    let (state, sample, pattern) = clips();
    let other = add_clip(&state, KICK.to_string(), 2, Some(20.)).unwrap().id;
    change_clip_settings(&state, other, daw::ClipSettings { pitch: 5., gain_db: -6., ..Default::default() }, None).unwrap();
    state.history.lock().unwrap().clear();
    let before = saved(&state);
    let node = |id: u64| state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().clone();
    let (was, other_was) = (node(sample), node(other));
    let ends = |clip: &daw::AudioNode| clip.start_offset + clip.duration();

    // one to twice its length from its start, the other to half its length
    // (an octave above where it was) from its end
    let edits = [
      StretchEdit { id: sample, pitch: -12., keep: Keep::Start },
      StretchEdit { id: other, pitch: 17., keep: Keep::End },
    ];
    let changed = stretch(&state, &edits).unwrap();
    assert_eq!(steps(&state), 1);
    assert_eq!(changed.clips.len(), 2);
    assert_eq!(changed.clips[0].info.settings.pitch, -12.);
    let after = saved(&state);

    // twice as long, starting where it did
    assert_eq!(node(sample).start_offset, was.start_offset);
    assert!(node(sample).duration().abs_diff(was.duration() * 2) < Duration::from_micros(1));
    // half as long, ending where it did, with its other settings as they were
    assert!(node(other).duration().abs_diff(other_was.duration() / 2) < Duration::from_micros(1));
    assert!(ends(&node(other)).abs_diff(ends(&other_was)) < Duration::from_nanos(2));
    assert!(node(other).start_offset > other_was.start_offset);
    assert_eq!(node(other).settings().gain_db, -6.);
    assert!(changed.clips[1].start_beat > 20.);

    assert!(history::undo(&state).unwrap().is_some());
    assert_eq!(saved(&state), before);
    assert_eq!(node(other).start_offset, other_was.start_offset);
    assert!(history::redo(&state).unwrap().is_some());
    assert_eq!(saved(&state), after);

    // a pitch out of range is kept in it
    let far = stretch(&state, &[StretchEdit { id: sample, pitch: -90., keep: Keep::Start }]).unwrap();
    assert_eq!(far.clips[0].info.settings.pitch, -24.);
    // a pattern clip can't be stretched, and then nothing is
    let had = (saved(&state), steps(&state));
    let bad = [StretchEdit { id: other, pitch: 0., keep: Keep::Start }, StretchEdit { id: pattern, pitch: 3., keep: Keep::Start }];
    assert!(stretch(&state, &bad).is_err());
    assert_eq!((saved(&state), steps(&state)), had);
    // the same pitch again is no step
    stretch(&state, &[StretchEdit { id: sample, pitch: -24., keep: Keep::Start }]).unwrap();
    assert_eq!(steps(&state), had.1);
  }

  #[test]
  fn test_a_clip_stretched_from_its_end_at_the_playlists_start_stays_in_the_playlist() {
    let state = state();
    let id = add_clip(&state, KICK.to_string(), 0, Some(0.)).unwrap().id;
    // four times as long, from its end: its start can't go before beat 0
    let was = state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().duration();
    let changed = stretch(&state, &[StretchEdit { id, pitch: -24., keep: Keep::End }]).unwrap();
    assert_eq!(changed.clips[0].start_beat, 0.);
    assert_eq!(changed.clips[0].info.settings.pitch, -24.);
    // so it's its end that moved: it's four times as long from beat 0
    assert!((changed.clips[0].info.played_duration - was.as_secs_f32() * 4.).abs() < 1e-4);
  }

  #[test]
  fn test_a_cut_lands_where_it_is_asked_for_whatever_the_snap_setting() {
    // "Split at playhead" cuts where the playhead is, and the Cut tool has
    // done its own snapping: the engine doesn't snap a cut
    let (state, sample, pattern) = clips();
    state.playlist.audiograph.lock().unwrap().move_node(pattern, Duration::from_millis(1_500), 1).unwrap();
    assert!(state.playlist.ui.lock().unwrap().snap_enabled);

    let changed = split(&state, &[sample, pattern], 4.0537).unwrap();
    assert_eq!((changed.clips.len(), changed.pattern_clips.len()), (2, 2));
    assert!((changed.clips[1].start_beat - 4.0537).abs() < 1e-5);
    assert!((changed.pattern_clips[1].start_beat - 4.0537).abs() < 1e-5);
    // the pattern clip started on beat 3: 4.2148 steps in
    assert!((changed.pattern_clips[1].window.unwrap().start - 4.2148).abs() < 1e-6);
    // and both halves still play the pattern
    assert_eq!(changed.pattern_clips[0].pattern_id, changed.pattern_clips[1].pattern_id);
  }

  // (run by hand, in a release build: `cargo test --release five_hundred -- --ignored --nocapture`)
  #[test]
  #[ignore = "a timing test"]
  fn test_edits_are_quick_with_five_hundred_clips() {
    use std::time::Instant;
    let state = state();
    state.playlist.ui.lock().unwrap().snap_enabled = false;
    let ids: Vec<u64> = (0..500)
      .map(|n| add_clip(&state, KICK.to_string(), n % 10, Some((n / 10) as f32 * 4. + (n % 10) as f32 * 0.25)).unwrap().id)
      .collect();
    state.history.lock().unwrap().clear();
    let limit = Duration::from_millis(50);
    let mut slowest = Duration::ZERO;
    let mut timed = |what: &str, run: &mut dyn FnMut()| {
      let started = Instant::now();
      run();
      let took = started.elapsed();
      println!("{:>28}: {:?}", what, took);
      assert!(took < limit, "{} took {:?}", what, took);
      slowest = slowest.max(took);
    };

    let one = ids[250];
    timed("trim one clip", &mut || { set_windows(&state, &[edit(one, 0.02, Some(0.1))]).unwrap(); });
    timed("cut one clip", &mut || { split(&state, &[one], 100.1).unwrap(); });
    timed("stretch one clip", &mut || { stretch(&state, &[StretchEdit { id: ids[251], pitch: -5., keep: Keep::Start }]).unwrap(); });
    timed("move one clip", &mut || { crate::clip_commands::move_clip(&state, ids[252], 3, Some(7.3)).unwrap(); });
    timed("trim fifty clips", &mut || {
      let edits: Vec<WindowEdit> = ids[..50].iter().map(|&id| edit(id, 0.01, Some(0.12))).collect();
      set_windows(&state, &edits).unwrap();
    });
    timed("copy fifty clips", &mut || { crate::clipboard::copy(&state, &ids[50..100]); });
    timed("paste fifty clips", &mut || { crate::clipboard::paste(&state, 210., None).unwrap(); });
    timed("repeat fifty clips", &mut || { crate::clipboard::repeat(&state, &ids[100..150]).unwrap(); });
    timed("delete fifty clips", &mut || { crate::clip_commands::remove_clips(&state, &ids[150..200]).unwrap(); });
    let state_arc = state.clone();
    timed("undo", &mut || { history::undo(&state_arc).unwrap(); });
    timed("the project after an undo", &mut || { crate::history_commands::project_view(&state); });
    timed("redo", &mut || { history::redo(&state_arc).unwrap(); });
    println!("{:>28}: {:?}", "slowest", slowest);
  }

  #[test]
  fn test_a_cut_that_cuts_nothing_is_no_step() {
    let (state, sample, pattern) = clips();
    let before = saved(&state);

    // beat 30 is inside neither clip, and beat 4 is the sample clip's edge
    for beat in [30., 4., 4.001] {
      assert_eq!(split(&state, &[sample, pattern], beat).unwrap(), ChangedClips::default());
    }
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);

    assert!(split(&state, &[sample, 9_999], 4.1).is_err());
    assert!(split(&state, &[sample], f64::NAN).is_err());
    assert!(split(&state, &[sample], -1.).is_err());
    assert_eq!(saved(&state), before);
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
