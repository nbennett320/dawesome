// the project's tape reaching the audio engine (stage W in
// context/tape-project/02-plan.md). the settings (daw/tape_settings.rs) come
// from the Transport and Mixer tabs; the mode, where on a track the tape
// sits, and how its record side is worked out are preferences (Settings →
// Controls → Tape)
//
// - tape off, or no track on tape: no tape in the engine at all, and every
//   track plays exactly as it did before there was one (T14)
// - print mode (the default): each taped track's record side is worked out
//   ahead in the background (print.rs) whenever what it depends on changes,
//   and the engine plays the print through the playback side. until a
//   track's print is in it's heard direct, as a 4-track's input is before
//   it's on tape (W2)
// - always-live mode: the record and playback sides run on every taped
//   track, in the engine (live.rs)
//
// what was last sent is kept, so a knob or a drive only sends the knob (the
// tape's wobble carries on), and the whole tape is rebuilt only when what's
// on it changes. prints are kept here too, and sent again when it is
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::daw;
use crate::daw::tape_settings::TapeSettings;
use crate::engine::{self, tape::Print};
use crate::preferences::{TapeMethod, TapeMode, TapeTap};

pub mod live;
pub mod print;

// how often the print job looks for tracks whose prints are out of date. a
// change has to hold still for one look before it's printed (a knob being
// turned is printed once it stops)
const LOOK_EVERY: Duration = Duration::from_millis(250);

// a taped track's print, and what it was made from
struct Printed {
  content: u64,
  recipe: print::Recipe,
  // None: it can't be printed yet (an instrument, or the tape after its
  // effects: D76), or its content changed and a new one's on its way
  print: Option<Arc<Print>>,
}

struct State {
  mode: TapeMode,
  tap: TapeTap,
  method: TapeMethod,
  // the project's tape, as last applied
  settings: Option<TapeSettings>,
  // what the engine's tape was built for, if it has one
  built: Option<live::Shape>,
  // print mode's prints, by track
  prints: BTreeMap<u32, Printed>,
  // a track's (content, recipe) seen out of date once, waiting to hold
  // still for another look
  waiting: BTreeMap<u32, (u64, print::Recipe)>,
  // the speed last sent to the engine (the varispeed: the PITCH knob with
  // the tape on). an engine starts at 1, so 1 is never sent (T14)
  speed: f64,
}

static STATE: Mutex<State> = Mutex::new(State {
  mode: TapeMode::Print,
  tap: TapeTap::BeforeEffects,
  method: TapeMethod::Modeled,
  settings: None,
  built: None,
  prints: BTreeMap::new(),
  waiting: BTreeMap::new(),
  speed: 1.,
});

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
  state.speed = 1.;
  send(&mut state);
}

fn send(state: &mut State) {
  // (no output open: nothing to send; output_opened sends it when one is)
  let Some(engine) = engine::engine_if_open() else {
    state.built = None;
    state.speed = 1.;
    return;
  };
  let Some(settings) = state.settings.as_ref() else { return };

  // the song's speed, whether or not anything's on tape (the varispeed is
  // the whole song's, after the master)
  let speed = settings.machine.speed();
  if speed != state.speed {
    match engine.set_speed(speed) {
      Ok(()) => state.speed = speed,
      Err(e) => println!("the tape's speed didn't reach the engine: {}", e),
    }
  }
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
    Ok(()) => {
      // a new tape has none of the prints: they go again
      if let Some(shape) = shape.as_ref().filter(|s| !s.live) {
        for &number in &shape.tracks {
          if let Some(print) = state.prints.get(&number).and_then(|p| p.print.clone()).filter(|p| p.rate == shape.rate) {
            let _ = engine.set_tape_print(number, Some(print));
          }
        }
      }
      state.built = shape;
    }
    Err(e) => {
      println!("the tape didn't reach the engine: {}", e);
      state.built = None;
    }
  }
}

// print mode's job: a thread of its own, looking a few times a second for
// taped tracks whose prints are out of date, and printing them
pub fn keep_printing(session: Arc<daw::Session>) {
  let started = std::thread::Builder::new().name("tape prints".into()).spawn(move || loop {
    std::thread::sleep(LOOK_EVERY);
    look(&session);
  });
  if let Err(e) = started {
    println!("the tape's print job didn't start: {}", e);
  }
}

// one look: print what's out of date (one track at a time, without holding
// the state's lock while it renders)
fn look(session: &daw::Session) {
  let Some(engine) = engine::engine_if_open() else { return };
  let (shape, settings, tap) = {
    let mut state = STATE.lock().unwrap();
    let printing = state.mode == TapeMode::Print && state.settings.as_ref().is_some_and(|s| s.machine.enabled);
    match (printing, state.built.clone(), state.settings.clone()) {
      (true, Some(shape), Some(settings)) if !shape.live => (shape, settings, state.tap),
      _ => {
        // not printing: what was printed goes (it's re-made if needed)
        state.prints.clear();
        state.waiting.clear();
        return;
      }
    }
  };

  for &track in &shape.tracks {
    let recipe = print::Recipe {
      rate: shape.rate,
      drive_db: settings.track(track).drive_db,
      dbx: settings.machine.dbx,
      neural: shape.neural,
    };
    let content = print::content(session, track);
    {
      let mut state = STATE.lock().unwrap();
      let now = state.prints.get(&track).map(|p| (p.content, p.recipe.clone()));
      if now == Some((content, recipe.clone())) {
        continue;
      }
      // what it plays changed: direct at once, rather than the old
      // arrangement (only a change of sound keeps the old print: D77)
      if let Some(p) = state.prints.get_mut(&track) {
        if p.content != content && p.print.take().is_some() {
          let _ = engine.set_tape_print(track, None);
        }
      }
      // it has to hold still for a look first
      if state.waiting.get(&track) != Some(&(content, recipe.clone())) {
        state.waiting.insert(track, (content, recipe));
        continue;
      }
      state.waiting.remove(&track);
    }

    let made = if tap == TapeTap::AfterEffects {
      Err(print::NEEDS_PLUGINS.to_string())
    } else {
      let started = Instant::now();
      let cancel = AtomicBool::new(false);
      print::render_pre_tape(session, track, recipe.rate, &cancel)
        .and_then(|mix| print::make_print(&mix, &recipe, &cancel))
        .map(|print| (Arc::new(print), started.elapsed()))
    };
    let mut state = STATE.lock().unwrap();
    match made {
      Ok((print, took)) => {
        println!("tape: printed track {} in {:.2} s, {:.1} MB", track, took.as_secs_f64(), print.bytes() as f64 / 1e6);
        let _ = engine.set_tape_print(track, Some(print.clone()));
        state.prints.insert(track, Printed { content, recipe, print: Some(print) });
      }
      Err(e) => {
        if state.prints.get(&track).is_none_or(|p| p.content != content) {
          println!("tape: track {} is heard direct: {}", track, e);
        }
        let _ = engine.set_tape_print(track, None);
        state.prints.insert(track, Printed { content, recipe, print: None });
      }
    }
  }

  // tracks taken off the tape: their prints go
  let mut state = STATE.lock().unwrap();
  state.prints.retain(|track, _| shape.tracks.contains(track));
  state.waiting.retain(|track, _| shape.tracks.contains(track));
}
