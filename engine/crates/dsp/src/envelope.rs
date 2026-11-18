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

// an attack in the sound: a quick follower jumping well over a slow one
// (by `over` times) and over a floor. then not again until the quick one
// has fallen back under (the note's settled), and at least HOLD on. what
// the Filter's Envelope mode fires on
pub struct Transient {
  fast: Envelope,
  slow: Envelope,
  hold: u32,
  hold_frames: u32,
  armed: bool,
}

// the least time between two attacks
const HOLD: f32 = 0.03;

impl Transient {
  pub fn new(rate: f32) -> Transient {
    let mut t = Transient { fast: Envelope::new(0.0005, 0.02, rate), slow: Envelope::new(0.03, 0.08, rate), hold: 0, hold_frames: 0, armed: true };
    t.set_rate(rate);
    t
  }

  pub fn set_rate(&mut self, rate: f32) {
    self.fast.set_times(0.0005, 0.02, rate);
    // (it lets go quickly, so a note struck again soon after counts)
    self.slow.set_times(0.03, 0.08, rate);
    self.hold_frames = (HOLD * rate) as u32;
  }

  // take a sample: whether an attack starts here
  #[inline]
  pub fn run(&mut self, x: f32, over: f32, floor: f32) -> bool {
    let fast = self.fast.run(x);
    let slow = self.slow.run(x);
    let over_it = fast > floor && fast > over * slow;
    if self.hold > 0 {
      self.hold -= 1;
      return false;
    }
    if !self.armed {
      self.armed = !over_it;
      return false;
    }
    if over_it {
      self.hold = self.hold_frames;
      self.armed = false;
    }
    over_it
  }

  // how loud it is now (the quick follower)
  pub fn level(&self) -> f32 {
    self.fast.level()
  }

  pub fn reset(&mut self) {
    self.fast.reset();
    self.slow.reset();
    self.hold = 0;
    self.armed = true;
  }

  pub fn flush(&mut self) {
    self.fast.flush();
    self.slow.flush();
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

  #[test]
  fn test_an_attack_fires_once_and_a_held_note_doesnt() {
    let rate = 48_000.;
    let mut t = Transient::new(rate);
    let mut hits = Vec::new();
    // silence, a struck note ringing 0.4 s, then another
    for i in 0..48_000 {
      let on = (4800..24_000).contains(&i) || i >= 28_800;
      let x = if on { 0.5 * (i as f32 * 0.05).sin() } else { 0. };
      if t.run(x, 2., 0.01) { hits.push(i); }
    }
    assert_eq!(hits.len(), 2, "{:?}", hits);
    assert!(hits[0] >= 4800 && hits[0] < 4900, "{:?}", hits);
    assert!(hits[1] >= 28_800 && hits[1] < 28_900, "{:?}", hits);
    // quiet sound under the floor fires nothing
    let mut t = Transient::new(rate);
    assert!(!(0..9600).any(|i| t.run(if i > 4800 { 0.001 } else { 0. }, 2., 0.01)));
  }
}
