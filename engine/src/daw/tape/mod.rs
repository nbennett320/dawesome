// the project's tape reaching the audio engine (stage W in
// context/tape-project/02-plan.md). the settings (daw/tape_settings.rs) come
// from the Transport and Mixer tabs; the mode and where on a track the tape
// sits are preferences (Settings → Controls → Tape)
//
// - tape off, or no track on tape: no tape in the engine at all, and every
//   track plays exactly as it did before there was one (T14)
// - print mode (the default): a track is heard direct until its print is
//   ready, as a 4-track's input is before it's on tape (W2). the print job
//   isn't built yet, so for now print mode is direct throughout
// - always-live mode: the record and playback sides run on every taped
//   track, in the engine (live.rs)
//
// what was last sent is kept, so a knob or a drive only sends the knob (the
// tape's wobble carries on), and the whole tape is rebuilt only when what's
// on it changes
use std::sync::Mutex;

use crate::daw::tape_settings::TapeSettings;
use crate::engine;
use crate::preferences::{TapeMethod, TapeMode, TapeTap};

pub mod live;

struct State {
  mode: TapeMode,
  tap: TapeTap,
  method: TapeMethod,
  // the project's tape, as last applied
  settings: Option<TapeSettings>,
  // what the engine's live tape was built for, if it has one
  built: Option<live::Shape>,
}

static STATE: Mutex<State> = Mutex::new(State { mode: TapeMode::Print, tap: TapeTap::BeforeEffects, method: TapeMethod::Modeled, settings: None, built: None });

// the project's tape changed (an edit, undo or redo, a project opened)
pub fn apply(settings: &TapeSettings) {
  let mut state = STATE.lock().unwrap();
  state.settings = Some(settings.clone());
  send(&mut state);
}

// the preferences changed, or were read at startup
pub fn set_preferences(mode: TapeMode, tap: TapeTap, method: TapeMethod) {
  let mut state = STATE.lock().unwrap();
  state.mode = mode;
  state.tap = tap;
  state.method = method;
  send(&mut state);
}

// a new output opened (the audio device changed): it has no tape yet
pub fn output_opened() {
  let mut state = STATE.lock().unwrap();
  state.built = None;
  send(&mut state);
}

fn send(state: &mut State) {
  // (no output open: nothing to send; output_opened sends it when one is)
  let Some(engine) = engine::engine_if_open() else {
    state.built = None;
    return;
  };
  let Some(settings) = state.settings.as_ref() else { return };
  let shape = live::shape(settings, state.mode, state.tap, state.method, engine.sample_rate());

  if shape == state.built {
    // the same tape: only its knobs and the drives
    if let Some(shape) = &shape {
      let _ = engine.set_tape_settings(live::machine_settings(settings));
      for &number in &shape.tracks {
        let _ = engine.set_tape_drive(number, settings.track(number).drive_db);
      }
    }
    return;
  }
  let tape = shape.as_ref().map(|shape| live::build(shape, settings));
  match engine.set_tape(tape) {
    Ok(()) => state.built = shape,
    Err(e) => {
      println!("the tape didn't reach the engine: {}", e);
      state.built = None;
    }
  }
}
