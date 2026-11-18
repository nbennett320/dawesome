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
