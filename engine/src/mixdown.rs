// Mixdown: the whole playlist, as it's heard, rendered offline
//
// rendered the way playback renders it (see run_playlist in daw.rs): a
// slice of the song's sounds a beat at a time, each clip through its
// track's strip, all of it through the master strip and the limiter, but
// on an engine of its own with no device, as fast as it can go. never the
// metronome or a count-in
//
// it renders a copy of the song, taken when it starts, with a mixer of its
// own: the song can be played and changed meanwhile, and the faders moved,
// without any of it reaching the render, or the render reaching the meters
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::daw;

// the run every voice belongs to (the engine is the render's own)
const RUN: u64 = 1;
// the most frames rendered between checks for a cancel
const CHUNK: usize = 4096;
// after the last clip ends, the mix goes on until it's been this quiet for
// this long (a hit still ringing), or for TAIL_MAX at most, then fades out
const QUIET: f32 = 3.162_278e-5; // -90 dBFS
const QUIET_FOR: Duration = Duration::from_millis(250);
const TAIL_MAX: Duration = Duration::from_secs(10);
const TAIL_FADE: Duration = Duration::from_millis(10);

pub const NOTHING_TO_MIX: &str = "There's nothing in the playlist to mix down";
pub const CANCELLED: &str = "Mixdown cancelled";

// the song as it was when the mixdown started
pub struct Snapshot {
  graph: daw::AudioGraph<'static>,
  // where the last clip ends
  end: Duration,
  // for the effects' transport
  time_signature: daw::TimeSignature,
}

impl Snapshot {
  pub fn end(&self) -> Duration {
    self.end
  }
}

// copy the song: its clips, patterns and mixer. the locks are held only
// while copying (the graph's, then the sequencer's: always in that order)
pub fn snapshot(state: &daw::InnerState) -> Result<Snapshot, String> {
  let time_signature = *state.playlist.time_signature.lock().unwrap();
  let graph = {
    let live = state.playlist.audiograph.lock().unwrap();
    let mut graph = daw::AudioGraph::new(live.sample_rate(), live.tempo(), live.max_beats());
    graph.nodes = live.nodes.clone();
    graph.pattern_nodes = live.pattern_nodes.clone();
    graph.sequencer = Arc::new(Mutex::new(live.sequencer.lock().unwrap().clone()));
    // a mixer of its own, set as the live one is now
    let mixer = daw::Mixer::default();
    mixer.apply(&live.mixer.changed());
    graph.mixer = Arc::new(mixer);
    graph
  };

  let end = graph.end();
  if end.is_zero() { return Err(NOTHING_TO_MIX.into()); }
  Ok(Snapshot { graph, end, time_signature })
}

// how far a render has got, readable from another thread while it works
#[derive(Default)]
pub struct Progress {
  // frames rendered, of the frames up to the song's end
  done: AtomicU64,
  total: AtomicU64,
}

impl Progress {
  pub fn reset(&self) {
    self.done.store(0, Ordering::Relaxed);
    self.total.store(0, Ordering::Relaxed);
  }

  // 0 to 1 (the tail isn't counted: it's short, and its length isn't known
  // until it's rendered)
  pub fn fraction(&self) -> f32 {
    let total = self.total.load(Ordering::Relaxed);
    if total == 0 { return 0.; }
    (self.done.load(Ordering::Relaxed) as f64 / total as f64).min(1.) as f32
  }
}

// render the song at `rate`, handing the mix to `sink` a chunk at a time,
// stereo and interleaved. from the playlist's start to the last clip's end,
// and the tail after it. stops with CANCELLED when `cancel` is set, or with
// the sink's error. returns how many frames it rendered
//
// (the sink doesn't know how the mix was made: a live render, played
// through the device and taken from the master, could feed it the same way)
pub fn render(
  snapshot: Snapshot,
  rate: u32,
  sink: &mut dyn FnMut(&[f32]) -> Result<(), String>,
  progress: &Progress,
  cancel: &AtomicBool,
) -> Result<u64, String> {
  let Snapshot { mut graph, end, time_signature } = snapshot;
  let beat = graph.beat_interval();
  if beat.is_zero() { return Err("the tempo is zero".into()); }

  let (engine, mut renderer) = daw::Engine::new(rate);
  let end_frame = engine.frames_in(end);
  progress.total.store(end_frame, Ordering::Relaxed);
  progress.done.store(0, Ordering::Relaxed);

  let mut out = vec![0f32; CHUNK * 2];
  let mut frame = 0u64;
  let mut render_to = |until: u64, frame: &mut u64, sink: &mut dyn FnMut(&[f32]) -> Result<(), String>| -> Result<(), String> {
    while *frame < until {
      if cancel.load(Ordering::Relaxed) { return Err(CANCELLED.into()); }
      let frames = ((until - *frame) as usize).min(CHUNK);
      let chunk = &mut out[..frames * 2];
      chunk.fill(0.);
      renderer.render(chunk, std::time::Instant::now(), Duration::ZERO);
      sink(chunk)?;
      *frame += frames as u64;
      progress.done.store(*frame, Ordering::Relaxed);
    }
    Ok(())
  };

  // a beat at a time, as the scheduler sends them: each slice's sounds are
  // sent before the frames they start in are rendered, and only a beat's
  // worth wait to start at once
  graph.init();
  let mut at = Duration::ZERO;
  while at < end {
    graph.set_current_offset(Some(at));
    let voices: Vec<_> = graph.run_sounds_at(at, rate, beat, false, at.is_zero())
      .into_iter()
      .map(|(delay, source, route)| engine.boxed_voice(source, Some(RUN), engine.frames_in(at + delay), route))
      .collect();
    if at.is_zero() {
      // the mix starts at the song's start
      let start = daw::RunStart {
        tempo: graph.tempo() as f64,
        beats_per_bar: time_signature.numerator,
        beat_unit: time_signature.denominator,
        origin: 0,
      };
      engine.start_run_at(RUN, graph.mixer.master(), start, voices)?;
    } else {
      engine.play(voices)?;
    }
    at += beat;
    render_to(engine.frames_in(at.min(end)), &mut frame, sink)?;
  }
  graph.pause();

  // the tail: held back until it's known how much of it to keep, which is
  // up to its last frame that isn't quiet
  let quiet_for = engine.frames_in(QUIET_FOR) as usize;
  let tail_max = engine.frames_in(TAIL_MAX) as usize;
  let mut tail: Vec<f32> = Vec::new();
  let mut loud_to = 0; // frames of the tail up to its last loud one
  let mut tail_frames = 0usize;
  while tail_frames < tail_max && tail_frames - loud_to < quiet_for {
    let step = (CHUNK.min(tail_max - tail_frames)) as u64;
    render_to(frame + step, &mut frame, &mut |chunk: &[f32]| {
      for (i, pair) in chunk.chunks_exact(2).enumerate() {
        if pair[0].abs() > QUIET || pair[1].abs() > QUIET { loud_to = tail.len() / 2 + i + 1; }
      }
      tail.extend_from_slice(chunk);
      Ok(())
    })?;
    tail_frames += step as usize;
  }

  let keep = loud_to.min(tail_frames);
  let tail = &mut tail[..keep * 2];
  // cut off while still sounding: faded, so it doesn't click
  if keep == tail_max {
    let fade = (engine.frames_in(TAIL_FADE) as usize).min(keep);
    for n in 0..fade {
      let gain = n as f32 / fade as f32;
      let at = (keep - 1 - n) * 2;
      tail[at] *= gain;
      tail[at + 1] *= gain;
    }
  }
  if !tail.is_empty() { sink(tail)?; }
  Ok(end_frame + keep as u64)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Instant;
  use crate::event_dispatch::add_clip;

  const KICK: &str = "assets/assets_66-bd-01.wav";
  const LONG: &str = "assets/slide_guitar.wav";

  fn mixdown(state: &daw::InnerState, rate: u32) -> Vec<f32> {
    let mut samples = Vec::new();
    render(snapshot(state).unwrap(), rate, &mut |chunk| { samples.extend_from_slice(chunk); Ok(()) },
      &Progress::default(), &AtomicBool::new(false)).unwrap();
    samples
  }

  // the playlist as the scheduler plays it, through the live mixer, `total`
  // long (see the same in bounce.rs)
  fn play(state: &daw::InnerState, rate: u32, total: Duration) -> Vec<f32> {
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

  fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0., |m, s| m.max(s.abs()))
  }

  fn worst(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0., f32::max)
  }

  // a pattern of kicks and a long sample, on track `track`, from `beat`
  fn pattern_clip(state: &daw::InnerState, track: u32, at: Duration) -> u64 {
    let pattern = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let kick = sequencer.add_row(KICK).unwrap();
      let long = sequencer.add_row(LONG).unwrap();
      let pattern = sequencer.add_pattern(1, 4).unwrap();
      for (row, step) in [(kick, 0), (kick, 6), (long, 15)] {
        sequencer.set_step(pattern, row, step, true).unwrap();
      }
      sequencer.set_row_volume(long, 0.3).unwrap();
      pattern
    };
    state.playlist.audiograph.lock().unwrap()
      .add_pattern_node::<daw::timing::Note>(pattern, at, track, None).unwrap()
  }

  #[test]
  fn test_a_mixdown_is_what_playback_plays() {
    for (tempo, rate) in [(120., 48_000), (87.3, 44_100), (90., 96_000)] {
      let state = daw::InnerState::default();
      state.playlist.audiograph.lock().unwrap().set_tempo(tempo);
      add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
      add_clip(&state, LONG.into(), 1, Some(1.37)).unwrap();
      pattern_clip(&state, 2, Duration::from_nanos(1_234_567_891));
      state.playlist.mixer.set_volume_db(1, -6.);
      state.playlist.mixer.set_pan(2, 0.4);
      state.playlist.mixer.set_master_volume_db(-3.);

      let mixed = mixdown(&state, rate);
      let played = play(&state, rate, Duration::from_secs_f64(mixed.len() as f64 / 2. / rate as f64));
      assert_eq!(mixed.len(), played.len());
      assert!(peak(&mixed) > 0.1);
      // (the same voices, summed in another order: as near as f32 gets)
      let off = worst(&mixed, &played);
      assert!(off < 1e-5, "off by {} at {} bpm, {}Hz", off, tempo, rate);
    }
  }

  #[test]
  fn test_mute_and_solo_are_heard_as_set() {
    let state = daw::InnerState::default();
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    add_clip(&state, LONG.into(), 1, Some(0.)).unwrap();
    let both = mixdown(&state, 48_000);

    state.playlist.mixer.set_mute(1, true);
    let kick_only = mixdown(&state, 48_000);
    state.playlist.mixer.set_mute(1, false);
    state.playlist.mixer.set_solo(0, true);
    let soloed = mixdown(&state, 48_000);
    assert_eq!(kick_only, soloed);
    assert_ne!(kick_only, both);

    // the kick is shorter than the long sample: past it, there's nothing
    let kick_len = {
      let graph = state.playlist.audiograph.lock().unwrap();
      daw::run_frame(graph.nodes.iter().find(|n| n.track_number == 0).unwrap().duration().as_nanos() as i128, 48_000) as usize
    };
    assert_eq!(peak(&soloed[(kick_len + 48) * 2..]), 0.);
    assert!(peak(&both[(kick_len + 48) * 2..]) > 0.01);
  }

  #[test]
  fn test_the_metronome_is_never_mixed_down() {
    let state = daw::InnerState::default();
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    state.playlist.mixer.set_mute(0, true);
    state.metronome_enabled.store(true, Ordering::SeqCst);
    let mixed = mixdown(&state, 48_000);
    assert!(!mixed.is_empty());
    assert_eq!(peak(&mixed), 0.);
  }

  #[test]
  fn test_the_master_fader_turns_it_down() {
    let state = daw::InnerState::default();
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    // (quiet to start with, so the limiter isn't in play)
    state.playlist.mixer.set_volume_db(0, -12.);
    let full = mixdown(&state, 48_000);
    state.playlist.mixer.set_master_volume_db(-6.);
    let down = mixdown(&state, 48_000);
    let ratio = peak(&down) / peak(&full);
    assert!((ratio - daw::db_to_gain(-6.)).abs() < 0.01, "{}", ratio);
  }

  #[test]
  fn test_it_runs_to_the_last_clips_end_and_a_ringing_hit_is_kept() {
    let state = daw::InnerState::default();
    // a pattern whose last hit, the long sample, rings well past its end
    pattern_clip(&state, 0, Duration::ZERO);
    let snap = snapshot(&state).unwrap();
    let end = daw::run_frame(snap.end().as_nanos() as i128, 48_000) as usize;
    let mixed = mixdown(&state, 48_000);
    let frames = mixed.len() / 2;
    assert!(frames > end + 48_000, "{} frames for a song {} long", frames, end);
    // ending on its last sound, not on silence
    assert!(peak(&mixed[mixed.len() - 2..]) > 0.);

    // a clip that ends in silence ends at its end
    let state = daw::InnerState::default();
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    let end = daw::run_frame(snapshot(&state).unwrap().end().as_nanos() as i128, 48_000) as usize;
    assert!(mixdown(&state, 48_000).len() / 2 <= end + 1);
  }

  #[test]
  fn test_an_empty_playlist_is_refused() {
    let state = daw::InnerState::default();
    assert_eq!(snapshot(&state).err().unwrap(), NOTHING_TO_MIX);
  }

  #[test]
  fn test_changes_after_it_starts_dont_reach_it_and_it_doesnt_reach_the_meters() {
    let state = daw::InnerState::default();
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    let before = mixdown(&state, 48_000);
    state.playlist.mixer.take_levels();

    let snap = snapshot(&state).unwrap();
    state.playlist.mixer.set_mute(0, true);
    state.playlist.audiograph.lock().unwrap().set_tempo(60.);
    let mut samples = Vec::new();
    render(snap, 48_000, &mut |chunk| { samples.extend_from_slice(chunk); Ok(()) },
      &Progress::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(samples, before);

    let levels = state.playlist.mixer.take_levels();
    assert_eq!(levels.master, 0.);
    assert!(levels.tracks.iter().all(|&(_, peak)| peak == 0.));
    // and the live song isn't left running
    assert!(!state.playlist.audiograph.lock().unwrap().running);
  }

  #[test]
  fn test_a_cancel_stops_it_and_progress_is_reported() {
    let state = daw::InnerState::default();
    add_clip(&state, LONG.into(), 0, Some(0.)).unwrap();
    let progress = Progress::default();
    let cancel = AtomicBool::new(false);
    let mut chunks = 0;
    let result = render(snapshot(&state).unwrap(), 48_000, &mut |_| {
      chunks += 1;
      if chunks == 3 { cancel.store(true, Ordering::Relaxed); }
      Ok(())
    }, &progress, &cancel);
    assert_eq!(result.err().unwrap(), CANCELLED);
    assert!(progress.fraction() > 0. && progress.fraction() < 1.);

    let progress = Progress::default();
    render(snapshot(&state).unwrap(), 48_000, &mut |_| Ok(()), &progress, &AtomicBool::new(false)).unwrap();
    assert_eq!(progress.fraction(), 1.);
  }

  #[test]
  fn test_more_hits_than_the_engine_holds_at_once_are_all_heard() {
    // 1100 kicks a fifth of a beat apart: more than MAX_VOICES over the
    // song (the scheduler only ever has a beat's worth waiting), though
    // only a few sound at once
    let state = daw::InnerState::default();
    state.playlist.ui.lock().unwrap().snap_enabled = false;
    let count = 1100;
    for n in 0..count {
      add_clip(&state, KICK.into(), 0, Some(n as f32 * 0.2)).unwrap();
    }
    let mixed = mixdown(&state, 48_000);
    let starts: Vec<usize> = state.playlist.audiograph.lock().unwrap().nodes.iter()
      .map(|node| daw::run_frame(node.start_offset.as_nanos() as i128, 48_000) as usize)
      .collect();
    assert_eq!(starts.len(), count);
    // each kick is heard where it starts: the mix jumps there
    let silent = starts.iter().filter(|&&at| peak(&mixed[at * 2..(at + 100) * 2]) < 0.01).count();
    assert_eq!(silent, 0);
  }
}
