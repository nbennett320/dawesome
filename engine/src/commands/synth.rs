// the synth rows (see daw_core/sequencer.rs): their plugins (see
// effects/synths.rs), and the keys auditioned on rows (see
// context/pattern-merge-project/04-api.md for the contract with the
// frontend)
//
// the plugins follow the rows and the playlist: whenever the rows, the
// patterns or the pattern clips change (follow, called after every recorded
// edit and undo, and a few times a second whatever changed them), each track
// that plays a synth row gets a copy of its plugin
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use serde::Serialize;

use crate::daw;
use crate::commands::playlist::beat_to_offset;
use crate::commands::sequencer::AddedRow;
use crate::history;
use crate::commands::history::{project_view, ProjectView};

// how often the synths are brought in line with the playlist, whatever
// changed it, and the settings of those showing passed on to their copies
const FOLLOW_EVERY: Duration = Duration::from_millis(250);

// the sequencer as the frontend sees it, with its synth rows' plugins
pub fn sequencer_state(state: &daw::InnerState) -> daw::SequencerState {
  state.playlist.sequencer.lock().unwrap().state(&*state.playlist.effects)
}

// the synth rows' plugins, and which of them each track plays
fn wanted(state: &daw::InnerState) -> (Vec<daw::Synth>, BTreeSet<(u64, u32)>) {
  // the graph is locked before the sequencer, as everywhere
  let audiograph = state.playlist.audiograph.lock().unwrap();
  let needed = audiograph.synth_tracks();
  let list = state.playlist.sequencer.lock().unwrap().synths();
  (list, needed)
}

// bring the synth rows' plugins in line with the rows and the pattern clips
// (see Effects::follow_synths). with none of the project's locks held
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

// an installed instrument, as a row at the bottom
pub fn add(state: &daw::InnerState, plugin_id: &str) -> Result<AddedRow, String> {
  let synth = state.playlist.effects.load_synth(plugin_id)?;
  let row_id = history::sequencer_edit(state, &[], None, || {
    Ok(state.playlist.sequencer.lock().unwrap().add_synth_row(synth))
  })?;
  Ok(AddedRow { row_id, sequencer: sequencer_state(state) })
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddedOnTrack {
  pub row_id: u64,
  pub pattern_id: u64,
  pub project: ProjectView,
}

// an instrument dropped on a mixer strip (P9): a new synth row, a blank
// one-bar pattern, and a clip of it on that track at `drop_beat` (snapped, as
// add_pattern_clip places one). one step to undo
pub fn add_on_track(state: &daw::InnerState, plugin_id: &str, track: u32, drop_beat: Option<f32>) -> Result<AddedOnTrack, String> {
  let synth = state.playlist.effects.load_synth(plugin_id)?;
  let beats_per_bar = state.playlist.time_signature.lock().unwrap().numerator;
  let start_offset = beat_to_offset(state, drop_beat);
  let subdivision = {
    let ui = state.playlist.ui.lock().unwrap();
    if ui.snap_enabled { Some(ui.snap_subdivision) } else { None }
  };

  let before = history::picture(state, &[]);
  let made = (|| -> Result<(u64, u64, u64), String> {
    // the graph is locked before the sequencer, as everywhere
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let (row, pattern) = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let row = sequencer.add_synth_row(synth);
      (row, sequencer.add_pattern(1, beats_per_bar)?)
    };
    let clip = audiograph.add_pattern_node(pattern, start_offset, track, subdivision)?;
    Ok((row, pattern, clip))
  })();
  let ids: Vec<u64> = made.iter().map(|&(_, _, clip)| clip).collect();
  // (which brings the plugins in line: refused, the new one goes)
  history::record_between(state, None, before, history::picture(state, &ids));
  let (row_id, pattern_id, _) = made?;
  Ok(AddedOnTrack { row_id, pattern_id, project: project_view(state) })
}

// start a key on a row, now (it's ended by stop_row_note, not by playback
// stopping). a synth row's plays on its plugin's master copy (which its mute
// silences), and a missing plugin plays nothing. a sample row is silent for
// now
pub fn play_row_note_on(state: &daw::InnerState, engine: &daw::Engine, row_id: u64, pitch: u8, velocity: f32) -> Result<(), String> {
  if pitch > 127 { return Err(format!("there's no note {}", pitch)); }
  let Some(slot) = synth_slot(state, row_id)? else { return Ok(()) };
  let velocity = if velocity.is_finite() { velocity.clamp(0., 1.) } else { 1. };
  let Some(&copy) = state.playlist.effects.copy_ids().get(&(slot, None)) else { return Ok(()) };
  engine.play_master_notes(copy, None, vec![daw::TimedNote::on(0, pitch, velocity)])
}

pub fn stop_row_note_on(state: &daw::InnerState, engine: &daw::Engine, row_id: u64, pitch: u8) -> Result<(), String> {
  let Some(slot) = synth_slot(state, row_id)? else { return Ok(()) };
  let Some(&copy) = state.playlist.effects.copy_ids().get(&(slot, None)) else { return Ok(()) };
  engine.play_master_notes(copy, None, vec![daw::TimedNote::off(0, pitch.min(127))])
}

// a row's plugin's slot, or none for a sample row
fn synth_slot(state: &daw::InnerState, row_id: u64) -> Result<Option<u64>, String> {
  let sequencer = state.playlist.sequencer.lock().unwrap();
  let row = sequencer.row(row_id).ok_or_else(|| format!("no row with id {}", row_id))?;
  Ok(row.synth().map(|synth| synth.slot))
}

#[tauri::command]
pub fn add_synth_row(state: daw::TState<'_>, plugin_id: String) -> Result<AddedRow, String> {
  add(&state, &plugin_id)
}

#[tauri::command]
pub fn add_synth_on_track(state: daw::TState<'_>, plugin_id: String, track_number: u32, drop_beat: Option<f32>) -> Result<AddedOnTrack, String> {
  add_on_track(&state, &plugin_id, track_number, drop_beat)
}

#[tauri::command]
pub fn play_row_note(state: daw::TState<'_>, row_id: u64, pitch: u8, velocity: f32) -> Result<(), String> {
  let engine = daw::engine()?;
  play_row_note_on(&state, &engine, row_id, pitch, velocity)
}

#[tauri::command]
pub fn stop_row_note(state: daw::TState<'_>, row_id: u64, pitch: u8) {
  // (with no output open there's nothing sounding)
  if let Some(engine) = daw::engine_if_open() {
    if let Err(e) = stop_row_note_on(&state, &engine, row_id, pitch) {
      println!("could not stop note {} on row {}: {}", pitch, row_id, e);
    }
  }
}
