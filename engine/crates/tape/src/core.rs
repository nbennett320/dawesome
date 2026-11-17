// the core: everything the record side of a tape does to one channel, the
// saturation, the squash, and the record and playback eq (head bump, hf
// loss, low cut). two kinds (D5): SimpleCore, built by hand, and
// NeuralCore, the network trained on the real deck. swapping one for the
// other changes how it sounds, not what any setting means
//
// both take the same audio and drive and give back the same level: 0 vu is
// the test file's -18 dbfs peak sine, and at 0 db drive small signals come
// back at unity (around 1 khz). driving harder makes it louder, as on tape

use std::fmt;
use std::sync::Arc;

use dsp::svf::{Coefs, Shape, State};

use crate::gru::{Gru, GruError, GruWeights};
use crate::oversample::Oversampler;

// 0 vu: the test file's reference sine's peak, -18 dbfs
pub const REFERENCE: f32 = 0.125_892_54;

pub trait TapeCore: Send {
  // back to silence
  fn reset(&mut self);
  // one channel. audio: 0 VU is the test file's -18 dBFS peak sine
  // (amplitude 10^(-18/20)). drive in dB (what the track's Drive knob
  // sets). out is normalized like the network: at 0 dB drive, small
  // signals come back at unity (around 1 kHz); driving harder makes it
  // louder, as on tape. the drive holds for the whole call: smoothing it
  // is the caller's (dsp's Runner hands effects held values)
  fn process(&mut self, audio: &[f32], drive_db: f32, out: &mut [f32]);
}

// ---- the hand-built core

// the most hf loss stages a SimpleCore has room for
pub const MAX_HF_STAGES: usize = 4;

// what shapes the hand-built core. the defaults are the fake deck's
// (tools/tape-model, faketape.Deck, fresh)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimpleCoreSettings {
  // a high shelf before the curve, and its opposite after: the highs
  // saturate first, as on tape
  pub emphasis_hz: f32,
  pub emphasis_db: f32,
  // where the curve bends, in multiples of 0 vu
  pub headroom: f32,
  // how lopsided the curve is: even harmonics
  pub asymmetry: f32,
  // a slow squash of hot passages: the curve's memory
  pub compress: f32,
  pub compress_seconds: f32,
  pub head_bump_hz: f32,
  pub head_bump_db: f32,
  pub hf_loss_hz: f32,
  // how many 12 db/octave stages the hf loss has (up to MAX_HF_STAGES)
  pub hf_loss_stages: usize,
  pub low_cut_hz: f32,
  // the curve runs at 1x, 2x or 4x the rate (anything else is taken up to
  // the next of those, and 4x at most)
  pub oversample: usize,
}

impl Default for SimpleCoreSettings {
  fn default() -> Self {
    SimpleCoreSettings {
      emphasis_hz: 3000.0,
      emphasis_db: 8.0,
      headroom: 2.8,
      asymmetry: 0.08,
      compress: 0.15,
      compress_seconds: 0.03,
      head_bump_hz: 70.0,
      head_bump_db: 2.5,
      hf_loss_hz: 14_000.0,
      hf_loss_stages: 1,
      low_cut_hz: 25.0,
      oversample: 2,
    }
  }
}

fn factor(oversample: usize) -> usize {
  match oversample {
    0 | 1 => 1,
    2 => 2,
    _ => 4,
  }
}

// the nonlinear part, run at the oversampled rate: an rms follower
// squashing loud passages (so the output depends on the level just before,
// not only the sample now), then an asymmetric soft clip with a gain of 1
// for small signals. it works in multiples of 0 vu
struct Curve {
  // how much of the follower's last value is kept each sample
  keep: f32,
  compress: f32,
  // 1 / headroom, the asymmetry, tanh of it, and the gain that makes the
  // curve's slope 1 at zero
  a: f32,
  s: f32,
  tanh_s: f32,
  slope: f32,
  env: f32,
}

impl Curve {
  fn new() -> Curve {
    Curve { keep: 0.0, compress: 0.0, a: 1.0, s: 0.0, tanh_s: 0.0, slope: 1.0, env: 0.0 }
  }

  // `rate` is the oversampled one: the follower's time is in seconds
  fn set(&mut self, settings: &SimpleCoreSettings, rate: f32) {
    self.keep = (-1.0 / (settings.compress_seconds.max(1e-5) * rate)).exp();
    self.compress = settings.compress;
    self.a = 1.0 / settings.headroom.max(1e-3);
    self.s = settings.asymmetry;
    self.tanh_s = self.s.tanh();
    self.slope = 1.0 / (self.a * (1.0 - self.tanh_s * self.tanh_s));
  }

  fn run(&mut self, buffer: &mut [f32]) {
    for v in buffer.iter_mut() {
      let u = *v * (1.0 / REFERENCE);
      self.env = (1.0 - self.keep) * u * u + self.keep * self.env;
      let u = u / (1.0 + self.compress * self.env).sqrt();
      let y = ((self.a * u + self.s).tanh() - self.tanh_s) * self.slope;
      *v = y * REFERENCE;
    }
    if self.env < 1e-20 {
      self.env = 0.0;
    }
  }
}

// a filter's settings and one channel's memory of it
#[derive(Clone, Copy)]
struct Filter {
  coefs: Coefs,
  state: State,
}

impl Filter {
  fn new(shape: Shape, freq: f32, gain: f32, q: f32, rate: f32) -> Filter {
    Filter { coefs: Coefs::new(shape, freq, gain, q, rate), state: State::default() }
  }

  fn set(&mut self, shape: Shape, freq: f32, gain: f32, q: f32, rate: f32) {
    self.coefs = Coefs::new(shape, freq, gain, q, rate);
  }

  #[inline]
  fn run(&mut self, x: f32) -> f32 {
    self.state.run(&self.coefs, x)
  }
}

// the fake deck's record side (faketape.core), for one channel: drive,
// pre-emphasis, squash and soft clip, de-emphasis, head bump, hf loss, low
// cut. the eq is dsp's state-variable filter: the same bilinear shapes as
// the python's rbj biquads (it matches them to float rounding), and
// steadier in f32 at 25 hz
//
// only the squash and clip run oversampled: the filters around them are
// linear (they make no new frequencies to alias), so at the base rate they
// cost a half or a quarter as much and match the python's response exactly
pub struct SimpleCore {
  settings: SimpleCoreSettings,
  rate: f32,
  oversampler: Oversampler,
  curve: Curve,
  pre: Filter,
  de: Filter,
  bump: Filter,
  hf: [Filter; MAX_HF_STAGES],
  low: Filter,
  drive_db: f32,
  drive_gain: f32,
}

impl SimpleCore {
  // its buffers are sized for 4x and blocks of `max_block` (longer blocks
  // are worked through in pieces), so set_settings can change anything
  // without allocating
  pub fn new(settings: SimpleCoreSettings, rate: f32, max_block: usize) -> SimpleCore {
    let blank = Filter::new(Shape::Bell, 1000.0, 0.0, 1.0, rate);
    let mut core = SimpleCore {
      settings,
      rate,
      oversampler: Oversampler::new(4, max_block),
      curve: Curve::new(),
      pre: blank,
      de: blank,
      bump: blank,
      hf: [blank; MAX_HF_STAGES],
      low: blank,
      drive_db: 0.0,
      drive_gain: 1.0,
    };
    core.set_settings(settings);
    core
  }

  pub fn settings(&self) -> &SimpleCoreSettings {
    &self.settings
  }

  // what it's set to now. the filters keep their memory (a change doesn't
  // click), except that a new oversampling factor starts the curve's
  // filters from silence
  pub fn set_settings(&mut self, settings: SimpleCoreSettings) {
    let rate = self.rate;
    let s = &settings;
    // (the python's q values, for the same shapes)
    self.pre.set(Shape::HighShelf, s.emphasis_hz, s.emphasis_db, 0.7, rate);
    self.de.set(Shape::HighShelf, s.emphasis_hz, -s.emphasis_db, 0.7, rate);
    self.bump.set(Shape::Bell, s.head_bump_hz, s.head_bump_db, 1.2, rate);
    for f in self.hf.iter_mut() {
      f.set(Shape::HighCut, s.hf_loss_hz, 0.0, 0.6, rate);
    }
    self.low.set(Shape::LowCut, s.low_cut_hz, 0.0, 0.7, rate);
    // stages coming back into use start from silence, not old sound
    let (was, now) = (self.stages(), s.hf_loss_stages.min(MAX_HF_STAGES));
    for f in self.hf[was.min(now)..now].iter_mut() {
      f.state.reset();
    }
    let factor = factor(s.oversample);
    self.oversampler.set_factor(factor);
    self.curve.set(s, rate * factor as f32);
    self.settings = settings;
  }

  fn stages(&self) -> usize {
    self.settings.hf_loss_stages.min(MAX_HF_STAGES)
  }
}

impl TapeCore for SimpleCore {
  fn reset(&mut self) {
    self.oversampler.reset();
    self.curve.env = 0.0;
    for f in [&mut self.pre, &mut self.de, &mut self.bump, &mut self.low].into_iter().chain(self.hf.iter_mut()) {
      f.state.reset();
    }
  }

  fn process(&mut self, audio: &[f32], drive_db: f32, out: &mut [f32]) {
    assert_eq!(audio.len(), out.len(), "audio and out differ in length");
    if drive_db != self.drive_db {
      self.drive_db = drive_db;
      self.drive_gain = 10f32.powf(drive_db / 20.0);
    }
    for (o, a) in out.iter_mut().zip(audio) {
      *o = self.pre.run(a * self.drive_gain);
    }
    let curve = &mut self.curve;
    self.oversampler.process_in_place(out, |b| curve.run(b));
    let stages = self.stages();
    for o in out.iter_mut() {
      let mut y = self.bump.run(self.de.run(*o));
      for f in self.hf[..stages].iter_mut() {
        y = f.run(y);
      }
      *o = self.low.run(y);
    }
    for f in [&mut self.pre, &mut self.de, &mut self.bump, &mut self.low].into_iter().chain(self.hf.iter_mut()) {
      f.state.flush();
    }
  }
}

// ---- the trained network

#[derive(Clone, Debug, PartialEq)]
pub enum CoreError {
  Network(GruError),
  // the network's rate, and the host's
  Rate(u32, f32),
}

impl fmt::Display for CoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      CoreError::Network(e) => write!(f, "{e}"),
      CoreError::Rate(network, host) => {
        write!(f, "the network runs at {network} hz, not {host} hz (resampling to it is still to come)")
      }
    }
  }
}

impl std::error::Error for CoreError {}

impl From<GruError> for CoreError {
  fn from(e: GruError) -> Self {
    CoreError::Network(e)
  }
}

// the network trained on the real deck, for one channel. it learned the
// deck's record and playback eq along with the curve, so there's nothing
// around it, and no oversampling: it was trained at its own rate on what
// the deck gave back, aliasing and all (D13: B4 checks it)
pub struct NeuralCore {
  gru: Gru,
}

impl NeuralCore {
  // the network only runs at the rate it was trained at. for now a host at
  // any other rate is refused: resampling to and from the network's rate
  // around it is later work (C2 in the plan)
  pub fn new(weights: Arc<GruWeights>, rate: f32) -> Result<NeuralCore, CoreError> {
    if rate != weights.rate as f32 {
      return Err(CoreError::Rate(weights.rate, rate));
    }
    Ok(NeuralCore { gru: Gru::new(weights)? })
  }
}

impl TapeCore for NeuralCore {
  fn reset(&mut self) {
    self.gru.reset();
  }

  fn process(&mut self, audio: &[f32], drive_db: f32, out: &mut [f32]) {
    assert_eq!(audio.len(), out.len(), "audio and out differ in length");
    self.gru.process(audio, drive_db, out);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::f64::consts::PI;

  const RATE: f32 = 48_000.0;

  fn sine(freq: f32, amp: f32, frames: usize) -> Vec<f32> {
    (0..frames).map(|i| (2.0 * PI * (freq / RATE) as f64 * i as f64).sin() as f32 * amp).collect()
  }

  // a sine's amplitude at `freq` in `y` (a whole number of cycles)
  fn level(y: &[f32], freq: f32) -> f64 {
    let f = (freq / RATE) as f64;
    let (mut c, mut s) = (0.0, 0.0);
    for (i, &v) in y.iter().enumerate() {
      let p = 2.0 * PI * f * i as f64;
      c += v as f64 * p.cos();
      s += v as f64 * p.sin();
    }
    2.0 * (c * c + s * s).sqrt() / y.len() as f64
  }

  fn db(v: f64) -> f64 {
    20.0 * v.log10()
  }

  fn with(oversample: usize) -> SimpleCore {
    SimpleCore::new(SimpleCoreSettings { oversample, ..Default::default() }, RATE, 512)
  }

  // through a core at a drive: the level at `freq` (and `harmonic` times
  // it) of a sine at `vu` db over 0 vu, once settled
  fn levels(core: &mut SimpleCore, freq: f32, vu: f32, drive: f32, harmonic: f32) -> (f64, f64) {
    let x = sine(freq, REFERENCE * 10f32.powf(vu / 20.0), 24_000);
    let mut y = vec![0.0; x.len()];
    core.process(&x, drive, &mut y);
    (level(&y[19_200..], freq), level(&y[19_200..], freq * harmonic))
  }

  #[test]
  fn small_signals_come_back_at_unity_at_0_db_drive() {
    for oversample in [1, 2, 4] {
      let (got, _) = levels(&mut with(oversample), 1000.0, -30.0, 0.0, 2.0);
      let want = REFERENCE as f64 * 10f64.powf(-1.5);
      assert!(db(got / want).abs() < 0.1, "{oversample}x: {} db", db(got / want));
    }
  }

  #[test]
  fn driving_harder_is_louder_and_dirtier() {
    let mut core = with(2);
    let (quiet, quiet_h3) = levels(&mut core, 1000.0, 0.0, 0.0, 3.0);
    core.reset();
    let (loud, loud_h3) = levels(&mut core, 1000.0, 0.0, 12.0, 3.0);
    assert!(loud > quiet * 2.0, "{quiet} then {loud}");
    assert!(loud_h3 / loud > 4.0 * quiet_h3 / quiet);
  }

  #[test]
  fn oversampling_keeps_the_passband() {
    // small and hot signals alike, at 1x, 2x and 4x: the same levels, and
    // the same harmonics (the oversampler's phase shift aside)
    for (freq, vu, drive) in [(100.0, -20.0, 0.0), (1000.0, -20.0, 0.0), (8000.0, -20.0, 0.0), (15_000.0, -20.0, 0.0), (1000.0, 0.0, 6.0)] {
      let (one, one_h3) = levels(&mut with(1), freq, vu, drive, 3.0);
      for oversample in [2, 4] {
        let (got, h3) = levels(&mut with(oversample), freq, vu, drive, 3.0);
        assert!(db(got / one).abs() < 0.05, "{freq} hz at {oversample}x: {} db", db(got / one));
        if drive > 0.0 {
          assert!(db(h3 / one_h3).abs() < 0.2, "3rd harmonic at {oversample}x: {} db", db(h3 / one_h3));
        }
      }
    }
  }

  // what's under 19.2 khz that isn't a harmonic of a hard-driven 9 khz
  // sine (each of its harmonics is above that), against its level, in db
  fn aliasing(oversample: usize) -> f64 {
    let mut core = with(oversample);
    let freq = 9000.0;
    let x = sine(freq, REFERENCE * 2.0, 48_000);
    let mut y = vec![0.0; x.len()];
    core.process(&x, 18.0, &mut y);
    let y = &y[24_000..];
    let fundamental = level(y, freq);
    // aliases land on multiples of 3 khz here (48 and 9 share it; the
    // curve's asymmetry makes even harmonics too): sum the ones under
    // 19.2 khz that aren't 9 or 18 khz
    let mut alias = 0.0;
    for k in 1..=6 {
      let f = k as f32 * 3000.0;
      if f != 9000.0 && f != 18_000.0 {
        alias += level(y, f).powi(2);
      }
    }
    db(alias.sqrt() / fundamental)
  }

  #[test]
  fn oversampling_cuts_the_aliasing_of_a_hard_driven_high_sine() {
    let (one, two, four) = (aliasing(1), aliasing(2), aliasing(4));
    eprintln!("aliasing: 1x {one:.1} db, 2x {two:.1} db, 4x {four:.1} db");
    assert!(two < one - 10.0, "1x {one} db, 2x {two} db");
    assert!(four < two - 6.0, "2x {two} db, 4x {four} db");
  }

  #[test]
  fn the_squash_takes_the_same_time_at_any_factor() {
    // a hot tone switched on: how far its level has settled after 10 ms
    // and 150 ms is the same oversampled or not (the follower's time is in
    // seconds, not samples)
    let settle = |oversample| {
      let mut core = with(oversample);
      let x = sine(500.0, REFERENCE * 4.0, 9600);
      let mut y = vec![0.0; x.len()];
      core.process(&x, 6.0, &mut y);
      // a window of 10 cycles at 10 ms and at 150 ms
      (level(&y[480..1440], 500.0), level(&y[7200..8160], 500.0))
    };
    let (early, late) = settle(1);
    for oversample in [2, 4] {
      let (e, l) = settle(oversample);
      assert!(db(e / early).abs() < 0.1 && db(l / late).abs() < 0.1, "{oversample}x: {} db, {} db", db(e / early), db(l / late));
    }
    // (and it does squash: later is quieter)
    assert!(late < early * 0.95);
  }

  #[test]
  fn changing_settings_keeps_going() {
    let mut core = with(2);
    let x = sine(440.0, REFERENCE, 4800);
    let mut y = vec![0.0; x.len()];
    core.process(&x, 0.0, &mut y);
    let settings = SimpleCoreSettings { hf_loss_stages: 3, oversample: 4, headroom: 1.5, ..Default::default() };
    core.set_settings(settings);
    assert_eq!(core.settings(), &settings);
    core.process(&x, 3.0, &mut y);
    assert!(y.iter().all(|v| v.is_finite() && v.abs() < 1.0));
    core.set_settings(SimpleCoreSettings { hf_loss_stages: 99, oversample: 3, ..Default::default() });
    assert_eq!(core.oversampler.factor(), 4);
    assert_eq!(core.stages(), MAX_HF_STAGES);
    core.process(&x, 3.0, &mut y);
    assert!(y.iter().all(|v| v.is_finite()));
  }

  #[test]
  fn reset_starts_again_from_silence() {
    let mut core = with(4);
    let x = sine(440.0, REFERENCE * 3.0, 3000);
    let (mut first, mut again) = (vec![0.0; x.len()], vec![0.0; x.len()]);
    core.process(&x, 6.0, &mut first);
    core.reset();
    core.process(&x, 6.0, &mut again);
    assert_eq!(first, again);
  }

  #[test]
  fn any_block_length_gives_the_same_sound() {
    let x = sine(2500.0, REFERENCE * 3.0, 3000);
    let mut whole = vec![0.0; x.len()];
    SimpleCore::new(Default::default(), RATE, 100).process(&x, 9.0, &mut whole);
    let mut core = SimpleCore::new(Default::default(), RATE, 100);
    let mut pieces = vec![0.0; x.len()];
    let mut at = 0;
    for len in [1, 99, 100, 101, 7, 1000].iter().cycle() {
      let end = (at + len).min(x.len());
      core.process(&x[at..end], 9.0, &mut pieces[at..end]);
      at = end;
      if at == x.len() {
        break;
      }
    }
    assert_eq!(whole, pieces);
  }

  fn tiny_network(rate: u32) -> Arc<GruWeights> {
    let h = 4;
    Arc::new(GruWeights {
      hidden: h,
      w_ih: vec![0.1; 6 * h],
      w_hh: vec![0.05; 3 * h * h],
      b_ih: vec![0.0; 3 * h],
      b_hh: vec![0.0; 3 * h],
      dense_w: vec![0.2; h],
      dense_b: 0.0,
      drive_scale: 1.0 / 18.0,
      rate,
    })
  }

  #[test]
  fn the_neural_core_runs_only_at_its_networks_rate() {
    assert!(NeuralCore::new(tiny_network(48_000), 48_000.0).is_ok());
    assert_eq!(NeuralCore::new(tiny_network(48_000), 44_100.0).err(), Some(CoreError::Rate(48_000, 44_100.0)));
    let mut bad = (*tiny_network(48_000)).clone();
    bad.dense_w.pop();
    assert_eq!(NeuralCore::new(Arc::new(bad), 48_000.0).err(), Some(CoreError::Network(GruError::Shape("dense_w", 3, 4))));
  }

  #[test]
  fn the_neural_core_is_its_network() {
    let weights = tiny_network(48_000);
    let mut core: Box<dyn TapeCore> = Box::new(NeuralCore::new(weights.clone(), 48_000.0).unwrap());
    let mut gru = Gru::new(weights).unwrap();
    let x = sine(300.0, 0.2, 500);
    let (mut a, mut b) = (vec![0.0; x.len()], vec![0.0; x.len()]);
    core.process(&x, 3.0, &mut a);
    gru.process(&x, 3.0, &mut b);
    assert_eq!(a, b);
    core.reset();
    core.process(&x, 3.0, &mut a);
    assert_eq!(a, b);
  }
}
