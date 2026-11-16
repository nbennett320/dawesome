// helpers the tests of more than one module share
use std::time::{Duration, Instant};

use crate::daw;

// what's heard of the playlist from its start, on an engine at `rate`, as the scheduler
// plays it: a beat at a time, through the live mixer
pub(crate) fn play(state: &daw::InnerState, rate: u32, total: Duration) -> Vec<f32> {
  let mut graph = state.playlist.audiograph.lock().unwrap();
  let (engine, mut renderer) = daw::Engine::new(rate);
  let beat = graph.beat_interval();
  graph.init();
  let mut voices = Vec::new();
  let mut at = Duration::ZERO;
  while at < total {
    graph.set_current_offset(Some(at));
    for (delay, source, route) in graph.run_sounds_at(at, rate, beat, false, at.is_zero()) {
      voices.push(engine.boxed_voice(source, Some(1), engine.frames_in(at + delay), route));
    }
    at += beat;
  }
  graph.pause();
  engine.start_run(1, graph.mixer.master(), voices).unwrap();
  let mut out = vec![0f32; engine.frames_in(total) as usize * 2];
  renderer.render(&mut out, Instant::now(), Duration::ZERO);
  out
}
