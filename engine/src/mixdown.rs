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
//
// the effects and synths too: every effect that's switched on gets a second
// instance of its plugin for the mixdown, and every synth one for each track
// that plays it, loaded at the mixdown's rate with the live one's settings
// copied over (a synth's master copy's), so playback carries on with the
// live ones. one that can't be loaded again fails the mixdown rather
// than being left out. the second instances are torn down (on the main
// thread) when the render's done. pattern clips' notes are sent to them a
// beat at a time, as playback sends them. a plugin loaded just now is given
// time to set itself up before its settings are copied, and the copies are
// given that time too, played silence, before the render starts (see SETTLE
// in effects.rs)
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
// the plugins' copies are played silence a step at a time, about as fast as
// they're heard, while they set themselves up (see SETTLE in effects.rs)
const SETTLE_STEP: Duration = Duration::from_millis(10);

pub const NOTHING_TO_MIX: &str = "There's nothing in the playlist to mix down";
pub const CANCELLED: &str = "Mixdown cancelled";

// the song as it was when the mixdown started
pub struct Snapshot {
  graph: daw::AudioGraph<'static>,
  // where the last clip ends
  end: Duration,
  // for the effects' transport
  time_signature: daw::TimeSignature,
  // second instances of the effects, for the render's own engine
  effects: crate::effects::MixdownEffects,
}

impl Snapshot {
  #[cfg(test)]
  pub fn end(&self) -> Duration {
    self.end
  }
}

// copy the song: its clips, patterns, mixer and effects, for a render at
// `rate`. the locks are held only while copying (the graph's, then the
// sequencer's: always in that order), and not while the effects are loaded
pub fn snapshot(state: &daw::InnerState, rate: u32) -> Result<Snapshot, String> {
  let time_signature = *state.playlist.time_signature.lock().unwrap();
  let graph = {
    let live = state.playlist.audiograph.lock().unwrap();
    let mut graph = daw::AudioGraph::new(live.sample_rate(), live.tempo());
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
  // (the live copies in step too, though the mixdown's take the master
  // copies' settings)
  state.playlist.effects.push_synth_states(true);
  let effects = state.playlist.effects.mixdown_chains(rate, &graph.synth_tracks())?;
  Ok(Snapshot { graph, end, time_signature, effects })
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

// render the song at `rate` (the one it was snapshot for), handing the mix to `sink` a chunk at a time,
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
  // (declared first, so the plugins go after the engine that runs them)
  let Snapshot { mut graph, end, time_signature, effects } = snapshot;
  let crate::effects::MixdownEffects { chains, copies, plugins: _plugins, settle } = effects;
  let beat = graph.beat_interval();
  if beat.is_zero() { return Err("the tempo is zero".into()); }

  let (engine, mut renderer) = daw::Engine::new(rate);
  for (channel, slots) in chains {
    match channel {
      daw::Channel::Track(n) => engine.set_track_chain(&graph.mixer.track_route(n), slots)?,
      daw::Channel::Master => engine.set_master_chain(slots)?,
    }
  }
  let end_frame = engine.frames_in(end);
  progress.total.store(end_frame, Ordering::Relaxed);
  progress.done.store(0, Ordering::Relaxed);

  // the copies have only just been loaded, and some can't play yet: they
  // finish setting up on the main run loop, some only once they've been
  // played. so they're played silence for a while first, before the run
  // starts (its frames are counted from its start), and what they make of
  // it is thrown away
  let settling = std::time::Instant::now();
  let mut silence = vec![0f32; engine.frames_in(SETTLE_STEP) as usize * 2];
  while settling.elapsed() < settle {
    if cancel.load(Ordering::Relaxed) { return Err(CANCELLED.into()); }
    silence.fill(0.);
    renderer.render(&mut silence, std::time::Instant::now(), Duration::ZERO);
    std::thread::sleep(SETTLE_STEP);
  }

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
    // (a mixdown plays the song once through: no loop to cut notes at)
    let notes = graph.run_notes(beat, None);
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
    daw::play_slice_notes(&engine, RUN, at, notes, &copies)?;

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
mod tests;
