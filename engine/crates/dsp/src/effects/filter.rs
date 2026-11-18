// the Filter: a low, high or band pass (12 or 24 dB/oct) with resonance
// and drive, its frequency still, or moved by an LFO, by an envelope each
// attack fires, or by how loud the playing is (an auto-wah)
//
//            ┌──────── Mode: LFO / Envelope / Dynamic ──────────┐
//            │   m (0 to 1)  →  f = Floor · (Cutoff / Floor)^m   │
//            ▼                                                   │
//   in ─┬─► drive ─► 12 dB ──────────────┐                       │
//       │       └──► 24 dB (two stages) ─┴─► (Vowel) ─► wet ─┐   │
//       └─────────────────────────── dry ────────────────────┴─► mix ─► output
//       └─────────► follower, attack detector ──────────────────────┘
//
// - one number moves it: `m`, 0 at Floor (the LFO's trough, where the
//   envelope starts) to 1 at Cutoff, evenly by pitch. Mode Off is m = 1:
//   Cutoff alone. Floor over Cutoff sweeps down instead
// - LFO: m = (lfo + 1) / 2. Envelope: each attack (envelope::Transient)
//   lifts m to 1 over Attack, and it falls back over Decay, the same each
//   time. Dynamic: m follows the playing's level (Attack and Decay its
//   follower's times), scaled by Sensitivity
// - m is smoothed over a few ms, so S&H, Square, a saw's reset or an attack
//   doesn't click
// - the SVF gives every pass at once from the same memory (svf::State::
//   outputs), and both slopes run all the time, so a change of Type or
//   Slope is a short crossfade with nothing starting cold
//
// the experimental three, each nothing at all at 0 (or Off):
// - Vowel: the filter turns into a voice's formants (u, o, a, e, i from low
//   to high), so the sweep talks
// - Spin: the right side's m turned the other way: one side opens as the
//   other closes
// - Tune: the frequency stepped to the nearest note (any semitone, or a
//   major key's: A minor is C), gliding 8 ms between them

use crate::effect::{Effect, EffectInfo, Entry, Io, Values};
use crate::envelope::{Envelope, Transient};
use crate::lfo::{Lfo, Random, Rate, Shape, NOTES};
use crate::param::{ParamDesc, Taper, Unit};
use crate::readout::READOUT;
use crate::svf::{Coefs, State, MAX_SHARE, TINY};
use crate::transport::Transport;

pub const TYPE: usize = 0;
pub const SLOPE: usize = 1;
pub const CUTOFF: usize = 2;
pub const RESONANCE: usize = 3;
pub const DRIVE: usize = 4;
pub const MODE: usize = 5;
pub const FLOOR: usize = 6;
pub const SYNC: usize = 7;
pub const NOTE: usize = 8;
pub const RATE: usize = 9;
pub const SHAPE: usize = 10;
pub const ATTACK: usize = 11;
pub const DECAY: usize = 12;
pub const SENSITIVITY: usize = 13;
pub const MIX: usize = 14;
pub const OUTPUT: usize = 15;
pub const VOWEL: usize = 16;
pub const SPIN: usize = 17;
pub const TUNE: usize = 18;

pub const TYPES: &[&str] = &["Low pass", "High pass", "Band pass"];
pub const SLOPES: &[&str] = &["12 dB", "24 dB"];
pub const MODES: &[&str] = &["Off", "LFO", "Envelope", "Dynamic"];
pub const OFF: usize = 0;
pub const LFO: usize = 1;
pub const ENVELOPE: usize = 2;
pub const DYNAMIC: usize = 3;
pub const SHAPES: &[&str] = &["Sine", "Triangle", "Saw up", "Saw down", "Square", "S&H"];
pub const SAMPLE_HOLD: usize = 5;
// Off, any semitone, then the major keys from C
pub const TUNINGS: &[&str] = &["Off", "Chromatic", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

pub static PARAMS: [ParamDesc; 19] = [
  ParamDesc::choice("type", "Type", "Filter", TYPES, 0),
  ParamDesc::choice("slope", "Slope", "Filter", SLOPES, 1),
  // (Cutoff and Floor bend the pitch of what a resonant filter rings with:
  // swept fast past a tone, it chirps, which the click test can't tell from
  // a click. see ParamDesc::bends_pitch)
  ParamDesc::continuous("cutoff", "Cutoff", "Filter", Unit::Hz, (20., 20_000.), 2000., Taper::Log).bending_pitch(),
  ParamDesc::continuous("resonance", "Resonance", "Filter", Unit::Percent, (0., 100.), 20., Taper::Linear),
  ParamDesc::continuous("drive", "Drive", "Filter", Unit::Db, (0., 24.), 0., Taper::Linear),
  ParamDesc::choice("mode", "Mode", "Motion", MODES, OFF),
  ParamDesc::continuous("floor", "Floor", "Motion", Unit::Hz, (20., 20_000.), 200., Taper::Log).bending_pitch(),
  ParamDesc::toggle("sync", "Sync", "LFO", true),
  ParamDesc::choice("note", "Note", "LFO", NOTES, 7),
  ParamDesc::continuous("rate", "Rate", "LFO", Unit::Hz, (0.03, 20.), 1., Taper::Log),
  ParamDesc::choice("shape", "Shape", "LFO", SHAPES, 0),
  ParamDesc::continuous("attack", "Attack", "Envelope", Unit::Ms, (0.5, 500.), 5., Taper::Log),
  ParamDesc::continuous("decay", "Decay", "Envelope", Unit::Ms, (10., 4000.), 300., Taper::Log),
  ParamDesc::continuous("sensitivity", "Sensitivity", "Envelope", Unit::Percent, (0., 100.), 50., Taper::Linear),
  ParamDesc::continuous("mix", "Mix", "Output", Unit::Percent, (0., 100.), 100., Taper::Linear),
  ParamDesc::continuous("output", "Output", "Output", Unit::Db, (-24., 12.), 0., Taper::Linear),
  ParamDesc::continuous("vowel", "Vowel", "Experimental", Unit::Percent, (0., 100.), 0., Taper::Linear),
  ParamDesc::continuous("spin", "Spin", "Experimental", Unit::Percent, (0., 100.), 0., Taper::Linear),
  ParamDesc::choice("tune", "Tune", "Experimental", TUNINGS, 0),
];

pub static ENTRY: Entry = Entry {
  info: EffectInfo { id: "filter", name: "Filter", category: "Filter", version: 1 },
  params: &PARAMS,
  make,
};

fn make(rate: f32) -> Box<dyn Effect> {
  Box::new(FilterFx::new(rate))
}

// ---- what the panel reads (Effect::readout's slots)

pub const OUT_PHASE: usize = 0;
pub const OUT_HZ: usize = 1;
// the frequency each side's at now (Hz), and its m
pub const OUT_FREQ_L: usize = 2;
pub const OUT_FREQ_R: usize = 3;
pub const OUT_M_L: usize = 4;
pub const OUT_M_R: usize = 5;
// how loud the input is (Dynamic's follower), and the envelope (0 to 1)
pub const OUT_LEVEL: usize = 6;
pub const OUT_ENVELOPE: usize = 7;
// attacks fired so far (it goes up by one each)
pub const OUT_TRIGGERS: usize = 8;

// ---- the numbers

// Q at no resonance (flat, Butterworth) and at full
pub const Q_LEAST: f32 = 0.707;
pub const Q_MOST: f32 = 25.;
// the 24 dB slope's two stages' Qs, flat (Butterworth): resonance raises
// the second only, so there's one peak
pub const Q_24: (f32, f32) = (0.541, 1.307);
const Q_24_MOST: f32 = 40.;
// m's smoothing: the LFO's steps (S&H, Square, a saw's reset) over 3 ms;
// the envelope and follower (already smooth) barely; and a change of mode
// over CROSSFADE, until it's landed
const M_SMOOTH: f32 = 0.003;
const M_FOLLOW: f32 = 0.0005;
// Type and Slope changes crossfade, and Tune glides between notes, over
const CROSSFADE: f32 = 0.01;
const TUNE_GLIDE: f32 = 0.008;
// Envelope's attack detector: Sensitivity from none to most moves how far
// the quick follower must jump over the slow, and the floor it must clear
const OVER: (f32, f32) = (4., 1.3);
const FLOOR_DB: (f32, f32) = (-24., -60.);
// Dynamic: Sensitivity from none to most is this much gain into it
const DYNAMIC_DB: (f32, f32) = (0., 36.);
// Vowel's voice, from low to high: u, o, a, e, i. three formants each (Hz),
// at these levels and Qs
pub const VOWELS: [[f32; 3]; 5] = [
  [325., 700., 2530.],
  [450., 800., 2830.],
  [800., 1150., 2900.],
  [400., 1600., 2700.],
  [290., 2200., 3000.],
];
// (with makeup: three narrow bands keep little of a full sound, so at the
// same level as the filter they'd sound some 12 dB down)
const FORMANT_LEVELS: [f32; 3] = [2.5, 1.5, 0.9];
const FORMANT_Q: [f32; 3] = [6., 9., 12.];
// where the filter's frequency picks a vowel: u at the lowest, i at the
// highest
pub const VOWEL_RANGE: (f32, f32) = (150., 5000.);
// a major key's notes, above its root
const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
  a + t * (b - a)
}

// Resonance (0 to 100) as a Q, by ratio
pub fn q_of(resonance: f32) -> f32 {
  Q_LEAST * (Q_MOST / Q_LEAST).powf((resonance / 100.).clamp(0., 1.))
}

// the frequency (log2 Hz) m puts it at, between Floor and Cutoff
#[inline]
pub fn octaves_at(m: f32, floor: f32, cutoff: f32) -> f32 {
  let (lo, hi) = (floor.max(1.).log2(), cutoff.max(1.).log2());
  lo + m * (hi - lo)
}

// a frequency (log2 Hz) stepped to the nearest note Tune allows (1 any
// semitone, 2.. the major key on C, C#, ...)
pub fn snap(octaves: f32, tune: usize) -> f32 {
  let a = 440f32.log2();
  let midi = 69. + 12. * (octaves - a);
  let near = midi.round() as i32;
  let note = if tune <= 1 {
    near
  } else {
    let root = tune as i32 - 2;
    (near - 2..=near + 2)
      .filter(|n| MAJOR.contains(&(n - root).rem_euclid(12)))
      .min_by(|x, y| (*x as f32 - midi).abs().total_cmp(&(*y as f32 - midi).abs()))
      .unwrap_or(near)
  };
  a + (note - 69) as f32 / 12.
}

// Vowel's three formants where the filter is at `hz`: between the two
// vowels either side, by octave
pub fn formants_at(hz: f32) -> [f32; 3] {
  let (lo, hi) = VOWEL_RANGE;
  let along = ((hz.max(1.) / lo).log2() / (hi / lo).log2()).clamp(0., 1.) * (VOWELS.len() - 1) as f32;
  let i = (along.floor() as usize).min(VOWELS.len() - 2);
  let t = along - i as f32;
  std::array::from_fn(|k| (VOWELS[i][k].log2() + t * (VOWELS[i + 1][k].log2() - VOWELS[i][k].log2())).exp2())
}

// a side's filters: the 12 dB one, and the 24 dB pair (the second stage one
// per type, each fed its type from the first)
#[derive(Clone, Copy, Default)]
struct Side {
  one: State,
  first: State,
  second: [State; 3],
  formants: [State; 3],
  // the frequency now (Hz), m, and Tune's glide (log2 Hz)
  freq: f32,
  m: f32,
  tuned: f32,
}

pub struct FilterFx {
  rate: f32,
  lfo: Lfo,
  hz: f64,
  random: Random,
  held: f32,
  last_phase: f64,
  sides: [Side; 2],
  // m before Spin, smoothed
  m: f32,
  // each type's share of the output, and the 24 dB slope's
  types: [f32; 3],
  steep: f32,
  follower: Envelope,
  transient: Transient,
  envelope: f32,
  rising: bool,
  triggers: u32,
  m_keep: f32,
  follow_keep: f32,
  // the mode last block, and whether m's still gliding from a change of it
  mode: usize,
  switching: bool,
  fade: f32,
  tune_keep: f32,
  started: bool,
}

impl FilterFx {
  pub fn new(rate: f32) -> FilterFx {
    let mut f = FilterFx {
      rate,
      lfo: Lfo::new(),
      hz: 0.,
      random: Random::new(77),
      held: 0.,
      last_phase: 0.,
      sides: [Side::default(); 2],
      m: 1.,
      types: [1., 0., 0.],
      steep: 1.,
      follower: Envelope::new(0.005, 0.3, rate),
      transient: Transient::new(rate),
      envelope: 0.,
      rising: false,
      triggers: 0,
      m_keep: 0.,
      follow_keep: 0.,
      mode: OFF,
      switching: false,
      fade: 0.,
      tune_keep: 0.,
      started: false,
    };
    f.set_sample_rate(rate);
    f
  }
}

impl Effect for FilterFx {
  fn set_sample_rate(&mut self, rate: f32) {
    self.rate = rate;
    self.m_keep = keep(M_SMOOTH, rate);
    self.follow_keep = keep(M_FOLLOW, rate);
    self.fade = keep(CROSSFADE, rate);
    self.tune_keep = keep(TUNE_GLIDE, rate);
    self.transient.set_rate(rate);
  }

  fn reset(&mut self) {
    for side in &mut self.sides {
      side.one.reset();
      side.first.reset();
      for s in &mut side.second { s.reset(); }
      for s in &mut side.formants { s.reset(); }
    }
    self.lfo.reset();
    self.random = Random::new(77);
    self.held = 0.;
    self.last_phase = 0.;
    self.follower.reset();
    self.transient.reset();
    self.envelope = 0.;
    self.rising = false;
    self.started = false;
  }

  fn process(&mut self, io: Io, params: &Values, transport: &Transport) {
    let Io { input, output, .. } = io;
    let [out_l, out_r] = output;
    let rate = self.rate;

    let mode = params.choice(MODE);
    let (floor, cutoff) = (params.get(FLOOR), params.get(CUTOFF));
    let q = q_of(params.get(RESONANCE));
    let q_second = (Q_24.1 * q / Q_LEAST).min(Q_24_MOST);
    let drive = 10f32.powf(params.get(DRIVE) / 20.);
    // (0 dB of drive is none at all: it fades in over its first 3 dB)
    let drive_share = (params.get(DRIVE) / 3.).min(1.);
    let shape = params.choice(SHAPE);
    let sensitivity = params.get(SENSITIVITY) / 100.;
    let over = lerp(OVER.0, OVER.1, sensitivity);
    let trigger_floor = 10f32.powf(lerp(FLOOR_DB.0, FLOOR_DB.1, sensitivity) / 20.);
    let dynamic_gain = 10f32.powf(lerp(DYNAMIC_DB.0, DYNAMIC_DB.1, sensitivity) / 20.);
    let (attack, decay) = (params.get(ATTACK) / 1000., params.get(DECAY) / 1000.);
    self.follower.set_times(attack, decay, rate);
    let attack_step = 1. / (attack * rate).max(1.);
    // (most of the way down, -40 dB, by Decay)
    let decay_keep = (-4.6 / (decay * rate).max(1.)).exp();
    let mix = (params.get(MIX) / 100.).clamp(0., 1.);
    let gain = 10f32.powf(params.get(OUTPUT) / 20.);
    let vowel = params.get(VOWEL) / 100.;
    let spin = params.get(SPIN) / 100.;
    let tune = params.choice(TUNE);

    let types_to: [f32; 3] = std::array::from_fn(|t| if params.choice(TYPE) == t { 1. } else { 0. });
    let steep_to = if params.choice(SLOPE) == 1 { 1. } else { 0. };
    if !std::mem::replace(&mut self.started, true) {
      self.types = types_to;
      self.steep = steep_to;
      self.m = if mode == OFF { 1. } else { 0. };
      self.mode = mode;
      for side in &mut self.sides { side.tuned = octaves_at(self.m, floor, cutoff); }
    }

    let (lo, hi) = (floor.max(1.).log2(), cutoff.max(1.).log2());
    let clock = Lfo::clock(Rate::of(params, SYNC, NOTE, RATE), transport);
    self.hz = clock.hz(rate);
    let lfo_shape = match shape { 1 => Shape::Triangle, 2 => Shape::SawUp, 3 => Shape::SawDown, 4 => Shape::Square, _ => Shape::Sine };
    if mode != std::mem::replace(&mut self.mode, mode) { self.switching = true; }
    let m_keep = if mode == LFO { self.m_keep } else { self.follow_keep };
    let (fade, tune_keep) = (self.fade, self.tune_keep);
    let top = MAX_SHARE * rate;

    for i in 0..input[0].len() {
      let ins = [input[0][i], input[1][i]];
      let mono = 0.5 * (ins[0] + ins[1]);

      // what moves it
      let phase = self.lfo.tick(&clock, i);
      if phase < self.last_phase { self.held = self.random.next(); }
      self.last_phase = phase;
      let lfo = if shape == SAMPLE_HOLD { self.held } else { lfo_shape.at(phase) };
      let level = self.follower.run(mono);
      if self.transient.run(mono, over, trigger_floor) {
        self.rising = true;
        self.triggers = self.triggers.wrapping_add(1);
      }
      if self.rising {
        self.envelope += attack_step;
        if self.envelope >= 1. {
          self.envelope = 1.;
          self.rising = false;
        }
      } else {
        self.envelope *= decay_keep;
      }
      let m_to = match mode {
        LFO => 0.5 * (lfo + 1.),
        ENVELOPE => self.envelope,
        DYNAMIC => (level * dynamic_gain).min(1.),
        _ => 1.,
      };
      if self.switching {
        glide(&mut self.m, m_to, fade);
        self.switching = (self.m - m_to).abs() > 0.02;
      } else {
        glide(&mut self.m, m_to, m_keep);
      }
      for t in 0..3 { glide(&mut self.types[t], types_to[t], fade); }
      glide(&mut self.steep, steep_to, fade);
      let (types, steep, m) = (self.types, self.steep, self.m);

      let outs: [f32; 2] = std::array::from_fn(|s| {
        let side = &mut self.sides[s];
        // Spin: the right side the other way (exactly m at 0)
        side.m = if s == 1 { m + spin * (1. - 2. * m) } else { m };
        let mut octaves = lo + side.m * (hi - lo);
        if tune == 0 {
          side.tuned = octaves;
        } else {
          glide(&mut side.tuned, snap(octaves, tune), tune_keep);
          octaves = side.tuned;
        }
        let freq = octaves.exp2().clamp(1., top);
        side.freq = freq;
        let g = (std::f32::consts::PI * freq / rate).tan();

        let x = if drive_share == 0. { ins[s] } else { ins[s] + drive_share * ((drive * ins[s]).tanh() / drive - ins[s]) };
        let one = side.one.outputs(&Coefs::poles(g, q), x);
        let first = side.first.outputs(&Coefs::poles(g, Q_24.0), x);
        let second_coefs = Coefs::poles(g, q_second);
        let mut wet = 0.;
        for t in 0..3 {
          let two = side.second[t].outputs(&second_coefs, first[t])[t];
          wet += types[t] * (one[t] + steep * (two - one[t]));
        }
        if vowel > 0. {
          let formants = formants_at(freq);
          let mut voice = 0.;
          for k in 0..3 {
            let g = (std::f32::consts::PI * formants[k].min(top) / rate).tan();
            voice += FORMANT_LEVELS[k] * side.formants[k].outputs(&Coefs::poles(g, FORMANT_Q[k]), x)[2];
          }
          wet += vowel * (voice - wet);
        }
        // (filters after nothing else: kept clean a sample at a time)
        let wet = clean(wet);
        gain * (ins[s] + mix * (wet - ins[s]))
      });
      out_l[i] = outs[0];
      out_r[i] = outs[1];
    }

    for side in &mut self.sides {
      side.one.flush();
      side.first.flush();
      for s in &mut side.second { s.flush(); }
      for s in &mut side.formants {
        // (Vowel's rest while it's off, and start from quiet)
        if vowel == 0. { s.reset() } else { s.flush() }
      }
    }
    self.follower.flush();
    self.transient.flush();
    if self.envelope < TINY { self.envelope = 0.; }
  }

  fn tail_frames(&self, params: &Values, rate: f32) -> Option<u64> {
    // a resonant filter rings about 2Q / (2 pi f) a time constant, at the
    // lowest it goes; 10.4 of those is -90 dB. Vowel's formants ring too
    let q = q_of(params.get(RESONANCE));
    let most_q = (Q_24.1 * q / Q_LEAST).min(Q_24_MOST).max(q).max(FORMANT_Q[2]);
    let lowest = params.get(FLOOR).min(params.get(CUTOFF)).max(20.);
    let seconds = 10.4 * 2. * most_q / (std::f32::consts::TAU * lowest) + 0.05;
    Some((seconds * rate).ceil() as u64)
  }

  fn readout(&self, out: &mut [f32; READOUT]) {
    out[OUT_PHASE] = self.lfo.phase() as f32;
    out[OUT_HZ] = self.hz as f32;
    out[OUT_FREQ_L] = self.sides[0].freq;
    out[OUT_FREQ_R] = self.sides[1].freq;
    out[OUT_M_L] = self.sides[0].m;
    out[OUT_M_R] = self.sides[1].m;
    out[OUT_LEVEL] = self.follower.level();
    out[OUT_ENVELOPE] = self.envelope;
    out[OUT_TRIGGERS] = self.triggers as f32;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::runner::{Runner, Store};
  use std::sync::Arc;

  const SR: f32 = 48_000.;

  // a plain low pass at 1 kHz, no resonance, all wet
  fn plain_store() -> Arc<Store> {
    let store = Store::new(&ENTRY);
    for (id, value) in [(TYPE, 0.), (SLOPE, 0.), (CUTOFF, 1000.), (RESONANCE, 0.), (MODE, OFF as f32), (FLOOR, 200.),
      (SYNC, 0.), (RATE, 1.), (MIX, 100.)] {
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

  fn db_at(store: &Arc<Store>, freq: f32) -> f32 {
    let ([l, _], _) = run(store, &sine(24_000, freq), 512);
    20. * (peak(&l[12_000..]) / 0.5).log10()
  }

  #[test]
  fn test_each_type_and_slope_cuts_as_a_filter_should() {
    let store = plain_store();
    // 12 dB low pass: -3 dB at the cutoff, about -24 two octaves up
    assert!((db_at(&store, 1000.) + 3.).abs() < 0.3, "{}", db_at(&store, 1000.));
    assert!((db_at(&store, 4000.) + 24.).abs() < 1.5, "{}", db_at(&store, 4000.));
    // 24 dB: still -3 at the cutoff, about -48 two octaves up
    store.set_plain(SLOPE, 1.);
    assert!((db_at(&store, 1000.) + 3.).abs() < 0.3, "{}", db_at(&store, 1000.));
    assert!((db_at(&store, 4000.) + 48.).abs() < 2., "{}", db_at(&store, 4000.));
    // high pass, the other way
    store.set_plain(TYPE, 1.);
    assert!(db_at(&store, 250.) < -45.);
    assert!(db_at(&store, 8000.).abs() < 0.5);
    // band pass: whole at the cutoff, down either side
    store.set_plain(TYPE, 2.);
    store.set_plain(SLOPE, 0.);
    assert!(db_at(&store, 1000.).abs() < 0.3);
    assert!(db_at(&store, 100.) < -15. && db_at(&store, 10_000.) < -15.);
  }

  #[test]
  fn test_resonance_raises_a_peak_at_the_cutoff() {
    let store = plain_store();
    store.set_plain(RESONANCE, 50.);
    let q = q_of(50.);
    assert!((db_at(&store, 1000.) - 20. * q.log10()).abs() < 0.5, "{} vs Q {}", db_at(&store, 1000.), q);
    assert!((q_of(0.) - Q_LEAST).abs() < 1e-6 && (q_of(100.) - Q_MOST).abs() < 1e-3);
  }

  #[test]
  fn test_off_it_sits_at_the_cutoff() {
    let store = plain_store();
    let (_, readouts) = run(&store, &vec![0.; 4800], 480);
    assert!((readouts.last().unwrap()[OUT_FREQ_L] - 1000.).abs() < 0.5);
  }

  #[test]
  fn test_the_lfo_sweeps_from_the_floor_to_the_cutoff_and_either_way_round() {
    for (floor, cutoff) in [(200., 3200.), (3200., 200.)] {
      let store = plain_store();
      store.set_plain(MODE, LFO as f32);
      store.set_plain(FLOOR, floor);
      store.set_plain(CUTOFF, cutoff);
      let (_, readouts) = run(&store, &vec![0.; 48_000], 48);
      let freqs: Vec<f32> = readouts[100..].iter().map(|r| r[OUT_FREQ_L]).collect();
      let (lo, hi) = freqs.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &f| (lo.min(f), hi.max(f)));
      assert!((lo / 200. - 1.).abs() < 0.03 && (hi / 3200. - 1.).abs() < 0.03, "{} to {}", lo, hi);
      // the trough's at the floor: a sine starts there
      assert!((readouts[0][OUT_FREQ_L] / floor - 1.).abs() < 0.05, "it started at {}", readouts[0][OUT_FREQ_L]);
    }
  }

  #[test]
  fn test_each_attack_fires_the_envelope_up_to_the_cutoff_and_back() {
    let store = plain_store();
    store.set_plain(MODE, ENVELOPE as f32);
    store.set_plain(ATTACK, 5.);
    store.set_plain(DECAY, 200.);
    let mut input = vec![0.; 4800];
    input.extend(sine(14_400, 220.));
    input.extend(vec![0.; 4800]);
    input.extend(sine(14_400, 220.));
    let (_, readouts) = run(&store, &input, 96);
    let at = |frame: usize| readouts[frame / 96];
    // up at the cutoff just after the attack, back near the floor 300 ms on
    let top = (4800..4800 + 2400).step_by(96).map(|f| at(f)[OUT_FREQ_L]).fold(0f32, f32::max);
    assert!(top > 950., "{}", top);
    assert!(at(4800 + 14_000)[OUT_FREQ_L] < 260., "{}", at(4800 + 14_000)[OUT_FREQ_L]);
    assert_eq!(readouts.last().unwrap()[OUT_TRIGGERS], 2.);
  }

  #[test]
  fn test_dynamic_opens_further_the_louder_it_plays() {
    let m_for = |level: f32| {
      let store = plain_store();
      store.set_plain(MODE, DYNAMIC as f32);
      let input: Vec<f32> = sine(24_000, 220.).iter().map(|x| x * level).collect();
      run(&store, &input, 480).1.last().unwrap()[OUT_M_L]
    };
    let (quiet, loud) = (m_for(0.05), m_for(0.5));
    assert!(quiet < loud && loud > 0.5 && quiet > 0., "{} vs {}", quiet, loud);
  }

  #[test]
  fn test_sample_and_hold_steps_once_a_cycle_but_never_jumps() {
    let store = plain_store();
    store.set_plain(MODE, LFO as f32);
    store.set_plain(SHAPE, SAMPLE_HOLD as f32);
    store.set_plain(RATE, 4.);
    let (_, readouts) = run(&store, &vec![0.; 48_000], 16);
    let ms: Vec<f32> = readouts.iter().map(|r| r[OUT_M_L]).collect();
    // it holds still most of the time, and moves between
    let still = ms.windows(2).filter(|w| (w[1] - w[0]).abs() < 1e-5).count();
    assert!(still > ms.len() * 3 / 4, "{} of {}", still, ms.len());
    // smoothed: no 16-frame step over what 3 ms allows (16/144 of a gap)
    assert!(ms.windows(2).all(|w| (w[1] - w[0]).abs() < 0.2));
  }

  #[test]
  fn test_spin_turns_the_right_side_the_other_way() {
    let store = plain_store();
    store.set_plain(MODE, LFO as f32);
    store.set_plain(SPIN, 100.);
    let (_, readouts) = run(&store, &vec![0.; 24_000], 240);
    for r in &readouts {
      assert!((r[OUT_M_L] + r[OUT_M_R] - 1.).abs() < 1e-5);
    }
  }

  #[test]
  fn test_tune_steps_to_notes_in_its_key() {
    let hz = |n: i32| 440. * 2f32.powf((n - 69) as f32 / 12.);
    // chromatic: the nearest semitone
    assert!((snap(450f32.log2(), 1).exp2() - 440.).abs() < 0.01);
    // C major: F# (66) isn't in it, so G (67) or F (65)
    let c = TUNINGS.iter().position(|t| *t == "C").unwrap();
    let got = snap(hz(66).log2() + 0.01, c).exp2();
    assert!((got - hz(67)).abs() < 0.01, "{}", got);
    let got = snap(hz(66).log2() - 0.01, c).exp2();
    assert!((got - hz(65)).abs() < 0.01, "{}", got);

    let store = plain_store();
    store.set_plain(CUTOFF, 450.);
    store.set_plain(TUNE, 1.);
    let (_, readouts) = run(&store, &vec![0.; 4800], 480);
    assert!((readouts.last().unwrap()[OUT_FREQ_L] - 440.).abs() < 0.1);
  }

  #[test]
  fn test_vowel_goes_from_u_low_to_i_high() {
    let (u, i) = (formants_at(10.), formants_at(20_000.));
    for k in 0..3 {
      assert!((u[k] - VOWELS[0][k]).abs() < 0.1);
      assert!((i[k] - VOWELS[4][k]).abs() < 0.1);
    }
    // between: in between
    let mid = formants_at((VOWEL_RANGE.0 * VOWEL_RANGE.1).sqrt());
    assert!((mid[1] - VOWELS[2][1]).abs() < 1.);
  }

  fn same_after(set: impl Fn(&Store), change: impl Fn(&mut FilterFx)) -> bool {
    let store = Store::new(&ENTRY);
    store.set_plain(MODE, LFO as f32);
    store.set_plain(RESONANCE, 50.);
    set(&store);
    let plains = store.plains();
    let params = Values(&plains);
    let input = sine(24_000, 220.);
    let outs: Vec<[Vec<f32>; 2]> = (0..2).map(|n| {
      let mut fx = FilterFx::new(SR);
      if n == 1 { change(&mut fx); }
      let mut out = [vec![0.; input.len()], vec![0.; input.len()]];
      let [l, r] = &mut out;
      for at in (0..input.len()).step_by(500) {
        let end = at + 500;
        let io = Io { input: [&input[at..end], &input[at..end]], output: [&mut l[at..end], &mut r[at..end]], sidechain: None };
        fx.process(io, &params, &transport());
      }
      out
    }).collect();
    outs[0] == outs[1]
  }

  #[test]
  fn test_at_0_the_experimental_three_change_nothing_at_all() {
    let none = |_: &Store| {};
    let noisy_formants = |f: &mut FilterFx| for side in &mut f.sides {
      for s in &mut side.formants { s.outputs(&Coefs::poles(0.3, 2.), 0.7); }
    };
    let wrong_tune = |f: &mut FilterFx| for side in &mut f.sides { side.tuned = 13.; };
    assert!(same_after(none, noisy_formants));
    assert!(same_after(none, wrong_tune));
    // (and turned up, they do change it)
    assert!(!same_after(|s| { s.set_plain(VOWEL, 50.); }, noisy_formants));
    // Spin at 0 leaves the right side as the left
    let store = plain_store();
    store.set_plain(MODE, LFO as f32);
    let (_, readouts) = run(&store, &vec![0.; 4800], 480);
    assert!(readouts.iter().all(|r| r[OUT_M_L] == r[OUT_M_R]));
  }

  #[test]
  fn test_switching_type_slope_mode_or_shape_doesnt_click() {
    for (id, from, to) in [(TYPE, 0., 1.), (TYPE, 1., 2.), (SLOPE, 0., 1.), (MODE, 0., 1.), (MODE, 1., 3.), (SHAPE, 0., 4.)] {
      let store = plain_store();
      store.set_plain(CUTOFF, 400.);
      store.set_plain(RESONANCE, 30.);
      store.set_plain(MIX, 50.);
      store.set_plain(id, from);
      let mut runner = Runner::new(store.clone(), SR);
      let input = sine(48_000, 110.);
      let (settled, _) = run_on(&mut runner, &store, &input, 256);
      store.set_plain(id, to);
      let (changed, _) = run_on(&mut runner, &store, &input[..4800], 256);
      let (after, _) = run_on(&mut runner, &store, &input, 256);
      let usual = most_step(&settled[0][24_000..]).max(most_step(&after[0][24_000..]));
      let moving = most_step(&changed[0]);
      assert!(moving < usual * 1.5 + 1e-3, "{} from {} to {}: a step of {} where it usually moves {}", PARAMS[id].key, from, to, moving, usual);
    }
  }
}
