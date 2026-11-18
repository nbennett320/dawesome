// how loud something's playing, followed a sample at a time: up quickly
// when it gets louder (attack), down slowly as it fades (release). the
// chorus's Bloom and the flanger's and phaser's Envelope listen with it

use crate::svf::TINY;

#[derive(Clone, Copy, Debug)]
pub struct Envelope {
  level: f32,
  // how much of the gap is left after each sample, rising and falling
  attack: f32,
  release: f32,
}

// how much of the gap is left after a sample, to cover ~63% of it in
// `seconds`
fn keep(seconds: f32, rate: f32) -> f32 {
  (-1. / (seconds.max(1e-5) * rate.max(1.))).exp()
}

impl Envelope {
  pub fn new(attack: f32, release: f32, rate: f32) -> Envelope {
    Envelope { level: 0., attack: keep(attack, rate), release: keep(release, rate) }
  }

  pub fn set_times(&mut self, attack: f32, release: f32, rate: f32) {
    self.attack = keep(attack, rate);
    self.release = keep(release, rate);
  }

  pub fn level(&self) -> f32 {
    self.level
  }

  // take a sample (its size: either sign)
  #[inline]
  pub fn run(&mut self, x: f32) -> f32 {
    let x = x.abs();
    let keep = if x > self.level { self.attack } else { self.release };
    self.level = x + keep * (self.level - x);
    self.level
  }

  pub fn reset(&mut self) {
    self.level = 0.;
  }

  pub fn flush(&mut self) {
    if self.level < TINY { self.level = 0.; }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_it_rises_with_the_attack_and_falls_with_the_release() {
    let rate = 48_000.;
    let mut env = Envelope::new(0.001, 0.1, rate);
    // 1 ms in, about 63% of the way up
    for _ in 0..48 { env.run(1.); }
    assert!((env.level() - 0.632).abs() < 0.01, "{}", env.level());
    for _ in 0..4800 { env.run(-1.); }
    assert!(env.level() > 0.999);
    // 100 ms after it stops, about 37% left
    for _ in 0..4800 { env.run(0.); }
    assert!((env.level() - 0.368).abs() < 0.01, "{}", env.level());
  }
}
