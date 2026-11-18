// scrubbing (part S in context/tape-project/02-plan.md, T25–T26, T31):
// dragging the playhead, or the Transport tab's reels, plays the song at the
// drag's speed, either way, as a tape rocked over the heads; holding Rewind
// or Fast forward plays it as it winds. normal playback pauses meanwhile,
// and a scrub source in the engine reads the scrub proxy (proxy.rs): a cheap
// render of the song, kept here and made again in the background whenever
// the song changes. it's made whether or not the tape is on: scrubbing isn't
// the tape's
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::daw;
use crate::engine::scrub::Proxy;

pub mod proxy;

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
