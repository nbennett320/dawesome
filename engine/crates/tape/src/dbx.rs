// dbx type ii-style noise reduction: a 2:1 compressor on the way onto the
// tape (DbxEncoder, the record side) and a 1:2 expander on the way off
// (DbxDecoder, the playback side). quiet passages go onto the tape louder
// and come back turned down, so the tape's hiss is turned down with them;
// under loud passages it isn't, and the hiss rises and falls with the
// music (the pumping dbx is known for). hiss and dropouts go in between
// the two (D8), which is why they're split
//
// what each does, per sample:
//
//   encoder  x ─► pre-emphasis ─► × g ─► y        g = ref / rms(w(y))
//   decoder  y ─► × 1/g ─► de-emphasis ─► x       the same g, from its input
//
// w() is the detector's weighting (it listens mostly to the highs, where
// hiss is heard), rms() a smoothed rms. the encoder listens to its own
// output (feedback: in steady state its output's level in db is half its
// input's, about ref) and the decoder to its input, so both detectors hear
// the same signal, the tape's, and with nothing in between the decoder
// undoes the encoder exactly. whatever the tape does in between (its
// saturation, its highs, a level error, a dropout) the decoder mistracks
// by, as the real one does: a level error comes back doubled
//
// what's real and what's guessed:
// - real: 2:1 in db around a pivot, decoded 1:2; rms detection;
//   pre-emphasis on the signal and a heavier high weighting in the
//   detector; a limited gain range
// - guessed (tune by ear in A4, check against the dbx-on passes in B3):
//   the emphasis corners (+10 db from 1.5 khz), the detector's weighting
//   (a 40 hz low cut, +12 db from 1.5 khz), its times (3 ms up, 30 ms
//   down; the real detector's are a single rms time), the pivot (0 vu)
//   and the gain range (+30 db to -24 db)
// - the real chip's topology isn't copied: this pair is the one that makes
//   the two exact inverses, which the real one is designed to be too
//
// nothing allocates

// 0 vu as an rms: the -18 dbfs peak sine the test file calibrates with
pub const VU0: f32 = 0.089_019;

// the signal's emphasis: a first-order high shelf, +10 db above
const EMPHASIS_HZ: (f32, f32) = (1_500.0, 4_743.0);
// the detector's weighting: a low cut, then a +12 db high shelf
const WEIGHT_LOW_CUT_HZ: f32 = 40.0;
const WEIGHT_HZ: (f32, f32) = (1_500.0, 6_000.0);
// how fast the detector rises and falls (one-pole on the power)
const ATTACK_SECONDS: f32 = 0.003;
const RELEASE_SECONDS: f32 = 0.030;
// the gain range: most boost (quiet passages) and most cut (loud)
const MOST_BOOST_DB: f32 = 30.0;
const MOST_CUT_DB: f32 = 24.0;

// a first-order filter (one pole, one zero)
#[derive(Clone, Copy, Debug)]
struct FirstOrder {
  b0: f32,
  b1: f32,
  a1: f32,
}

impl FirstOrder {
  // a shelf: flat, then rising 6 db/octave from `zero` hz to `pole` hz,
  // flat again above (bilinear, each corner pre-warped). the zero is
  // inside the unit circle, so its inverse is stable
  fn shelf(zero: f32, pole: f32, rate: f32) -> FirstOrder {
    let k = 2.0 * rate as f64;
    let warp = |f: f32| k * (std::f64::consts::PI * f.min(rate * 0.45) as f64 / k).tan();
    let (z, p) = (k / warp(zero), k / warp(pole));
    let a0 = 1.0 + p;
    FirstOrder { b0: ((1.0 + z) / a0) as f32, b1: ((1.0 - z) / a0) as f32, a1: ((1.0 - p) / a0) as f32 }
  }

  fn low_cut(hz: f32, rate: f32) -> FirstOrder {
    let k = 2.0 * rate as f64;
    let w = k * (std::f64::consts::PI * hz as f64 / k).tan();
    FirstOrder { b0: (k / (k + w)) as f32, b1: (-k / (k + w)) as f32, a1: ((w - k) / (k + w)) as f32 }
  }

  // the filter that undoes this one
  fn inverse(&self) -> FirstOrder {
    FirstOrder { b0: 1.0 / self.b0, b1: self.a1 / self.b0, a1: self.b1 / self.b0 }
  }

  // (direct form 1: the state is the last input and output)
  #[inline]
  fn run(&self, s: &mut [f32; 2], x: f32) -> f32 {
    let y = self.b0 * x + self.b1 * s[0] - self.a1 * s[1];
    *s = [x, if y.abs() < 1e-30 { 0.0 } else { y }];
    y
  }
}

// the smoothed, weighted rms both sides listen with
#[derive(Clone, Copy, Debug)]
struct Detector {
  low_cut: FirstOrder,
  shelf: FirstOrder,
  state: [[f32; 2]; 2],
  // the mean square
  power: f32,
  attack: f32,
  release: f32,
  // the rms range the gain law is held to
  least: f32,
  most: f32,
}

impl Detector {
  fn new(rate: f32) -> Detector {
    let one_pole = |seconds: f32| 1.0 - (-1.0 / (seconds * rate)).exp();
    let least = VU0 * 10f32.powf(-MOST_BOOST_DB / 20.0);
    Detector {
      low_cut: FirstOrder::low_cut(WEIGHT_LOW_CUT_HZ, rate),
      shelf: FirstOrder::shelf(WEIGHT_HZ.0, WEIGHT_HZ.1, rate),
      state: [[0.0; 2]; 2],
      power: least * least,
      attack: one_pole(ATTACK_SECONDS),
      release: one_pole(RELEASE_SECONDS),
      least,
      most: VU0 * 10f32.powf(MOST_CUT_DB / 20.0),
    }
  }

  fn reset(&mut self) {
    self.state = [[0.0; 2]; 2];
    // as after a long silence
    self.power = self.least * self.least;
  }

  // the level the gain follows, now (before this sample)
  #[inline]
  fn rms(&self) -> f32 {
    self.power.sqrt().clamp(self.least, self.most)
  }

  // hear the tape's signal
  #[inline]
  fn listen(&mut self, y: f32) {
    let w = self.low_cut.run(&mut self.state[0], y);
    let w = self.shelf.run(&mut self.state[1], w);
    let p = w * w;
    let k = if p > self.power { self.attack } else { self.release };
    // (held above zero: it would decay into slow denormals in silence)
    self.power = (self.power + k * (p - self.power)).max(1e-20);
  }
}

// the record side's half: 2:1 compression, highs lifted
#[derive(Clone, Copy, Debug)]
pub struct DbxEncoder {
  emphasis: FirstOrder,
  state: [f32; 2],
  detector: Detector,
}

impl DbxEncoder {
  pub fn new(rate: f32) -> DbxEncoder {
    DbxEncoder {
      emphasis: FirstOrder::shelf(EMPHASIS_HZ.0, EMPHASIS_HZ.1, rate),
      state: [0.0; 2],
      detector: Detector::new(rate),
    }
  }

  pub fn reset(&mut self) {
    self.state = [0.0; 2];
    self.detector.reset();
  }

  #[inline]
  pub fn step(&mut self, x: f32) -> f32 {
    let x = self.emphasis.run(&mut self.state, x);
    let y = x * (VU0 / self.detector.rms());
    self.detector.listen(y);
    y
  }

  pub fn process(&mut self, x: &[f32], out: &mut [f32]) {
    for (o, &v) in out.iter_mut().zip(x) {
      *o = self.step(v);
    }
  }
}

// the playback side's half: 1:2 expansion, highs put back
#[derive(Clone, Copy, Debug)]
pub struct DbxDecoder {
  de_emphasis: FirstOrder,
  state: [f32; 2],
  detector: Detector,
}

impl DbxDecoder {
  pub fn new(rate: f32) -> DbxDecoder {
    DbxDecoder {
      de_emphasis: FirstOrder::shelf(EMPHASIS_HZ.0, EMPHASIS_HZ.1, rate).inverse(),
      state: [0.0; 2],
      detector: Detector::new(rate),
    }
  }

  pub fn reset(&mut self) {
    self.state = [0.0; 2];
    self.detector.reset();
  }

  #[inline]
  pub fn step(&mut self, y: f32) -> f32 {
    let x = y * (self.detector.rms() / VU0);
    self.detector.listen(y);
    self.de_emphasis.run(&mut self.state, x)
  }

  pub fn process(&mut self, y: &[f32], out: &mut [f32]) {
    for (o, &v) in out.iter_mut().zip(y) {
      *o = self.step(v);
    }
  }

  pub fn process_in_place(&mut self, buf: &mut [f32]) {
    for v in buf.iter_mut() {
      *v = self.step(*v);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::f32::consts::TAU;

  const RATE: f32 = 48_000.0;

  fn db(x: f32) -> f32 {
    VU0 * 10f32.powf(x / 20.0) * std::f32::consts::SQRT_2
  }

  fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt() as f32
  }

  fn level_db(x: &[f32]) -> f32 {
    20.0 * (rms(x) / VU0).log10()
  }

  // something like music: a chord whose notes come and go, a bass line, a
  // hat's noise bursts, a loud hit, silences, and levels from -50 to +12
  // db re 0 vu
  fn music(seconds: f32) -> Vec<f32> {
    let n = (seconds * RATE) as usize;
    let mut seed = 7u32;
    let mut noise = move || {
      seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
      (seed >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    };
    (0..n)
      .map(|i| {
        let t = i as f32 / RATE;
        let section = (t * 0.5) as usize % 6;
        let level = [-50.0, -30.0, -12.0, 0.0, 12.0, -20.0][section];
        let beat = (t * 4.0).fract();
        let env = (-beat * 6.0).exp();
        let chord = [220.0, 277.2, 329.6, 440.0, 1_318.5].iter().enumerate().map(|(k, f)| (TAU * f * t).sin() * (0.6 + 0.4 * (TAU * (0.3 + k as f32 * 0.17) * t).sin())).sum::<f32>() * 0.25;
        let bass = (TAU * 55.0 * t).sin() * env;
        let hat = if beat < 0.03 { noise() * 0.7 } else { 0.0 };
        let gap = if (t * 0.7).fract() > 0.92 { 0.0 } else { 1.0 };
        (chord + bass + hat) * db(level) * 0.5 * gap
      })
      .collect()
  }

  #[test]
  fn decoding_undoes_encoding() {
    let x = music(12.0);
    let (mut enc, mut dec) = (DbxEncoder::new(RATE), DbxDecoder::new(RATE));
    let mut y = vec![0.0; x.len()];
    enc.process(&x, &mut y);
    let mut back = vec![0.0; x.len()];
    dec.process(&y, &mut back);
    // the error against what went in, 50 ms at a time, where there's
    // anything to hear
    let w = (0.05 * RATE) as usize;
    let mut worst = f32::MIN;
    for (a, b) in x.chunks(w).zip(back.chunks(w)) {
      if level_db(a) < -60.0 {
        continue;
      }
      let err: Vec<f32> = a.iter().zip(b).map(|(p, q)| p - q).collect();
      worst = worst.max(20.0 * (rms(&err) / rms(a)).log10());
    }
    println!("round trip: the worst 50 ms's error is {worst:.1} db under the signal");
    assert!(worst < -100.0, "{worst}");
  }

  // a dull tape between (a one-pole at 10 khz): the decoder hears less of
  // the highs than the encoder did, and mistracks a little where the
  // music is bright
  #[test]
  fn a_dull_tape_between_mistracks_a_little() {
    let x = music(12.0);
    let (mut enc, mut dec) = (DbxEncoder::new(RATE), DbxDecoder::new(RATE));
    let mut y = vec![0.0; x.len()];
    enc.process(&x, &mut y);
    let a = 1.0 - (-TAU * 10_000.0 / RATE).exp();
    let (mut s, mut t) = (0.0, 0.0);
    let dull = |v: &mut f32, s: &mut f32| {
      *s += a * (*v - *s);
      *v = *s;
    };
    let mut back = vec![0.0; x.len()];
    y.iter_mut().for_each(|v| dull(v, &mut s));
    dec.process(&y, &mut back);
    // against the input through the same dulling, 50 ms at a time
    let mut want = x.clone();
    want.iter_mut().for_each(|v| dull(v, &mut t));
    let w = (0.05 * RATE) as usize;
    let mut off: Vec<f32> = want.chunks(w).zip(back.chunks(w)).skip(2).filter(|(p, _)| level_db(p) > -60.0).map(|(p, q)| (level_db(q) - level_db(p)).abs()).collect();
    off.sort_by(f32::total_cmp);
    let (median, worst) = (off[off.len() / 2], off[off.len() - 1]);
    println!("through a dull tape, 50 ms levels are off by {median:.2} db (median), {worst:.2} db at worst");
    assert!(median < 0.5 && worst < 6.0, "{median} {worst}");
  }

  // a level error on the tape comes back doubled
  #[test]
  fn a_level_error_between_comes_back_doubled() {
    let n = (2.0 * RATE) as usize;
    let x: Vec<f32> = (0..n).map(|i| (TAU * 1_000.0 * i as f32 / RATE).sin() * db(-20.0)).collect();
    let (mut enc, mut dec) = (DbxEncoder::new(RATE), DbxDecoder::new(RATE));
    let mut y = vec![0.0; n];
    enc.process(&x, &mut y);
    y.iter_mut().for_each(|v| *v *= 10f32.powf(-1.0 / 20.0));
    let mut back = vec![0.0; n];
    dec.process(&y, &mut back);
    let change = level_db(&back[n / 2..]) - level_db(&x[n / 2..]);
    assert!((change + 2.0).abs() < 0.1, "{change}");
  }

  #[test]
  fn it_compresses_two_to_one() {
    let n = (1.0 * RATE) as usize;
    let mut levels = Vec::new();
    for input in [-40.0, -20.0, 0.0, 10.0] {
      let x: Vec<f32> = (0..n).map(|i| (TAU * 1_000.0 * i as f32 / RATE).sin() * db(input)).collect();
      let mut enc = DbxEncoder::new(RATE);
      let mut y = vec![0.0; n];
      enc.process(&x, &mut y);
      levels.push(level_db(&y[n / 2..]));
    }
    for (pair, step) in levels.windows(2).zip([20.0, 20.0, 10.0]) {
      let ratio = step / (pair[1] - pair[0]);
      assert!((ratio - 2.0).abs() < 0.05, "{levels:?}");
    }
  }

  // hiss added between the two is pushed down under quiet passages and
  // rises under loud ones
  #[test]
  fn hiss_between_is_pushed_down_and_pumps() {
    let n = (2.0 * RATE) as usize;
    let mut seed = 3u32;
    let hiss: Vec<f32> = (0..n)
      .map(|_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((seed >> 8) as f32 / (1u32 << 23) as f32 - 1.0) * 3f32.sqrt() * VU0 * 10f32.powf(-50.0 / 20.0)
      })
      .collect();
    let mut out = Vec::new();
    for level in [None, Some(-40.0), Some(-20.0), Some(0.0), Some(10.0)] {
      let x: Vec<f32> = (0..n).map(|i| level.map_or(0.0, |l| (TAU * 1_000.0 * i as f32 / RATE).sin() * db(l))).collect();
      let (mut enc, mut dec, mut clean) = (DbxEncoder::new(RATE), DbxDecoder::new(RATE), DbxDecoder::new(RATE));
      let mut y = vec![0.0; n];
      enc.process(&x, &mut y);
      let noisy: Vec<f32> = y.iter().zip(&hiss).map(|(a, b)| a + b).collect();
      let (mut a, mut b) = (vec![0.0; n], vec![0.0; n]);
      dec.process(&noisy, &mut a);
      clean.process(&y, &mut b);
      let left: Vec<f32> = a.iter().zip(&b).map(|(p, q)| p - q).collect();
      out.push(level_db(&left[n / 2..]));
    }
    println!("hiss at -50 db re 0 vu, after decoding: silence {:.1}, under -40 {:.1}, -20 {:.1}, 0 {:.1}, +10 {:.1} db", out[0], out[1], out[2], out[3], out[4]);
    // pushed far down in silence, and well down under quiet passages
    assert!(out[0] < -75.0 && out[1] < -65.0, "{out:?}");
    // and it rises with the music
    assert!(out.windows(2).all(|p| p[1] > p[0] + 3.0), "{out:?}");
  }
}
