// scrubbing (part S): a tape rocked over the heads by hand, or wound past
// them. it reads a recording kept in memory (mono, 16-bit, at a rate of its
// own: the DAW's scrub proxy) at a moving speed, either way
//
// - following a drag: the tape has mass, so it's pulled towards where the
//   drag is by a spring and held back by a damper, critically damped: a
//   quick flick runs on a little, a held drag settles still
// - winding: the motor drives it at a speed (forwards or backwards), getting
//   there at a steady rate as a flywheel does
//
// read with the playback side's sinc up to about 2x (nothing in the
// recording can alias below that), and averaged over the stretch the tape
// moved past each frame above it (a box filter: cheap, and enough against
// aliasing at winding speeds), the two crossfaded between 1.5x and 2.5x so
// the change doesn't click. faster than 1x it's low-passed (a head off the
// tape as it winds), and below 0.1x it fades with the speed: a still tape
// is silent, as a head's output falls with the tape's speed
//
// nothing allocates after new()

use crate::playback::{sinc_read, table, TAPS};

const HALF: usize = TAPS / 2;
// how quickly the tape follows a drag: the spring's natural frequency (a
// time constant of about 40 ms)
const FOLLOW_HZ: f64 = 4.0;
// the fastest the tape's moved, in multiples of play speed
pub const MAX_SPEED: f64 = 16.0;
// how fast winding gets up to speed and back, in multiples of play speed a
// second (10x in a third of a second)
pub const WIND_ACCEL: f64 = 30.0;
// winding is a little quieter (-6 dB), as a head lifted off the tape is
pub const WIND_LEVEL: f64 = 0.5;
// under this speed the head's output fades with the speed
const QUIET_BELOW: f64 = 0.1;
// how fast the level moves (fading in and out, the wind's level)
const LEVEL_SECONDS: f64 = 0.01;
// the sinc up to here, the box average from here, in multiples of play speed
const BOX_FROM: f64 = 1.5;
const BOX_TO: f64 = 2.5;
// faster than 1x, the low-pass's corner is this over the speed, no lower
// than LOWEST_HZ
const LOSS_HZ: f64 = 12_000.0;
const LOWEST_HZ: f64 = 1_500.0;
// the low-pass's corner is worked out this often (frames)
const STEP: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drive {
  // pulled towards a place, in seconds of the recording
  Follow(f64),
  // driven at a speed, in multiples of play speed (negative winds back)
  Wind(f64),
}

pub struct Scrubber {
  drive: Drive,
  // where the tape is (seconds of the recording) and how fast it's moving
  // (seconds a second: multiples of play speed)
  at: f64,
  speed: f64,
  // the recording's rate and the output's
  rate: f64,
  out_rate: f64,
  // the level now, and whether it's fading out to end
  level: f64,
  ending: bool,
  // the head-loss low-pass
  lp: f32,
  lp_a: f32,
  table: &'static [f32],
}

impl Scrubber {
  // - rate: the recording's; out_rate: the output's
  // - at: where the tape starts, in seconds of the recording
  pub fn new(rate: f64, out_rate: f64, at: f64, drive: Drive) -> Scrubber {
    Scrubber {
      drive,
      at,
      speed: 0.0,
      rate,
      out_rate,
      level: 0.0,
      ending: false,
      lp: 0.0,
      lp_a: 1.0,
      table: table(),
    }
  }

  pub fn set_drive(&mut self, drive: Drive) {
    if !self.ending {
      self.drive = drive;
    }
  }

  pub fn drive(&self) -> Drive {
    self.drive
  }

  // where the tape is, in seconds of the recording
  pub fn at(&self) -> f64 {
    self.at
  }

  // how fast it's moving, in multiples of play speed
  pub fn speed(&self) -> f64 {
    self.speed
  }

  // let go: it fades out, then it's done
  pub fn end(&mut self) {
    self.ending = true;
  }

  pub fn done(&self) -> bool {
    self.ending && self.level == 0.0
  }

  // one frame on: the tape's motion
  #[inline]
  fn step(&mut self, dt: f64) {
    match self.drive {
      Drive::Follow(target) => {
        let w = 2.0 * std::f64::consts::PI * FOLLOW_HZ;
        let a = w * w * (target - self.at) - 2.0 * w * self.speed;
        self.speed = (self.speed + a * dt).clamp(-MAX_SPEED, MAX_SPEED);
      }
      Drive::Wind(speed) => {
        let d = speed.clamp(-MAX_SPEED, MAX_SPEED) - self.speed;
        self.speed += d.clamp(-WIND_ACCEL * dt, WIND_ACCEL * dt);
      }
    }
    self.at += self.speed * dt;
  }

  // the recording's sample `i`, silence outside it
  #[inline]
  fn sample(recording: &[i16], i: i64) -> f32 {
    if i < 0 || i as usize >= recording.len() { 0.0 } else { recording[i as usize] as f32 / 32_768.0 }
  }

  // read at `pos` (the recording's frames) with the sinc
  #[inline]
  fn sinc(&self, recording: &[i16], pos: f64) -> f32 {
    let i = pos.floor();
    let u = (pos - i) as f32;
    let first = i as i64 + 1 - HALF as i64;
    let mut taps = [0.0f32; TAPS];
    for (k, tap) in taps.iter_mut().enumerate() {
      *tap = Scrubber::sample(recording, first + k as i64);
    }
    sinc_read(self.table, &taps, u)
  }

  // the average of what the tape moved past between `from` and `to` (the
  // recording's frames, either way)
  #[inline]
  fn swept(recording: &[i16], from: f64, to: f64) -> f32 {
    let (a, b) = if from <= to { (from, to) } else { (to, from) };
    let (first, last) = (a.floor() as i64 + 1, b.floor() as i64);
    if last < first {
      return Scrubber::sample(recording, b.round() as i64);
    }
    let mut sum = 0.0;
    for i in first..=last {
      sum += Scrubber::sample(recording, i);
    }
    sum / (last - first + 1) as f32
  }

  // add the tape's sound to `out` (stereo interleaved), moving it on
  pub fn add_to(&mut self, recording: &[i16], out: &mut [f32]) {
    let dt = 1.0 / self.out_rate;
    let level_step = dt / LEVEL_SECONDS;
    for (n, frame) in out.chunks_exact_mut(2).enumerate() {
      if n % STEP == 0 {
        let v = self.speed.abs();
        let corner = if v <= 1.0 { self.out_rate * 0.49 } else { (LOSS_HZ / v).max(LOWEST_HZ) };
        self.lp_a = (1.0 - (-2.0 * std::f64::consts::PI * corner.min(self.out_rate * 0.49) / self.out_rate).exp()) as f32;
      }
      let from = self.at * self.rate;
      self.step(dt);
      let to = self.at * self.rate;
      let v = self.speed.abs();

      // the level: faded with the speed near standstill, quieter winding,
      // and fading out to end
      let want = if self.ending {
        0.0
      } else {
        let wind = if matches!(self.drive, Drive::Wind(_)) { WIND_LEVEL } else { 1.0 };
        (v / QUIET_BELOW).min(1.0) * wind
      };
      self.level = if want > self.level { (self.level + level_step).min(want) } else { (self.level - level_step).max(want) };
      if self.level == 0.0 {
        self.lp = 0.0;
        continue;
      }

      let boxed = ((v - BOX_FROM) / (BOX_TO - BOX_FROM)).clamp(0.0, 1.0) as f32;
      let x = if boxed == 0.0 {
        self.sinc(recording, to)
      } else if boxed == 1.0 {
        Scrubber::swept(recording, from, to)
      } else {
        self.sinc(recording, to) * (1.0 - boxed) + Scrubber::swept(recording, from, to) * boxed
      };
      self.lp += self.lp_a * (x - self.lp);
      let s = self.lp * self.level as f32;
      frame[0] += s;
      frame[1] += s;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn following_a_drag_it_settles_where_the_drag_is() {
    let mut s = Scrubber::new(24_000.0, 48_000.0, 1.0, Drive::Follow(3.0));
    let recording = vec![0i16; 24_000 * 5];
    let mut out = vec![0.0; 2 * 4_800];
    // a second
    for _ in 0..10 {
      s.add_to(&recording, &mut out);
    }
    assert!((s.at() - 3.0).abs() < 1e-3, "{}", s.at());
    assert!(s.speed().abs() < 1e-2, "{}", s.speed());
  }

  #[test]
  fn winding_it_gets_up_to_speed_at_the_flywheel_s_rate() {
    let mut s = Scrubber::new(24_000.0, 48_000.0, 10.0, Drive::Wind(-10.0));
    let recording = vec![0i16; 24_000];
    let mut out = vec![0.0; 2 * 4_800];
    // 0.1 s: a third of the way to -10x
    s.add_to(&recording, &mut out);
    assert!((s.speed() + 3.0).abs() < 1e-6, "{}", s.speed());
    for _ in 0..9 {
      s.add_to(&recording, &mut out);
    }
    assert_eq!(s.speed(), -10.0);
    // a second in: 1/3 s getting there (covering 10/6 s), then 2/3 s at 10x
    let covered = 10.0 / 6.0 + 10.0 * 2.0 / 3.0;
    assert!((s.at() - (10.0 - covered)).abs() < 1e-3, "{}", s.at());
  }
}
