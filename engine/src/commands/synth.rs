// the synth rows' commands (see actions/synth.rs)
use crate::daw::actions::sequencer::AddedRow;
use crate::daw::sequencer;
use crate::engine;
use crate::commands::TState;
use crate::daw::actions::synth::{add, add_on_track, play_row_note_on, replace, stop_row_note_on, AddedOnTrack};

#[tauri::command]
pub fn add_synth_row(state: TState<'_>, plugin_id: String) -> Result<AddedRow, String> {
  add(&state, &plugin_id)
}

#[tauri::command]
pub fn replace_row_synth(state: TState<'_>, row_id: u64, plugin_id: String) -> Result<sequencer::RowChange, String> {
  replace(&state, row_id, &plugin_id)
}

#[tauri::command]
pub fn add_synth_on_track(state: TState<'_>, plugin_id: String, track_number: u32, drop_beat: Option<f32>) -> Result<AddedOnTrack, String> {
  add_on_track(&state, &plugin_id, track_number, drop_beat)
}

#[tauri::command]
pub fn play_row_note(state: TState<'_>, row_id: u64, pitch: u8, velocity: f32) -> Result<(), String> {
  let engine = engine::engine()?;
  play_row_note_on(&state, &engine, row_id, pitch, velocity)
}

#[tauri::command]
pub fn stop_row_note(state: TState<'_>, row_id: u64, pitch: u8) {
  // (with no output open there's nothing sounding)
  if let Some(engine) = engine::engine_if_open() {
    if let Err(e) = stop_row_note_on(&state, &engine, row_id, pitch) {
      println!("could not stop note {} on row {}: {}", pitch, row_id, e);
    }
  }
}
