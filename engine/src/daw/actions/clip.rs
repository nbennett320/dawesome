// trimming and cutting clips in the playlist (see the windows in
// daw/daw_core/audiograph.rs and clip_reader.rs)
//
// the edits return every clip they changed or made, as it is now, for the
// frontend to show. commands/clip.rs is the frontend's side of these
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::daw::audiograph;
use crate::engine::{clip, clip_reader};
use crate::daw;
use crate::daw::history;
use crate::daw::actions::playlist::{beat_to_offset, displayed_beats_and_duration, node_beat, offset_to_beat};
use crate::daw::project::{LoadedClip, LoadedPatternClip};

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
  let mut node = audiograph::AudioNode::new(0, sample_path, std::time::Duration::ZERO, track_number)?;
  node.start_offset = start_offset(state);

  let subdivision = state.playlist.ui.lock().unwrap().snap_subdivision;
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  if snap_enabled {
    // snap to nearest snap subdivision
    node.start_offset = audiograph.find_nearest_offset_subdivision(node.start_offset, subdivision);
  }

  Ok(audiograph.insert_node(node))
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

// a clip's playback settings and lengths (seconds), for the Sample tab
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipInfo {
  pub(crate) settings: clip::ClipSettings,
  // the sample's length, before the settings change its speed
  pub(crate) sample_duration: f32,
  // how long the clip plays for
  pub(crate) played_duration: f32,
  // the part of the sample it plays, in seconds of the sample. None: all
  pub(crate) window: Option<ClipWindow>,
}

// the part of its source a clip plays, as the frontend and project files
// have it: seconds of the sample for a sample clip, steps of the pattern
// for a pattern clip (whose end can be past its pattern's: see
// PatternNode::start)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipWindow {
  pub start: f64,
  // None: to the sample's end. (a pattern clip's is never None, and one
  // sent as None keeps the clip's end)
  #[serde(default)]
  pub end: Option<f64>,
}

impl ClipWindow {
  pub fn of_sample(window: clip_reader::Window) -> Self {
    ClipWindow { start: window.start.as_secs_f64(), end: window.end.map(|end| end.as_secs_f64()) }
  }

  // where in its pattern a pattern clip starts and ends (see
  // PatternNode::start). the end is never None, and can be past the
  // pattern's end
  pub fn of_pattern(node: &audiograph::PatternNode) -> Self {
    ClipWindow { start: node.start, end: Some(node.end()) }
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

pub(crate) fn clip_info(node: &audiograph::AudioNode) -> ClipInfo {
  ClipInfo {
    settings: node.settings(),
    sample_duration: node.sample_duration().as_secs_f32(),
    played_duration: node.duration().as_secs_f32(),
    window: node.window().map(ClipWindow::of_sample),
  }
}

pub(crate) fn get_clip_of(state: &daw::InnerState, id: u64) -> Result<ClipInfo, String> {
  let audiograph = state.playlist.audiograph.lock().unwrap();
  let node = audiograph.get_node_by_id(id)
    .ok_or_else(|| format!("no node with id {}", id))?;

  Ok(clip_info(node))
}

fn set_clip_settings_of(
  state: &daw::InnerState,
  id: u64,
  settings: clip::ClipSettings,
) -> Result<ClipInfo, String> {
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let node = audiograph.get_mut_node_by_id(id)
    .ok_or_else(|| format!("no node with id {}", id))?;
  node.set_settings(settings);

  Ok(clip_info(node))
}

pub(crate) fn change_clip_settings(
  state: &daw::InnerState,
  id: u64,
  settings: clip::ClipSettings,
  gesture: Option<u64>,
) -> Result<ClipInfo, String> {
  history::clip_edit(state, &[id], gesture, || set_clip_settings_of(state, id, settings), |_| vec![])
}

#[cfg(test)]
mod tests;
