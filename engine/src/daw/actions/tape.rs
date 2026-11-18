// changing the project's tape (see daw/tape_settings.rs), each change
// recorded so it can be undone, as the mixer's are. commands/tape.rs is the
// Transport and Mixer tabs' side of these
//
// the setters return what was kept, after it's put in range
use crate::daw;
use crate::daw::history;
use crate::daw::tape_settings::{TapeMachine, TapeSettings, TrackTape};

pub fn tape_settings(state: &daw::Session) -> TapeSettings {
  state.tape.lock().unwrap().clone()
}

// tell the engine the tape's settings changed: after an edit, an undo or
// redo, and a project being opened or started
pub fn to_engine(state: &daw::Session) {
  daw::tape::apply(&state.tape.lock().unwrap().clone());
}

// tell the engine tracks on tape are now worked out another way, or the
// tape sits elsewhere on them (Settings → Controls → Tape). preferences,
// not part of the project: not undone or saved with it
pub fn mode_to_engine(mode: crate::preferences::TapeMode, tap: crate::preferences::TapeTap, method: crate::preferences::TapeMethod) {
  daw::tape::set_preferences(mode, tap, method);
}

// change the tape, and record the change so it can be undone
// - gesture: the knob drag it's part of, if any. one drag is one step
fn edit<T>(state: &daw::Session, gesture: Option<u64>, change: impl FnOnce(&mut TapeSettings) -> T) -> T {
  // (the history is locked on its own, after the tape's lock is let go)
  let (before, after, result) = {
    let mut tape = state.tape.lock().unwrap();
    let before = tape.clone();
    let result = change(&mut tape);
    (before, tape.clone(), result)
  };
  to_engine(state);
  history::record(state, history::Step::new().in_gesture(gesture).with_tape(before, after));
  result
}

// the switch and the machine's controls. while recording the song's speed
// stays as it is (as the tempo does): a take is fitted to the song at the
// speed it was recorded at, so the knob, and the switch when the knob's off
// its click, keep what they were
pub fn change_tape_machine(state: &daw::Session, machine: TapeMachine, gesture: Option<u64>) -> TapeMachine {
  let recording = daw::recording::is_recording(state);
  edit(state, gesture, |tape| {
    let mut machine = machine.sanitized();
    if recording && machine.speed() != tape.machine.speed() {
      machine.pitch = tape.machine.pitch;
      machine.enabled = tape.machine.enabled;
    }
    tape.machine = machine;
    tape.machine
  })
}

// whether a track is on tape, and its drive
pub fn change_track_tape(state: &daw::Session, track: TrackTape, gesture: Option<u64>) -> TrackTape {
  edit(state, gesture, |tape| {
    let track = track.sanitized();
    tape.set_track(track);
    track
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;
  use crate::daw::project;

  #[test]
  fn test_tape_changes_are_undone_and_redone() {
    let state = Arc::new(daw::Session::default());
    project::mark_unchanged(&state);

    let on = TapeMachine { enabled: true, age: 40., ..TapeMachine::default() };
    assert_eq!(change_tape_machine(&state, on, None), on);
    change_track_tape(&state, TrackTape { track_number: 2, on_tape: true, drive_db: 30. }, None);
    // kept in range
    assert_eq!(tape_settings(&state).track(2).drive_db, 18.);
    assert!(project::has_unsaved_changes(&state));

    history::undo(&state).unwrap();
    assert_eq!(tape_settings(&state).track(2), TrackTape { track_number: 2, ..TrackTape::default() });
    assert_eq!(tape_settings(&state).machine, on);
    history::undo(&state).unwrap();
    assert!(tape_settings(&state).is_default());
    assert!(!project::has_unsaved_changes(&state));

    history::redo(&state).unwrap();
    assert_eq!(tape_settings(&state).machine, on);
  }

  #[test]
  fn test_a_drag_is_one_step() {
    let state = Arc::new(daw::Session::default());
    for drive_db in [1., 2., 3.] {
      change_track_tape(&state, TrackTape { track_number: 0, on_tape: true, drive_db }, Some(7));
    }
    history::undo(&state).unwrap();
    assert!(tape_settings(&state).is_default());
    assert!(!state.history.lock().unwrap().can_undo());
  }

  #[test]
  fn test_the_speed_holds_while_recording() {
    let state = Arc::new(daw::Session::default());
    let on = TapeMachine { enabled: true, pitch: 5., ..TapeMachine::default() };
    change_tape_machine(&state, on, None);
    daw::recording::fake_session(&state);
    // the knob and the switch keep the speed; the rest still change
    let kept = change_tape_machine(&state, TapeMachine { pitch: -3., age: 20., ..on }, None);
    assert_eq!((kept.pitch, kept.age), (5., 20.));
    let kept = change_tape_machine(&state, TapeMachine { enabled: false, ..on }, None);
    assert!(kept.enabled);
    // with the knob at its click, the switch can go: the speed's 1 either way
    let state = Arc::new(daw::Session::default());
    let clicked = TapeMachine { pitch: 0., ..on };
    change_tape_machine(&state, clicked, None);
    daw::recording::fake_session(&state);
    assert_eq!(change_tape_machine(&state, on, None).pitch, 0.);
    let off = TapeMachine { enabled: false, ..clicked };
    assert_eq!(change_tape_machine(&state, off, None), off);
  }

  #[test]
  fn test_no_change_is_no_step() {
    let state = Arc::new(daw::Session::default());
    change_tape_machine(&state, TapeMachine::default(), None);
    assert!(!state.history.lock().unwrap().can_undo());
  }
}
