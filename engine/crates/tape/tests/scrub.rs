// scrubbing: a recording read at the tape's moving speed, either way. the
// pitch follows the speed, nothing's heard off the ends, and nothing
// allocates

use tape::scrub::{Drive, Scrubber, WIND_LEVEL};

mod counting {
  use std::alloc::{GlobalAlloc, Layout, System};
  use std::cell::Cell;

  thread_local! {
    static MADE: Cell<usize> = const { Cell::new(0) };
  }

  pub struct Counting;

  unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
      let _ = MADE.try_with(|made| made.set(made.get() + 1));
      System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
      System.dealloc(ptr, layout)
    }
  }

  #[global_allocator]
  static COUNTING: Counting = Counting;

  pub fn made() -> usize {
    MADE.with(|made| made.get())
  }
}

const RATE: f64 = 24_000.0;
const OUT: f64 = 48_000.0;

// `seconds` of a sine at `hz`, half scale, at the recording's rate
fn tone(hz: f64, seconds: f64) -> Vec<i16> {
  (0..(RATE * seconds) as usize)
    .map(|i| ((2.0 * std::f64::consts::PI * hz * i as f64 / RATE).sin() * 16_000.0) as i16)
    .collect()
}

// the frequency of the left side, from its rising zero crossings
fn frequency(x: &[f32]) -> f64 {
  let left: Vec<f32> = x.iter().step_by(2).copied().collect();
  let ups: Vec<usize> = (1..left.len()).filter(|&i| left[i - 1] < 0.0 && left[i] >= 0.0).collect();
  (ups.len() - 1) as f64 * OUT / (ups[ups.len() - 1] - ups[0]) as f64
}

fn peak(x: &[f32]) -> f32 {
  x.iter().fold(0f32, |m, s| m.max(s.abs()))
}

#[test]
fn wound_either_way_the_pitch_follows_the_speed() {
  let recording = tone(500.0, 20.0);
  for speed in [1.0, -1.0, 2.0, -6.0] {
    let mut s = Scrubber::new(RATE, OUT, 10.0, Drive::Wind(speed));
    let mut out = vec![0.0; 2 * 24_000];
    // past the spin up
    s.add_to(&recording, &mut out);
    let mut out = vec![0.0; 2 * 24_000];
    s.add_to(&recording, &mut out);
    let f = frequency(&out);
    assert!((f / (500.0 * speed.abs()) - 1.0).abs() < 2e-3, "{speed}: {f} hz");
    // a little quieter winding; and dulled when fast
    let p = peak(&out);
    assert!(p <= 0.5 * WIND_LEVEL as f32 * 1.02, "{speed}: {p}");
    if speed.abs() <= 1.0 {
      assert!(p > 0.45 * WIND_LEVEL as f32, "{speed}: {p}");
    }
  }
}

#[test]
fn off_the_ends_of_the_recording_it_s_silent() {
  let recording = tone(500.0, 1.0);
  let mut s = Scrubber::new(RATE, OUT, 0.5, Drive::Wind(-4.0));
  let mut out = vec![0.0; 2 * 48_000];
  s.add_to(&recording, &mut out);
  assert!(s.at() < -1.0);
  // (the low-pass rings for a moment as it runs off)
  assert!(out[2 * 24_000..].iter().all(|&x| x.abs() < 1e-4));
}

#[test]
fn a_held_drag_is_silent_and_letting_go_fades_out() {
  let recording = tone(500.0, 5.0);
  let mut s = Scrubber::new(RATE, OUT, 1.0, Drive::Follow(1.5));
  let mut out = vec![0.0; 2 * 48_000];
  s.add_to(&recording, &mut out);
  // it got there and stopped: nothing heard at the end
  let still = &out[out.len() - 2_000..];
  assert!(peak(still) < 1e-3, "{}", peak(still));
  assert!(peak(&out) > 0.2);
  s.set_drive(Drive::Follow(3.0));
  s.end();
  let mut out = vec![0.0; 2 * 4_800];
  s.add_to(&recording, &mut out);
  assert!(s.done());
}

#[test]
fn the_warble_wavers_the_tape_s_motion() {
  let recording = tone(500.0, 20.0);
  let mut plain = Scrubber::new(RATE, OUT, 1.0, Drive::Wind(2.0));
  let mut warbled = Scrubber::new(RATE, OUT, 1.0, Drive::Wind(2.0));
  let mut out = vec![0.0; 2 * 480];
  let fast = vec![1.01f32; 480];
  for _ in 0..100 {
    plain.add_to(&recording, &mut out);
    warbled.add_warbled(&recording, &mut out, &fast);
  }
  // a second, at 1 % fast: 1 % further
  let (a, b) = (plain.at() - 1.0, warbled.at() - 1.0);
  assert!((b / a - 1.01).abs() < 1e-6, "{a} {b}");
}

#[test]
fn reading_allocates_nothing() {
  let recording = tone(500.0, 30.0);
  let mut s = Scrubber::new(RATE, OUT, 10.0, Drive::Follow(12.0));
  let mut out = vec![0.0; 2 * 512];
  let before = counting::made();
  for n in 0..400 {
    // forwards, a flip backwards, a flick, then winding both ways
    let drive = match n / 80 {
      0 => Drive::Follow(12.0),
      1 => Drive::Follow(9.0),
      2 => Drive::Follow(20.0),
      3 => Drive::Wind(10.0),
      _ => Drive::Wind(-10.0),
    };
    s.set_drive(drive);
    s.add_to(&recording, &mut out);
  }
  s.end();
  s.add_to(&recording, &mut out);
  assert_eq!(counting::made() - before, 0, "allocated on the audio thread");
}

// S4's numbers: what reading costs a frame, at 1x and winding, and what the
// tape's warble (the transport's speed, multiplied into the reading speed)
// would add. run with `cargo test --release -p dawesome-tape --test scrub
// -- --ignored --nocapture`
#[test]
#[ignore = "a measurement"]
fn measure_costs() {
  use std::time::Instant;
  let recording = tone(500.0, 120.0);
  let frames = 48_000 * 20;
  let mut out = vec![0.0; 2 * 512];
  for (name, drive) in [
    ("dragged steadily at 1x", Drive::Follow(1_000.0)),
    ("wound at 1x", Drive::Wind(1.0)),
    ("wound at 2x (the crossfade)", Drive::Wind(2.0)),
    ("wound at 10x", Drive::Wind(10.0)),
    ("wound back at 10x", Drive::Wind(-10.0)),
  ] {
    let mut s = Scrubber::new(RATE, OUT, 60.0, drive);
    // (past the spin up; a drag far off runs at its top speed meanwhile)
    if let Drive::Follow(_) = drive { s.set_drive(Drive::Follow(60.0)); }
    for _ in 0..100 { s.add_to(&recording, &mut out); }
    if let Drive::Follow(_) = drive { s.set_drive(Drive::Follow(80.0)); }
    let started = Instant::now();
    let mut target = 80.0;
    for _ in 0..frames / 512 {
      // (a drag moving steadily at play speed)
      if let Drive::Follow(_) = drive { target += 512.0 / OUT; s.set_drive(Drive::Follow(target)); }
      s.add_to(&recording, &mut out);
    }
    let ns = started.elapsed().as_nanos() as f64 / frames as f64;
    println!("scrub reader, {name}: {ns:.1} ns a frame ({:.3} % of a core at 48 kHz)", ns * 48_000.0 / 1e7);
  }

  let mut transport = tape::TapeTransport::new(tape::TransportSettings::default(), OUT as f32, 7);
  let mut speed = vec![0.0f32; 512];
  let started = Instant::now();
  for _ in 0..frames / 512 { transport.fill_speed(&mut speed); }
  let ns = started.elapsed().as_nanos() as f64 / frames as f64;
  println!("the tape's warble (the transport's speed a frame): {ns:.1} ns a frame ({:.3} % of a core)", ns * 48_000.0 / 1e7);
}
