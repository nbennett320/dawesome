// a delay line: one channel's recent sound, read back any time ago,
// between samples too (4-point Hermite). a delay's echoes, and later a
// chorus's and a reverb's lines
//
// its memory is set aside once, when it's made, and never grows: effects
// are told of a new sample rate on the audio thread, which mustn't
// allocate. so a line is made long enough for the longest time it's for at
// the highest rate it's likely to run at (MAX_RATE). at a lower rate it
// goes round only as much of that as it needs (`set_rate`), so the rest is
// never touched
//
// each sample: read what's due (`read`), then write the new one (`write`).
// so a read `d` samples back gets what was written `d` samples before this
// one. `d` is at least MIN_DELAY (the interpolation needs a sample either
// side)

use crate::svf::TINY;

// the highest sample rate a line is made long enough for. above it, the
// longest time it can hold is shorter
pub const MAX_RATE: f32 = 192_000.;

// the shortest delay it reads, in samples
pub const MIN_DELAY: f64 = 2.;

// (the samples either side of the read the interpolation uses)
const MARGIN: usize = 4;

pub struct DelayLine {
  samples: Box<[f32]>,
  // how much of it's in use: it goes round these
  len: usize,
  // how long it's for, in seconds
  seconds: f32,
  // where the next sample is written
  at: usize,
}

impl DelayLine {
  // long enough to hold `seconds` at MAX_RATE, used at `rate`
  pub fn new(seconds: f32, rate: f32) -> DelayLine {
    let len = (seconds * MAX_RATE).round() as usize + MARGIN;
    let mut line = DelayLine { samples: vec![0.; len].into_boxed_slice(), len, seconds, at: 0 };
    line.set_rate(rate);
    line
  }

  // go round as much as `seconds` needs at `rate` (all of it, above
  // MAX_RATE). it's cleared
  pub fn set_rate(&mut self, rate: f32) {
    let len = (self.seconds * rate).round() as usize + MARGIN;
    self.len = len.clamp(MARGIN + 4, self.samples.len());
    self.clear();
  }

  // the longest delay it can read, in samples
  pub fn longest(&self) -> f64 {
    (self.len - MARGIN) as f64
  }

  // forget everything in it
  pub fn clear(&mut self) {
    self.samples[..self.len].fill(0.);
    self.at = 0;
  }

  // the sample written `back` samples before the next one (1: the last)
  #[inline]
  fn ago(&self, back: usize) -> f32 {
    let i = if back <= self.at { self.at - back } else { self.at + self.len - back };
    self.samples[i]
  }

  // what it held `delay` samples ago (clamped to what it can read). in
  // f64: an f32 holds a delay of seconds only to a sixteenth of a sample,
  // too coarse for a slow glide
  #[inline]
  pub fn read(&self, delay: f64) -> f32 {
    let delay = delay.clamp(MIN_DELAY, self.longest());
    let whole = delay as usize;
    let t = (delay - whole as f64) as f32;
    // the sample at the delay, its newer neighbour, and the two older
    let (newer, x0, x1, x2) = (self.ago(whole - 1), self.ago(whole), self.ago(whole + 1), self.ago(whole + 2));
    // 4-point, 3rd-order Hermite, from x0 (t = 0) toward x1 (t = 1)
    let c1 = 0.5 * (x1 - newer);
    let c2 = newer - 2.5 * x0 + 2. * x1 - 0.5 * x2;
    let c3 = 0.5 * (x2 - newer) + 1.5 * (x0 - x1);
    ((c3 * t + c2) * t + c1) * t + x0
  }

  // put the next sample in (what's too small to hear as 0: a line holds
  // what fades out for seconds, and would hold it as slow denormals)
  #[inline]
  pub fn write(&mut self, x: f32) {
    self.samples[self.at] = if x.abs() < TINY { 0. } else { x };
    self.at += 1;
    if self.at == self.len { self.at = 0; }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_it_reads_back_what_was_written_that_long_ago() {
    let mut line = DelayLine::new(0.001, MAX_RATE);
    let mut heard = Vec::new();
    for i in 0..400 {
      heard.push(line.read(10.));
      line.write(i as f32);
    }
    // (zeros until the first write is 10 samples back)
    assert_eq!(&heard[..10], &[0.; 10]);
    assert_eq!(heard[10], 0.);
    assert_eq!(heard[11], 1.);
    // and round the end of its memory, more than once
    assert_eq!(heard[399], 389.);
  }

  #[test]
  fn test_between_samples_it_follows_the_curve() {
    // a slow sine, read half a sample later: Hermite lands on it closely
    let mut line = DelayLine::new(0.01, MAX_RATE);
    let wave = |n: f32| (n * 0.05).sin();
    for n in 0..100 { line.write(wave(n as f32)); }
    // the next sample would be n = 100; 10.5 back is n = 89.5
    let got = line.read(10.5);
    assert!((got - wave(89.5)).abs() < 1e-5, "{} vs {}", got, wave(89.5));
  }

  #[test]
  fn test_it_reads_no_further_back_than_it_holds() {
    let line = DelayLine::new(0.001, MAX_RATE);
    assert_eq!(line.longest(), 192.);
    assert_eq!(line.read(1e9), line.read(192.));
    assert_eq!(line.read(0.), line.read(MIN_DELAY));
  }

  #[test]
  fn test_at_a_lower_rate_it_goes_round_only_what_it_needs() {
    let mut line = DelayLine::new(0.001, 48_000.);
    assert_eq!(line.longest(), 48.);
    let mut heard = Vec::new();
    for i in 0..200 {
      heard.push(line.read(48.));
      line.write(i as f32);
    }
    assert_eq!(heard[199], 151.);
    // and all of it, back at the highest rate
    line.set_rate(MAX_RATE);
    assert_eq!(line.longest(), 192.);
  }

  #[test]
  fn test_clearing_it_forgets_everything() {
    let mut line = DelayLine::new(0.001, MAX_RATE);
    for _ in 0..50 { line.write(1.); }
    line.clear();
    assert!((2..190).all(|d| line.read(d as f64) == 0.));
  }

  #[test]
  fn test_tiny_values_go_in_as_silence() {
    let mut line = DelayLine::new(0.001, MAX_RATE);
    line.write(1e-25);
    line.write(0.);
    assert_eq!(line.read(2.), 0.);
  }
}
