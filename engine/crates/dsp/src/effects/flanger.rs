// the Flanger: the sound mixed with a copy of itself a few ms late. the
// two cancel where the copy is half a cycle out, a comb of notches up the
// spectrum, and the LFO sweeps the copy's lateness, so the comb glides up
// and down: the jet. fed back, the teeth between the notches ring:
// metallic, resonant
//
//   in ─┬────────────────── (Through-zero: late by the centre) ─────────── dry ─┐
//       └─► [ + ] ─► line ─► read at centre · 2^(sweep) ─┬────────────── wet ──┴─► mix
//             ▲                                          │
//             └──── ± feedback ◄── tone ◄────────────────┘
//
// - the sweep is in octaves of delay (2 octaves either way at full
//   Depth), so the notches move evenly in pitch, as a jet does
// - positive feedback rings at every harmonic of 1/delay (metallic),
//   negative at the odd ones only (hollow)
// - Through-zero delays the dry by the centre too, and turns the copy
//   over: as the sweep passes the centre, the two meet and cancel, the
//   tape flanger's thump. it delays the channel by the centre while on
//
// the experimental three, each nothing at all at 0 (or Off):
// - Barberpole: the copy's phase turned round and round inside the loop
//   (hilbert.rs), a whole turn per cycle of the LFO at full, so the comb's
//   teeth and what rings between them slide up (or down) for ever, a
//   tooth a turn. it takes over from the sweep as it's turned up
// - Tune: the centre pinned to the nearest period of a note (Manual picks
//   the octave): the comb rings in key
// - Envelope: playing harder pushes the comb up (or down), up to two
//   octaves

use crate::delay_line::{DelayLine, MIN_DELAY};
use crate::effect::{Effect, EffectInfo, Entry, Io, Values};
use crate::envelope::Envelope;
use crate::hilbert::Rotator;
use crate::lfo::{Lfo, Rate, NOTES, SHAPES};
use crate::param::{ParamDesc, Taper, Unit};
use crate::readout::READOUT;
use crate::svf::{Coefs, Shape as Filter, State, TINY};
use crate::transport::Transport;

pub const SYNC: usize = 0;
pub const NOTE: usize = 1;
pub const RATE: usize = 2;
pub const DEPTH: usize = 3;
pub const SHAPE: usize = 4;
pub const MANUAL: usize = 5;
pub const FEEDBACK: usize = 6;
pub const TONE: usize = 7;
pub const ZERO: usize = 8;
pub const SPREAD: usize = 9;
pub const MIX: usize = 10;
pub const BARBERPOLE: usize = 11;
pub const TUNE: usize = 12;
pub const ENVELOPE: usize = 13;

// Off, then the twelve notes from C
pub const TUNINGS: &[&str] = &["Off", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
pub const MOST_FEEDBACK: f32 = 95.;

pub static PARAMS: [ParamDesc; 14] = [
  ParamDesc::toggle("sync", "Sync", "Sweep", false),
  ParamDesc::choice("note", "Note", "Sweep", NOTES, 15),
  ParamDesc::continuous("rate", "Rate", "Sweep", Unit::Hz, (0.01, 10.), 0.2, Taper::Log),
  ParamDesc::continuous("depth", "Depth", "Sweep", Unit::Percent, (0., 100.), 70., Taper::Linear).bending_pitch(),
  ParamDesc::choice("shape", "Shape", "Sweep", SHAPES, 0),
  ParamDesc::continuous("manual", "Manual", "Comb", Unit::Ms, (0.2, 10.), 2., Taper::Log).bending_pitch(),
  ParamDesc::continuous("feedback", "Feedback", "Comb", Unit::Percent, (-MOST_FEEDBACK, MOST_FEEDBACK), 60., Taper::Linear),
  ParamDesc::continuous("tone", "Tone", "Comb", Unit::Hz, (1000., 20_000.), 12_000., Taper::Log),
  ParamDesc::toggle("zero", "Through-zero", "Comb", false),
  ParamDesc::continuous("spread", "Spread", "Stereo", Unit::Percent, (0., 100.), 25., Taper::Linear).bending_pitch(),
  ParamDesc::continuous("mix", "Mix", "Mix", Unit::Percent, (0., 100.), 50., Taper::Linear),
  ParamDesc::continuous("barberpole", "Barberpole", "Experimental", Unit::Percent, (-100., 100.), 0., Taper::Linear).bending_pitch(),
  ParamDesc::choice("tune", "Tune", "Experimental", TUNINGS, 0),
  ParamDesc::continuous("envelope", "Envelope", "Experimental", Unit::Percent, (-100., 100.), 0., Taper::Linear).bending_pitch(),
];

pub static ENTRY: Entry = Entry {
  info: EffectInfo { id: "flanger", name: "Flanger", category: "Modulation", version: 1 },
  params: &PARAMS,
  make,
};

fn make(rate: f32) -> Box<dyn Effect> {
  Box::new(Flanger::new(rate))
}

// ---- what the panel reads (Effect::readout's slots)

pub const OUT_PHASE: usize = 0;
pub const OUT_HZ: usize = 1;
// the copy's delay now (the LFO's sweep, Tune and Envelope in it), ms,
// left and right
pub const OUT_DELAY_L: usize = 2;
pub const OUT_DELAY_R: usize = 3;
// the sweep's centre (Manual, or Tune's period), ms
pub const OUT_CENTER: usize = 4;
// how far round Barberpole's turned the copy (0 to 1, a whole turn)
pub const OUT_BARBER: usize = 5;
// how far Envelope has pushed the comb, octaves (up is more)
pub const OUT_PUSH: usize = 6;
// how far into Through-zero it is (0 to 1)
pub const OUT_ZERO: usize = 7;

// ---- the numbers

// the line: the longest a copy can be (10 ms, half an octave of Tune, two
// of sweep and two of Envelope) and some to spare. the dry's, for
// Through-zero: Manual and Tune
const LINE: f32 = 0.18;
const DRY_LINE: f32 = 0.016;
// octaves either way of the centre at full Depth, and Envelope's most
const SWEEP: f32 = 2.;
const PUSH: f32 = 2.;
// Through-zero switching, Tune changing note, and Barberpole starting or
// stopping, glide over this long
const SETTLE: f32 = 0.03;
// how long Barberpole's all-passes take to ring 90 dB down, at most
const TURN_RING: f32 = 0.2;
// Envelope's follower, and how it's smoothed on its way to the delay
const ENV_ATTACK: f32 = 0.01;
const ENV_RELEASE: f32 = 0.2;
const ENV_SMOOTH: f32 = 0.02;

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

// the LFO at `phase`, `triangle` of the way from a sine to a triangle
#[inline]
fn shape_at(phase: f32, triangle: f32) -> f32 {
  let p = phase - phase.floor();
  let tri = if p < 0.5 { 4. * p - 1. } else { 3. - 4. * p };
  if triangle == 1. { return tri; }
  let sine = -(std::f32::consts::TAU * p).cos();
  sine + triangle * (tri - sine)
}

// the centre, as a share of Manual, for Tune's choice: the period of the
// note nearest Manual's (1 when Off)
pub fn tuned(tune: usize, manual_ms: f32) -> f32 {
  if tune == 0 { return 1.; }
  // the note in the octave of A 440 (C is 9 semitones under it)
  let note = 440. * 2f32.powf((tune as f32 - 10.) / 12.);
  let manual = 1000. / manual_ms.max(1e-3);
  let octaves = (manual / note).log2().round();
  let period_ms = 1000. / (note * 2f32.powf(octaves));
  period_ms / manual_ms
}

struct Side {
  line: DelayLine,
  dry: DelayLine,
  tone: State,
  // Barberpole: turns the copy, and puts the dry through the same
  // all-passes so the two still line up
  turner: Rotator,
  aligner: Rotator,
  // what came out of the line last, for feedback
  back: f32,
  // its copy's delay last (samples), for the panel
  delay: f32,
}

pub struct Flanger {
  rate: f32,
  lfo: Lfo,
  hz: f64,
  sides: [Side; 2],
  // how far round Barberpole's turned (0 to 1), and how far it's on (it
  // fades in and out from nothing)
  barber: f64,
  turning: f32,
  // gliding: the triangle's share of the shape, Through-zero's blend, and
  // Tune's centre as a share of Manual (in octaves)
  triangle: f32,
  zero: f32,
  tune: f32,
  settle: f32,
  env: Envelope,
  push: f32,
  push_keep: f32,
  tone: Coefs,
  tone_for: Option<f32>,
  center_ms: f32,
  started: bool,
}

impl Flanger {
  pub fn new(rate: f32) -> Flanger {
    let mut flanger = Flanger {
      rate,
      lfo: Lfo::new(),
      hz: 0.,
      sides: std::array::from_fn(|_| Side {
        line: DelayLine::new(LINE, rate),
        dry: DelayLine::new(DRY_LINE, rate),
        tone: State::default(),
        turner: Rotator::new(rate),
        aligner: Rotator::new(rate),
        back: 0.,
        delay: 0.,
      }),
      barber: 0.,
      turning: 0.,
      triangle: 0.,
      zero: 0.,
      tune: 0.,
      settle: 0.,
      env: Envelope::new(ENV_ATTACK, ENV_RELEASE, rate),
      push: 0.,
      push_keep: 0.,
      tone: Coefs::new(Filter::Bell, 1000., 0., 1., rate),
      tone_for: None,
      center_ms: 0.,
      started: false,
    };
    flanger.set_sample_rate(rate);
    flanger
  }
}

impl Effect for Flanger {
  fn set_sample_rate(&mut self, rate: f32) {
    self.rate = rate;
    self.settle = keep(SETTLE, rate);
    self.push_keep = keep(ENV_SMOOTH, rate);
    self.env.set_times(ENV_ATTACK, ENV_RELEASE, rate);
    self.tone_for = None;
    for side in &mut self.sides {
      side.line.set_rate(rate);
      side.dry.set_rate(rate);
      side.turner.set_rate(rate);
      side.aligner.set_rate(rate);
    }
  }

  fn reset(&mut self) {
    for side in &mut self.sides {
      side.line.clear();
      side.dry.clear();
      side.tone.reset();
      side.turner.reset();
      side.aligner.reset();
      side.back = 0.;
    }
    self.lfo.reset();
    self.barber = 0.;
    self.turning = 0.;
    self.env.reset();
    self.push = 0.;
    self.started = false;
  }

  fn process(&mut self, io: Io, params: &Values, transport: &Transport) {
    let Io { input, output, .. } = io;
    let [out_l, out_r] = output;
    let rate = self.rate;

    let manual = params.get(MANUAL) / 1000. * rate;
    let barber = params.get(BARBERPOLE) / 100.;
    // (Barberpole takes over from the sweep as it's turned up)
    let depth = params.get(DEPTH) / 100. * (1. - barber.abs());
    let feedback = params.get(FEEDBACK) / 100.;
    let spread = 0.5 * params.get(SPREAD) / 100.;
    let mix = (params.get(MIX) / 100.).clamp(0., 1.);
    let envelope = params.get(ENVELOPE) / 100.;

    let triangle_to = if params.choice(SHAPE) == 1 { 1. } else { 0. };
    let zero_to = if params.on(ZERO) { 1. } else { 0. };
    let tune_to = tuned(params.choice(TUNE), params.get(MANUAL)).log2();
    if !std::mem::replace(&mut self.started, true) {
      self.triangle = triangle_to;
      self.zero = zero_to;
      self.tune = tune_to;
    }
    let tone = params.get(TONE);
    if self.tone_for != Some(tone) {
      self.tone = Coefs::new(Filter::HighCut, tone, 0., 0.707, rate);
      self.tone_for = Some(tone);
    }
    let tone = self.tone;

    let clock = Lfo::clock(Rate::of(params, SYNC, NOTE, RATE), transport);
    self.hz = clock.hz(rate);
    // Barberpole turns once a cycle of the LFO at full, either way
    let barber_step = barber as f64 * clock.step;
    let turning_to = if barber != 0. { 1. } else { 0. };
    let (settle, push_keep) = (self.settle, self.push_keep);
    let longest = self.sides[0].line.longest() as f32;

    for i in 0..input[0].len() {
      let ins = [input[0][i], input[1][i]];
      let phase = self.lfo.tick(&clock, i) as f32;
      let barber_at = self.barber as f32;
      self.barber += barber_step;
      self.barber -= self.barber.floor();
      glide(&mut self.turning, turning_to, settle);
      let turning = self.turning;

      glide(&mut self.triangle, triangle_to, settle);
      glide(&mut self.zero, zero_to, settle);
      glide(&mut self.tune, tune_to, settle);
      let level = self.env.run(0.5 * (ins[0] + ins[1]));
      let push = PUSH * envelope * (2. * level).min(1.);
      self.push = push + push_keep * (self.push - push);
      // (Tune's share is exactly 1 when Off: 2^0)
      let center = manual * self.tune.exp2();
      let zero = self.zero;

      let outs: [f32; 2] = std::array::from_fn(|s| {
        let side = &mut self.sides[s];
        let lfo = shape_at(phase + s as f32 * spread, self.triangle);
        let octaves = SWEEP * depth * lfo - self.push;
        let delay = (center * octaves.exp2()).clamp(MIN_DELAY as f32, longest);
        side.delay = delay;
        let mut copy = side.line.read(delay as f64);
        // Barberpole: the copy turned (and the dry lined up with it), faded
        // in from where it was
        let mut input = ins[s];
        if turning > 0. {
          // (the right side a little further round, as Spread has the sweep)
          let (sin, cos) = (std::f32::consts::TAU * (barber_at + s as f32 * spread)).sin_cos();
          // (kept clean a sample at a time: they're after the line, so what
          // they hold decays into denormals mid-block)
          let turned = clean(side.turner.turn(copy, cos, sin));
          let aligned = clean(side.aligner.align(input));
          copy += turning * (turned - copy);
          input += turning * (aligned - input);
        }
        // the loop: in, plus what came round, through the tone
        side.back = clean(side.tone.run(&tone, copy));
        side.tone.flush();
        side.line.write(ins[s] + feedback * side.back);
        // Through-zero: the dry late by the centre, the copy turned over
        let late = side.dry.read((center as f64).clamp(MIN_DELAY, side.dry.longest()));
        side.dry.write(input);
        let dry = if zero == 0. { input } else { input + zero * (late - input) };
        let copy = copy * (1. - 2. * zero);
        dry + mix * (copy - dry)
      });
      out_l[i] = outs[0];
      out_r[i] = outs[1];
    }
    self.center_ms = manual * self.tune.exp2() / rate * 1000.;
    self.env.flush();
    for side in &mut self.sides {
      // (they rest while Barberpole's off, and start from quiet)
      if self.turning == 0. {
        side.turner.reset();
        side.aligner.reset();
      } else {
        side.turner.flush();
        side.aligner.flush();
      }
    }
  }

  fn tail_frames(&self, params: &Values, rate: f32) -> Option<u64> {
    // passes round the loop until it's 90 dB down (as the Delay works it
    // out), each as long as the longest the copy can be
    let feedback = (params.get(FEEDBACK) / 100.).abs();
    let passes = if feedback <= 0. {
      1.
    } else {
      1. + ((3.2e-5 * (1. - feedback)).ln() / feedback.ln()).ceil().max(0.)
    };
    // the longest the copy can be at these settings: Manual, half an
    // octave of Tune, the sweep, and Envelope's push
    let octaves = 0.5 + SWEEP * params.get(DEPTH) / 100. + PUSH * (params.get(ENVELOPE) / 100.).abs();
    let longest = self.sides[0].line.longest() as f32 / self.rate.max(1.);
    let pass = (params.get(MANUAL) / 1000. * octaves.exp2()).min(longest);
    // (Barberpole's all-passes ring on a little at the bottom of the band)
    let turning = if params.get(BARBERPOLE) != 0. { TURN_RING } else { 0. };
    Some(((passes * (pass + turning) + DRY_LINE + 0.02) * rate).ceil() as u64)
  }

  fn readout(&self, out: &mut [f32; READOUT]) {
    let ms = 1000. / self.rate.max(1.);
    out[OUT_PHASE] = self.lfo.phase() as f32;
    out[OUT_HZ] = self.hz as f32;
    out[OUT_DELAY_L] = self.sides[0].delay * ms;
    out[OUT_DELAY_R] = self.sides[1].delay * ms;
    out[OUT_CENTER] = self.center_ms;
    out[OUT_BARBER] = self.barber as f32;
    out[OUT_PUSH] = self.push;
    out[OUT_ZERO] = self.zero;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::runner::{Runner, Store};
  use std::sync::Arc;

  const SR: f32 = 48_000.;

  // no sweep, no feedback, half and half, the tone wide open, free at 1 Hz
  fn plain_store() -> Arc<Store> {
    let store = Store::new(&ENTRY);
    for (id, value) in [(SYNC, 0.), (RATE, 1.), (DEPTH, 0.), (SHAPE, 0.), (MANUAL, 1.), (FEEDBACK, 0.),
      (TONE, 20_000.), (SPREAD, 0.), (MIX, 50.)] {
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

  #[test]
  fn test_the_sweep_goes_two_octaves_either_side_of_manual() {
    let store = plain_store();
    store.set_plain(DEPTH, 100.);
    store.set_plain(MANUAL, 2.);
    let (_, readouts) = run(&store, &vec![0.; 48_000], 48);
    let (lo, hi) = readouts.iter().fold((f32::MAX, f32::MIN), |(lo, hi), r| (lo.min(r[OUT_DELAY_L]), hi.max(r[OUT_DELAY_L])));
    assert!((lo - 0.5).abs() < 0.01 && (hi - 8.).abs() < 0.05, "it went from {} to {} ms", lo, hi);
  }

  #[test]
  fn test_the_comb_cancels_half_a_cycle_out_and_doubles_a_whole_one() {
    // 1 ms late: 500 Hz is half a cycle out, 1 kHz a whole one
    let level = |freq| {
      let ([l, _], _) = run(&plain_store(), &sine(9600, freq), 512);
      peak(&l[4800..])
    };
    assert!(level(500.) < 0.01, "{}", level(500.));
    assert!((level(1000.) - 0.5).abs() < 0.01, "{}", level(1000.));
  }

  #[test]
  fn test_feedback_rings_on_the_same_way_up_or_turned_over_each_pass() {
    for fb in [80f32, -80.] {
      let store = plain_store();
      store.set_plain(FEEDBACK, fb);
      store.set_plain(MIX, 100.);
      let mut click = vec![0.; 4800];
      click[0] = 1.;
      let ([l, _], _) = run(&store, &click, 512);
      // 1 ms is 48 frames: each pass 0.8 the last, turned over when < 0
      // (the tone filter smears each a little: what it adds up to is kept)
      for n in 1..4 {
        let expected = (fb / 100.).powi(n - 1);
        let at = 48 * n as usize;
        let got: f32 = l[at - 12..at + 12].iter().sum();
        assert!((got - expected).abs() < 0.02, "{}%: pass {} is {} not {}", fb, n, got, expected);
      }
    }
  }

  #[test]
  fn test_tune_puts_the_centre_on_the_nearest_period_of_its_note() {
    let a = TUNINGS.iter().position(|t| *t == "A").unwrap();
    // 2 ms is 500 Hz: the nearest A is 440 (2.27 ms)
    assert!((tuned(a, 2.) * 2. - 1000. / 440.).abs() < 1e-4);
    // 0.5 ms is 2 kHz: the nearest A is 1760
    assert!((tuned(a, 0.5) * 0.5 - 1000. / 1760.).abs() < 1e-4);
    let c = TUNINGS.iter().position(|t| *t == "C").unwrap();
    assert!((tuned(c, 1000. / 261.63) - 1.).abs() < 1e-3);
    assert_eq!(tuned(0, 3.7), 1.);

    let store = plain_store();
    store.set_plain(MANUAL, 2.);
    store.set_plain(TUNE, a as f32);
    let (_, readouts) = run(&store, &vec![0.; 4800], 480);
    assert!((readouts.last().unwrap()[OUT_CENTER] - 1000. / 440.).abs() < 1e-3);
    store.set_plain(TUNE, 0.);
    let (_, readouts) = run(&store, &vec![0.; 4800], 480);
    assert_eq!(readouts.last().unwrap()[OUT_CENTER], 2.);
  }

  #[test]
  fn test_through_zero_cancels_where_the_copy_meets_the_dry() {
    let store = plain_store();
    store.set_plain(ZERO, 1.);
    // no sweep: the copy sits at the centre, as late as the dry
    let ([l, _], _) = run(&store, &sine(9600, 300.), 512);
    assert!(peak(&l[4800..]) < 1e-3, "{}", peak(&l[4800..]));
  }

  #[test]
  fn test_barberpole_shifts_the_copy_up_or_down_a_turn_a_cycle() {
    // how much of `hz` is in `x`
    let amount = |x: &[f32], hz: f32| {
      let (mut c, mut s) = (0f32, 0f32);
      for (n, v) in x.iter().enumerate() {
        let w = std::f32::consts::TAU * hz * n as f32 / SR;
        c += v * w.cos();
        s += v * w.sin();
      }
      2. * (c * c + s * s).sqrt() / x.len() as f32
    };
    for (barber, to, away) in [(100., 1010., 990.), (-100., 990., 1010.)] {
      let store = plain_store();
      store.set_plain(FEEDBACK, 0.);
      store.set_plain(MIX, 100.);
      store.set_plain(RATE, 10.);
      store.set_plain(BARBERPOLE, barber);
      let ([l, _], _) = run(&store, &sine(96_000, 1000.), 512);
      let (to, away) = (amount(&l[48_000..], to), amount(&l[48_000..], away));
      assert!(to > 0.45 && away < 0.03, "{}: {} vs {}", barber, to, away);
    }
  }

  #[test]
  fn test_barberpole_sweeps_on_without_clicking() {
    let store = plain_store();
    store.set_plain(DEPTH, 50.);
    store.set_plain(FEEDBACK, 70.);
    store.set_plain(RATE, 0.5);
    let input = sine(96_000, 110.);
    let ([still, _], _) = run(&store, &input, 512);
    store.set_plain(BARBERPOLE, 100.);
    let ([barber, _], _) = run(&store, &input, 512);
    assert!(barber.iter().all(|x| x.is_finite()));
    assert!(peak(&barber) < 4.);
    // a low sine through it moves no faster than through the plain sweep
    // does, give or take
    assert!(most_step(&barber[4800..]) < 2. * most_step(&still[4800..]), "{} vs {}", most_step(&barber[4800..]), most_step(&still[4800..]));
  }

  #[test]
  fn test_envelope_pushes_the_comb_up_as_it_plays_harder() {
    let store = plain_store();
    store.set_plain(MANUAL, 4.);
    store.set_plain(ENVELOPE, 100.);
    let mut input = vec![0.; 4800];
    input.extend(sine(9600, 220.).iter().map(|x| x * 2.));
    let (_, readouts) = run(&store, &input, 480);
    assert!((readouts[5][OUT_DELAY_L] - 4.).abs() < 1e-4);
    // loud: two octaves up, a quarter of the delay
    let pushed = readouts.last().unwrap();
    assert!(pushed[OUT_PUSH] > 1.9, "{}", pushed[OUT_PUSH]);
    assert!(pushed[OUT_DELAY_L] < 1.1, "{}", pushed[OUT_DELAY_L]);
  }

  // the same flanger twice, `change` done to the second: whether they sound
  // the same, bit for bit
  fn same_after(set: impl Fn(&Store), change: impl Fn(&mut Flanger)) -> bool {
    let store = Store::new(&ENTRY);
    set(&store);
    let plains = store.plains();
    let params = Values(&plains);
    let mut input = sine(24_000, 220.);
    for x in &mut input[12_000..12_100] { *x = 0.9; }
    let outs: Vec<[Vec<f32>; 2]> = (0..2).map(|n| {
      let mut flanger = Flanger::new(SR);
      if n == 1 { change(&mut flanger); }
      let mut out = [vec![0.; input.len()], vec![0.; input.len()]];
      let [l, r] = &mut out;
      for at in (0..input.len()).step_by(500) {
        let end = at + 500;
        let io = Io { input: [&input[at..end], &input[at..end]], output: [&mut l[at..end], &mut r[at..end]], sidechain: None };
        flanger.process(io, &params, &transport());
      }
      out
    }).collect();
    outs[0] == outs[1]
  }

  #[test]
  fn test_at_0_the_experimental_three_change_nothing_at_all() {
    let defaults = |_: &Store| {};
    // Barberpole: wherever its slide is
    assert!(same_after(defaults, |f| f.barber = 0.37));
    // Envelope: however loud it thinks it's playing
    assert!(same_after(defaults, |f| { for _ in 0..1000 { f.env.run(1.); } }));
    // Tune Off: Manual itself
    assert_eq!(tuned(0, 2.), 1.);
    // (and turned up, each does change it)
    assert!(!same_after(|s| { s.set_plain(BARBERPOLE, 50.); }, |f| f.barber = 0.37));
    assert!(!same_after(|s| { s.set_plain(ENVELOPE, 50.); }, |f| { for _ in 0..1000 { f.env.run(1.); } }));
  }

  #[test]
  fn test_switching_through_zero_or_tune_doesnt_click() {
    for (id, to) in [(ZERO, 1.), (TUNE, 5.)] {
      let store = plain_store();
      store.set_plain(DEPTH, 40.);
      store.set_plain(FEEDBACK, 50.);
      let mut runner = Runner::new(store.clone(), SR);
      let input = sine(48_000, 110.);
      let (settled, _) = run_on(&mut runner, &store, &input, 256);
      store.set_plain(id, to);
      let (changed, _) = run_on(&mut runner, &store, &input[..4800], 256);
      let usual = most_step(&settled[0][24_000..]);
      let moving = most_step(&changed[0]);
      assert!(moving < usual * 1.5, "{}: a step of {} where it usually moves {}", PARAMS[id].key, moving, usual);
    }
  }
}
