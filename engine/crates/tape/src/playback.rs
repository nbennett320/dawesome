// the playback side, one channel: what comes off the tape. the record
// side's output (the printed signal) is read back as the tape moves past
// the head, then worn and decoded:
//
//   printed ─► read at the tape's speed ─► dropouts ─► + hiss ─► [dbx decode] ─► out
//
// - the read: a delay line a few ms long whose read point follows the
//   shared speed (from the transport), so the pitch wobbles as the tape
//   does. it reads between samples with a short windowed sinc, flat to
//   16 khz wherever it lands (hermite, dsp's interp, is 2.5 db down at
//   15 khz half-way between samples, so the highs would wobble in level)
// - the read point follows the speed's deviation from its mean, not the
//   speed itself: a lasting speed error would need the read point to run
//   ahead of what's been written, which an insert can't do. that's the
//   engine's varispeed (part E). a slow leak keeps it near the base delay
// - each channel's head gap sits a hair off the others' (azimuth): a fixed
//   offset, in frames, from the profile's channel delays
// - dropouts: dips in level, and some of the highs, where the oxide is
//   thin or the tape lifts off the head
// - hiss: the tape's own noise, at a level re 0 vu
// - hiss and dropouts go in before the dbx decode, as on the deck, so the
//   noise pumps with the music (D8)
//
// nothing allocates after new()

use std::f64::consts::PI;
use std::sync::OnceLock;

use dsp::svf::{Coefs, Shape, State};

use crate::dbx::{DbxDecoder, VU0};
use crate::transport::{Rng, C};

// the sinc's length, and how many fractions of a sample its table holds
// (it's interpolated between them)
pub const TAPS: usize = 16;
const PHASES: usize = 256;
const KAISER_BETA: f64 = 7.0;
// the longest the read point can fall behind: slowing tape only costs
// memory, so there's plenty that way. only the base delay (the other
// way's room) costs latency
pub const MAX_DELAY_SECONDS: f64 = 0.1;
// the read's nearest to the write: the sinc's half, and one to spare
const MIN_DELAY: f64 = (TAPS / 2 + 1) as f64;
// how long the leak takes to pull the read point most of the way back:
// slow, so the pitch keeps the drift (a highpass at 0.005 hz)
const LEAK_SECONDS: f64 = 30.0;
// past this share of the room either way, the read point's travel is
// eased off (so it never hits the end of the line; the wobble flattens
// there instead)
const KNEE: f64 = 0.5;
// a new base delay glides there over this long
const BASE_SECONDS: f64 = 0.2;
// what the hiss level counts (measure.py's band)
const HISS_BAND: (f64, f64) = (20.0, 20_000.0);
// what a dropout dulls toward
const DULL_HZ: f64 = 2_500.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackSettings {
  // how far behind the record side it reads, when the tape's at its mean
  // speed: the latency, and the room the read point has to run ahead
  pub base_delay_ms: f32,
  // this channel's head-gap offset (azimuth), in frames
  pub channel_delay: f32,
  // the hiss's rms over 20 hz to 20 khz, in db re 0 vu (at or below -120:
  // none), and its colour: 0 white, 1 pink
  pub hiss_db: f32,
  pub hiss_tilt: f32,
  // dropouts: how often, how deep (db at their deepest), how long
  pub dropouts_per_minute: f32,
  pub dropout_depth_db: (f32, f32),
  pub dropout_seconds: (f32, f32),
  // how much of the highs a dropout takes besides its level (0 to 1)
  pub dropout_dulling: f32,
  // the tape's own loss of highs as it wears (oxide shed, a worn
  // surface): a 12 db/octave high cut here, in hz; LOSS_OFF_HZ or above,
  // none (Age sets it: wear.rs)
  pub loss_hz: f32,
}

// at or above this the tape loses no highs
pub const LOSS_OFF_HZ: f32 = 20_000.0;

// a fresh cassette: the fake deck's measured hiss, no dropouts. 8 ms of
// base delay: a fresh tape's wobble never needs easing off at 100 % (at
// 5 ms it's eased 3 % of the time; examples/playback_bench.rs)
impl Default for PlaybackSettings {
  fn default() -> PlaybackSettings {
    PlaybackSettings {
      base_delay_ms: 8.0,
      channel_delay: 0.0,
      hiss_db: -48.6,
      hiss_tilt: 1.0,
      dropouts_per_minute: 0.0,
      dropout_depth_db: (3.0, 15.0),
      dropout_seconds: (0.01, 0.08),
      dropout_dulling: 0.5,
      loss_hz: LOSS_OFF_HZ,
    }
  }
}

pub struct Playback {
  settings: PlaybackSettings,
  rate: f64,
  seed: u64,
  max_block: usize,
  table: &'static [f32],

  // the printed signal, written twice (at i and i + len) so a read's taps
  // are always one unbroken slice
  ring: Box<[f32]>,
  len: usize,
  mask: usize,
  written: u64,

  // the read point: the speed it's measured against, how far it's run
  // ahead (frames, + is ahead: less delay), the leak a frame, the base
  // delay (frames) and where it's gliding to
  mean: f64,
  ahead: f64,
  leak: f64,
  base: f64,
  base_target: f64,
  base_k: f64,
  limited: u64,

  dropouts: Dropouts,
  hiss: Hiss,
  dbx: DbxDecoder,
  dbx_on: bool,
  // the tape's loss of highs (None: none)
  loss: Option<Coefs>,
  loss_state: State,
}

impl Playback {
  // `max_block`: the longest block process() is handed at once (longer
  // ones are split): the line holds that much beyond its longest delay,
  // since a block is written before it's read
  pub fn new(settings: PlaybackSettings, rate: f32, seed: u64, max_block: usize) -> Playback {
    let rate = rate.max(1.0) as f64;
    let max_block = max_block.max(1);
    let len = ((MAX_DELAY_SECONDS * rate) as usize + max_block + TAPS + 4).next_power_of_two();
    let mut p = Playback {
      settings,
      rate,
      seed,
      max_block,
      table: table(),
      ring: vec![0.0; 2 * len].into_boxed_slice(),
      len,
      mask: len - 1,
      written: 0,
      mean: 1.0,
      ahead: 0.0,
      leak: 1.0 / (LEAK_SECONDS * rate),
      base: 0.0,
      base_target: 0.0,
      base_k: 1.0 - (-1.0 / (BASE_SECONDS * rate)).exp(),
      limited: 0,
      dropouts: Dropouts::new(seed, rate),
      hiss: Hiss::new(seed, rate),
      dbx: DbxDecoder::new(rate as f32),
      dbx_on: false,
      loss: None,
      loss_state: State::default(),
    };
    p.set_settings(settings);
    p.hiss.set(&settings, true);
    p.reset();
    p
  }

  pub fn set_settings(&mut self, settings: PlaybackSettings) {
    let old = self.settings;
    self.settings = settings;
    self.base_target = (settings.base_delay_ms.max(0.0) as f64 * 1e-3 * self.rate).clamp(MIN_DELAY, self.longest() * 0.5);
    self.hiss.set(&settings, old.hiss_tilt != settings.hiss_tilt);
    self.dropouts.set(&settings, old.dropouts_per_minute != settings.dropouts_per_minute);
    // (a butterworth q: no bump before it rolls off)
    self.loss = (settings.loss_hz < LOSS_OFF_HZ && (settings.loss_hz as f64) < 0.45 * self.rate)
      .then(|| Coefs::new(Shape::HighCut, settings.loss_hz.max(20.0), 0.0, std::f32::consts::FRAC_1_SQRT_2, self.rate as f32));
  }

  pub fn settings(&self) -> &PlaybackSettings {
    &self.settings
  }

  // the speed the tape runs at on average (the transport's mean_speed()):
  // the read point follows the speed's deviation from it
  pub fn set_mean_speed(&mut self, mean: f64) {
    self.mean = mean;
  }

  // the delay at the mean speed, in frames: its latency
  pub fn latency(&self) -> f64 {
    self.base_target + self.settings.channel_delay as f64
  }

  // frames whose read point was eased off near the end of its room since
  // reset (the wobble was flattened there)
  pub fn limited(&self) -> u64 {
    self.limited
  }

  // back to silence, and the same dropouts and hiss again
  pub fn reset(&mut self) {
    self.ring.fill(0.0);
    self.written = 0;
    self.ahead = 0.0;
    self.base = self.base_target;
    self.limited = 0;
    self.dropouts.reset(self.seed, &self.settings);
    self.hiss.reset(self.seed);
    self.dbx.reset();
    self.dbx_on = false;
    self.loss_state.reset();
  }

  // start again from a new place on the tape: the line cleared and
  // `printed` (what's on the tape just ahead of it, latency() frames of it)
  // written in without reading, so the next process() reads from there
  // straight away, with nothing stale from before. for a run starting, a
  // seek, or a loop going round
  pub fn prime(&mut self, printed: &[f32]) {
    self.ring.fill(0.0);
    self.written = 0;
    self.ahead = 0.0;
    self.base = self.base_target;
    self.loss_state.reset();
    self.dbx.reset();
    self.write(printed);
  }

  // `printed`: what the record side put on the tape; `speed`: the
  // transport's speed for each frame; `dbx`: decode
  pub fn process(&mut self, printed: &[f32], speed: &[f32], dbx: bool, out: &mut [f32]) {
    let n = printed.len().min(speed.len()).min(out.len());
    if dbx && !self.dbx_on {
      self.dbx.reset();
    }
    self.dbx_on = dbx;
    let mut at = 0;
    while at < n {
      let end = (at + self.max_block).min(n);
      self.write(&printed[at..end]);
      self.read(&speed[at..end], &mut out[at..end], self.written - (end - at) as u64);
      if let Some(loss) = &self.loss {
        for o in out[at..end].iter_mut() {
          *o = self.loss_state.run(loss, *o);
        }
        self.loss_state.flush();
      }
      self.dropouts.run(&mut out[at..end]);
      self.hiss.add(&mut out[at..end]);
      if dbx {
        self.dbx.process_in_place(&mut out[at..end]);
      }
      at = end;
    }
  }

  fn longest(&self) -> f64 {
    (MAX_DELAY_SECONDS * self.rate).floor()
  }

  fn write(&mut self, x: &[f32]) {
    for &v in x {
      let i = self.written as usize & self.mask;
      self.ring[i] = v;
      self.ring[i + self.len] = v;
      self.written += 1;
    }
  }

  // each frame of the block, read at its delay. `first` is the block's
  // first frame's index in the line
  fn read(&mut self, speed: &[f32], out: &mut [f32], first: u64) {
    let offset = self.settings.channel_delay as f64;
    let longest = self.longest();
    for (i, (&v, o)) in speed.iter().zip(out.iter_mut()).enumerate() {
      self.base += (self.base_target - self.base) * self.base_k;
      self.ahead += (v as f64 - self.mean) - self.ahead * self.leak;
      // the room each way, and the read point eased off as it nears either
      // end of it
      let centre = (self.base + offset).clamp(MIN_DELAY, longest);
      let room = if self.ahead >= 0.0 { centre - MIN_DELAY } else { longest - centre };
      let (ahead, eased) = ease(self.ahead, room);
      self.limited += eased as u64;
      let delay = (centre - ahead).clamp(MIN_DELAY, longest);
      // the frame's position in the line, less the delay: a whole sample
      // and the fraction past it
      let whole = delay.floor();
      let frac = delay - whole;
      // (wrapping: at the start the read is before the first write, where
      // the line is silent)
      let mut k = (first + i as u64).wrapping_sub(whole as u64);
      let u = if frac > 0.0 {
        k = k.wrapping_sub(1);
        1.0 - frac
      } else {
        0.0
      };
      let start = k.wrapping_sub((TAPS / 2 - 1) as u64) as usize & self.mask;
      *o = sinc_read(self.table, &self.ring[start..start + TAPS], u as f32);
    }
  }
}

// how far the read point runs ahead, eased off past KNEE of its room so it
// never quite reaches the end. (and whether it was eased)
#[inline]
fn ease(ahead: f64, room: f64) -> (f64, bool) {
  let knee = KNEE * room;
  let a = ahead.abs();
  if a <= knee {
    return (ahead, false);
  }
  let soft = room - knee;
  if soft <= 0.0 {
    return (0.0, true);
  }
  ((knee + soft * ((a - knee) / soft).tanh()).copysign(ahead), true)
}

// ---- reading between samples

// the sinc's taps for each of PHASES + 1 fractions of a sample, each row
// followed by its difference to the next: made once, shared by every
// channel (the first Playback::new makes it), and by varispeed.rs
pub(crate) fn table() -> &'static [f32] {
  static TABLE: OnceLock<Box<[f32]>> = OnceLock::new();
  TABLE.get_or_init(|| {
    let row = |u: f64| -> [f64; TAPS] {
      let mut h = [0.0; TAPS];
      let half = (TAPS / 2) as f64;
      for (j, v) in h.iter_mut().enumerate() {
        // the tap's distance from the point read
        let t = j as f64 - (TAPS / 2 - 1) as f64 - u;
        let sinc = if t.abs() < 1e-12 { 1.0 } else { (PI * t).sin() / (PI * t) };
        let w = bessel_i0(KAISER_BETA * (1.0 - (t / half).powi(2)).max(0.0).sqrt()) / bessel_i0(KAISER_BETA);
        *v = sinc * w;
      }
      // each row sums to 1: a steady signal reads back exactly
      let sum: f64 = h.iter().sum();
      h.map(|v| v / sum)
    };
    let mut out = vec![0f32; PHASES * 2 * TAPS];
    for p in 0..PHASES {
      let (a, b) = (row(p as f64 / PHASES as f64), row((p + 1) as f64 / PHASES as f64));
      for j in 0..TAPS {
        out[p * 2 * TAPS + j] = a[j] as f32;
        out[p * 2 * TAPS + TAPS + j] = (b[j] - a[j]) as f32;
      }
    }
    out.into_boxed_slice()
  })
}

// the value `u` (0 to 1) of the way from x[TAPS/2 - 1] to x[TAPS/2]
#[inline]
pub(crate) fn sinc_read(table: &[f32], x: &[f32], u: f32) -> f32 {
  let at = u * PHASES as f32;
  let p = (at as usize).min(PHASES - 1);
  let a = at - p as f32;
  let row = &table[p * 2 * TAPS..(p + 1) * 2 * TAPS];
  let (taps, step) = row.split_at(TAPS);
  let mut sum = 0.0;
  for ((&x, &t), &d) in x[..TAPS].iter().zip(taps).zip(step) {
    sum += x * (t + a * d);
  }
  sum
}

fn bessel_i0(x: f64) -> f64 {
  let (mut sum, mut term) = (1.0, 1.0);
  let q = x * x / 4.0;
  for k in 1..60 {
    term *= q / (k * k) as f64;
    sum += term;
    if term < sum * 1e-17 {
      break;
    }
  }
  sum
}

// ---- dropouts

struct Dropouts {
  rng: Rng,
  rate: f64,
  // frames to the next one's start (it waits if one's still going)
  next: f64,
  // the one going: how far in, how long, how deep (its gain at the
  // deepest), and how much of the highs it takes
  at: f64,
  length: f64,
  floor: f32,
  dulling: f32,
  // the highs-less copy a dropout fades toward
  dull: f32,
  dull_a: f32,
  settings: PlaybackSettings,
}

impl Dropouts {
  fn new(seed: u64, rate: f64) -> Dropouts {
    Dropouts {
      rng: Rng::new(seed ^ 0x6472_6f70),
      rate,
      next: f64::INFINITY,
      at: 0.0,
      length: 0.0,
      floor: 1.0,
      dulling: 0.0,
      dull: 0.0,
      dull_a: (1.0 - (-2.0 * PI * DULL_HZ.min(rate * 0.45) / rate).exp()) as f32,
      settings: PlaybackSettings::default(),
    }
  }

  fn set(&mut self, s: &PlaybackSettings, rate_changed: bool) {
    self.settings = *s;
    if rate_changed {
      self.next = self.rng.wait(s.dropouts_per_minute) * self.rate;
    }
  }

  fn reset(&mut self, seed: u64, s: &PlaybackSettings) {
    self.rng = Rng::new(seed ^ 0x6472_6f70);
    self.settings = *s;
    self.next = self.rng.wait(s.dropouts_per_minute) * self.rate;
    self.at = 0.0;
    self.length = 0.0;
    self.dull = 0.0;
  }

  fn run(&mut self, x: &mut [f32]) {
    let idle = self.length == 0.0 && self.next > x.len() as f64;
    for v in x.iter_mut() {
      self.dull += self.dull_a * (*v - self.dull);
      if idle {
        continue;
      }
      self.next -= 1.0;
      if self.length == 0.0 && self.next <= 0.0 {
        let s = &self.settings;
        let (lo, hi) = (s.dropout_seconds.0 as f64, s.dropout_seconds.1.max(s.dropout_seconds.0) as f64);
        self.length = (self.rng.range(lo, hi) * self.rate).max(2.0);
        let (lo, hi) = (s.dropout_depth_db.0 as f64, s.dropout_depth_db.1.max(s.dropout_depth_db.0) as f64);
        self.floor = 10f64.powf(-self.rng.range(lo, hi) / 20.0) as f32;
        self.dulling = s.dropout_dulling.clamp(0.0, 1.0);
        self.at = 0.0;
        // the next is timed from this one's start, so they come at the
        // set rate however long they last
        self.next += self.rng.wait(s.dropouts_per_minute) * self.rate;
      }
      if self.length > 0.0 {
        // a hann-shaped dip
        let dip = (0.5 - 0.5 * (2.0 * PI * self.at / self.length).cos()) as f32;
        let y = *v - self.dulling * dip * (*v - self.dull);
        *v = y * (1.0 - (1.0 - self.floor) * dip);
        self.at += 1.0;
        if self.at >= self.length {
          self.length = 0.0;
        }
      }
    }
    if idle {
      self.next -= x.len() as f64;
    }
  }
}

// ---- hiss

// white noise, part pinked (paul kellet's economy filter: three one-poles,
// pink within a db or so across the band), at a level re 0 vu over 20 hz
// to 20 khz. the profile's third-octave hiss spectrum can replace the
// shape here later (shape() and the filter in add())
struct Hiss {
  rng: Rng,
  rate: f64,
  state: [f32; 3],
  white: f32,
  pink: f32,
  gain: f32,
  // the band rms of the shape, for unit white noise
  band: f64,
}

const PINK_POLES: [f32; 3] = [0.997_65, 0.963, 0.57];
const PINK_GAINS: [f32; 3] = [0.099_046, 0.296_516_4, 1.052_691_3];
const PINK_DIRECT: f32 = 0.1848;

impl Hiss {
  fn new(seed: u64, rate: f64) -> Hiss {
    Hiss { rng: Rng::new(seed ^ 0x6869_7373), rate, state: [0.0; 3], white: 0.0, pink: 0.0, gain: 0.0, band: 1.0 }
  }

  fn reset(&mut self, seed: u64) {
    self.rng = Rng::new(seed ^ 0x6869_7373);
    self.state = [0.0; 3];
  }

  // the pink filter's response at f hz
  fn pink_at(&self, f: f64) -> C {
    let z1 = C::expj(-2.0 * PI * f / self.rate);
    let one = C::new(1.0, 0.0);
    let mut h = C::new(PINK_DIRECT as f64, 0.0);
    for (&p, &g) in PINK_POLES.iter().zip(&PINK_GAINS) {
      h = h + C::new(g as f64, 0.0) / (one - z1 * p as f64);
    }
    h
  }

  // the level, and (when the colour's changed) the shape's band rms again
  fn set(&mut self, s: &PlaybackSettings, reshape: bool) {
    if reshape {
      let tilt = s.hiss_tilt.clamp(0.0, 1.0) as f64;
      // pink as loud as white at 1 khz, so the tilt only tilts
      let pink = tilt / self.pink_at(1_000.0).norm().sqrt();
      let white = 1.0 - tilt;
      self.white = white as f32;
      self.pink = pink as f32;
      let (lo, hi) = (HISS_BAND.0, HISS_BAND.1.min(self.rate * 0.5));
      // (2 / rate) of the band's integral of |h|² is the variance there
      let steps = 2000;
      let mut power = 0.0;
      let ratio = (hi / lo).ln() / steps as f64;
      for i in 0..steps {
        let f = lo * (ratio * (i as f64 + 0.5)).exp();
        let h = C::new(white, 0.0) + self.pink_at(f) * pink;
        power += h.norm() * f * ratio;
      }
      self.band = (power * 2.0 / self.rate).sqrt();
    }
    self.gain = if s.hiss_db <= -120.0 { 0.0 } else { (10f64.powf(s.hiss_db as f64 / 20.0) * VU0 as f64 / self.band) as f32 };
  }

  fn add(&mut self, x: &mut [f32]) {
    if self.gain == 0.0 {
      return;
    }
    let root3 = 3f32.sqrt();
    for v in x.iter_mut() {
      let w = self.rng.bipolar() * root3;
      let mut pink = w * PINK_DIRECT;
      for ((s, &p), &g) in self.state.iter_mut().zip(&PINK_POLES).zip(&PINK_GAINS) {
        *s = p * *s + w * g;
        pink += *s;
      }
      *v += (self.white * w + self.pink * pink) * self.gain;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // a sine read through the table at a fixed fraction: its level, against
  // the sine itself, in db
  fn level_at(hz: f64, u: f32, rate: f64) -> f64 {
    let table = table();
    let w = 2.0 * PI * hz / rate;
    let x: Vec<f32> = (0..4096).map(|n| (w * n as f64).sin() as f32).collect();
    let (mut heard, mut real) = (0.0, 0.0);
    for n in 100..4000 {
      let got = sinc_read(table, &x[n - (TAPS / 2 - 1)..], u) as f64;
      heard += got * got;
      let want = (w * (n as f64 + u as f64)).sin();
      real += want * want;
    }
    10.0 * (heard / real).log10()
  }

  #[test]
  fn reading_between_samples_is_flat_to_16_khz() {
    let mut worst: f64 = 0.0;
    for rate in [44_100.0, 48_000.0] {
      for u in (0..=40).map(|i| i as f32 / 40.0 + 0.0013) {
        for hz in [100.0, 1_000.0, 5_000.0, 10_000.0, 13_000.0, 15_000.0, 16_000.0] {
          worst = worst.max(level_at(hz, u.min(0.9999), rate).abs());
        }
      }
    }
    println!("the sinc read is within {worst:.4} db of flat to 16 khz");
    assert!(worst < 0.05, "{worst}");
  }

  #[test]
  fn reading_between_samples_lands_on_the_curve() {
    // and in time, not only in level: a 5 khz sine read at fractions
    // between the table's rows
    let table = table();
    let w = 2.0 * PI * 5_000.0 / 48_000.0;
    let x: Vec<f32> = (0..256).map(|n| (w * n as f64).sin() as f32).collect();
    for u in [0.0, 0.1234, 0.5, 0.77777, 0.999] {
      let got = sinc_read(table, &x[100 - (TAPS / 2 - 1)..], u);
      let want = (w * (100.0 + u as f64)).sin() as f32;
      assert!((got - want).abs() < 2e-4, "at {u}: {got} against {want}");
    }
  }

  #[test]
  fn easing_is_smooth_and_stays_inside() {
    assert_eq!(ease(10.0, 100.0), (10.0, false));
    let (mut prev, mut step) = (0.0, f64::MAX);
    for i in 1..2000 {
      let (a, _) = ease(i as f64 * 0.1, 100.0);
      assert!(a > prev && a < 100.0);
      // its slope only ever falls (no kink upward at the knee)
      assert!(a - prev <= step + 1e-12);
      step = a - prev;
      prev = a;
    }
  }
}
