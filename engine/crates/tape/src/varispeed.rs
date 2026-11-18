// varispeed (part E): the whole song read at another speed, as a deck's
// PITCH knob runs the tape faster or slower, pitch and tempo together. the
// song is rendered a little ahead into a short queue and read back between
// its samples with the playback side's sinc (playback.rs), at a speed that
// moves as a flywheel does: it glides to a new speed, spins up on a start
// and coasts down on a stop (T27, T30)
//
// settled at 1x it steps out of the way: what's rendered goes straight
// out, sample for sample (T14), and the last few frames are remembered so
// it can step back in without a click. stepping out, it eases its read
// point onto a whole frame (a fraction of a cent, for a moment) and plays
// out what's left in the queue as it is
//
// a head's output falls with the tape's speed, so near standstill it's
// faded: a stopped tape is silent, not a held sample
//
// nothing allocates after new()

use crate::playback::{sinc_read, table, TAPS};

const HALF: usize = TAPS / 2;
// how fast a new speed is glided to: the motor's servo, not a jump
pub const GLIDE_SECONDS: f64 = 0.08;
// closer than this to the speed it's going to, it's there
const SETTLED: f64 = 1e-6;
// stepping out: the read point is eased onto a whole frame this much
// faster than 1x (0.4 cents), for at most 4096 frames
const EASE: f64 = 1.0 / 4096.0;
// under this speed the head's output fades with the speed
const QUIET_BELOW: f64 = 0.1;
// how fast that fade can rise and fall, so what's still sounding when the
// speed jumps (a stop's end, a spin up) doesn't click
const GAIN_RISE_SECONDS: f64 = 0.05;
const GAIN_FALL_SECONDS: f64 = 0.005;

// the tape's speed: a flywheel turned by a motor whose torque is limited
// (it spins up at a steady rate) against a steady drag (it coasts down at
// one when the motor's cut), as crates/tape's transport has it (D46), with
// a servo gliding it to the speed it's set to
#[derive(Clone, Debug)]
pub struct Flywheel {
  speed: f64,
  target: f64,
  motor: bool,
  // the glide's share of the way a frame, and the most the motor and the
  // drag move the speed in one
  glide: f64,
  accel: f64,
  drag: f64,
  rate: f64,
}

impl Flywheel {
  // - start_seconds: from standstill to 1x; stop_seconds: from 1x to a
  //   standstill (the transport's TransportSettings has the deck's)
  pub fn new(rate: f64, start_seconds: f64, stop_seconds: f64) -> Flywheel {
    Flywheel {
      speed: 1.0,
      target: 1.0,
      motor: true,
      glide: 1.0 - (-1.0 / (GLIDE_SECONDS * rate)).exp(),
      accel: 1.0 / (start_seconds.max(0.01) * rate),
      drag: 1.0 / (stop_seconds.max(0.01) * rate),
      rate,
    }
  }

  pub fn speed(&self) -> f64 {
    self.speed
  }

  pub fn target(&self) -> f64 {
    self.target
  }

  pub fn motor(&self) -> bool {
    self.motor
  }

  // the speed the motor holds (1 + the pitch knob)
  pub fn set_target(&mut self, target: f64) {
    if target.is_finite() && target > 0.0 {
      self.target = target;
    }
  }

  // the motor's cut: it coasts down to a standstill
  pub fn cut(&mut self) {
    self.motor = false;
  }

  // the motor's on: from a standstill (spinning up, `from_rest`), or at
  // its speed straight away
  pub fn run(&mut self, from_rest: bool) {
    self.motor = true;
    self.speed = if from_rest { 0.0 } else { self.target };
  }

  // the motor's back on, from whatever speed it's at
  pub fn power(&mut self) {
    self.motor = true;
  }

  // at its speed, with nothing moving it: 1x like this needs no varispeed
  pub fn settled(&self) -> bool {
    self.motor && self.speed == self.target
  }

  // a frame on
  #[inline]
  pub fn step(&mut self) -> f64 {
    if self.motor {
      let d = self.target - self.speed;
      if d != 0.0 {
        let dv = (d * self.glide).clamp(-self.accel, self.accel);
        self.speed = if d.abs() < SETTLED || dv.abs() >= d.abs() { self.target } else { self.speed + dv };
      }
    } else {
      self.speed = (self.speed - self.drag).max(0.0);
    }
    self.speed
  }

  // how far, in song seconds, the tape will fall behind (or run ahead of)
  // where running at its speed from now on would take it, while it gets
  // there. 0 with the motor cut
  pub fn lag_seconds(&self) -> f64 {
    if !self.motor {
      return 0.0;
    }
    let d = self.target - self.speed;
    let a = self.accel * self.rate;
    // under this, the glide is slower than the motor's limit
    let knee = GLIDE_SECONDS * a;
    if d.abs() <= knee {
      d * GLIDE_SECONDS
    } else {
      d.signum() * ((d * d - knee * knee) / (2.0 * a) + knee * GLIDE_SECONDS)
    }
  }

  // how much song a coast from here covers before it stops, in seconds
  pub fn coast_seconds(&self) -> f64 {
    let a = self.drag * self.rate;
    self.speed * self.speed / (2.0 * a)
  }

  // how long a coast from here takes, in seconds
  pub fn coast_time(&self) -> f64 {
    self.speed / (self.drag * self.rate)
  }
}

// the song, rendered by whoever owns this (`fill`: the next frames,
// stereo interleaved, into the slice it's given), read at the flywheel's
// speed
pub struct Varispeed {
  pub flywheel: Flywheel,
  // what's rendered and not yet read past, one channel each
  queue: [Vec<f32>; 2],
  len: usize,
  // the read point, in the queue's frames
  pos: f64,
  resampling: bool,
  // the last frames played straight
  history: [[f32; HALF]; 2],
  // what fill renders into, stereo interleaved
  scratch: Vec<f32>,
  max_block: usize,
  // the head's fade near standstill, and how far it moves a frame
  gain: f64,
  rise: f64,
  fall: f64,
  rate: f64,
  // song seconds read, less real seconds played
  drift: f64,
  table: &'static [f32],
}

impl Varispeed {
  // - max_block: the most frames fill is asked for at once
  pub fn new(rate: f64, max_block: usize, start_seconds: f64, stop_seconds: f64) -> Varispeed {
    let room = 2 * max_block + TAPS + 4;
    Varispeed {
      flywheel: Flywheel::new(rate, start_seconds, stop_seconds),
      queue: [vec![0.0; room], vec![0.0; room]],
      len: 0,
      pos: 0.0,
      resampling: false,
      history: [[0.0; HALF]; 2],
      scratch: vec![0.0; max_block * 2],
      max_block,
      gain: 1.0,
      rise: 1.0 / (GAIN_RISE_SECONDS * rate),
      fall: 1.0 / (GAIN_FALL_SECONDS * rate),
      rate,
      drift: 0.0,
      table: table(),
    }
  }

  // whether what's rendered can go straight out: at 1x, settled, with
  // nothing left in the queue. then the owner renders as if this weren't
  // here, and tells `remember` what it played
  pub fn straight(&self) -> bool {
    !self.resampling && self.flywheel.settled() && self.flywheel.speed == 1.0 && self.gain == 1.0
  }

  // the last frames of what was played straight (stereo interleaved), to
  // step in from
  pub fn remember(&mut self, played: &[f32]) {
    let frames = played.len() / 2;
    let keep = frames.min(HALF);
    for side in 0..2 {
      let h = &mut self.history[side];
      h.copy_within(keep.., 0);
      for (j, f) in (frames - keep..frames).enumerate() {
        h[HALF - keep + j] = played[f * 2 + side];
      }
    }
  }

  // song seconds read so far, less real seconds played: how far the song
  // has run ahead of the clock (behind, when negative)
  pub fn drift(&self) -> f64 {
    self.drift
  }

  // out of the path, the clock can start again from the song: what's
  // anchored from now on is on the clock itself (a new run). the drift
  // only matters while something's timed across it
  pub fn reset_drift(&mut self) {
    if self.straight() {
      self.drift = 0.0;
    }
  }

  // song frames rendered ahead of what's being heard
  pub fn ahead(&self) -> f64 {
    if self.resampling { self.len as f64 - self.pos } else { 0.0 }
  }

  // fill `out` (stereo interleaved) with the song at the flywheel's speed,
  // asking `fill` for song frames as they're needed (at most max_block at
  // once, but any length once it's stepped out: it should render those in
  // blocks)
  pub fn render(&mut self, out: &mut [f32], fill: &mut dyn FnMut(&mut [f32])) {
    let frames = out.len() / 2;
    if !self.resampling {
      // step in: the frames just played are the sinc's past
      for side in 0..2 {
        self.queue[side][..HALF].copy_from_slice(&self.history[side]);
      }
      self.len = HALF;
      self.pos = HALF as f64;
      self.resampling = true;
    }

    let mut k = 0;
    while k < frames {
      if !self.resampling {
        // stepped out: the rest straight
        fill(&mut out[k * 2..]);
        self.remember(out);
        return;
      }
      let v = self.flywheel.step();
      let want = (v / QUIET_BELOW).min(1.0);
      self.gain = if want > self.gain { (self.gain + self.rise).min(want) } else { (self.gain - self.fall).max(want) };

      let i = self.pos as usize;
      let u = self.pos - i as f64;
      let leaving = self.flywheel.settled() && v == 1.0 && self.gain == 1.0;
      if leaving && u == 0.0 {
        // on a whole frame at 1x: what's queued goes out as it is
        if i >= self.len {
          self.resampling = false;
          continue;
        }
        out[k * 2] = self.queue[0][i];
        out[k * 2 + 1] = self.queue[1][i];
        self.pos += 1.0;
        k += 1;
        continue;
      }

      while (self.pos as usize) + HALF >= self.len {
        self.pull(fill, (frames - k) as f64 * v.max(1.0));
      }
      let i = self.pos as usize;
      let from = i + 1 - HALF;
      let g = self.gain as f32;
      for side in 0..2 {
        out[k * 2 + side] = sinc_read(self.table, &self.queue[side][from..from + TAPS], u as f32) * g;
      }

      // (stepping out: onto the next whole frame, a hair early)
      let step = if leaving {
        let next = self.pos + 1.0 + EASE;
        if next.floor() > self.pos.floor() + 1.0 { next.floor() - self.pos } else { 1.0 + EASE }
      } else {
        v
      };
      self.pos += step;
      self.drift += (step - 1.0) / self.rate;
      k += 1;
    }
  }

  // render more of the song into the queue: about `wanted` frames, and the
  // sinc's reach past them
  fn pull(&mut self, fill: &mut dyn FnMut(&mut [f32]), wanted: f64) {
    let room = self.queue[0].len();
    // what the read has gone past (less the sinc's past) makes room
    let read = (self.pos as usize + 1).saturating_sub(HALF);
    if read > 0 {
      for side in 0..2 {
        self.queue[side].copy_within(read..self.len, 0);
      }
      self.len -= read;
      self.pos -= read as f64;
    }
    let n = ((wanted.ceil() as usize) + HALF + 2).min(room - self.len).min(self.max_block).max(1);
    let chunk = &mut self.scratch[..n * 2];
    chunk.fill(0.0);
    fill(chunk);
    for (j, frame) in chunk.chunks_exact(2).enumerate() {
      self.queue[0][self.len + j] = frame[0];
      self.queue[1][self.len + j] = frame[1];
    }
    self.len += n;
  }
}

// the same read at a fixed speed, offline (a mixdown at the varispeed,
// T28): the whole render goes in a chunk at a time and comes out at
// `speed`, pitched and its length changed. allocates as it goes
pub struct Stretch {
  speed: f64,
  queue: [Vec<f32>; 2],
  pos: f64,
  // song frames in, and frames out
  fed: u64,
  made: u64,
  table: &'static [f32],
}

impl Stretch {
  pub fn new(speed: f64) -> Stretch {
    // (silence before the start, for the sinc's past)
    Stretch { speed, queue: [vec![0.0; HALF], vec![0.0; HALF]], pos: HALF as f64, fed: 0, made: 0, table: table() }
  }

  // more of the song (stereo interleaved): what can be read of it so far
  // is added to `out`
  pub fn push(&mut self, song: &[f32], out: &mut Vec<f32>) {
    self.fed += (song.len() / 2) as u64;
    self.take(song, out);
  }

  fn take(&mut self, song: &[f32], out: &mut Vec<f32>) {
    for frame in song.chunks_exact(2) {
      self.queue[0].push(frame[0]);
      self.queue[1].push(frame[1]);
    }
    while (self.pos as usize) + HALF < self.queue[0].len() {
      let i = self.pos as usize;
      let u = (self.pos - i as f64) as f32;
      let from = i + 1 - HALF;
      for side in 0..2 {
        out.push(sinc_read(self.table, &self.queue[side][from..from + TAPS], u));
      }
      self.pos += self.speed;
      self.made += 1;
    }
    // what's been read past goes
    let read = (self.pos as usize + 1).saturating_sub(HALF);
    if read > 4096 {
      for side in 0..2 {
        self.queue[side].drain(..read);
      }
      self.pos -= read as f64;
    }
  }

  // the song's over: the last of it, read against silence after it, to
  // the song's length at the speed
  pub fn finish(&mut self, out: &mut Vec<f32>) {
    let total = (self.fed as f64 / self.speed).ceil() as u64;
    let before = out.len() as u64 / 2;
    while self.made < total {
      self.take(&[0.0; 64], out);
    }
    let over = self.made - total;
    let keep = (before + (out.len() as u64 / 2 - before).saturating_sub(over)) as usize;
    out.truncate(keep * 2);
    self.made = total;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_flywheel_glides_to_a_new_speed_and_settles_on_it() {
    let mut f = Flywheel::new(1_000.0, 0.5, 0.6);
    f.set_target(1.12);
    let mut v = 1.0;
    for _ in 0..100 {
      v = f.step();
    }
    // 1.25 of the glide's time constant in: 71 % of the way
    assert!(v > 1.08 && v < 1.095, "{v}");
    for _ in 0..3_000 {
      f.step();
    }
    assert!(f.settled());
    assert_eq!(f.speed(), 1.12);
  }

  #[test]
  fn the_flywheel_coasts_down_and_spins_up_at_the_motor_s_rate() {
    let mut f = Flywheel::new(1_000.0, 0.5, 0.6);
    assert!((f.coast_seconds() - 0.3).abs() < 1e-9);
    assert!((f.coast_time() - 0.6).abs() < 1e-9);
    f.cut();
    for _ in 0..300 {
      f.step();
    }
    assert!((f.speed() - 0.5).abs() < 0.01, "{}", f.speed());
    for _ in 0..301 {
      f.step();
    }
    assert_eq!(f.speed(), 0.0);
    f.run(true);
    for _ in 0..250 {
      f.step();
    }
    assert!((f.speed() - 0.5).abs() < 0.02, "{}", f.speed());
  }

  #[test]
  fn the_lag_is_what_the_flywheel_falls_behind() {
    for (from, to) in [(0.0, 1.0), (1.0, 0.88), (1.0, 1.12), (0.95, 1.0)] {
      let rate = 10_000.0;
      let mut f = Flywheel::new(rate, 0.5, 0.6);
      f.set_target(from);
      f.run(from == 0.0);
      f.set_target(to);
      let said = f.lag_seconds();
      let mut lag = 0.0;
      for _ in 0..(rate as usize * 5) {
        lag += (to - f.step()) / rate;
      }
      assert!((lag - said).abs() < 2e-3 * said.abs().max(0.01), "{from} → {to}: {lag} vs {said}");
    }
  }
}
