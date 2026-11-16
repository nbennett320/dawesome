// helpers the tests of more than one module share
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::daw::{self, audiograph};
use crate::engine;

// samples the tests play
pub(crate) const KICK: &str = "assets/assets_66-bd-01.wav";
pub(crate) const SNARE: &str = "assets/assets_66-sd-01.wav";

// what's heard of the playlist from its start, on an engine at `rate`, as the scheduler
// plays it: a beat at a time, through the live mixer
pub(crate) fn play(state: &daw::Session, rate: u32, total: Duration) -> Vec<f32> {
  let mut graph = state.playlist.audiograph.lock().unwrap();
  let (engine, mut renderer) = engine::Engine::new(rate);
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

// a new project's state, shared as the app shares it
pub(crate) fn state() -> Arc<daw::Session> {
  Arc::new(daw::Session::default())
}

// the clip with this id, as it is now
pub(crate) fn node(state: &daw::Session, id: u64) -> audiograph::AudioNode {
  state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().clone()
}

// the furthest apart two sounds of the same length are, sample for sample
pub(crate) fn worst(a: &[f32], b: &[f32]) -> f32 {
  assert_eq!(a.len(), b.len());
  a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0., f32::max)
}
