// the Phaser: the sound through a chain of all-passes (allpass.rs), mixed
// back with itself. each pair of stages turns one frequency half a cycle
// round, and there the two cancel: a notch. the LFO sweeps the stages'
// frequency, so the notches sweep up and down: the swirl. fed back, the
// peaks between them ring, the wet, vocal sound of a phaser turned up
//
//   in ─┬─────────────────────────────────────────── dry ─┐
//       └─► [ + ] ─► AP₁ ─► AP₂ ─► … ─► APₙ ─┬──────── wet ─┴─► mix
//             ▲                              │
//             └────── ± feedback ◄───────────┘
//
// - the sweep's in octaves round Center (2.5 either way at full Depth)
// - Vibe mode is a Uni-Vibe's throb: four stages at widely spread
//   frequencies (its four capacitors, all different), swept by a lamp
//   (quick to brighten, slow to fade) instead of the LFO's own shape
// - a change of Stages or Mode doesn't jump: a stage that's going glides
//   its frequency up to near the top of the spectrum, where it passes
//   everything nearly untouched, and is then left out (and a new one
//   comes in from there)
//
// the experimental three, each nothing at all at 0:
// - Melt: each stage drifts off on its own (a random line each, up to an
//   octave), so the notches stop moving in step and smear
// - Barberpole: two more chains, their frequencies sliding one way and
//   wrapping round, faded in and out a half apart: the notches rise (or
//   fall) for ever
// - Envelope: playing harder pushes the notches up (or down), up to three
//   octaves: the funky side

use crate::allpass::{Allpass, Coef};
use crate::effect::{Effect, EffectInfo, Entry, Io, Values};
use crate::envelope::Envelope;
use crate::lfo::{Lfo, Rate, Shape, Wander, NOTES, SHAPES};
use crate::param::{ParamDesc, Taper, Unit};
use crate::readout::READOUT;
use crate::svf::{MAX_SHARE, TINY};
use crate::transport::Transport;

pub const SYNC: usize = 0;
pub const NOTE: usize = 1;
pub const RATE: usize = 2;
pub const DEPTH: usize = 3;
pub const SHAPE: usize = 4;
pub const CENTER: usize = 5;
pub const STAGES: usize = 6;
pub const FEEDBACK: usize = 7;
pub const MODE: usize = 8;
pub const SPREAD: usize = 9;
pub const MIX: usize = 10;
pub const MELT: usize = 11;
pub const BARBERPOLE: usize = 12;
pub const ENVELOPE: usize = 13;

pub const STAGE_COUNTS: &[&str] = &["2", "4", "6", "8", "10", "12"];
pub const MOST_STAGES: usize = 12;
pub const MODES: &[&str] = &["Classic", "Vibe"];
pub const VIBE: usize = 1;
pub const MOST_FEEDBACK: f32 = 95.;

pub static PARAMS: [ParamDesc; 14] = [
  ParamDesc::toggle("sync", "Sync", "Sweep", false),
  ParamDesc::choice("note", "Note", "Sweep", NOTES, 13),
  ParamDesc::continuous("rate", "Rate", "Sweep", Unit::Hz, (0.01, 10.), 0.5, Taper::Log),
  ParamDesc::continuous("depth", "Depth", "Sweep", Unit::Percent, (0., 100.), 80., Taper::Linear),
  ParamDesc::choice("shape", "Shape", "Sweep", SHAPES, 0),
  ParamDesc::continuous("center", "Center", "Notches", Unit::Hz, (100., 4000.), 700., Taper::Log),
  ParamDesc::choice("stages", "Stages", "Notches", STAGE_COUNTS, 2),
  ParamDesc::continuous("feedback", "Feedback", "Notches", Unit::Percent, (-MOST_FEEDBACK, MOST_FEEDBACK), 50., Taper::Linear),
  ParamDesc::choice("mode", "Mode", "Notches", MODES, 0),
  ParamDesc::continuous("spread", "Spread", "Stereo", Unit::Percent, (0., 100.), 50., Taper::Linear),
  ParamDesc::continuous("mix", "Mix", "Mix", Unit::Percent, (0., 100.), 50., Taper::Linear),
  ParamDesc::continuous("melt", "Melt", "Experimental", Unit::Percent, (0., 100.), 0., Taper::Linear),
  ParamDesc::continuous("barberpole", "Barberpole", "Experimental", Unit::Percent, (-100., 100.), 0., Taper::Linear),
  ParamDesc::continuous("envelope", "Envelope", "Experimental", Unit::Percent, (-100., 100.), 0., Taper::Linear),
];

pub static ENTRY: Entry = Entry {
  info: EffectInfo { id: "phaser", name: "Phaser", category: "Modulation", version: 1 },
  params: &PARAMS,
  make,
};

fn make(rate: f32) -> Box<dyn Effect> {
  Box::new(Phaser::new(rate))
}

// ---- what the panel reads (Effect::readout's slots)

pub const OUT_PHASE: usize = 0;
pub const OUT_HZ: usize = 1;
// the sweep's frequency now (Center, swept and pushed), left and right
pub const OUT_FREQ_L: usize = 2;
pub const OUT_FREQ_R: usize = 3;
// Barberpole's slide (0 to 1: the two chains are half apart)
pub const OUT_BARBER: usize = 4;
// how far Envelope has pushed the notches (octaves, up is more)
pub const OUT_PUSH: usize = 5;
// how far into Vibe mode it is (0 to 1)
pub const OUT_VIBE: usize = 6;
// each stage's own offset from the sweep's frequency (octaves: Vibe's
// spread and Melt's drift), MOST_STAGES of them
pub const OUT_OFFSETS: usize = 7;
// and how present each is (1 in, 0 out: see `stage_freq`)
pub const OUT_PRESENT: usize = 19;

// ---- the numbers

// octaves either way of Center at full Depth, and Envelope's most push
pub const SWEEP: f32 = 2.5;
const PUSH: f32 = 3.;
// Melt's most drift (octaves), and how fast its lines move
const MELT_MOST: f32 = 1.;
const MELT_HZ: f64 = 0.5;
// Barberpole's least range (octaves either way), so it moves at no Depth
const BARBER_LEAST: f32 = 1.5;
pub const BARBER_CHAINS: usize = 2;
// Vibe's four stages, as shares of the sweep's frequency (in octaves): a
// Uni-Vibe's capacitors spread them over six octaves or so
pub const VIBE_SPREAD: [f32; 4] = [-3.8, -1.7, 0.5, 2.7];
pub const VIBE_STAGES: usize = 4;
// the lowest a stage goes
const LOWEST: f32 = 10.;
const SETTLE: f32 = 0.04;
const ENV_ATTACK: f32 = 0.005;
const ENV_RELEASE: f32 = 0.15;
const ENV_SMOOTH: f32 = 0.015;

fn keep(seconds: f32, rate: f32) -> f32 {
  (-1. / (seconds * rate.max(1.))).exp()
}

#[inline]
fn glide(x: &mut f32, to: f32, keep: f32) {
  *x = to + keep * (*x - to);
  if (*x - to).abs() < 1e-6 { *x = to; }
}

#[inline]
fn clean(x: f32) -> f32 {
  if x.abs() < TINY { 0. } else { x }
}

// a stage's frequency (Hz): the sweep's `octaves` (log2 Hz) plus its own
// `offset`, then as far toward the top as it's absent (`present` 0 is the
// top: there it passes nearly everything untouched)
#[inline]
pub fn stage_freq(octaves: f32, offset: f32, present: f32, rate: f32) -> f32 {
  let top = (MAX_SHARE * rate).log2();
  let at = (octaves + offset).clamp(LOWEST.log2(), top);
  (at + (1. - present) * (top - at)).exp2()
}

// the LFO at `phase`: `triangle` of the way from a sine to a triangle,
// then `lamp` of the way to the lamp
#[inline]
fn shape_at(phase: f32, triangle: f32, lamp: f32) -> f32 {
  let p = phase - phase.floor();
  let tri = if p < 0.5 { 4. * p - 1. } else { 3. - 4. * p };
  let mut x = if triangle == 1. {
    tri
  } else {
    let sine = -(std::f32::consts::TAU * p).cos();
    sine + triangle * (tri - sine)
  };
  if lamp != 0. { x += lamp * (Shape::Lamp.at(p as f64) - x); }
  x
}

// Barberpole's chain at `u` (0 to 1 through its slide): its offset from
// Center (octaves), and its level (a Hann window: the two, half apart,
// always add to 1)
#[inline]
pub fn barber_chain(u: f32, range: f32, up: bool) -> (f32, f32) {
  let along = if up { 2. * u - 1. } else { 1. - 2. * u };
  let s = (std::f32::consts::PI * u).sin();
  (range * along, s * s)
}

// how many stages, and their offsets, the knobs say
fn stage_targets(params: &Values) -> ([f32; MOST_STAGES], [f32; MOST_STAGES]) {
  let vibe = params.choice(MODE) == VIBE;
  let n = if vibe { VIBE_STAGES } else { 2 * (params.choice(STAGES) + 1) };
  let present = std::array::from_fn(|k| if k < n { 1. } else { 0. });
  let offsets = std::array::from_fn(|k| if vibe && k < VIBE_STAGES { VIBE_SPREAD[k] } else { 0. });
  (present, offsets)
}

// how often the stages' frequencies are worked out, in frames (counted
// from the start, so it's the same however the sound is cut into blocks).
// a sweep moves too slowly for steps this small to be heard
const CONTROL: u64 = 8;

// a chain of stages, one side's
#[derive(Clone, Copy, Default)]
struct Chain {
  stages: [Allpass; MOST_STAGES],
  // each stage's coefficients, as last worked out (none while it's out)
  coefs: [Option<Coef>; MOST_STAGES],
}

impl Chain {
  // `x` through every stage that's present, each at its frequency
  // (`octaves` + its offset), worked out again if `update`. stages at the
  // same frequency share it, so a plain chain works out one
  #[inline]
  fn run(&mut self, x: f32, update: bool, octaves: f32, offsets: &[f32; MOST_STAGES], present: &[f32; MOST_STAGES], rate: f32) -> f32 {
    let mut y = x;
    let mut last: Option<(f32, f32, Coef)> = None;
    for k in 0..MOST_STAGES {
      if present[k] == 0. {
        self.stages[k].rest(y);
        self.coefs[k] = None;
        continue;
      }
      if update || self.coefs[k].is_none() {
        self.coefs[k] = Some(match last {
          Some((o, p, c)) if o == offsets[k] && p == present[k] => c,
          _ => {
            let c = Coef::new(stage_freq(octaves, offsets[k], present[k], rate), rate);
            last = Some((offsets[k], present[k], c));
            c
          }
        });
      }
      let coef = self.coefs[k].unwrap_or_else(|| Coef::new(1000., rate));
      y = self.stages[k].run(&coef, y);
    }
    y
  }

  fn reset(&mut self) {
    for s in &mut self.stages { s.reset(); }
    self.coefs = [None; MOST_STAGES];
  }

  fn flush(&mut self) {
    for s in &mut self.stages { s.flush(); }
  }
}

struct Side {
  // the chain the LFO sweeps, then Barberpole's two
  chains: [Chain; 1 + BARBER_CHAINS],
  // what came out last, for feedback
  back: f32,
  // the sweep's frequency last (Hz), for the panel
  freq: f32,
}

pub struct Phaser {
  rate: f32,
  lfo: Lfo,
  hz: f64,
  sides: [Side; 2],
  barber: f64,
  // gliding: each stage's offset and presence, the triangle's and the
  // lamp's shares of the shape
  offsets: [f32; MOST_STAGES],
  present: [f32; MOST_STAGES],
  triangle: f32,
  lamp: f32,
  settle: f32,
  // Melt's lines, a stage each, and what they come to now (octaves)
  melts: [Wander; MOST_STAGES],
  melted: [f32; MOST_STAGES],
  env: Envelope,
  push: f32,
  push_keep: f32,
  // frames run since it was made or reset (for CONTROL)
  frames: u64,
  started: bool,
}

impl Phaser {
  pub fn new(rate: f32) -> Phaser {
    let mut phaser = Phaser {
      rate,
      lfo: Lfo::new(),
      hz: 0.,
      sides: std::array::from_fn(|_| Side { chains: [Chain::default(); 1 + BARBER_CHAINS], back: 0., freq: 0. }),
      barber: 0.,
      offsets: [0.; MOST_STAGES],
      present: [0.; MOST_STAGES],
      triangle: 0.,
      lamp: 0.,
      settle: 0.,
      melts: std::array::from_fn(|k| Wander::new(301 + k as u64)),
      melted: [0.; MOST_STAGES],
      env: Envelope::new(ENV_ATTACK, ENV_RELEASE, rate),
      push: 0.,
      push_keep: 0.,
      frames: 0,
      started: false,
    };
    phaser.set_sample_rate(rate);
    phaser
  }
}

impl Effect for Phaser {
  fn set_sample_rate(&mut self, rate: f32) {
    self.rate = rate;
    self.settle = keep(SETTLE, rate);
    self.push_keep = keep(ENV_SMOOTH, rate);
    self.env.set_times(ENV_ATTACK, ENV_RELEASE, rate);
  }

  fn reset(&mut self) {
    for side in &mut self.sides {
      for chain in &mut side.chains { chain.reset(); }
      side.back = 0.;
    }
    for melt in &mut self.melts { melt.reset(); }
    self.lfo.reset();
    self.barber = 0.;
    self.env.reset();
    self.push = 0.;
    self.frames = 0;
    self.started = false;
  }

  fn process(&mut self, io: Io, params: &Values, transport: &Transport) {
    let Io { input, output, .. } = io;
    let [out_l, out_r] = output;
    let rate = self.rate;

    let center = params.get(CENTER).log2();
    let depth = params.get(DEPTH) / 100.;
    let feedback = params.get(FEEDBACK) / 100.;
    // (what goes into the loop is trimmed as feedback rises, so its peaks
    // don't run far over the dry)
    let trim = 1. - 0.5 * feedback.abs();
    let spread = 0.5 * params.get(SPREAD) / 100.;
    let mix = (params.get(MIX) / 100.).clamp(0., 1.);
    let melt = MELT_MOST * params.get(MELT) / 100.;
    let barber = params.get(BARBERPOLE) / 100.;
    let (barber_amount, up) = (barber.abs(), barber > 0.);
    let barber_range = (SWEEP * depth).max(BARBER_LEAST);
    let envelope = params.get(ENVELOPE) / 100.;

    let (present_to, offsets_to) = stage_targets(params);
    let triangle_to = if params.choice(SHAPE) == 1 { 1. } else { 0. };
    let lamp_to = if params.choice(MODE) == VIBE { 1. } else { 0. };
    if !std::mem::replace(&mut self.started, true) {
      self.present = present_to;
      self.offsets = offsets_to;
      self.triangle = triangle_to;
      self.lamp = lamp_to;
    }

    let clock = Lfo::clock(Rate::of(params, SYNC, NOTE, RATE), transport);
    self.hz = clock.hz(rate);
    let (settle, push_keep) = (self.settle, self.push_keep);
    // (the stages' layout only counts when their frequencies are worked
    // out, so it moves on then, CONTROL frames at a time)
    let melt_step = MELT_HZ * CONTROL as f64 / rate as f64;
    let stage_settle = settle.powi(CONTROL as i32);
    let mut offsets = std::array::from_fn(|k| self.offsets[k] + self.melted[k]);

    for i in 0..input[0].len() {
      let ins = [input[0][i], input[1][i]];
      let phase = self.lfo.tick(&clock, i) as f32;
      let update = self.frames % CONTROL == 0;
      self.frames += 1;
      let barber_at = self.barber as f32;
      self.barber += clock.step;
      self.barber -= self.barber.floor();

      glide(&mut self.triangle, triangle_to, settle);
      glide(&mut self.lamp, lamp_to, settle);
      if update {
        for k in 0..MOST_STAGES {
          glide(&mut self.present[k], present_to[k], stage_settle);
          glide(&mut self.offsets[k], offsets_to[k], stage_settle);
          self.melted[k] = melt * self.melts[k].value();
          self.melts[k].step(melt_step);
          offsets[k] = self.offsets[k] + self.melted[k];
        }
      }
      let present = self.present;
      let level = self.env.run(0.5 * (ins[0] + ins[1]));
      let push = PUSH * envelope * (2. * level).min(1.);
      self.push = push + push_keep * (self.push - push);

      let outs: [f32; 2] = std::array::from_fn(|s| {
        let side = &mut self.sides[s];
        let lfo = shape_at(phase + s as f32 * spread, self.triangle, self.lamp);
        let octaves = center + SWEEP * depth * lfo + self.push;
        side.freq = octaves.exp2();
        let x = trim * ins[s] + feedback * side.back;
        let mut wet = side.chains[0].run(x, update, octaves, &offsets, &present, rate);
        if barber_amount > 0. {
          let mut sum = 0.;
          for c in 0..BARBER_CHAINS {
            let u = barber_at + s as f32 * spread + c as f32 / BARBER_CHAINS as f32;
            let (oct, level) = barber_chain(u - u.floor(), barber_range, up);
            sum += level * side.chains[1 + c].run(x, update, center + oct + self.push, &offsets, &present, rate);
          }
          wet += barber_amount * (sum - wet);
        }
        // (kept clean a sample at a time: the stages' memory decays into
        // denormals mid-block otherwise)
        let wet = clean(wet);
        side.back = wet;
        ins[s] + mix * (wet - ins[s])
      });
      out_l[i] = outs[0];
      out_r[i] = outs[1];
    }

    for side in &mut self.sides {
      for (c, chain) in side.chains.iter_mut().enumerate() {
        // (Barberpole's rest while it's off, and start from quiet)
        if c > 0 && barber_amount == 0. { chain.reset() } else { chain.flush() }
      }
    }
    self.env.flush();
  }

  fn tail_frames(&self, params: &Values, rate: f32) -> Option<u64> {
    // a pass round the loop takes about the chain's delay at its lowest
    // stage (an all-pass delays low sounds by up to 1/(pi f)), and it's
    // 90 dB down after as many passes as the Delay works out
    let feedback = (params.get(FEEDBACK) / 100.).abs();
    let passes = if feedback <= 0. {
      1.
    } else {
      1. + ((3.2e-5 * (1. - feedback)).ln() / feedback.ln()).ceil().max(0.)
    };
    let vibe = if params.choice(MODE) == VIBE { -VIBE_SPREAD[0] } else { 0. };
    let melt = MELT_MOST * 1.25 * params.get(MELT) / 100.;
    let reach = SWEEP.max(BARBER_LEAST) + PUSH * (params.get(ENVELOPE) / 100.).abs() + melt + vibe;
    let lowest = (params.get(CENTER).log2() - reach).exp2().max(LOWEST);
    let pass = MOST_STAGES as f32 / (std::f32::consts::PI * lowest);
    Some(((passes * pass + 0.05) * rate).ceil() as u64)
  }

  fn readout(&self, out: &mut [f32; READOUT]) {
    out[OUT_PHASE] = self.lfo.phase() as f32;
    out[OUT_HZ] = self.hz as f32;
    out[OUT_FREQ_L] = self.sides[0].freq;
    out[OUT_FREQ_R] = self.sides[1].freq;
    out[OUT_BARBER] = self.barber as f32;
    out[OUT_PUSH] = self.push;
    out[OUT_VIBE] = self.lamp;
    for k in 0..MOST_STAGES {
      out[OUT_OFFSETS + k] = self.offsets[k] + self.melted[k];
      out[OUT_PRESENT + k] = self.present[k];
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::runner::{Runner, Store};
  use std::sync::Arc;

  const SR: f32 = 48_000.;

  // two stages (one notch) at 700 Hz, no sweep, no feedback, half and half
  fn plain_store() -> Arc<Store> {
    let store = Store::new(&ENTRY);
    for (id, value) in [(SYNC, 0.), (RATE, 1.), (DEPTH, 0.), (SHAPE, 0.), (CENTER, 700.), (STAGES, 0.),
      (FEEDBACK, 0.), (MODE, 0.), (SPREAD, 0.), (MIX, 50.)] {
      store.set_plain(id, value);
    }
    store
  }

  fn transport() -> Transport {
    Transport { sample_rate: SR as u32, ..Transport::default() }
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

  fn run(store: &Arc<Store>, input: &[f32], block: usize) -> ([Vec<f32>; 2], Vec<[f32; READOUT]>) {
    run_on(&mut Runner::new(store.clone(), SR), store, input, block)
  }

  fn sine(frames: usize, freq: f32) -> Vec<f32> {
    (0..frames).map(|i| 0.5 * (std::f32::consts::TAU * freq * i as f32 / SR).sin()).collect()
  }

  fn peak(s: &[f32]) -> f32 {
    s.iter().fold(0f32, |m, x| m.max(x.abs()))
  }

  fn most_step(s: &[f32]) -> f32 {
    s.windows(2).fold(0f32, |m, w| m.max((w[1] - w[0]).abs()))
  }

  fn level(store: &Arc<Store>, freq: f32) -> f32 {
    let ([l, _], _) = run(store, &sine(24_000, freq), 512);
    peak(&l[12_000..]) / 0.5
  }

  #[test]
  fn test_two_stages_cut_a_notch_at_their_frequency() {
    let store = plain_store();
    assert!(level(&store, 700.) < 0.01, "{}", level(&store, 700.));
    // far from it, the two add (an octave and more away, mostly)
    assert!(level(&store, 60.) > 0.95);
    assert!(level(&store, 12_000.) > 0.9);
  }

  #[test]
  fn test_four_stages_cut_two_notches_where_the_maths_says() {
    let store = plain_store();
    store.set_plain(STAGES, 1.);
    // each stage turns these an eighth and three eighths of a cycle: four
    // of them, half a cycle and one and a half
    let coef = Coef::new(700., SR);
    let at = |turn: f32| {
      // (phase falls from 0 to -pi as the frequency rises: search)
      (0..4000).map(|i| 20. * 1000f32.powf(i as f32 / 4000.))
        .min_by(|a, b| (coef.phase(*a, SR) + turn).abs().total_cmp(&(coef.phase(*b, SR) + turn).abs())).unwrap()
    };
    for turn in [std::f32::consts::FRAC_PI_4, 3. * std::f32::consts::FRAC_PI_4] {
      let freq = at(turn);
      assert!(level(&store, freq) < 0.02, "{} Hz is at {}", freq, level(&store, freq));
    }
  }

  #[test]
  fn test_a_stage_thats_out_sits_at_the_top_and_passes_nearly_everything() {
    assert!((stage_freq(700f32.log2(), 0., 1., SR) - 700.).abs() < 1e-2);
    assert!((stage_freq(700f32.log2(), 1., 1., SR) - 1400.).abs() < 1e-1);
    let top = stage_freq(700f32.log2(), 0., 0., SR);
    assert!((top - MAX_SHARE * SR).abs() < 1.);
    // up there it turns 1 kHz less than a hundredth of a cycle
    assert!(Coef::new(top, SR).phase(1000., SR).abs() < 0.01 * std::f32::consts::TAU);
  }

  #[test]
  fn test_the_sweep_goes_two_and_a_half_octaves_either_side_of_center() {
    let store = plain_store();
    store.set_plain(DEPTH, 100.);
    let (_, readouts) = run(&store, &vec![0.; 48_000], 48);
    let (lo, hi) = readouts.iter().fold((f32::MAX, f32::MIN), |(lo, hi), r| (lo.min(r[OUT_FREQ_L]), hi.max(r[OUT_FREQ_L])));
    let reach = 2f32.powf(SWEEP);
    assert!((lo / (700. / reach) - 1.).abs() < 0.01 && (hi / (700. * reach) - 1.).abs() < 0.01, "{} to {} Hz", lo, hi);
  }

  #[test]
  fn test_vibe_spreads_four_stages_and_leaves_the_rest_out() {
    let store = plain_store();
    store.set_plain(STAGES, 5.);
    store.set_plain(MODE, VIBE as f32);
    let (_, readouts) = run(&store, &vec![0.; 9600], 480);
    let last = readouts.last().unwrap();
    for k in 0..MOST_STAGES {
      let (offset, present) = (last[OUT_OFFSETS + k], last[OUT_PRESENT + k]);
      if k < VIBE_STAGES {
        assert_eq!((offset, present), (VIBE_SPREAD[k], 1.));
      } else {
        assert_eq!(present, 0.);
      }
    }
    assert_eq!(last[OUT_VIBE], 1.);
  }

  #[test]
  fn test_barberpole_chains_always_add_up_the_same() {
    for i in 0..200 {
      let u = i as f32 / 200.;
      let total: f32 = (0..BARBER_CHAINS).map(|c| {
        let v = u + c as f32 / BARBER_CHAINS as f32;
        barber_chain(v - v.floor(), 1., true).1
      }).sum();
      assert!((total - 1.).abs() < 1e-4);
    }
    // up, the notches rise
    assert!(barber_chain(0.9, 2., true).0 > barber_chain(0.1, 2., true).0);
  }

  #[test]
  fn test_barberpole_sweeps_on_without_clicking() {
    let store = plain_store();
    store.set_plain(STAGES, 3.);
    store.set_plain(DEPTH, 60.);
    store.set_plain(FEEDBACK, 60.);
    store.set_plain(RATE, 0.5);
    let input = sine(96_000, 110.);
    let ([still, _], _) = run(&store, &input, 512);
    store.set_plain(BARBERPOLE, -100.);
    let ([barber, _], _) = run(&store, &input, 512);
    assert!(barber.iter().all(|x| x.is_finite()) && peak(&barber) < 4.);
    assert!(most_step(&barber[4800..]) < 2. * most_step(&still[4800..]), "{} vs {}", most_step(&barber[4800..]), most_step(&still[4800..]));
  }

  #[test]
  fn test_envelope_pushes_the_notches_up_as_it_plays_harder() {
    let store = plain_store();
    store.set_plain(ENVELOPE, 100.);
    let mut input = vec![0.; 4800];
    input.extend(sine(9600, 220.).iter().map(|x| x * 2.));
    let (_, readouts) = run(&store, &input, 480);
    assert!((readouts[5][OUT_FREQ_L] - 700.).abs() < 0.1);
    let pushed = readouts.last().unwrap();
    assert!(pushed[OUT_PUSH] > 2.9, "{}", pushed[OUT_PUSH]);
    assert!(pushed[OUT_FREQ_L] > 700. * 7.5, "{}", pushed[OUT_FREQ_L]);
  }

  #[test]
  fn test_melt_moves_each_stage_on_its_own() {
    let store = plain_store();
    store.set_plain(STAGES, 5.);
    store.set_plain(MELT, 100.);
    let (_, readouts) = run(&store, &vec![0.; 5 * 48_000], 4800);
    let last = readouts.last().unwrap();
    let offsets: Vec<f32> = (0..MOST_STAGES).map(|k| last[OUT_OFFSETS + k]).collect();
    assert!(offsets.iter().all(|o| o.abs() <= MELT_MOST * 1.25));
    let (lo, hi) = offsets.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &o| (lo.min(o), hi.max(o)));
    assert!(hi - lo > 0.2, "they're all within {} to {}", lo, hi);
  }

  fn same_after(set: impl Fn(&Store), change: impl Fn(&mut Phaser)) -> bool {
    let store = Store::new(&ENTRY);
    set(&store);
    let plains = store.plains();
    let params = Values(&plains);
    let mut input = sine(24_000, 220.);
    for x in &mut input[12_000..12_100] { *x = 0.9; }
    let outs: Vec<[Vec<f32>; 2]> = (0..2).map(|n| {
      let mut phaser = Phaser::new(SR);
      if n == 1 { change(&mut phaser); }
      let mut out = [vec![0.; input.len()], vec![0.; input.len()]];
      let [l, r] = &mut out;
      for at in (0..input.len()).step_by(500) {
        let end = at + 500;
        let io = Io { input: [&input[at..end], &input[at..end]], output: [&mut l[at..end], &mut r[at..end]], sidechain: None };
        phaser.process(io, &params, &transport());
      }
      out
    }).collect();
    outs[0] == outs[1]
  }

  #[test]
  fn test_at_0_the_experimental_three_change_nothing_at_all() {
    let defaults = |_: &Store| {};
    let reseed = |p: &mut Phaser| for (k, m) in p.melts.iter_mut().enumerate() { *m = Wander::new(700 + k as u64); };
    assert!(same_after(defaults, reseed));
    assert!(same_after(defaults, |p| p.barber = 0.61));
    assert!(same_after(defaults, |p| { for _ in 0..1000 { p.env.run(1.); } }));
    assert!(!same_after(|s| { s.set_plain(MELT, 50.); }, reseed));
    assert!(!same_after(|s| { s.set_plain(BARBERPOLE, 50.); }, |p| p.barber = 0.61));
    assert!(!same_after(|s| { s.set_plain(ENVELOPE, 50.); }, |p| { for _ in 0..1000 { p.env.run(1.); } }));
  }

  #[test]
  fn test_changing_stages_mode_or_shape_doesnt_click() {
    for (id, from, to) in [(STAGES, 0., 5.), (STAGES, 5., 1.), (MODE, 0., 1.), (MODE, 1., 0.), (SHAPE, 0., 1.)] {
      let store = plain_store();
      store.set_plain(DEPTH, 60.);
      store.set_plain(FEEDBACK, 50.);
      store.set_plain(id, from);
      let mut runner = Runner::new(store.clone(), SR);
      let input = sine(48_000, 110.);
      let (settled, _) = run_on(&mut runner, &store, &input, 256);
      store.set_plain(id, to);
      let (changed, _) = run_on(&mut runner, &store, &input[..9600], 256);
      let (_, after) = (0, run_on(&mut runner, &store, &input, 256).0);
      let usual = most_step(&settled[0][24_000..]).max(most_step(&after[0][24_000..]));
      let moving = most_step(&changed[0]);
      assert!(moving < usual * 1.5, "{} from {} to {}: a step of {} where it usually moves {}", PARAMS[id].key, from, to, moving, usual);
    }
  }
}
