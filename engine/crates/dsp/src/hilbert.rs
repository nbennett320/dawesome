// turning a sound's phase round, every frequency by the same angle: two
// chains of all-passes whose outputs stay a quarter turn apart across the
// band (Olli Niemitalo's pair), so together they're the sound as a turning
// arrow, and the arrow can be turned by any angle. turned steadily, each
// frequency moves up (or down) by the same number of Hz: a frequency
// shifter. the Flanger's and Phaser's Barberpole turn their copy this way,
// inside the loop, so the comb or the notches slide for ever
//
// both outputs are the sound through all-passes (its level untouched, its
// phase not): what's mixed with the turned copy goes through the same
// first chain (`Rotator::align`), so the two still line up
//
// at the very bottom and top of the band the two aren't a quarter turn
// apart (at 0 Hz and at half the rate they're the same), and turned there
// a sound comes out up to √2 louder: fed back at 95%, it'd run away. so
// what's turned is cut under 20 Hz and over 45% of the rate

use crate::svf::{Coefs, Shape, State, TINY};

// each section is (a² - z⁻²) / (1 - a² z⁻²): these are the a's
const FIRST: [f32; 4] = [0.692_387_8, 0.936_065_4, 0.988_229_5, 0.998_748_8];
const SECOND: [f32; 4] = [0.402_192_1, 0.856_171_1, 0.972_291, 0.995_288_5];

#[derive(Clone, Copy, Default)]
struct Section {
  x1: f32,
  x2: f32,
  y1: f32,
  y2: f32,
}

impl Section {
  #[inline]
  fn run(&mut self, a2: f32, x: f32) -> f32 {
    let y = a2 * (x + self.y2) - self.x2;
    self.x2 = self.x1;
    self.x1 = x;
    self.y2 = self.y1;
    self.y1 = y;
    y
  }

  fn flush(&mut self) {
    for v in [&mut self.x1, &mut self.x2, &mut self.y1, &mut self.y2] {
      if v.abs() < TINY { *v = 0.; }
    }
  }
}

#[derive(Clone, Copy, Default)]
struct Chain {
  sections: [Section; 4],
}

impl Chain {
  #[inline]
  fn run(&mut self, coefs: &[f32; 4], x: f32) -> f32 {
    let mut y = x;
    for (s, a) in self.sections.iter_mut().zip(coefs) {
      y = s.run(a * a, y);
    }
    y
  }
}

// what's turned is cut under and over these
const LOWEST: f32 = 20.;
const HIGHEST: f32 = 0.45;

// one sound's pair of chains
#[derive(Clone, Copy)]
pub struct Rotator {
  first: Chain,
  second: Chain,
  // the first chain's last output: it's taken a sample late
  late: f32,
  // the cuts on what's turned
  edges: (Coefs, Coefs),
  low: State,
  high: State,
}

impl Rotator {
  pub fn new(rate: f32) -> Rotator {
    let mut r = Rotator {
      first: Chain::default(),
      second: Chain::default(),
      late: 0.,
      edges: (Coefs::new(Shape::Bell, 1000., 0., 1., rate), Coefs::new(Shape::Bell, 1000., 0., 1., rate)),
      low: State::default(),
      high: State::default(),
    };
    r.set_rate(rate);
    r
  }

  pub fn set_rate(&mut self, rate: f32) {
    self.edges = (
      Coefs::new(Shape::LowCut, LOWEST, 0., 0.707, rate),
      Coefs::new(Shape::HighCut, HIGHEST * rate, 0., 0.707, rate),
    );
  }

  // `x` with its phase turned by the angle whose cosine and sine these are
  // (positive turns raise it), cut at the edges
  #[inline]
  pub fn turn(&mut self, x: f32, cos: f32, sin: f32) -> f32 {
    let (i, q) = self.pair(x);
    let y = self.low.run(&self.edges.0, i * cos + q * sin);
    self.high.run(&self.edges.1, y)
  }

  // the two a quarter turn apart: the first, then the second (which
  // trails it)
  #[inline]
  pub fn pair(&mut self, x: f32) -> (f32, f32) {
    let i = self.late;
    self.late = self.first.run(&FIRST, x);
    (i, self.second.run(&SECOND, x))
  }

  // `x` through the first chain only: as `turn` leaves it at no angle, for
  // what's mixed with a turned sound
  #[inline]
  pub fn align(&mut self, x: f32) -> f32 {
    let i = self.late;
    self.late = self.first.run(&FIRST, x);
    i
  }

  pub fn reset(&mut self) {
    self.first = Chain::default();
    self.second = Chain::default();
    self.late = 0.;
    self.low.reset();
    self.high.reset();
  }

  pub fn flush(&mut self) {
    for s in self.first.sections.iter_mut().chain(self.second.sections.iter_mut()) { s.flush(); }
    if self.late.abs() < TINY { self.late = 0.; }
    self.low.flush();
    self.high.flush();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // how much of `hz` is in `x` (the size of its correlation)
  fn amount(x: &[f32], hz: f32, rate: f32) -> f32 {
    let (mut c, mut s) = (0f32, 0f32);
    for (n, v) in x.iter().enumerate() {
      let w = std::f32::consts::TAU * hz * n as f32 / rate;
      c += v * w.cos();
      s += v * w.sin();
    }
    2. * (c * c + s * s).sqrt() / x.len() as f32
  }

  #[test]
  fn test_its_pair_stays_a_quarter_turn_apart() {
    let rate = 48_000.;
    for hz in [40., 200., 1000., 5000., 15_000.] {
      let mut r = Rotator::new(rate);
      let (mut ii, mut qq, mut iq) = (0f32, 0f32, 0f32);
      for n in 0..48_000 {
        let (i, q) = r.pair((std::f32::consts::TAU * hz * n as f32 / rate).sin());
        if n > 24_000 {
          ii += i * i;
          qq += q * q;
          iq += i * q;
        }
      }
      // as loud as each other, and uncorrelated (90° apart)
      assert!((ii / qq - 1.).abs() < 0.02, "{} Hz: {} vs {}", hz, ii, qq);
      assert!((iq / (ii * qq).sqrt()).abs() < 0.03, "{} Hz: {}", hz, iq / (ii * qq).sqrt());
    }
  }

  #[test]
  fn test_turning_steadily_shifts_a_tone_up_or_down() {
    let rate = 48_000.;
    for (shift, to, away) in [(100., 1100., 900.), (-100., 900., 1100.)] {
      let mut r = Rotator::new(rate);
      let out: Vec<f32> = (0..48_000).map(|n| {
        let t = n as f32 / rate;
        let angle = std::f32::consts::TAU * shift * t;
        r.turn((std::f32::consts::TAU * 1000. * t).sin(), angle.cos(), angle.sin())
      }).collect();
      let (to, away) = (amount(&out[24_000..], to, rate), amount(&out[24_000..], away, rate));
      assert!(to > 0.95 && away < 0.03, "{}: {} vs {}", shift, to, away);
    }
  }

  #[test]
  fn test_turned_never_comes_out_louder_than_it_went_in() {
    // at the edges, where the pair isn't a quarter turn apart, turned by
    // 45° (the worst): the cuts keep it under, so a loop can't run away
    let rate = 48_000.;
    for hz in [0., 2., 10., 23_500.] {
      let mut r = Rotator::new(rate);
      let a = std::f32::consts::FRAC_PI_4;
      let out: Vec<f32> = (0..48_000).map(|n| r.turn((std::f32::consts::TAU * hz * n as f32 / rate).cos(), a.cos(), a.sin())).collect();
      let most = out[24_000..].iter().fold(0f32, |m, x| m.max(x.abs()));
      assert!(most < 1., "{} Hz: {}", hz, most);
    }
  }
}
