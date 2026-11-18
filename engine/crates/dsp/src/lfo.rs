// the slow movement the modulation effects (chorus, flanger, phaser) are
// made of: an LFO, and a smooth random line (Wander) for the parts meant
// to drift
//
// an LFO's phase goes from 0 to 1 once a cycle, a sample at a time, in f64
// (an f32 phase, after minutes, would step unevenly). its rate is Hz, or a
// note value at the song's tempo. synced and playing, it follows the
// song's position, so its sweep lands on the same beat every time through.
// a seek or a loop doesn't jump it (a chorus whose LFO jumps, clicks): it
// speeds up or slows down, by at most a quarter, until it's caught up

use std::f64::consts::TAU;

use crate::interp::hermite;
use crate::transport::Transport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
  Sine,
  Triangle,
  // a lamp lighting a photocell, as a Uni-Vibe's does: quick to brighten,
  // slow to fade
  Lamp,
}

// the shapes the effects offer, as a choice's labels (Lamp is the
// phaser's Vibe mode's own)
pub const SHAPES: &[&str] = &["Sine", "Triangle"];

impl Shape {
  pub fn of_choice(choice: usize) -> Shape {
    if choice == 1 { Shape::Triangle } else { Shape::Sine }
  }

  // its value at `phase` (any number: only the part after the point
  // counts), from -1 to 1. every shape is -1 at phase 0 and 1 halfway,
  // so a sweep starts at its bottom
  pub fn at(self, phase: f64) -> f32 {
    let p = phase - phase.floor();
    match self {
      Shape::Sine => -(TAU * p).cos() as f32,
      Shape::Triangle => (if p < 0.5 { 4. * p - 1. } else { 3. - 4. * p }) as f32,
      Shape::Lamp => {
        // rises over the first LAMP_RISE of the cycle (a sine's quarter
        // to its peak), then falls the rest of the way as a cosine: both
        // halves meet smoothly at the top, and at the bottom
        let x = if p < LAMP_RISE {
          (0.5 - 0.5 * (std::f64::consts::PI * p / LAMP_RISE).cos()) * 2. - 1.
        } else {
          (0.5 + 0.5 * (std::f64::consts::PI * (p - LAMP_RISE) / (1. - LAMP_RISE)).cos()) * 2. - 1.
        };
        x as f32
      }
    }
  }
}

// how much of a cycle the lamp takes to brighten
const LAMP_RISE: f64 = 0.3;

// ---- note values

// cycle lengths, shortest first, as a choice's labels. a label is its own
// formula: a/b of a whole note, × 1.5 dotted (D), × 2/3 triplet (T). the
// app works lengths out from the labels. longer than the Delay's: a slow
// flanger sweeps over bars
pub const NOTES: &[&str] = &[
  "1/16 T", "1/16", "1/8 T", "1/16 D", "1/8", "1/4 T", "1/8 D", "1/4", "1/2 T",
  "1/4 D", "1/2", "1/1 T", "1/2 D", "1/1", "1/1 D", "2/1", "4/1", "8/1",
];
// each one's length in quarter notes (a test checks these against the
// labels)
const QUARTERS: [f64; 18] = [
  0.25 * 2. / 3., 0.25, 0.5 * 2. / 3., 0.375, 0.5, 2. / 3., 0.75, 1., 4. / 3.,
  1.5, 2., 8. / 3., 3., 4., 6., 8., 16., 32.,
];

// a note value's length in quarter notes
pub fn quarters(note: usize) -> f64 {
  QUARTERS[note.min(QUARTERS.len() - 1)]
}

// the tempo, or 120 if it's none
fn tempo_of(transport: &Transport) -> f64 {
  if transport.tempo > 0. { transport.tempo } else { 120. }
}

// how fast it goes, as its knobs set it: `hz` free, or once each `note`
// synced
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rate {
  Free(f64),
  Synced(usize),
}

impl Rate {
  pub fn hz(self, transport: &Transport) -> f64 {
    match self {
      Rate::Free(hz) => hz,
      Rate::Synced(note) => tempo_of(transport) / 60. / quarters(note),
    }
  }
}

// how far behind (or ahead of) the song it's caught up over: a quarter of
// a second, but never faster or slower than CATCH_UP_MOST of its rate
const CATCH_UP: f64 = 0.25;
const CATCH_UP_MOST: f64 = 0.25;

#[derive(Clone, Copy, Debug)]
pub struct Lfo {
  // 0 to 1
  phase: f64,
}

impl Default for Lfo {
  fn default() -> Self {
    Lfo::new()
  }
}

impl Lfo {
  pub fn new() -> Lfo {
    Lfo { phase: 0. }
  }

  pub fn phase(&self) -> f64 {
    self.phase
  }

  pub fn reset(&mut self) {
    self.phase = 0.;
  }

  // where the song says a synced LFO should be at `frame` frames into
  // the block (none when free, or stopped)
  pub fn song_phase(rate: Rate, transport: &Transport, frame: usize) -> Option<f64> {
    let Rate::Synced(note) = rate else { return None };
    if !transport.playing || transport.sample_rate == 0 { return None; }
    let seconds = (transport.position + frame as i64) as f64 / transport.sample_rate as f64;
    let cycles = seconds * tempo_of(transport) / 60. / quarters(note);
    Some(cycles - cycles.floor())
  }

  // a stretch of `frames`: a step for each frame, `each(i, phase)` called
  // with the phase the frame starts at
  #[inline]
  pub fn run(&mut self, rate: Rate, transport: &Transport, frames: usize, mut each: impl FnMut(usize, f64)) {
    let sample_rate = transport.sample_rate.max(1) as f64;
    let step = rate.hz(transport) / sample_rate;
    let catch_up = 1. / (CATCH_UP * sample_rate);
    let synced = Lfo::song_phase(rate, transport, 0).is_some();
    for i in 0..frames {
      each(i, self.phase);
      let mut step = step;
      if synced {
        let song = Lfo::song_phase(rate, transport, i).unwrap_or(self.phase);
        // the shorter way round
        let mut behind = song - self.phase;
        behind -= behind.round();
        step += (behind * catch_up).clamp(-CATCH_UP_MOST * step, CATCH_UP_MOST * step);
      }
      self.phase += step;
      self.phase -= self.phase.floor();
    }
  }
}

// ---- Wander: a smooth random line from -1 to 1

// a small, repeatable random number source (xorshift): the same seed gives
// the same line, so an effect sounds the same however it's cut into blocks
// and however many times it's played
#[derive(Clone, Copy, Debug)]
pub struct Random(u64);

impl Random {
  pub fn new(seed: u64) -> Random {
    Random(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
  }

  // -1 to 1
  pub fn next(&mut self) -> f32 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2. - 1.
  }
}

// a curve through random points, one every 1/hz seconds, as smooth as the
// Hermite through them (no corners). it stays within -1 to 1, a little
// over at most: the effects that use it scale it by what they can take
#[derive(Clone, Copy, Debug)]
pub struct Wander {
  seed: u64,
  random: Random,
  points: [f32; 4],
  // how far it's from points[1] to points[2], 0 to 1
  t: f64,
}

impl Wander {
  pub fn new(seed: u64) -> Wander {
    let mut wander = Wander { seed, random: Random::new(seed), points: [0.; 4], t: 0. };
    wander.reset();
    wander
  }

  // back to the start of its line
  pub fn reset(&mut self) {
    self.random = Random::new(self.seed);
    for p in &mut self.points { *p = self.random.next(); }
    self.t = 0.;
  }

  // where it is now
  pub fn value(&self) -> f32 {
    let [a, b, c, d] = self.points;
    hermite(a, b, c, d, self.t as f32).clamp(-1.25, 1.25)
  }

  // move on a frame: `step` is the points' rate over the sample rate
  #[inline]
  pub fn step(&mut self, step: f64) {
    self.t += step;
    while self.t >= 1. {
      self.t -= 1.;
      self.points = [self.points[1], self.points[2], self.points[3], self.random.next()];
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn playing(position: i64, tempo: f64) -> Transport {
    Transport { playing: true, tempo, position, sample_rate: 48_000, ..Transport::default() }
  }

  #[test]
  fn test_each_note_value_is_as_long_as_its_label_says() {
    for (i, label) in NOTES.iter().enumerate() {
      let (fraction, kind) = label.split_once(' ').unwrap_or((label, ""));
      let (a, b) = fraction.split_once('/').unwrap();
      let wholes: f64 = a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap();
      let times = match kind { "D" => 1.5, "T" => 2. / 3., _ => 1. };
      assert!((quarters(i) - 4. * wholes * times).abs() < 1e-9, "{} is {} quarters", label, quarters(i));
    }
    // shortest first
    assert!((1..NOTES.len()).all(|i| quarters(i) > quarters(i - 1)));
  }

  #[test]
  fn test_the_shapes_go_from_their_bottom_to_their_top_and_back() {
    for shape in [Shape::Sine, Shape::Triangle, Shape::Lamp] {
      assert!((shape.at(0.) + 1.).abs() < 1e-6, "{:?}", shape);
      assert!((shape.at(1.) + 1.).abs() < 1e-6, "{:?}", shape);
      let top = (0..1000).map(|i| shape.at(i as f64 / 1000.)).fold(f32::MIN, f32::max);
      assert!((top - 1.).abs() < 1e-3, "{:?} tops out at {}", shape, top);
      // no jumps
      for i in 0..1000 {
        let step = (shape.at((i + 1) as f64 / 1000.) - shape.at(i as f64 / 1000.)).abs();
        assert!(step < 0.02, "{:?} jumps {} at {}", shape, step, i);
      }
    }
    // the sine and triangle peak halfway; the lamp sooner
    assert!((Shape::Sine.at(0.5) - 1.).abs() < 1e-6);
    assert!((Shape::Triangle.at(0.5) - 1.).abs() < 1e-6);
    assert!((Shape::Lamp.at(LAMP_RISE) - 1.).abs() < 1e-6);
  }

  #[test]
  fn test_free_it_goes_round_at_its_rate() {
    let mut lfo = Lfo::new();
    let transport = Transport { sample_rate: 48_000, ..Transport::default() };
    // 2 Hz for 3/4 of a second: a cycle and a half
    lfo.run(Rate::Free(2.), &transport, 36_000, |_, _| {});
    assert!((lfo.phase() - 0.5).abs() < 1e-6, "it's at {}", lfo.phase());
  }

  #[test]
  fn test_synced_it_goes_round_once_a_note() {
    // a half note at 120 bpm is a second
    let half = NOTES.iter().position(|n| *n == "1/2").unwrap();
    assert!((Rate::Synced(half).hz(&playing(0, 120.)) - 1.).abs() < 1e-9);
    let two_bars = NOTES.iter().position(|n| *n == "2/1").unwrap();
    assert!((Rate::Synced(two_bars).hz(&playing(0, 120.)) - 0.25).abs() < 1e-9);
  }

  #[test]
  fn test_synced_and_playing_it_catches_up_to_the_song_without_jumping() {
    let quarter = NOTES.iter().position(|n| *n == "1/4").unwrap();
    let rate = Rate::Synced(quarter);
    let mut lfo = Lfo::new();
    // the song's at a quarter of a cycle (an eighth note at 120 bpm is
    // 0.25 s): the LFO starts at 0, behind it
    let mut position = 6000;
    let step = 2. / 48_000.;
    for _ in 0..200 {
      let transport = playing(position, 120.);
      let mut last = lfo.phase();
      lfo.run(rate, &transport, 512, |_, phase| {
        // each step is within a quarter of its usual
        let mut moved = phase - last;
        moved -= moved.floor();
        assert!(moved <= step * 1.25 + 1e-12, "it moved {}", moved);
        last = phase;
      });
      position += 512;
    }
    let song = Lfo::song_phase(rate, &playing(position, 120.), 0).unwrap();
    let mut gap = song - lfo.phase();
    gap -= gap.round();
    assert!(gap.abs() < 1e-4, "it's {} from the song", gap);
  }

  #[test]
  fn test_stopped_a_synced_lfo_runs_free() {
    let quarter = NOTES.iter().position(|n| *n == "1/4").unwrap();
    let mut lfo = Lfo::new();
    let transport = Transport { playing: false, position: 6000, sample_rate: 48_000, ..Transport::default() };
    // a quarter at 120 is half a second: 12_000 frames is half a cycle
    lfo.run(Rate::Synced(quarter), &transport, 12_000, |_, _| {});
    assert!((lfo.phase() - 0.5).abs() < 1e-6);
  }

  #[test]
  fn test_wander_is_smooth_bounded_and_the_same_each_time() {
    let line = |seed| {
      let mut w = Wander::new(seed);
      (0..48_000).map(|_| { w.step(4. / 48_000.); w.value() }).collect::<Vec<f32>>()
    };
    let a = line(7);
    assert_eq!(a, line(7));
    assert_ne!(a, line(8));
    assert!(a.iter().all(|x| x.abs() <= 1.25));
    // it gets about: not stuck near one value
    let (lo, hi) = a.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
    assert!(hi - lo > 0.5, "it only went from {} to {}", lo, hi);
    // smooth: at 4 points a second, no sample moves much
    assert!(a.windows(2).all(|w| (w[1] - w[0]).abs() < 1e-3));
  }

  #[test]
  fn test_random_numbers_stay_in_range() {
    let mut r = Random::new(1);
    for _ in 0..10_000 {
      let x = r.next();
      assert!((-1. ..1.).contains(&x));
    }
  }
}
