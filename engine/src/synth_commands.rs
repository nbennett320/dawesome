// the Note Sequencer's synths: the list (see daw_core/sequencer.rs), their
// plugins (see effects/synths.rs), and the keys auditioned on them (see
// context/vst-project/04-api.md for the contract with the frontend)
//
// the plugins follow the list and the playlist: whenever the synths, the
// note patterns or the pattern clips change (follow, called after every
// recorded edit and undo, and a few times a second whatever changed them),
// each track that plays a synth gets a copy of it
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use serde::Serialize;

use crate::daw;
use crate::playlist_commands::beat_to_offset;
use crate::history;
use crate::history_commands::{project_view, ProjectView};

// how often the synths are brought in line with the playlist, whatever
// changed it, and the settings of those showing passed on to their copies
const FOLLOW_EVERY: Duration = Duration::from_millis(250);

// the sequencer as the frontend sees it, with its synths' plugins
pub fn sequencer_state(state: &daw::InnerState) -> daw::SequencerState {
  state.playlist.sequencer.lock().unwrap().state(&*state.playlist.effects)
}

// which synths each track plays, and the synth list
fn wanted(state: &daw::InnerState) -> (Vec<daw::Synth>, BTreeSet<(u64, u32)>) {
  // the graph is locked before the sequencer, as everywhere
  let audiograph = state.playlist.audiograph.lock().unwrap();
  let needed = audiograph.synth_tracks();
  let list = state.playlist.sequencer.lock().unwrap().synths().to_vec();
  (list, needed)
}

// bring the synths' plugins in line with the synth list and the pattern
// clips (see Effects::follow_synths). with none of the project's locks held
pub fn follow(state: &daw::InnerState) {
  let (list, needed) = wanted(state);
  state.playlist.effects.follow_synths(&list, &needed);
}

// before a run starts: every copy there, and in step with its synth
pub fn before_playing(state: &daw::InnerState) {
  follow(state);
  state.playlist.effects.push_synth_states(true);
}

// follow the playlist, pass showing synths' settings on, and let go of
// copies no track has needed for a while, for as long as the app runs
pub fn keep_following(state: Arc<daw::InnerState>) {
  let started = std::thread::Builder::new().name("synths".into()).spawn(move || loop {
    std::thread::sleep(FOLLOW_EVERY);
    follow(&state);
    state.playlist.effects.push_synth_states(false);
    state.playlist.effects.drop_spare_copies();
  });
  if let Err(e) = started {
    println!("synths won't follow the playlist: {}", e);
  }
}

// change the synth list (and its notes), as a step that can be undone, then
// bring the plugins in line and return the whole sequencer
fn edit(
  state: &daw::InnerState,
  gesture: Option<u64>,
  f: impl FnOnce(&mut daw::Sequencer) -> Result<(), String>,
) -> Result<daw::SequencerState, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  Ok(sequencer_state(state))
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddedSynth {
  pub synth_id: u64,
  pub sequencer: daw::SequencerState,
}

// an installed instrument, at the end of the synth list
pub fn add(state: &daw::InnerState, plugin_id: &str) -> Result<AddedSynth, String> {
  let synth = state.playlist.effects.load_synth(plugin_id)?;
  let synth_id = synth.id;
  let sequencer = edit(state, None, |seq| seq.add_synth(synth));
  // (refused: its plugin goes)
  if sequencer.is_err() { follow(state); }
  Ok(AddedSynth { synth_id, sequencer: sequencer? })
}

// a synth out of the list, with its notes in every pattern
pub fn remove(state: &daw::InnerState, synth_id: u64) -> Result<daw::SequencerState, String> {
  edit(state, None, |seq| seq.remove_synth(synth_id))
}

pub fn set_volume(state: &daw::InnerState, synth_id: u64, volume: f32, gesture: Option<u64>) -> Result<daw::SequencerState, String> {
  edit(state, gesture, |seq| seq.set_synth_volume(synth_id, volume))
}

pub fn set_muted(state: &daw::InnerState, synth_id: u64, muted: bool) -> Result<daw::SequencerState, String> {
  edit(state, None, |seq| seq.set_synth_muted(synth_id, muted))
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddedOnTrack {
  pub synth_id: u64,
  pub pattern_id: u64,
  pub project: ProjectView,
}

// an instrument dropped on a mixer strip (P9): a new synth, a blank one-bar
// note pattern, and a clip of it on that track at `drop_beat` (snapped, as
// add_pattern_clip places one). one step to undo
pub fn add_on_track(state: &daw::InnerState, plugin_id: &str, track: u32, drop_beat: Option<f32>) -> Result<AddedOnTrack, String> {
  let synth = state.playlist.effects.load_synth(plugin_id)?;
  let synth_id = synth.id;
  let beats_per_bar = state.playlist.time_signature.lock().unwrap().numerator;
  let start_offset = beat_to_offset(state, drop_beat);
  let subdivision = {
    let ui = state.playlist.ui.lock().unwrap();
    if ui.snap_enabled { Some(ui.snap_subdivision) } else { None }
  };

  let before = history::picture(state, &[]);
  let made = (|| -> Result<(u64, u64), String> {
    // the graph is locked before the sequencer, as everywhere
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let pattern = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      sequencer.add_synth(synth)?;
      sequencer.add_pattern(1, beats_per_bar, daw::PatternKind::Note)?
    };
    let clip = audiograph.add_pattern_node(pattern, start_offset, track, subdivision)?;
    Ok((pattern, clip))
  })();
  let ids: Vec<u64> = made.iter().map(|&(_, clip)| clip).collect();
  history::record_between(state, None, before, history::picture(state, &ids));
  if made.is_err() { follow(state); }
  let (pattern_id, _) = made?;
  Ok(AddedOnTrack { synth_id, pattern_id, project: project_view(state) })
}

// start a key on a synth's master copy, now (it's ended by stop_note, not
// by playback stopping). a missing synth plays nothing
pub fn play_note_on(state: &daw::InnerState, engine: &daw::Engine, synth_id: u64, pitch: u8, velocity: f32) -> Result<(), String> {
  if pitch > 127 { return Err(format!("there's no note {}", pitch)); }
  if state.playlist.sequencer.lock().unwrap().synth(synth_id).is_none() {
    return Err(format!("no synth with id {}", synth_id));
  }
  let velocity = if velocity.is_finite() { velocity.clamp(0., 1.) } else { 1. };
  let Some(&copy) = state.playlist.effects.copy_ids().get(&(synth_id, None)) else { return Ok(()) };
  engine.play_master_notes(copy, None, vec![daw::TimedNote::on(0, pitch, velocity)])
}

pub fn stop_note_on(state: &daw::InnerState, engine: &daw::Engine, synth_id: u64, pitch: u8) -> Result<(), String> {
  let Some(&copy) = state.playlist.effects.copy_ids().get(&(synth_id, None)) else { return Ok(()) };
  engine.play_master_notes(copy, None, vec![daw::TimedNote::off(0, pitch.min(127))])
}

#[tauri::command]
pub fn add_synth(state: daw::TState<'_>, plugin_id: String) -> Result<AddedSynth, String> {
  add(&state, &plugin_id)
}

#[tauri::command]
pub fn remove_synth(state: daw::TState<'_>, synth_id: u64) -> Result<daw::SequencerState, String> {
  remove(&state, synth_id)
}

// a synth's volume, as a gain (1 leaves it as it is). one drag (the same
// gesture) is one step to undo
#[tauri::command]
pub fn set_synth_volume(state: daw::TState<'_>, synth_id: u64, volume: f32, gesture: Option<u64>) -> Result<daw::SequencerState, String> {
  set_volume(&state, synth_id, volume, gesture)
}

#[tauri::command]
pub fn set_synth_muted(state: daw::TState<'_>, synth_id: u64, muted: bool) -> Result<daw::SequencerState, String> {
  set_muted(&state, synth_id, muted)
}

#[tauri::command]
pub fn add_synth_on_track(state: daw::TState<'_>, plugin_id: String, track_number: u32, drop_beat: Option<f32>) -> Result<AddedOnTrack, String> {
  add_on_track(&state, &plugin_id, track_number, drop_beat)
}

#[tauri::command]
pub fn play_note(state: daw::TState<'_>, synth_id: u64, pitch: u8, velocity: f32) -> Result<(), String> {
  let engine = daw::engine()?;
  play_note_on(&state, &engine, synth_id, pitch, velocity)
}

#[tauri::command]
pub fn stop_note(state: daw::TState<'_>, synth_id: u64, pitch: u8) {
  // (with no output open there's nothing sounding)
  if let Some(engine) = daw::engine_if_open() {
    if let Err(e) = stop_note_on(&state, &engine, synth_id, pitch) {
      println!("could not stop note {} on synth {}: {}", pitch, synth_id, e);
    }
  }
}
