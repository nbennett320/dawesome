// the scrub proxy (S1, T25–T26): the whole song rendered cheaply into
// memory, for scrubbing to read at any place and speed. minimal on purpose:
// audio clips and pattern clips' samples through each track's fader, pan,
// mute and solo and the master's fader, summed to mono, at 24 kHz, as
// 16-bit samples (about 2.9 MB a minute). no effects, no instruments or
// synths (their notes aren't sent), no master chain. rendered as a mixdown
// renders (mixdown.rs), on an engine of its own, with the soft limiter left
// in so a loud song stays inside 16 bits
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::daw;
use crate::daw::audiograph;
use crate::engine::{self, mixer, scrub::Proxy};

pub const RATE: u32 = 24_000;
// the run every voice belongs to (the engine is the render's own)
const RUN: u64 = 1;
const CHUNK: usize = 4096;
// played a little past the last clip's end, for its last samples
const AFTER_END: Duration = Duration::from_millis(100);

pub const CANCELLED: &str = "scrub proxy cancelled";

// render the proxy at `rate`. an empty playlist is an empty proxy
pub fn render(state: &daw::Session, rate: u32, cancel: &AtomicBool) -> Result<Proxy, String> {
  // the song as it is, with a mixer of its own set as the live one is now
  // (the graph's lock, then the sequencer's: always in that order)
  let mut graph = {
    let live = state.playlist.audiograph.lock().unwrap();
    let mut graph = audiograph::AudioGraph::new(live.sample_rate(), live.tempo());
    graph.nodes = live.nodes.clone();
    graph.pattern_nodes = live.pattern_nodes.clone();
    graph.sequencer = Arc::new(Mutex::new(live.sequencer.lock().unwrap().clone()));
    let mixer = mixer::Mixer::default();
    mixer.apply(&live.mixer.changed());
    graph.mixer = Arc::new(mixer);
    graph
  };
  let end = graph.end();
  if end.is_zero() {
    return Ok(Proxy { rate, samples: Box::new([]) });
  }
  let beat = graph.beat_interval();
  if beat.is_zero() {
    return Err("the tempo is zero".into());
  }

  let (engine, mut renderer) = engine::Engine::new(rate);
  let until = engine.frames_in(end + AFTER_END);
  let mut samples = Vec::with_capacity(until as usize);
  let mut out = vec![0f32; CHUNK * 2];
  let mut frame = 0u64;

  // a beat at a time, as playback sends them (see mixdown::render). no
  // notes: instruments are left out
  graph.init();
  let mut at = Duration::ZERO;
  while frame < until {
    if at < end {
      graph.set_current_offset(Some(at));
      let voices: Vec<_> = graph.run_sounds_at(at, rate, beat, false, at.is_zero())
        .into_iter()
        .map(|(delay, source, route)| engine.boxed_voice(source, Some(RUN), engine.frames_in(at + delay), route))
        .collect();
      if at.is_zero() {
        engine.start_run_at(RUN, graph.mixer.master(), engine::RunStart::default(), voices)?;
      } else {
        engine.play(voices)?;
      }
      at += beat;
    }
    let to = if at < end { engine.frames_in(at) } else { until };
    while frame < to.min(until) {
      if cancel.load(Ordering::Relaxed) {
        return Err(CANCELLED.into());
      }
      let frames = ((to.min(until) - frame) as usize).min(CHUNK);
      let chunk = &mut out[..frames * 2];
      chunk.fill(0.);
      renderer.render(chunk, std::time::Instant::now(), Duration::ZERO);
      // mono: the pan's a balance law, so a centred clip keeps its level
      samples.extend(chunk.chunks_exact(2).map(|lr| to_i16((lr[0] + lr[1]) * 0.5)));
      frame += frames as u64;
    }
  }
  graph.pause();
  Ok(Proxy { rate, samples: samples.into_boxed_slice() })
}

fn to_i16(s: f32) -> i16 {
  (s * 32_767.).round().clamp(-32_768., 32_767.) as i16
}

// what the proxy is made from, into a hash: every track's clips and
// patterns (and the sequencer), the tempo and time signature, and the
// mixer's levels (not its names or colours). when this changes, the proxy
// is out of date
pub fn content(state: &daw::Session) -> u64 {
  use std::hash::{Hash, Hasher};
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  let signature = *state.playlist.time_signature.lock().unwrap();
  format!("{:?}", signature).hash(&mut hasher);
  // (the graph's lock, then the sequencer's: always in that order)
  let graph = state.playlist.audiograph.lock().unwrap();
  graph.tempo().to_bits().hash(&mut hasher);
  for node in &graph.nodes {
    node.fingerprint(&mut hasher);
  }
  for node in &graph.pattern_nodes {
    format!("{:?}", node).hash(&mut hasher);
  }
  if !graph.pattern_nodes.is_empty() {
    format!("{:?}", graph.sequencer.lock().unwrap().saved()).hash(&mut hasher);
  }
  let mix = graph.mixer.changed();
  let levels: Vec<_> = mix.tracks.iter()
    .filter(|track| track.settings != mixer::ChannelSettings::default())
    .map(|track| (track.track_number, &track.settings))
    .collect();
  format!("{:?} {:?}", levels, mix.master).hash(&mut hasher);
  hasher.finish()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::daw::actions::clip::add_clip;
  use crate::daw::test_support::{play, KICK};

  const LONG: &str = "assets/slide_guitar.wav";

  #[test]
  fn test_the_proxy_is_the_song_as_played_summed_to_mono() {
    // a song of two tracks, one panned and turned down: the proxy is what
    // playback plays of it (no effects in it), as mono at the proxy's rate
    let state = daw::Session::default();
    add_clip(&state, LONG.into(), 1, Some(0.5)).unwrap();
    add_clip(&state, KICK.into(), 2, Some(3.0)).unwrap();
    state.playlist.mixer.set_pan(1, 0.6);
    state.playlist.mixer.set_volume_db(1, -9.);
    let proxy = render(&state, RATE, &AtomicBool::new(false)).unwrap();
    assert_eq!(proxy.rate, RATE);

    let played = play(&state, RATE, Duration::from_secs_f64(proxy.seconds()));
    assert_eq!(played.len() / 2, proxy.samples.len());
    let worst = played.chunks_exact(2).zip(proxy.samples.iter())
      .map(|(lr, &s)| ((lr[0] + lr[1]) * 0.5 - s as f32 / 32_767.).abs())
      .fold(0., f32::max);
    // (16 bits: within a step)
    assert!(worst < 2. / 32_767., "off by {worst}");
    assert!(proxy.samples.iter().any(|&s| s.abs() > 3_000));
  }

  #[test]
  fn test_a_hit_lands_on_its_frame() {
    // a kick 3 beats in (1.5 s at 120 bpm) starts on frame 36,000
    let state = daw::Session::default();
    add_clip(&state, KICK.into(), 0, Some(3.0)).unwrap();
    let proxy = render(&state, RATE, &AtomicBool::new(false)).unwrap();
    let first = proxy.samples.iter().position(|&s| s != 0).unwrap();
    assert_eq!(first, 36_000);
    assert!(proxy.samples[..first].iter().all(|&s| s == 0));
  }

  #[test]
  fn test_muted_tracks_are_left_out_and_an_empty_song_is_empty() {
    let state = daw::Session::default();
    assert!(render(&state, RATE, &AtomicBool::new(false)).unwrap().samples.is_empty());
    add_clip(&state, KICK.into(), 0, Some(0.0)).unwrap();
    state.playlist.mixer.set_mute(0, true);
    let proxy = render(&state, RATE, &AtomicBool::new(false)).unwrap();
    assert!(proxy.samples.iter().all(|&s| s == 0));
  }

  #[test]
  fn test_the_proxy_is_out_of_date_when_the_song_or_its_levels_change() {
    let state = daw::Session::default();
    add_clip(&state, KICK.into(), 1, Some(0.0)).unwrap();
    let first = content(&state);
    assert_eq!(content(&state), first);
    // a name isn't heard
    state.playlist.mixer.set_name(1, "Kick");
    assert_eq!(content(&state), first);
    // a fader is
    state.playlist.mixer.set_volume_db(1, -6.);
    let second = content(&state);
    assert_ne!(second, first);
    // and another track's clip, and the tempo
    add_clip(&state, KICK.into(), 2, Some(1.0)).unwrap();
    let third = content(&state);
    assert_ne!(third, second);
    state.playlist.audiograph.lock().unwrap().set_tempo(97.0);
    assert_ne!(content(&state), third);
  }

  #[test]
  fn test_a_cancel_stops_it() {
    let state = daw::Session::default();
    add_clip(&state, LONG.into(), 0, Some(0.0)).unwrap();
    assert_eq!(render(&state, RATE, &AtomicBool::new(true)).err().unwrap(), CANCELLED);
  }
}
