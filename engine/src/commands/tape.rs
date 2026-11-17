// the tape's commands: its switch and machine (the Transport tab) and the
// tracks on it (the Mixer tab). see daw/actions/tape.rs
use crate::daw::actions::tape::{change_tape_machine, change_track_tape, tape_settings};
use crate::daw::tape_settings::{TapeMachine, TapeSettings, TrackTape};
use crate::commands::TState;

#[tauri::command]
pub fn get_tape(state: TState<'_>) -> TapeSettings {
  tape_settings(&state)
}

// returns what was kept, in range
#[tauri::command]
pub fn set_tape_machine(state: TState<'_>, machine: TapeMachine, gesture: Option<u64>) -> TapeMachine {
  change_tape_machine(&state, machine, gesture)
}

#[tauri::command]
pub fn set_track_tape(state: TState<'_>, track: TrackTape, gesture: Option<u64>) -> TrackTape {
  change_track_tape(&state, track, gesture)
}
