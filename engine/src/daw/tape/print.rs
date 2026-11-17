// print mode (T23): a taped track's record side worked out ahead, in the
// background, so playback only runs the playback side (engine/tape.rs).
// remove this file (and its uses in mod.rs) to drop the mode: see
// context/tape-project/08-mode-removal.md
//
// a print is made in three steps:
// 1. the track rendered as playback sums it, up to where it meets the tape:
//    its clips (and its patterns' samples) before its strip, on an engine
//    of its own, as a mixdown renders (mixdown.rs), with nothing else
// 2. only the stretches with sound kept (a minute of stereo is 23 MB)
// 3. each stretch through the record side: dbx encode, then the core
//
// for now only tracks whose sound is all samples, with the tape before
// their effects: a track playing an instrument, or with the tape after its
// effects, needs second copies of its plugins (as a mixdown makes) and is
// heard direct until that's built (D76)
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::daw;
use crate::daw::audiograph;
use crate::engine::{self, mixer, tape::{Print, Region}};
use tape::RecordSide;

// the run every voice belongs to (the engine is the render's own)
const RUN: u64 = 1;
const CHUNK: usize = 4096;
// what's quieter than this (-120 dbfs) is silence, for finding the
// stretches with sound
const SILENT: f32 = 1e-6;
// a stretch is looked at in blocks of this long
const BLOCK_SECONDS: f64 = 0.1;
// stretches closer than this are one (the tape keeps running across a
// short gap), and each runs on this long after its sound, so the record
// side's filters ring out on the print
const JOIN_SECONDS: f64 = 1.0;
const PAD_SECONDS: f64 = 0.5;
// played a little past the last clip's end, for its last samples
const AFTER_END: Duration = Duration::from_millis(100);

pub const CANCELLED: &str = "print cancelled";

// why a track can't be printed yet: it's heard direct
pub const NEEDS_PLUGINS: &str = "a track playing an instrument can't be printed yet";

// 1. the track's sound before its strip, stereo interleaved, from the
// song's start, at `rate`. Err(NEEDS_PLUGINS) for a track that plays an
// instrument
pub fn render_pre_tape(state: &daw::Session, track: u32, rate: u32, cancel: &AtomicBool) -> Result<Vec<f32>, String> {
  // the track alone: its clips and patterns, the sequencer as it is, and a
  // mixer of its own at unity (the strip comes after the tape)
  let mut graph = {
    let live = state.playlist.audiograph.lock().unwrap();
    let mut graph = audiograph::AudioGraph::new(live.sample_rate(), live.tempo());
    graph.nodes = live.nodes.iter().filter(|n| n.track_number == track).cloned().collect();
    graph.pattern_nodes = live.pattern_nodes.iter().filter(|n| n.track_number == track).cloned().collect();
    graph.sequencer = Arc::new(Mutex::new(live.sequencer.lock().unwrap().clone()));
    graph.mixer = Arc::new(mixer::Mixer::default());
    graph
  };
  if graph.synth_tracks().iter().any(|&(_, t)| t == track) {
    return Err(NEEDS_PLUGINS.into());
  }
  let end = graph.end();
  if end.is_zero() {
    return Ok(Vec::new());
  }
  let beat = graph.beat_interval();
  if beat.is_zero() {
    return Err("the tempo is zero".into());
  }

  let (engine, mut renderer) = engine::Engine::new(rate);
  renderer.without_limiter();
  let until = engine.frames_in(end + AFTER_END);
  let mut mix = Vec::with_capacity(until as usize * 2);
  let mut out = vec![0f32; CHUNK * 2];
  let mut frame = 0u64;

  // a beat at a time, as playback sends them (see mixdown::render)
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
      mix.extend_from_slice(chunk);
      frame += frames as u64;
    }
  }
  graph.pause();
  Ok(mix)
}

// 2. where `mix` (stereo interleaved) has sound: (first frame, frames),
// padded, and joined across short gaps
pub fn stretches(mix: &[f32], rate: u32) -> Vec<(usize, usize)> {
  let frames = mix.len() / 2;
  let block = ((BLOCK_SECONDS * rate as f64) as usize).max(1);
  let (join, pad) = ((JOIN_SECONDS * rate as f64) as usize, (PAD_SECONDS * rate as f64) as usize);
  let mut found: Vec<(usize, usize)> = Vec::new();
  for start in (0..frames).step_by(block) {
    let end = (start + block).min(frames);
    if !mix[start * 2..end * 2].iter().any(|s| s.abs() > SILENT) {
      continue;
    }
    match found.last_mut() {
      Some(last) if start <= last.1 + join => last.1 = end,
      _ => found.push((start, end)),
    }
  }
  found.into_iter().map(|(a, b)| (a.saturating_sub(pad), (b + pad).min(frames))).fold(Vec::new(), |mut joined: Vec<(usize, usize)>, (a, b)| {
    match joined.last_mut() {
      Some(last) if a <= last.1 => last.1 = last.1.max(b),
      _ => joined.push((a, b)),
    }
    joined
  }).into_iter().map(|(a, b)| (a, b - a)).collect()
}

// what a print is made with: the record side's settings
#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
  pub rate: u32,
  pub drive_db: f32,
  pub dbx: bool,
  pub neural: bool,
}

// 3. the stretches with sound through the record side: the print
pub fn make_print(mix: &[f32], recipe: &Recipe, cancel: &AtomicBool) -> Result<Print, String> {
  let rate = recipe.rate as f32;
  let mut regions = Vec::new();
  for (start, frames) in stretches(mix, recipe.rate) {
    let mut sides = [Vec::with_capacity(frames), Vec::with_capacity(frames)];
    for (side, out) in sides.iter_mut().enumerate() {
      if cancel.load(Ordering::Relaxed) {
        return Err(CANCELLED.into());
      }
      // each stretch from silence (they're a second or more apart)
      let mut record = RecordSide::new(super::live::core(recipe.neural, rate), rate, CHUNK);
      let input: Vec<f32> = mix[start * 2..(start + frames) * 2].iter().skip(side).step_by(2).copied().collect();
      out.resize(frames, 0.0);
      record.process(&input, recipe.drive_db, recipe.dbx, out);
    }
    regions.push(Region { start: start as i64, sides });
  }
  Ok(Print { rate: recipe.rate, regions })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::daw::actions::clip::add_clip;
  use crate::daw::test_support::{play, KICK};

  const LONG: &str = "assets/slide_guitar.wav";

  fn worst(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0., f32::max)
  }

  #[test]
  fn test_a_track_s_render_is_what_playback_plays_of_it_before_its_strip() {
    // advisor's check: the print's input must be the track as heard, up to
    // where it meets the tape. a song with two tracks, the one printed
    // panned and turned down (its strip comes after the tape: no effect)
    let rate = 48_000;
    let state = daw::Session::default();
    add_clip(&state, LONG.into(), 1, Some(0.5)).unwrap();
    add_clip(&state, KICK.into(), 1, Some(3.0)).unwrap();
    add_clip(&state, KICK.into(), 2, Some(0.0)).unwrap();
    state.playlist.mixer.set_pan(1, 0.6);
    state.playlist.mixer.set_volume_db(1, -9.);
    let rendered = render_pre_tape(&state, 1, rate, &AtomicBool::new(false)).unwrap();

    // the same clips alone, at unity, played as playback plays
    let alone = daw::Session::default();
    add_clip(&alone, LONG.into(), 1, Some(0.5)).unwrap();
    add_clip(&alone, KICK.into(), 1, Some(3.0)).unwrap();
    let played = play(&alone, rate, Duration::from_secs_f64(rendered.len() as f64 / 2. / rate as f64));
    assert_eq!(rendered.len(), played.len());
    assert!(rendered.iter().any(|&s| s.abs() > 0.05));
    // (playback's mix goes through the soft limiter at the end; the print
    // leaves it out, so the tape sees the track as it is. limited the same
    // way, it's what playback plays)
    let limited: Vec<f32> = rendered.iter().map(|&s| mixer::soft_limit(s)).collect();
    let off = worst(&limited, &played);
    assert!(off < 1e-5, "off by {off}");
    assert!(rendered.iter().any(|&s| s.abs() > 1.0), "loud enough to show the limiter's left out");
  }

  #[test]
  fn test_an_empty_track_renders_nothing() {
    let state = daw::Session::default();
    add_clip(&state, KICK.into(), 2, Some(0.0)).unwrap();
    assert!(render_pre_tape(&state, 1, 48_000, &AtomicBool::new(false)).unwrap().is_empty());
  }

  #[test]
  fn test_only_the_stretches_with_sound_are_kept() {
    let rate = 1_000;
    // sound at 1-2 s and 2.5-3 s (one stretch: the gap is short), and at
    // 8-9 s (another)
    let mut mix = vec![0.0; 10 * rate * 2];
    for (a, b) in [(1000, 2000), (2500, 3000), (8000, 9000)] {
      for f in a..b {
        mix[f * 2] = 0.5;
      }
    }
    let found = stretches(&mix, rate as u32);
    assert_eq!(found, vec![(500, 3000), (7500, 2000)]);
  }

  #[test]
  fn test_a_print_is_the_record_side_on_each_stretch() {
    let rate = 48_000;
    let mut mix = vec![0.0; 6 * rate * 2];
    for f in rate..2 * rate {
      mix[f * 2] = (f as f32 * 0.01).sin() * 0.1;
      mix[f * 2 + 1] = mix[f * 2];
    }
    let recipe = Recipe { rate: rate as u32, drive_db: 6.0, dbx: true, neural: false };
    let print = make_print(&mix, &recipe, &AtomicBool::new(false)).unwrap();
    assert_eq!(print.regions.len(), 1);
    let region = &print.regions[0];
    assert_eq!(region.start, (rate / 2) as i64);
    // driven 6 db: louder than what went in, and not just scaled
    let peak = |x: &[f32]| x.iter().fold(0f32, |m, s| m.max(s.abs()));
    assert!(peak(&region.sides[0]) > 0.15);
    assert_eq!(region.sides[0], region.sides[1]);
  }
}
