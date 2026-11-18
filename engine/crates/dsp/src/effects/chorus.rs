// the Chorus: copies of the sound a few ms late, each copy's lateness
// swaying with a slow LFO, so each is a touch sharp then a touch flat
// against the dry. mixed back in, the copies shimmer against it like a
// second player doubling the part
//
//   in ─┬──────────────────────────────────────────────────────────── dry ─┐
//       └─► low cut ─► line ─┬─ voice 1: read at delay·(1 + swing·LFO(φ + 0))   ─┐   │
//                            ├─ voice 2: read at delay·(1 + swing·LFO(φ + 1/n)) ─┼─► tone ─► wet ─┴─► out
//                            └─ …                                               ─┘
//
// - each side has its own line, so a stereo sound stays stereo. each
//   voice reads both, the right a little further round the LFO (Spread),
//   and is panned across the field (Spread too). one voice, at full
//   Spread, reads the right side on the opposite slope, as a pedal into
//   two amps does
// - the low cut is on what's chorused only: the bass stays put
// - no feedback: that's the Flanger
//
// the experimental three, each nothing at all at 0:
// - Bloom: on each attack the copies duck out and the swing eases off,
//   then they open back up as the note rings (0.2 to 2.5 s)
// - Drift: each voice's place in the LFO, and its delay, wander at random
//   (seeded: it's the same every time it's played)
// - Prism: each voice band-passed to its own slice of the spectrum, the
//   slices fanned from low on the left to high on the right

use crate::delay_line::{DelayLine, MIN_DELAY};
use crate::effect::{Effect, EffectInfo, Entry, Io, Values};
use crate::effects::delay::levels;
use crate::envelope::Envelope;
use crate::lfo::{Lfo, Rate, Wander, NOTES, SHAPES};
use crate::param::{ParamDesc, Taper, Unit};
use crate::readout::READOUT;
use crate::svf::{Coefs, Shape as Filter, State, TINY};
use crate::transport::Transport;

// the parameters' ids (places in PARAMS: only ever add to the end)
pub const SYNC: usize = 0;
pub const NOTE: usize = 1;
pub const RATE: usize = 2;
pub const DEPTH: usize = 3;
pub const SHAPE: usize = 4;
pub const DELAY: usize = 5;
pub const VOICES: usize = 6;
pub const SPREAD: usize = 7;
pub const LOW_CUT: usize = 8;
pub const TONE: usize = 9;
pub const MIX: usize = 10;
pub const BLOOM: usize = 11;
pub const DRIFT: usize = 12;
pub const PRISM: usize = 13;

pub const VOICE_COUNTS: &[&str] = &["1", "2", "3", "4"];
pub const MOST_VOICES: usize = 4;
const TRIANGLE: usize = 1;

pub static PARAMS: [ParamDesc; 14] = [
  ParamDesc::toggle("sync", "Sync", "Motion", false),
  ParamDesc::choice("note", "Note", "Motion", NOTES, 13),
  ParamDesc::continuous("rate", "Rate", "Motion", Unit::Hz, (0.03, 10.), 0.8, Taper::Log),
  ParamDesc::continuous("depth", "Depth", "Motion", Unit::Percent, (0., 100.), 40., Taper::Linear).bending_pitch(),
  ParamDesc::choice("shape", "Shape", "Motion", SHAPES, TRIANGLE),
  ParamDesc::continuous("delay", "Delay", "Voices", Unit::Ms, (3., 40.), 12., Taper::Log).bending_pitch(),
  ParamDesc::choice("voices", "Voices", "Voices", VOICE_COUNTS, 1),
  ParamDesc::continuous("spread", "Spread", "Voices", Unit::Percent, (0., 100.), 70., Taper::Linear).bending_pitch(),
  ParamDesc::continuous("lowcut", "Low cut", "Tone", Unit::Hz, (20., 1000.), 120., Taper::Log),
  ParamDesc::continuous("tone", "Tone", "Tone", Unit::Hz, (1000., 20_000.), 9000., Taper::Log),
  ParamDesc::continuous("mix", "Mix", "Mix", Unit::Percent, (0., 100.), 50., Taper::Linear),
  ParamDesc::continuous("bloom", "Bloom", "Experimental", Unit::Percent, (0., 100.), 0., Taper::Linear).bending_pitch(),
  ParamDesc::continuous("drift", "Drift", "Experimental", Unit::Percent, (0., 100.), 0., Taper::Linear).bending_pitch(),
  ParamDesc::continuous("prism", "Prism", "Experimental", Unit::Percent, (0., 100.), 0., Taper::Linear),
];

pub static ENTRY: Entry = Entry {
  info: EffectInfo { id: "chorus", name: "Chorus", category: "Modulation", version: 1 },
  params: &PARAMS,
  make,
};

fn make(rate: f32) -> Box<dyn Effect> {
  Box::new(Chorus::new(rate))
}

// ---- what the panel reads (Effect::readout's slots)

// the LFO's phase (0 to 1) and rate (Hz)
pub const OUT_PHASE: usize = 0;
pub const OUT_HZ: usize = 1;
// how open Bloom has the copies (1 = fully; 1 always with Bloom at 0)
pub const OUT_OPEN: usize = 2;
// how loud the input is (0 to 1, a peak follower)
pub const OUT_LEVEL: usize = 3;
// each voice's delay now, in ms: left, then right (MOST_VOICES each)
pub const OUT_DELAYS_L: usize = 4;
pub const OUT_DELAYS_R: usize = 8;
// each voice's level (0 while it's not one of the Voices)
pub const OUT_GAINS: usize = 12;
// each voice's Drift: how far it's moved round the LFO (cycles) and how
// much it's added to the delay (ms)
pub const OUT_DRIFT_PHASE: usize = 16;
pub const OUT_DRIFT_MS: usize = 20;
// how far round the LFO the right side reads, past the left (cycles)
pub const OUT_STEREO: usize = 24;

// ---- the numbers

// the line: the longest delay (40 ms, its spacing, swing and drift) and
// some to spare
const LINE: f32 = 0.08;
// Depth at 100% swings the delay this share of itself either way
const SWING: f32 = 0.45;
// the voices' delays are spaced this share of it apart, first to last
const SPACING: f32 = 0.1;
// a voice layout change (Voices, Shape, Spread's stereo offset) glides
// over this long, so nothing jumps
const SETTLE: f32 = 0.03;
// Drift at 100%: how far a voice wanders round the LFO (cycles), and
// how much its delay wanders (a share of it), and how quickly
const DRIFT_PHASE: f32 = 0.15;
const DRIFT_DELAY: f32 = 0.25;
const DRIFT_PHASE_HZ: f64 = 0.7;
const DRIFT_DELAY_HZ: f64 = 0.35;
// Prism's slices: centres spread evenly by octave from LOWEST to HIGHEST
const PRISM_LOWEST: f32 = 200.;
const PRISM_HIGHEST: f32 = 6000.;
// Bloom: what's an attack (the quick follower this far over the slow
// one, and over the floor), how fast the copies duck, and how long they
// take to open again at Bloom's least and most
const BLOOM_OVER: f32 = 1.5;
const BLOOM_FLOOR: f32 = 0.003;
const BLOOM_DUCK: f32 = 0.01;
const BLOOM_EASE: f32 = 0.1;
const BLOOM_FASTEST: f32 = 0.2;
const BLOOM_SLOWEST: f32 = 2.5;

// the share of the gap left after a sample, to cover ~63% of it in
// `seconds`
fn keep(seconds: f32, rate: f32) -> f32 {
  (-1. / (seconds * rate.max(1.))).exp()
}

// the LFO at `phase`, `triangle` of the way from a sine to a triangle
// (as lfo::Shape has them, in f32: a chorus reads it eight times a frame)
#[inline]
fn shape_at(phase: f32, triangle: f32) -> f32 {
  let p = phase - phase.floor();
  let tri = || if p < 0.5 { 4. * p - 1. } else { 3. - 4. * p };
  let sine = || -(std::f32::consts::TAU * p).cos();
  if triangle == 0. {
    sine()
  } else if triangle == 1. {
    tri()
  } else {
    let s = sine();
    s + triangle * (tri() - s)
  }
}

// nothing, if it's small enough to be decaying into denormals
#[inline]
fn clean(x: f32) -> f32 {
  if x.abs() < TINY { 0. } else { x }
}

// one-pole toward `to`, landing on it once close
#[inline]
fn glide(x: &mut f32, to: f32, keep: f32) {
  *x = to + keep * (*x - to);
  if (*x - to).abs() < 1e-6 { *x = to; }
}

// how many voices the knob says
fn voice_count(params: &Values) -> usize {
  (params.choice(VOICES) + 1).min(MOST_VOICES)
}

// where each voice is meant to be, from the knobs
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout {
  gain: f32,
  // round the LFO (cycles)
  offset: f32,
  // -1 left to 1 right
  pan: f32,
  // its delay, as a share of Delay
  scale: f32,
}

fn layouts(n: usize, spread: f32, prism: f32, now: &[Layout; MOST_VOICES]) -> [Layout; MOST_VOICES] {
  std::array::from_fn(|v| {
    if v >= n {
      // (it fades out where it is)
      return Layout { gain: 0., ..now[v] };
    }
    if n == 1 {
      return Layout { gain: 1., offset: 0., pan: 0., scale: 1. };
    }
    let along = v as f32 / (n - 1) as f32;
    // Prism fans them all the way across, whatever the Spread
    let width = spread + prism * (1. - spread);
    Layout {
      gain: 1.,
      offset: v as f32 / n as f32,
      pan: (2. * along - 1.) * width,
      scale: 1. + SPACING * (along - 0.5),
    }
  })
}

// Prism's slice for voice `v` of `n` on side `side` (one voice: low on
// the left, high on the right)
fn slice(v: usize, n: usize, side: usize) -> f32 {
  let along = if n == 1 { side as f32 } else { v as f32 / (n - 1) as f32 };
  // (one voice's two slices sit a little in from the ends)
  let along = if n == 1 { 0.15 + 0.7 * along } else { along };
  PRISM_LOWEST * (PRISM_HIGHEST / PRISM_LOWEST).powf(along)
}

struct Voice {
  now: Layout,
  phase_wander: Wander,
  delay_wander: Wander,
  // Prism's band-passes, a side each
  bands: [State; 2],
  // where it read from last (samples), and its Drift (cycles round the
  // LFO, and a share of the delay), for the panel
  delays: [f32; 2],
  wander: [f32; 2],
}

struct Side {
  line: DelayLine,
  low_cut: State,
  tone: State,
}

pub struct Chorus {
  rate: f32,
  lfo: Lfo,
  hz: f64,
  // Delay as the last block had it, in ms
  delay_ms: f32,
  voices: [Voice; MOST_VOICES],
  sides: [Side; 2],
  // the right side's lead round the LFO, and how far from a sine to a
  // triangle the shape is (0 to 1): both glide
  stereo: f32,
  triangle: f32,
  settle: f32,
  // the cuts' filters, and what they were made from
  cuts: (Coefs, Coefs),
  cuts_for: Option<(f32, f32)>,
  // Prism's filters, for each voice and side, and the voice count they're
  // for
  bands: [[Coefs; 2]; MOST_VOICES],
  bands_for: Option<usize>,
  // Bloom: a quick and a slow follower, how open the copies are (and a
  // slower copy of it for the swing), and how fast they move
  fast: Envelope,
  slow: Envelope,
  open: f32,
  open_swing: f32,
  duck: f32,
  ease: f32,
  last_open: f32,
  // it's run since it was made or reset: until then its glides start
  // where they're going
  started: bool,
}

impl Chorus {
  pub fn new(rate: f32) -> Chorus {
    let flat = Coefs::new(Filter::Bell, 1000., 0., 1., rate);
    let start = Layout { gain: 0., offset: 0., pan: 0., scale: 1. };
    let mut chorus = Chorus {
      rate,
      lfo: Lfo::new(),
      hz: 0.,
      delay_ms: 0.,
      voices: std::array::from_fn(|v| Voice {
        now: start,
        phase_wander: Wander::new(11 + v as u64),
        delay_wander: Wander::new(101 + v as u64),
        bands: [State::default(); 2],
        delays: [0.; 2],
        wander: [0.; 2],
      }),
      sides: std::array::from_fn(|_| Side {
        line: DelayLine::new(LINE, rate),
        low_cut: State::default(),
        tone: State::default(),
      }),
      stereo: 0.,
      triangle: 0.,
      settle: 0.,
      cuts: (flat, flat),
      cuts_for: None,
      bands: [[flat; 2]; MOST_VOICES],
      bands_for: None,
      fast: Envelope::new(0.0005, 0.02, rate),
      slow: Envelope::new(0.03, 0.3, rate),
      open: 1.,
      open_swing: 1.,
      duck: 0.,
      ease: 0.,
      last_open: 1.,
      started: false,
    };
    chorus.set_sample_rate(rate);
    chorus
  }
}

impl Effect for Chorus {
  fn set_sample_rate(&mut self, rate: f32) {
    self.rate = rate;
    self.settle = keep(SETTLE, rate);
    self.duck = keep(BLOOM_DUCK, rate);
    self.ease = keep(BLOOM_EASE, rate);
    self.fast.set_times(0.0005, 0.02, rate);
    self.slow.set_times(0.03, 0.3, rate);
    self.cuts_for = None;
    self.bands_for = None;
    for side in &mut self.sides { side.line.set_rate(rate); }
  }

  fn reset(&mut self) {
    for side in &mut self.sides {
      side.line.clear();
      side.low_cut.reset();
      side.tone.reset();
    }
    for voice in &mut self.voices {
      voice.phase_wander.reset();
      voice.delay_wander.reset();
      for band in &mut voice.bands { band.reset(); }
    }
    self.lfo.reset();
    self.fast.reset();
    self.slow.reset();
    self.open = 1.;
    self.open_swing = 1.;
    self.started = false;
  }

  fn process(&mut self, io: Io, params: &Values, transport: &Transport) {
    let Io { input, output, .. } = io;
    let [out_l, out_r] = output;
    let rate = self.rate;

    let n = voice_count(params);
    let spread = params.get(SPREAD) / 100.;
    let prism = params.get(PRISM) / 100.;
    let drift = params.get(DRIFT) / 100.;
    let bloom = params.get(BLOOM) / 100.;
    let swing = SWING * params.get(DEPTH) / 100.;
    self.delay_ms = params.get(DELAY);
    let base = params.get(DELAY) / 1000. * rate;
    let (dry, wet) = levels(params.get(MIX));

    let now: [Layout; MOST_VOICES] = std::array::from_fn(|v| self.voices[v].now);
    let targets = layouts(n, spread, prism, &now);
    let stereo_to = if n == 1 { 0.5 * spread } else { 0.25 * spread };
    let triangle_to = if params.choice(SHAPE) == TRIANGLE { 1. } else { 0. };
    if !std::mem::replace(&mut self.started, true) {
      for (voice, to) in self.voices.iter_mut().zip(targets) { voice.now = to; }
      self.stereo = stereo_to;
      self.triangle = triangle_to;
    }

    let cuts = (params.get(LOW_CUT), params.get(TONE));
    if self.cuts_for != Some(cuts) {
      self.cuts = (
        Coefs::new(Filter::LowCut, cuts.0, 0., 0.707, rate),
        Coefs::new(Filter::HighCut, cuts.1, 0., 0.707, rate),
      );
      self.cuts_for = Some(cuts);
    }
    if self.bands_for != Some(n) {
      let q = 0.5 + 0.15 * n.max(2) as f32;
      for v in 0..MOST_VOICES {
        for side in 0..2 {
          self.bands[v][side] = Coefs::new(Filter::BandPass, slice(v, n, side), 0., q, rate);
        }
      }
      self.bands_for = Some(n);
    }
    let (low_cut, tone) = self.cuts;
    let bands = self.bands;

    let clock = Lfo::clock(Rate::of(params, SYNC, NOTE, RATE), transport);
    self.hz = clock.hz(rate);
    let settle = self.settle;
    // (most of the way open, 90%, in that time)
    let ease = keep((BLOOM_FASTEST + bloom * (BLOOM_SLOWEST - BLOOM_FASTEST)) / 2.3, rate);
    let (duck, ease_swing) = (self.duck, self.ease);
    let phase_step = DRIFT_PHASE_HZ / rate as f64;
    let delay_step = DRIFT_DELAY_HZ / rate as f64;

    for i in 0..input[0].len() {
      let (in_l, in_r) = (input[0][i], input[1][i]);
      let phase = self.lfo.tick(&clock, i) as f32;

      // Bloom: how open the copies are, ducking at an attack
      let fast = self.fast.run(0.5 * (in_l + in_r));
      let slow = self.slow.run(0.5 * (in_l + in_r));
      let hit = if fast > BLOOM_FLOOR { ((fast / (slow + 1e-9) - BLOOM_OVER) / BLOOM_OVER).clamp(0., 1.) } else { 0. };
      let to = 1. - hit;
      self.open = if to < self.open { to + duck * (self.open - to) } else { 1. + ease * (self.open - 1.) };
      self.open_swing = self.open + ease_swing * (self.open_swing - self.open);
      let wet_open = 1. - bloom * (1. - self.open);
      let swing = swing * (1. - bloom * (1. - self.open_swing));

      glide(&mut self.stereo, stereo_to, settle);
      glide(&mut self.triangle, triangle_to, settle);
      let (stereo, triangle) = (self.stereo, self.triangle);

      let mut sum = [0f32; 2];
      let mut gains = 0.;
      for (v, voice) in self.voices.iter_mut().enumerate() {
        let to = targets[v];
        glide(&mut voice.now.gain, to.gain, settle);
        glide(&mut voice.now.offset, to.offset, settle);
        glide(&mut voice.now.pan, to.pan, settle);
        glide(&mut voice.now.scale, to.scale, settle);
        let wander_phase = drift * DRIFT_PHASE * voice.phase_wander.value();
        let wander_delay = drift * DRIFT_DELAY * voice.delay_wander.value();
        voice.phase_wander.step(phase_step);
        voice.delay_wander.step(delay_step);
        voice.wander = [wander_phase, wander_delay];
        let Layout { gain, offset, pan, scale } = voice.now;
        if gain == 0. { continue; }
        gains += gain;
        let pans = [(1. - pan).max(0.).sqrt(), (1. + pan).max(0.).sqrt()];
        for side in 0..2 {
          let lfo = shape_at(phase + offset + wander_phase + side as f32 * stereo, triangle);
          let delay = (base * (scale + wander_delay + swing * lfo)).max(MIN_DELAY as f32);
          voice.delays[side] = delay;
          let tap = self.sides[side].line.read(delay as f64);
          // (Prism's filters rest while it's at 0, and start from quiet)
          let tap = if prism == 0. { tap } else { tap + prism * (voice.bands[side].run(&bands[v][side], tap) - tap) };
          sum[side] += tap * gain * pans[side];
        }
      }
      let norm = 1. / gains.max(1.).sqrt();

      let [left, right] = &mut self.sides;
      left.line.write(left.low_cut.run(&low_cut, in_l));
      right.line.write(right.low_cut.run(&low_cut, in_r));
      // (the tone filter is after the line, so what it holds would decay
      // into denormals mid-block: it's kept clean a sample at a time)
      let wet = wet * wet_open * norm;
      let (wet_l, wet_r) = (left.tone.run(&tone, clean(sum[0])), right.tone.run(&tone, clean(sum[1])));
      left.tone.flush();
      right.tone.flush();
      out_l[i] = dry * in_l + wet * clean(wet_l);
      out_r[i] = dry * in_r + wet * clean(wet_r);
      self.last_open = wet_open;
    }

    for side in &mut self.sides {
      side.low_cut.flush();
      side.tone.flush();
    }
    for voice in &mut self.voices {
      for band in &mut voice.bands {
        if prism == 0. { band.reset() } else { band.flush() }
      }
    }
    self.fast.flush();
    self.slow.flush();
  }

  fn tail_frames(&self, params: &Values, rate: f32) -> Option<u64> {
    // the longest a copy can be, then the cuts ringing out (generously: a
    // low cut's time constant, as the Delay works it)
    let smear = 1. / (std::f32::consts::PI * params.get(LOW_CUT));
    Some(((LINE + 14. * smear + 0.05) * rate).ceil() as u64)
  }

  fn readout(&self, out: &mut [f32; READOUT]) {
    let ms = 1000. / self.rate.max(1.);
    out[OUT_PHASE] = self.lfo.phase() as f32;
    out[OUT_HZ] = self.hz as f32;
    out[OUT_OPEN] = self.last_open;
    out[OUT_LEVEL] = self.fast.level();
    for (v, voice) in self.voices.iter().enumerate() {
      out[OUT_DELAYS_L + v] = voice.delays[0] * ms;
      out[OUT_DELAYS_R + v] = voice.delays[1] * ms;
      out[OUT_GAINS + v] = voice.now.gain;
      out[OUT_DRIFT_PHASE + v] = voice.wander[0];
      out[OUT_DRIFT_MS + v] = voice.wander[1] * self.delay_ms;
    }
    out[OUT_STEREO] = self.stereo;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::runner::{Runner, Store};
  use std::sync::Arc;

  const RATE: f32 = 48_000.;

  // one voice, all wet, free at 1 Hz, sine, no spread, cuts wide open
  fn plain_store() -> Arc<Store> {
    let store = Store::new(&ENTRY);
    for (id, value) in [(SYNC, 0.), (RATE_ID, 1.), (DEPTH, 100.), (SHAPE, 0.), (DELAY, 10.), (VOICES, 0.),
      (SPREAD, 0.), (LOW_CUT, 20.), (TONE, 20_000.), (MIX, 100.)] {
      store.set_plain(id, value);
    }
    store
  }
  const RATE_ID: usize = super::RATE;

  fn transport() -> Transport {
    Transport { sample_rate: RATE as u32, ..Transport::default() }
  }

  // `input` (both sides) through it, `block` frames at a time: both sides
  // out, and the readout after each block
  fn run(store: &Arc<Store>, input: &[f32], block: usize) -> ([Vec<f32>; 2], Vec<[f32; READOUT]>) {
    let mut runner = Runner::new(store.clone(), RATE);
    run_on(&mut runner, store, input, block)
  }

  fn run_on(runner: &mut Runner, store: &Arc<Store>, input: &[f32], block: usize) -> ([Vec<f32>; 2], Vec<[f32; READOUT]>) {
    let mut out = [vec![0.; input.len()], vec![0.; input.len()]];
    let mut readouts = Vec::new();
    let [l, r] = &mut out;
    for at in (0..input.len()).step_by(block) {
      let end = (at + block).min(input.len());
      runner.process([&input[at..end], &input[at..end]], [&mut l[at..end], &mut r[at..end]], &transport());
      readouts.push(store.readout().read().0);
    }
    (out, readouts)
  }

  fn sine(frames: usize, freq: f32) -> Vec<f32> {
    (0..frames).map(|i| 0.5 * (std::f32::consts::TAU * freq * i as f32 / RATE).sin()).collect()
  }

  fn most_step(s: &[f32]) -> f32 {
    s.windows(2).fold(0f32, |m, w| m.max((w[1] - w[0]).abs()))
  }

  #[test]
  fn test_a_voice_sways_either_side_of_its_delay_at_the_lfo_rate() {
    let store = plain_store();
    let (_, readouts) = run(&store, &vec![0.; 48_000], 48);
    let delays: Vec<f32> = readouts.iter().map(|r| r[OUT_DELAYS_L]).collect();
    let (lo, hi) = delays.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &d| (lo.min(d), hi.max(d)));
    // 10 ms, swung 45% either way
    assert!((lo - 5.5).abs() < 0.05 && (hi - 14.5).abs() < 0.05, "it went from {} to {} ms", lo, hi);
    // at 1 Hz, its peak is halfway through the second (a sine starts at its
    // bottom)
    let top = delays.iter().position(|d| *d == hi).unwrap();
    assert!((top as i32 - 500).abs() <= 2, "it peaked at block {}", top);
  }

  #[test]
  fn test_voices_sit_evenly_round_the_lfo() {
    let store = plain_store();
    store.set_plain(VOICES, 3.);
    store.set_plain(SPREAD, 100.);
    let (_, readouts) = run(&store, &vec![0.; 4800], 48);
    let last = readouts.last().unwrap();
    let phase = last[OUT_PHASE] as f64;
    for v in 0..4 {
      let scale = 1. + SPACING * (v as f32 / 3. - 0.5);
      // (the phase is read after the block: each read was a frame before)
      let at = phase - 1. / RATE as f64 + v as f64 / 4.;
      let expected = 10. * (scale + SWING * crate::lfo::Shape::Sine.at(at));
      assert!((last[OUT_DELAYS_L + v] - expected).abs() < 0.01, "voice {} is at {} ms, not {}", v, last[OUT_DELAYS_L + v], expected);
      assert_eq!(last[OUT_GAINS + v], 1.);
    }
  }

  #[test]
  fn test_one_voice_at_full_spread_sways_the_other_way_on_the_right() {
    for shape in [0., 1.] {
      let store = plain_store();
      store.set_plain(SPREAD, 100.);
      store.set_plain(SHAPE, shape);
      let (_, readouts) = run(&store, &vec![0.; 48_000], 48);
      for r in &readouts {
        // the two either side of 10 ms by the same amount
        assert!((r[OUT_DELAYS_L] + r[OUT_DELAYS_R] - 20.).abs() < 0.01, "{} and {}", r[OUT_DELAYS_L], r[OUT_DELAYS_R]);
      }
    }
  }

  #[test]
  fn test_at_no_depth_all_wet_its_the_sound_late_by_the_delay() {
    let store = plain_store();
    store.set_plain(DEPTH, 0.);
    let mut click = vec![0.; 4800];
    click[0] = 1.;
    let ([l, r], _) = run(&store, &click, 512);
    let at = |s: &[f32]| s.iter().enumerate().fold((0, 0f32), |(at, m), (i, x)| if x.abs() > m { (i, x.abs()) } else { (at, m) }).0;
    // 10 ms is 480 frames
    assert_eq!(at(&l), 480);
    assert_eq!(at(&r), 480);
  }

  #[test]
  fn test_with_no_mix_its_the_dry_sound() {
    let store = plain_store();
    store.set_plain(MIX, 0.);
    let input = sine(4800, 440.);
    let ([l, r], _) = run(&store, &input, 512);
    assert_eq!(l, input);
    assert_eq!(r, input);
  }

  #[test]
  fn test_the_low_cut_keeps_the_bass_out_of_the_copies() {
    let store = plain_store();
    store.set_plain(DEPTH, 0.);
    store.set_plain(LOW_CUT, 1000.);
    let ([l, _], _) = run(&store, &sine(48_000, 50.), 512);
    let loudest = l[24_000..].iter().fold(0f32, |m, x| m.max(x.abs()));
    // 50 Hz is over four octaves under a 12 dB/oct cut: under -48 dB
    assert!(loudest < 0.5 * 0.004, "{}", loudest);
  }

  #[test]
  fn test_synced_it_sways_once_a_note_at_the_tempo() {
    let store = plain_store();
    store.set_plain(SYNC, 1.);
    store.set_plain(NOTE, NOTES.iter().position(|n| *n == "1/4").unwrap() as f32);
    let (_, readouts) = run(&store, &vec![0.; 4800], 480);
    // a quarter at 120 bpm is half a second: 2 Hz
    assert!((readouts.last().unwrap()[OUT_HZ] - 2.).abs() < 1e-4);
  }

  // the same chorus run twice, `change` done to the second before it
  // runs: whether the two sound the same, bit for bit
  fn same_after(set: impl Fn(&Store), change: impl Fn(&mut Chorus)) -> bool {
    let store = Store::new(&ENTRY);
    set(&store);
    let plains = store.plains();
    let params = Values(&plains);
    let input = sine(24_000, 220.);
    let mut input_r = input.clone();
    // (an attack halfway, for Bloom)
    for x in &mut input_r[12_000..12_100] { *x = 0.9; }
    let outs: Vec<[Vec<f32>; 2]> = (0..2).map(|n| {
      let mut chorus = Chorus::new(RATE);
      if n == 1 { change(&mut chorus); }
      let mut out = [vec![0.; input.len()], vec![0.; input.len()]];
      let [l, r] = &mut out;
      for at in (0..input.len()).step_by(500) {
        let end = at + 500;
        let io = Io { input: [&input_r[at..end], &input_r[at..end]], output: [&mut l[at..end], &mut r[at..end]], sidechain: None };
        chorus.process(io, &params, &transport());
      }
      out
    }).collect();
    outs[0] == outs[1]
  }

  #[test]
  fn test_at_0_the_experimental_three_change_nothing_at_all() {
    let four = |s: &Store| { s.set_plain(VOICES, 3.); };
    // Bloom: however open it thinks the copies are
    assert!(same_after(four, |c| { c.open = 0.1; c.open_swing = 0.3; }));
    // Drift: wherever its random lines go
    assert!(same_after(four, |c| for (v, voice) in c.voices.iter_mut().enumerate() {
      voice.phase_wander = Wander::new(900 + v as u64);
      voice.delay_wander = Wander::new(990 + v as u64);
    }));
    // Prism: whatever its slices are
    assert!(same_after(four, |c| {
      c.bands = [[Coefs::new(Filter::BandPass, 3000., 0., 4., RATE); 2]; MOST_VOICES];
      c.bands_for = Some(4);
    }));
    // (and each, turned up, does change it: the checks can fail)
    assert!(!same_after(|s| { four(s); s.set_plain(BLOOM, 50.); }, |c| { c.open = 0.1; c.open_swing = 0.3; }));
    assert!(!same_after(|s| { four(s); s.set_plain(DRIFT, 50.); }, |c| c.voices[0].phase_wander = Wander::new(900)));
  }

  #[test]
  fn test_bloom_ducks_the_copies_at_an_attack_and_opens_them_as_it_rings() {
    let store = plain_store();
    store.set_plain(BLOOM, 100.);
    store.set_plain(MIX, 50.);
    // silence, then a long note
    let mut input = vec![0.; 24_000];
    input.extend(sine(4 * 48_000, 220.));
    let (_, readouts) = run(&store, &input, 480);
    let open_at = |seconds: f32| readouts[((0.5 + seconds) * 100.) as usize][OUT_OPEN];
    assert!(open_at(0.05) < 0.2, "{}", open_at(0.05));
    assert!(open_at(1.) > open_at(0.3));
    assert!(open_at(3.5) > 0.9, "{}", open_at(3.5));
  }

  #[test]
  fn test_drift_moves_the_voices_on_their_own() {
    let store = plain_store();
    store.set_plain(VOICES, 1.);
    store.set_plain(DRIFT, 100.);
    let (_, readouts) = run(&store, &vec![0.; 10 * 48_000], 4800);
    for v in 0..2 {
      let wanders: Vec<f32> = readouts.iter().map(|r| r[OUT_DRIFT_PHASE + v]).collect();
      let (lo, hi) = wanders.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
      assert!(hi - lo > 0.05 && lo >= -DRIFT_PHASE * 1.25 && hi <= DRIFT_PHASE * 1.25, "voice {}: {} to {}", v, lo, hi);
    }
    assert_ne!(readouts[50][OUT_DRIFT_PHASE], readouts[50][OUT_DRIFT_PHASE + 1]);
  }

  #[test]
  fn test_its_lfo_shapes_are_the_lfos() {
    for i in 0..100 {
      let p = i as f32 / 100.;
      assert!((shape_at(p, 0.) - crate::lfo::Shape::Sine.at(p as f64)).abs() < 1e-5);
      assert!((shape_at(p, 1.) - crate::lfo::Shape::Triangle.at(p as f64)).abs() < 1e-5);
    }
  }

  #[test]
  fn test_prism_splits_one_voice_low_left_and_high_right() {
    let level = |freq: f32| {
      let store = plain_store();
      store.set_plain(DEPTH, 0.);
      store.set_plain(PRISM, 100.);
      let ([l, r], _) = run(&store, &sine(48_000, freq), 512);
      let peak = |s: &[f32]| s[24_000..].iter().fold(0f32, |m, x| m.max(x.abs()));
      (peak(&l), peak(&r))
    };
    let (low_l, low_r) = level(300.);
    let (high_l, high_r) = level(4000.);
    assert!(low_l > 2. * low_r, "300 Hz: {} left, {} right", low_l, low_r);
    assert!(high_r > 2. * high_l, "4 kHz: {} left, {} right", high_l, high_r);
  }

  #[test]
  fn test_changing_voices_or_shape_doesnt_click() {
    for (id, from, to) in [(VOICES, 0., 3.), (VOICES, 3., 1.), (SHAPE, 0., 1.)] {
      let store = plain_store();
      store.set_plain(MIX, 50.);
      store.set_plain(DEPTH, 40.);
      store.set_plain(SPREAD, 70.);
      store.set_plain(id, from);
      let mut runner = Runner::new(store.clone(), RATE);
      let input = sine(48_000, 110.);
      let (settled, _) = run_on(&mut runner, &store, &input, 256);
      store.set_plain(id, to);
      let (changed, _) = run_on(&mut runner, &store, &input[..4800], 256);
      let usual = most_step(&settled[0][24_000..]).max(most_step(&settled[1][24_000..]));
      let moving = most_step(&changed[0]).max(most_step(&changed[1]));
      assert!(moving < usual * 1.5, "{} from {} to {}: a step of {} where it usually moves {}", PARAMS[id].key, from, to, moving, usual);
    }
  }
}
