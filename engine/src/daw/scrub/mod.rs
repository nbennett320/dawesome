// scrubbing (part S in context/tape-project/02-plan.md, T25–T26, T31):
// dragging the playhead, or the Transport tab's reels, plays the song at the
// drag's speed, either way, as a tape rocked over the heads; holding Rewind
// or Fast forward plays it as it winds. normal playback pauses meanwhile,
// and a scrub source in the engine reads the scrub proxy (proxy.rs): a cheap
// render of the song, kept here and made again in the background whenever
// the song changes. it's made whether or not the tape is on: scrubbing isn't
// the tape's
//
// a scrub is held from begin() to end(): normal playback stops at once
// (no tape coast) and, if the playlist was playing, carries on at once from
// where the scrub's let go. a pattern playing on its own stops, and isn't
// picked up again. not while recording
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::daw::{self, playback};
use crate::engine::{self, scrub::Proxy};

pub mod proxy;

pub const NOT_WHILE_RECORDING: &str = "The song can't be scrubbed while recording";

// a scrub being held: whether the playlist was playing when it began
static HELD: Mutex<Option<bool>> = Mutex::new(None);

// begin scrubbing at `at` on the timeline: following a drag (`wind` None),
// or winding at a speed (multiples of play speed, negative back)
pub fn begin(state: &Arc<daw::Session>, at: Duration, wind: Option<f64>) -> Result<(), String> {
  if daw::recording::is_recording(state) {
    return Err(NOT_WHILE_RECORDING.into());
  }
  let engine = engine::engine()?;
  let playing = state.playlist.playing_playlist();
  if state.playlist.playing() {
    playback::pause_playlist(state);
  }
  // (nothing to read until the proxy's first made: a silent scrub)
  let proxy = current().unwrap_or_else(|| Arc::new(Proxy { rate: proxy::RATE, samples: Box::new([]) }));
  // with the tape on, its wow and flutter waver the scrub too (S4)
  let tape = state.tape.lock().unwrap().clone();
  let warble = tape.machine.enabled.then(|| {
    let settings = daw::tape::live::machine_settings(&tape);
    (settings.transport, settings.wow_flutter)
  });
  engine.scrub_start(proxy, at.as_secs_f64(), wind, warble)?;
  // (a scrub begun over another keeps the first's playing)
  let mut held = HELD.lock().unwrap();
  if held.is_none() { *held = Some(playing); }
  Ok(())
}

// the drag's moved to `at` on the timeline (or, winding, the speed's
// changed to `at` seconds a second)
pub fn drive(value: f64) {
  if let Some(engine) = engine::engine_if_open() {
    engine.scrub_drive(value);
  }
}

pub fn drag(at: Duration) {
  drive(at.as_secs_f64());
}

// let go, leaving the playhead at `beat`: playing carries on from there if
// the playlist was playing when the scrub began. returns the beat it's at
pub fn end(state: &Arc<daw::Session>, beat: f64) -> f64 {
  let playing = HELD.lock().unwrap().take().unwrap_or(false);
  if let Some(engine) = engine::engine_if_open() {
    let _ = engine.scrub_end();
  }
  let beat = playback::seek_playlist(state, beat);
  if playing && !state.playlist.playing() {
    playback::resume_playlist(state);
  }
  beat
}

// where a wind has got to, in seconds of the song (None with none going)
pub fn wound_to() -> Option<f64> {
  engine::engine_if_open().and_then(|engine| engine.scrub_at())
}

// how often the job looks for a song that's changed. a change has to hold
// still for one look before it's rendered (a fader being dragged is
// rendered once it stops)
const LOOK_EVERY: Duration = Duration::from_millis(250);

struct State {
  // the proxy, and what it was made from. the old one is kept until a new
  // one's in: a moment out of date beats silence
  made: Option<(u64, Arc<Proxy>)>,
  // a change seen once, waiting to hold still for another look
  waiting: Option<u64>,
}

static STATE: Mutex<State> = Mutex::new(State { made: None, waiting: None });

// the proxy as it is now, if there's one
pub fn current() -> Option<Arc<Proxy>> {
  STATE.lock().unwrap().made.as_ref().map(|(_, proxy)| proxy.clone())
}

// the job: a thread of its own, looking a few times a second for a song
// that's changed since its proxy was made
pub fn keep_making(session: Arc<daw::Session>) {
  let started = std::thread::Builder::new().name("scrub proxy".into()).spawn(move || loop {
    std::thread::sleep(LOOK_EVERY);
    look(&session);
  });
  if let Err(e) = started {
    println!("the scrub proxy's job didn't start: {}", e);
  }
}

// one look: make the proxy again if the song's changed and held still
fn look(session: &daw::Session) {
  let content = proxy::content(session);
  {
    let mut state = STATE.lock().unwrap();
    if state.made.as_ref().is_some_and(|(made, _)| *made == content) {
      state.waiting = None;
      return;
    }
    if state.waiting != Some(content) {
      state.waiting = Some(content);
      return;
    }
    state.waiting = None;
  }

  let started = Instant::now();
  match proxy::render(session, proxy::RATE, &AtomicBool::new(false)) {
    Ok(made) => {
      println!(
        "scrub: proxy of {:.1} s made in {:.2} s, {:.1} MB",
        made.seconds(), started.elapsed().as_secs_f64(), made.bytes() as f64 / 1e6);
      STATE.lock().unwrap().made = Some((content, Arc::new(made)));
    }
    Err(e) => println!("scrub: the proxy couldn't be made: {}", e),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_no_scrubbing_while_recording() {
    let state = Arc::new(daw::Session::default());
    daw::recording::fake_session(&state);
    assert_eq!(begin(&state, Duration::ZERO, None).err().unwrap(), NOT_WHILE_RECORDING);
    assert!(HELD.lock().unwrap().is_none());
  }
}
