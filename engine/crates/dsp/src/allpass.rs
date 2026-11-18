// a first-order all-pass: every frequency through at the same level, each
// delayed by a different share of a cycle (none far below its frequency,
// half a cycle far above, a quarter at it). a phaser's stages: mixed with
// the dry sound, each pair of them cancels it at one frequency, a notch
//
// it's the trapezoidal (TPT) one-pole, as the SVF (svf.rs) is made: its
// frequency can move every sample, as a phaser's sweep moves it, without
// zipper noise or blowing up

use std::f32::consts::PI;

use crate::svf::{MAX_SHARE, TINY};

// what a frequency works out to: the one-pole's gain, G = g / (1 + g)
// with g = tan(pi f / rate). worked out once, shared by every stage at
// that frequency
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coef {
  g: f32,
  big_g: f32,
}

impl Coef {
  pub fn new(freq: f32, rate: f32) -> Coef {
    let g = (PI * (freq / rate).clamp(1e-6, MAX_SHARE)).tan();
    Coef { g, big_g: g / (1. + g) }
  }

  // how far it delays `freq`, as a phase (radians, 0 to -pi)
  pub fn phase(&self, freq: f32, rate: f32) -> f32 {
    let w = (PI * (freq / rate).clamp(0., 0.5)).tan();
    -2. * (w / self.g).atan()
  }
}

// one channel's memory
#[derive(Clone, Copy, Debug, Default)]
pub struct Allpass {
  s: f32,
}

impl Allpass {
  #[inline]
  pub fn run(&mut self, c: &Coef, x: f32) -> f32 {
    let v = (x - self.s) * c.big_g;
    let low = v + self.s;
    self.s = low + v;
    2. * low - x
  }

  pub fn reset(&mut self) {
    self.s = 0.;
  }

  // left out of a chain, but ready to come in near the top of the
  // spectrum: where its memory would settle there, with `x` going in (a
  // one-pole that high follows its input), so it joins without a jump
  #[inline]
  pub fn rest(&mut self, x: f32) {
    self.s = x;
  }

  // (see svf::State::flush)
  pub fn flush(&mut self) {
    if self.s.abs() < TINY { self.s = 0.; }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const RATE: f32 = 48_000.;

  // a sine through it: its level, and how far it's delayed (radians)
  fn measure(c: &Coef, freq: f32) -> (f32, f32) {
    let mut ap = Allpass::default();
    let frames = 48_000;
    let (mut re, mut im, mut level) = (0f64, 0f64, 0f32);
    for i in 0..frames {
      let t = std::f32::consts::TAU * freq * i as f32 / RATE;
      let y = ap.run(c, t.sin());
      if i >= frames / 2 {
        re += (y * t.sin()) as f64;
        im += (y * t.cos()) as f64;
        level = level.max(y.abs());
      }
    }
    (level, im.atan2(re) as f32)
  }

  #[test]
  fn test_it_passes_every_frequency_at_the_same_level() {
    let c = Coef::new(1000., RATE);
    for freq in [50., 300., 1000., 4000., 15_000.] {
      let (level, _) = measure(&c, freq);
      assert!((level - 1.).abs() < 0.01, "{} Hz comes out at {}", freq, level);
    }
  }

  #[test]
  fn test_it_delays_its_own_frequency_a_quarter_cycle_as_phase_says() {
    let c = Coef::new(1000., RATE);
    assert!((c.phase(1000., RATE) + PI / 2.).abs() < 1e-4);
    for freq in [100., 1000., 5000.] {
      let (_, measured) = measure(&c, freq);
      let expected = c.phase(freq, RATE);
      let mut gap = measured - expected;
      gap -= (gap / std::f32::consts::TAU).round() * std::f32::consts::TAU;
      assert!(gap.abs() < 0.01, "{} Hz: {} measured, {} worked out", freq, measured, expected);
    }
  }

  #[test]
  fn test_it_stays_quiet_and_bounded_while_its_frequency_sweeps_every_sample() {
    let mut ap = Allpass::default();
    for i in 0..96_000 {
      let freq = 100. * 2f32.powf(7. * (0.5 + 0.5 * (i as f32 / 2000.).sin()));
      let c = Coef::new(freq, RATE);
      let x = (std::f32::consts::TAU * 220. * i as f32 / RATE).sin();
      let y = ap.run(&c, x);
      assert!(y.is_finite() && y.abs() < 3., "{} at {}", y, i);
    }
  }
}
