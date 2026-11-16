// the playlist's clip commands (see actions/clip.rs)
use std::thread;
use crate::actions::clip::{
  add_clip, change_clip_settings, get_clip_of, move_clip, move_clips, remove_clips, set_windows, split, stretch,
  ChangedClips, ClipInfo, MovedNode, PlacedNode, StretchEdit, WindowEdit,
};
use crate::bounce::deep_copy;
use crate::commands::TState;
use crate::daw;
use crate::join::{join, Joined};

// trim clips: set the part of its sample or pattern each one plays. the
// sound stays where it is in the playlist, so a clip whose start is
// trimmed starts later. every window is kept within its sample, or for a
// pattern clip from its pattern's start (its end can be past the pattern's,
// where the clip plays it again), and no shorter than a clip can be. all
// the clips are trimmed, as one step to undo, or (when an id names no clip)
// none are
#[tauri::command]
pub fn set_clip_windows(state: TState<'_>, edits: Vec<WindowEdit>) -> Result<ChangedClips, String> {
  set_windows(&state, &edits)
}

// cut clips in two at a beat of the playlist. each clip keeps what's
// before the cut, and a new clip plays what's after it (see
// AudioGraph::split_node). clips the beat isn't inside, or is too near an
// edge of, are left as they are. one step to undo, however many are cut
//
// returns both halves of every clip that was cut
#[tauri::command]
pub fn split_clips(state: TState<'_>, ids: Vec<u64>, beat: f64) -> Result<ChangedClips, String> {
  split(&state, &ids, beat)
}

// stretch sample clips, like tape: each is given a pitch, which changes
// how fast and so how long it plays, with one of its edges staying where it
// is in the playlist (see AudioGraph::stretch_node). the pitch is kept in
// its range. all the clips are stretched, as one step to undo, or (when an
// id doesn't name a sample clip) none are
#[tauri::command]
pub fn stretch_clips(state: TState<'_>, edits: Vec<StretchEdit>) -> Result<ChangedClips, String> {
  stretch(&state, &edits)
}

// runs off the main thread (async): decoding the sample takes a while
#[tauri::command(async)]
pub fn add_audiograph_node(
  state: TState<'_>,
  sample_path: String,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<PlacedNode, String> {
  add_clip(state.inner(), sample_path, track_number, drop_beat)
}

#[tauri::command]
pub fn move_audiograph_node(
  state: TState<'_>,
  id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<f32, String> {
  move_clip(state.inner(), id, track_number, drop_beat)
}

// move several clips together (a selection being dragged). the grabbed clip
// goes to `drop_beat` on `track_number`, snapped by the snap setting, and the
// rest move by as much, so their spacing is kept
#[tauri::command]
pub fn move_audiograph_nodes(
  state: TState<'_>,
  ids: Vec<u64>,
  anchor_id: u64,
  drop_beat: f32,
  track_number: u32,
) -> Result<Vec<MovedNode>, String> {
  move_clips(state.inner(), &ids, anchor_id, drop_beat, track_number)
}

// take several clips out of the playlist (a selection). all of them, or (if
// one isn't there) none
#[tauri::command]
pub fn remove_audiograph_nodes(
  state: TState<'_>,
  ids: Vec<u64>,
) -> Result<(), String> {
  remove_clips(state.inner(), &ids)
}

#[tauri::command]
pub fn get_node_data(
  state: TState<'_>,
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

#[tauri::command]
pub fn get_clip(
  state: TState<'_>,
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
  state: TState<'_>,
  id: u64,
  settings: daw::ClipSettings,
  gesture: Option<u64>,
) -> Result<ClipInfo, String> {
  change_clip_settings(&state, id, settings, gesture)
}

// play a clip on its own, with its settings
#[tauri::command]
pub fn preview_node(
  state: TState<'_>,
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

// join clips into one (see the top of join.rs). the clips have to be on
// one track and of one kind. the joined clip is the earliest of them, with
// its id. one step to undo
// (async: writing sample clips out as one file can take a while)
#[tauri::command(async)]
pub fn join_clips(state: TState<'_>, ids: Vec<u64>) -> Result<Joined, String> {
  join(&state, &ids, daw::open_engine_rate())
}

// write each clip out to a file of its own in the project's clips folder,
// and switch the clip to it: the same clip, where it was, playing the new
// file with its settings at their defaults. a pattern clip becomes a
// sample clip. one step to undo (which leaves the files where they are)
//
// every file is written before any clip is changed: if one can't be, the
// files written so far are removed and no clip changes
#[tauri::command(async)]
pub fn deep_copy_clips(state: TState<'_>, ids: Vec<u64>) -> Result<ChangedClips, String> {
  deep_copy(&state, &ids, daw::open_engine_rate())
}
