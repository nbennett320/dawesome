// the Delay: echoes, each side its own delay line, in Stereo (each side
// echoes itself) or Ping-pong (echoes bounce between the sides). the time
// is a note value at the song's tempo, or free in ms
//
//            ┌──────────── feedback ◄──────────────────────────┐
//            ▼                                                 │
//   in ──► [ + ] ──► low cut ──► high cut ──► line ──► read ───┴──► wet
//
// the cuts shape what goes into the line, so every echo has them and each
// one passes through them again (darker, or thinner, as they fade). they
// don't resonate (Q 0.707), so the loop can't gain on itself
//
// a time change glides, like tape: the read slides to the new time and
// the echoes bend in pitch on the way. note values, the sync switch and
// tempo changes glide too (they'd jump otherwise). see Glide

use crate::delay_line::{DelayLine, MIN_DELAY};
use crate::effect::{Effect, EffectInfo, Entry, Io, Values};
use crate::param::{ParamDesc, Taper, Unit};
use crate::svf::{Coefs, Shape, State};
use crate::transport::Transport;

// the parameters' ids (places in PARAMS: only ever add to the end)
pub const SYNC: usize = 0;
pub const NOTE: usize = 1;
pub const TIME: usize = 2;
pub const OFFSET: usize = 3;
pub const MODE: usize = 4;
pub const FEEDBACK: usize = 5;
pub const LOW_CUT: usize = 6;
pub const HIGH_CUT: usize = 7;
pub const MIX: usize = 8;

// the note values, shortest first. a label is its own length: 1/n of a
// whole note, × 1.5 dotted (D), × 2/3 triplet (T). the app works lengths
// out from the labels, so they're its formula
pub const NOTES: &[&str] = &[
  "1/32 T", "1/32", "1/16 T", "1/32 D", "1/16", "1/8 T", "1/16 D", "1/8", "1/4 T",
  "1/8 D", "1/4", "1/2 T", "1/4 D", "1/2", "1/1 T", "1/2 D", "1/1", "1/1 D",
];
// and each one's length in quarter notes (a test checks these against the
// labels)
const QUARTERS: [f32; 18] = [
  0.125 * 2. / 3., 0.125, 0.25 * 2. / 3., 0.1875, 0.25, 0.5 * 2. / 3., 0.375, 0.5, 2. / 3.,
  0.75, 1., 4. / 3., 1.5, 2., 8. / 3., 3., 4., 6.,
];

pub const MODES: &[&str] = &["Stereo", "Ping-pong"];
pub const STEREO: usize = 0;
pub const PING_PONG: usize = 1;

// the longest time a side can be (a dotted whole note at 90 bpm, or 2 s
// with +50% offset). longer ones are held at it
pub const LONGEST: f32 = 4.;
// the most feedback (always under 1: echoes always fade)
pub const MOST_FEEDBACK: f32 = 95.;

pub static PARAMS: [ParamDesc; 9] = [
  ParamDesc::toggle("sync", "Sync", "Time", true).bending_pitch(),
  ParamDesc::choice("note", "Note", "Time", NOTES, 12).bending_pitch(),
  ParamDesc::continuous("time", "Time", "Time", Unit::Ms, (1., 2000.), 375., Taper::Log).bending_pitch(),
  ParamDesc::continuous("offset", "Offset", "Time", Unit::Percent, (-50., 50.), 0., Taper::Linear).bending_pitch(),
  ParamDesc::choice("mode", "Mode", "Echoes", MODES, STEREO),
  ParamDesc::continuous("feedback", "Feedback", "Echoes", Unit::Percent, (0., MOST_FEEDBACK), 40., Taper::Linear),
  ParamDesc::continuous("lowcut", "Low cut", "Tone", Unit::Hz, (20., 2000.), 80., Taper::Log),
  ParamDesc::continuous("highcut", "High cut", "Tone", Unit::Hz, (1000., 20_000.), 8000., Taper::Log),
  ParamDesc::continuous("mix", "Mix", "Mix", Unit::Percent, (0., 100.), 30., Taper::Linear),
];

pub static ENTRY: Entry = Entry {
  info: EffectInfo { id: "delay", name: "Delay", category: "Delay", version: 1 },
  params: &PARAMS,
  make,
};

fn make(rate: f32) -> Box<dyn Effect> {
  Box::new(Delay::new(rate))
}

// a note value's length in quarter notes
pub fn quarters(note: usize) -> f32 {
  QUARTERS[note.min(QUARTERS.len() - 1)]
}

// each side's time, in seconds, as `params` set it at `tempo` (quarter
// notes a minute)
pub fn times(params: &Values, tempo: f64) -> [f32; 2] {
  let base = if params.on(SYNC) {
    let tempo = if tempo > 0. { tempo as f32 } else { 120. };
    quarters(params.choice(NOTE)) * 60. / tempo
  } else {
    params.get(TIME) / 1000.
  };
  // offset lengthens one side: the right one when it's over 0
  let longer = base * (1. + params.get(OFFSET).abs() / 100.);
  let [l, r] = if params.get(OFFSET) >= 0. { [base, longer] } else { [longer, base] };
  [l.min(LONGEST), r.min(LONGEST)]
}

// how the dry sound and the echoes are mixed at `mix` (0 to 100): the dry
// stays whole to the middle, then falls; the echoes are whole from there
pub(crate) fn levels(mix: f32) -> (f32, f32) {
  let m = (mix / 100.).clamp(0., 1.);
  ((2. * (1. - m)).min(1.), (2. * m).min(1.))
}

// ---- the glide
//
// where a side reads from slides toward its time a sample at a time: most
// of the way in GLIDE seconds, slowing as it nears, and never so fast the
// echoes go more than a fifth up or an octave down as it moves (the read
// moving through the line at 1.5 to 0.5 times its usual speed)

const GLIDE: f32 = 0.1;
// the most the delay can shrink (pitch up) and grow (pitch down) by in a
// sample
const MOST_SHRINK: f64 = 0.5;
const MOST_GROW: f64 = 0.5;
// close enough to land on it (a thousandth of a sample)
const LAND: f64 = 1e-3;

// how long a change of mode takes to blend in
const MODE_FADE: f32 = 0.010;

#[derive(Clone, Copy)]
struct Glide {
  // where it reads from now, in samples. in f64: an f32 four seconds
  // back can't move by the tiny steps a glide ends with, and stalls
  now: f64,
  // and where it's going
  to: f64,
}

impl Glide {
  #[inline]
  fn step(&mut self, keep: f64) -> f64 {
    let gap = self.to - self.now;
    if gap.abs() <= LAND {
      self.now = self.to;
    } else {
      self.now += (gap * (1. - keep)).clamp(-MOST_SHRINK, MOST_GROW);
    }
    self.now
  }
}

// ---- the effect

struct Side {
  line: DelayLine,
  glide: Glide,
  low_cut: State,
  high_cut: State,
}

pub struct Delay {
  rate: f32,
  sides: [Side; 2],
  // the cuts' filters, and what they were made from
  cuts: (Coefs, Coefs),
  made_for: Option<(f32, f32)>,
  // how much of a glide's gap is left after each sample
  keep: f64,
  // how far it's into Ping-pong's routing, 0 to 1: a mode change blends
  // the old routing into the new over MODE_FADE (each one writes something
  // different into the lines, and the step would come back round as a
  // click an echo later)
  ping_pong: f32,
  fade_step: f32,
  // the tempo the last block ran at (for the tail)
  tempo: f64,
  // it's run since it was made or reset: until then, there's nothing to
  // glide from
  started: bool,
}

impl Delay {
  pub fn new(rate: f32) -> Delay {
    let side = || Side {
      line: DelayLine::new(LONGEST, rate),
      glide: Glide { now: 0., to: 0. },
      low_cut: State::default(),
      high_cut: State::default(),
    };
    let flat = Coefs::new(Shape::Bell, 1000., 0., 1., rate);
    let mut delay = Delay {
      rate,
      sides: [side(), side()],
      cuts: (flat, flat),
      made_for: None,
      keep: 0.,
      ping_pong: 0.,
      fade_step: 0.,
      tempo: 120.,
      started: false,
    };
    delay.set_sample_rate(rate);
    delay
  }

  // each side's time, in samples, as `params` set it
  fn targets(&self, params: &Values, tempo: f64) -> [f64; 2] {
    let [l, r] = times(params, tempo);
    let side = |seconds: f32| (seconds as f64 * self.rate as f64).clamp(MIN_DELAY, self.sides[0].line.longest());
    [side(l), side(r)]
  }
}

impl Effect for Delay {
  fn set_sample_rate(&mut self, rate: f32) {
    self.rate = rate;
    self.keep = (-1. / (GLIDE as f64 * rate.max(1.) as f64)).exp();
    self.fade_step = 1. / (MODE_FADE * rate.max(1.));
    self.made_for = None;
    for side in &mut self.sides { side.line.set_rate(rate); }
  }

  fn reset(&mut self) {
    for side in &mut self.sides {
      side.line.clear();
      side.low_cut.reset();
      side.high_cut.reset();
    }
    self.started = false;
  }

  fn process(&mut self, io: Io, params: &Values, transport: &Transport) {
    let Io { input, output, .. } = io;
    let [out_l, out_r] = output;

    self.tempo = transport.tempo;
    let targets = self.targets(params, transport.tempo);
    let starting = !std::mem::replace(&mut self.started, true);
    for (side, to) in self.sides.iter_mut().zip(targets) {
      side.glide.to = to;
      if starting { side.glide.now = to; }
    }
    let mode = if params.choice(MODE) == PING_PONG { 1. } else { 0. };
    if starting { self.ping_pong = mode; }

    let cuts = (params.get(LOW_CUT), params.get(HIGH_CUT));
    if self.made_for != Some(cuts) {
      self.cuts = (
        Coefs::new(Shape::LowCut, cuts.0, 0., 0.707, self.rate),
        Coefs::new(Shape::HighCut, cuts.1, 0., 0.707, self.rate),
      );
      self.made_for = Some(cuts);
    }
    let (low, high) = self.cuts;

    let feedback = params.get(FEEDBACK) / 100.;
    let (dry, wet) = levels(params.get(MIX));
    let (mut ping_pong, fade_step) = (self.ping_pong, self.fade_step);
    let keep = self.keep;
    let [left, right] = &mut self.sides;

    for i in 0..input[0].len() {
      let (in_l, in_r) = (input[0][i], input[1][i]);
      let echo_l = left.line.read(left.glide.step(keep));
      let echo_r = right.line.read(right.glide.step(keep));
      // what goes into each line: in Stereo, each side's input and its
      // own echoes. in Ping-pong, the input (as one) into the left only,
      // and each side's echoes into the other
      if ping_pong != mode {
        ping_pong = if mode > ping_pong { (ping_pong + fade_step).min(1.) } else { (ping_pong - fade_step).max(0.) };
      }
      let stereo = (in_l + feedback * echo_l, in_r + feedback * echo_r);
      let (into_l, into_r) = if ping_pong == 0. {
        stereo
      } else {
        let bounced = (0.5 * (in_l + in_r) + feedback * echo_r, feedback * echo_l);
        (stereo.0 + ping_pong * (bounced.0 - stereo.0), stereo.1 + ping_pong * (bounced.1 - stereo.1))
      };
      left.line.write(left.high_cut.run(&high, left.low_cut.run(&low, into_l)));
      right.line.write(right.high_cut.run(&high, right.low_cut.run(&low, into_r)));
      out_l[i] = dry * in_l + wet * echo_l;
      out_r[i] = dry * in_r + wet * echo_r;
    }
    self.ping_pong = ping_pong;

    for side in &mut self.sides {
      side.low_cut.flush();
      side.high_cut.flush();
    }
  }

  fn tail_frames(&self, params: &Values, rate: f32) -> Option<u64> {
    // the longest a pass round the loop takes: the longer side, as set or
    // as it's gliding from (before it's run, the longest it could be)
    let targets = self.targets(params, self.tempo);
    let pass = if self.started {
      self.sides.iter().zip(targets).map(|(s, to)| s.glide.now.max(to)).fold(0., f64::max)
    } else {
      self.sides[0].line.longest()
    } as f32 / self.rate.max(1.);
    // how many passes until it's 90 dB down, from the most the loop can
    // build up to (1 / (1 - feedback), for a sound longer than a pass)
    let feedback = params.get(FEEDBACK) / 100.;
    let passes = if feedback <= 0. {
      1.
    } else {
      1. + ((3.2e-5 * (1. - feedback)).ln() / feedback.ln()).ceil().max(0.)
    };
    // each pass through the cuts smears it a little later (generously: a
    // low cut's time constant), and the cuts ring at the end
    let smear = 1. / (std::f32::consts::PI * params.get(LOW_CUT));
    let seconds = passes * (pass + smear) + 14. * smear;
    Some((seconds * rate).ceil() as u64)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::runner::{Runner, Store};
  use std::sync::Arc;

  const RATE: f32 = 48_000.;

  // an echo at full wet, nothing dry, flat cuts, no feedback, the time free
  fn plain_store() -> Arc<Store> {
    let store = Store::new(&ENTRY);
    store.set_plain(SYNC, 0.);
    store.set_plain(FEEDBACK, 0.);
    store.set_plain(MIX, 100.);
    store.set_plain(LOW_CUT, 20.);
    store.set_plain(HIGH_CUT, 20_000.);
    store
  }

  // `input` (on the left, or both) through it: both sides out
  fn run(store: &Arc<Store>, input: &[f32], both: bool, tempo: f64) -> [Vec<f32>; 2] {
    let mut runner = Runner::new(store.clone(), RATE);
    let transport = Transport { sample_rate: RATE as u32, tempo, ..Transport::default() };
    let silent = vec![0.; input.len()];
    let right = if both { input } else { &silent };
    let mut out = [vec![0.; input.len()], vec![0.; input.len()]];
    let [l, r] = &mut out;
    for at in (0..input.len()).step_by(512) {
      let end = (at + 512).min(input.len());
      runner.process([&input[at..end], &right[at..end]], [&mut l[at..end], &mut r[at..end]], &transport);
    }
    out
  }

  // a short 1 kHz burst (which the cuts, flat, leave alone) at the start of
  // `frames` frames
  fn burst(frames: usize) -> Vec<f32> {
    let len = 480;
    (0..frames).map(|i| {
      if i >= len { return 0.; }
      let window = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / len as f32).cos();
      window * (std::f32::consts::TAU * 1000. * i as f32 / RATE).sin()
    }).collect()
  }

  // the loudest moment between frames `from` and `to`: (where, how loud)
  fn loudest(samples: &[f32], from: usize, to: usize) -> (usize, f32) {
    (from..to).fold((from, 0.), |(at, most), i| if samples[i].abs() > most { (i, samples[i].abs()) } else { (at, most) })
  }

  #[test]
  fn test_an_echo_comes_when_its_time_says() {
    let store = plain_store();
    store.set_plain(TIME, 100.);
    let mut click = vec![0.; 9600];
    click[0] = 1.;
    let [l, r] = run(&store, &click, true, 120.);
    // 100 ms at 48 kHz is 4800 frames (the cuts, wide open, barely smear it)
    assert!(l[..4790].iter().all(|x| x.abs() < 1e-3));
    assert_eq!(loudest(&l, 0, 9600).0, 4800);
    assert_eq!(loudest(&r, 0, 9600).0, 4800);
  }

  #[test]
  fn test_synced_it_echoes_a_note_value_at_the_tempo() {
    let store = plain_store();
    store.set_plain(SYNC, 1.);
    store.set_plain(NOTE, NOTES.iter().position(|n| *n == "1/8").unwrap() as f32);
    let mut click = vec![0.; 20_000];
    click[0] = 1.;
    // an eighth at 120 bpm is a quarter of a second
    assert_eq!(loudest(&run(&store, &click, true, 120.)[0], 0, 20_000).0, 12_000);
    // and at 90, a third
    assert_eq!(loudest(&run(&store, &click, true, 90.)[0], 0, 20_000).0, 16_000);
  }

  #[test]
  fn test_each_note_value_is_as_long_as_its_label_says() {
    for (i, label) in NOTES.iter().enumerate() {
      let (fraction, kind) = label.split_once(' ').unwrap_or((label, ""));
      let n: f32 = fraction.strip_prefix("1/").unwrap().parse().unwrap();
      let times = match kind { "D" => 1.5, "T" => 2. / 3., _ => 1. };
      assert!((quarters(i) - 4. / n * times).abs() < 1e-6, "{} is {} quarters", label, quarters(i));
    }
    // shortest first
    assert!((1..NOTES.len()).all(|i| quarters(i) > quarters(i - 1)));
  }

  #[test]
  fn test_echoes_fall_by_the_feedback_each_time() {
    let store = plain_store();
    store.set_plain(TIME, 50.);
    store.set_plain(FEEDBACK, 50.);
    let [l, _] = run(&store, &burst(12_000), false, 120.);
    let pass = 2400;
    let peaks: Vec<f32> = (1..4).map(|n| loudest(&l, n * pass, (n + 1) * pass).1).collect();
    assert!((peaks[0] - 1.).abs() < 0.02, "the first echo is {}", peaks[0]);
    for w in peaks.windows(2) {
      assert!((w[1] / w[0] - 0.5).abs() < 0.02, "an echo fell from {} to {}", w[0], w[1]);
    }
  }

  #[test]
  fn test_ping_pong_bounces_from_left_to_right() {
    let store = plain_store();
    store.set_plain(TIME, 50.);
    store.set_plain(FEEDBACK, 50.);
    store.set_plain(MODE, PING_PONG as f32);
    // the source in the middle: half of it from each side
    let [l, r] = run(&store, &burst(12_000), true, 120.);
    let pass = 2400;
    let at = |side: &[f32], n: usize| loudest(side, n * pass, (n + 1) * pass).1;
    assert!(at(&l, 1) > 0.9 && at(&r, 1) < 1e-3);
    assert!(at(&r, 2) > 0.45 && at(&l, 2) < 1e-3);
    assert!(at(&l, 3) > 0.2 && at(&r, 3) < 1e-3);
  }

  #[test]
  fn test_offset_lengthens_one_side() {
    let store = plain_store();
    store.set_plain(TIME, 100.);
    store.set_plain(OFFSET, 50.);
    let mut click = vec![0.; 9600];
    click[0] = 1.;
    let [l, r] = run(&store, &click, true, 120.);
    assert_eq!((loudest(&l, 0, 9600).0, loudest(&r, 0, 9600).0), (4800, 7200));
    store.set_plain(OFFSET, -50.);
    let [l, r] = run(&store, &click, true, 120.);
    assert_eq!((loudest(&l, 0, 9600).0, loudest(&r, 0, 9600).0), (7200, 4800));
  }

  #[test]
  fn test_switching_mode_doesnt_click_when_the_echo_comes_round() {
    // a held sine through it, the mode switched halfway: what was going
    // into the lines changes then, and comes out an echo (100 ms) later
    let store = plain_store();
    store.set_plain(TIME, 100.);
    store.set_plain(FEEDBACK, 40.);
    store.set_plain(MIX, 30.);
    let mut runner = Runner::new(store.clone(), RATE);
    let transport = Transport { sample_rate: RATE as u32, ..Transport::default() };
    let input: Vec<f32> = (0..48_000).map(|i| 0.5 * (std::f32::consts::TAU * 110. * i as f32 / RATE).sin()).collect();
    let mut out = [vec![0.; 48_000], vec![0.; 48_000]];
    for at in (0..48_000).step_by(512) {
      if at == 24_064 { store.set_plain(MODE, PING_PONG as f32); }
      let end = (at + 512).min(48_000);
      let [l, r] = &mut out;
      runner.process([&input[at..end], &input[at..end]], [&mut l[at..end], &mut r[at..end]], &transport);
    }
    let step = |s: &[f32]| s.windows(2).fold(0f32, |m, w| m.max((w[1] - w[0]).abs()));
    for side in &out {
      let usual = step(&side[14_000..24_000]);
      let around = step(&side[24_000..34_000]);
      assert!(around <= usual * 2. + 1e-3, "a jump of {} where the sound moves at most {}", around, usual);
    }
  }

  #[test]
  fn test_the_mix_keeps_the_dry_sound_whole_up_to_the_middle() {
    assert_eq!(levels(0.), (1., 0.));
    assert_eq!(levels(30.), (1., 0.6));
    assert_eq!(levels(50.), (1., 1.));
    assert_eq!(levels(75.), (0.5, 1.));
    assert_eq!(levels(100.), (0., 1.));
  }

  // a delay run a frame at a time, from 100 ms, until `frames` after its
  // time is set to `to` ms: where its left side read from, each frame (the
  // first, where it started)
  fn glide(to: f32, frames: usize) -> Vec<f64> {
    let mut plains = ENTRY.defaults();
    plains[SYNC] = 0.;
    plains[TIME] = 100.;
    let mut delay = Delay::new(RATE);
    let transport = Transport { sample_rate: RATE as u32, ..Transport::default() };
    let step = |delay: &mut Delay, plains: &[f32]| {
      let (mut l, mut r) = ([0.], [0.]);
      delay.process(Io { input: [&[0.], &[0.]], output: [&mut l, &mut r], sidechain: None }, &Values(plains), &transport);
      delay.sides[0].glide.now
    };
    let start = step(&mut delay, &plains);
    plains[TIME] = to;
    std::iter::once(start).chain((0..frames).map(|_| step(&mut delay, &plains))).collect()
  }

  #[test]
  fn test_a_new_time_glides_there_no_faster_than_a_fifth_up_or_an_octave_down() {
    for to in [400., 10., 101.] {
      // (the last of a glide eases in: a second and a half to land)
      let read = glide(to, 120_000);
      for (i, w) in read.windows(2).enumerate() {
        let moved = w[1] - w[0];
        assert!((-MOST_SHRINK..=MOST_GROW).contains(&moved), "to {} ms: moved {} samples at frame {}", to, moved, i);
      }
      // and it gets there
      assert!((read.last().unwrap() - to as f64 * 48.).abs() < 1e-2, "to {} ms: got to {}", to, read.last().unwrap());
    }
    // a small change goes most of the way in GLIDE
    let read = glide(101., 4800);
    assert!((read[4800] - 4848.).abs() < 48. * 0.4);
  }

  #[test]
  fn test_it_starts_where_its_time_is_set() {
    let read = glide(400., 1);
    assert!((read[0] - 4800.).abs() < 1e-3);
    assert_eq!(read[1] - read[0], MOST_GROW);
    let mut delay = Delay::new(RATE);
    let mut plains = ENTRY.defaults();
    plains[SYNC] = 0.;
    plains[TIME] = 250.;
    let (mut l, mut r) = ([0.; 4], [0.; 4]);
    let transport = Transport { sample_rate: RATE as u32, ..Transport::default() };
    delay.process(Io { input: [&[0.; 4], &[0.; 4]], output: [&mut l, &mut r], sidechain: None }, &Values(&plains), &transport);
    assert!((delay.sides[0].glide.now - 12_000.).abs() < 1e-3);
  }

  #[test]
  fn test_the_longest_time_is_held_at_what_the_line_holds() {
    let mut plains = ENTRY.defaults();
    plains[SYNC] = 1.;
    plains[NOTE] = (NOTES.len() - 1) as f32;
    plains[OFFSET] = 50.;
    // a dotted whole note at 40 bpm is 9 s, and half again on the right
    assert_eq!(times(&Values(&plains), 40.), [LONGEST, LONGEST]);
  }
}
