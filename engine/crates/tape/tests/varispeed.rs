// varispeed: the song read at another speed. it must step out of the way
// at 1x (what's rendered goes straight out), keep the song in order when
// it steps in and out, read at the speed it's set to, and never allocate

use tape::varispeed::{Stretch, Varispeed};

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

const RATE: f64 = 48_000.0;
const BLOCK: usize = 4096;

// a song that's a sine at `hz`, counted out a frame at a time
struct Song {
  next: u64,
  hz: f64,
}

impl Song {
  fn fill(&mut self, out: &mut [f32]) {
    for frame in out.chunks_exact_mut(2) {
      let v = (2.0 * std::f64::consts::PI * self.hz * self.next as f64 / RATE).sin() as f32;
      frame[0] = v;
      frame[1] = -v;
      self.next += 1;
    }
  }
}

// play `buffers` buffers of `frames` through the varispeed, as the engine
// does: straight while it's straight
fn play(v: &mut Varispeed, song: &mut Song, buffers: usize, frames: usize) -> Vec<f32> {
  let mut all = Vec::new();
  let mut out = vec![0.0; frames * 2];
  for _ in 0..buffers {
    if v.straight() {
      song.fill(&mut out);
      v.remember(&out);
    } else {
      v.render(&mut out, &mut |chunk| song.fill(chunk));
    }
    all.extend_from_slice(&out);
  }
  all
}

// the frequency of the left side, from its rising zero crossings
fn frequency(x: &[f32]) -> f64 {
  let left: Vec<f32> = x.iter().step_by(2).copied().collect();
  let ups: Vec<usize> = (1..left.len()).filter(|&i| left[i - 1] < 0.0 && left[i] >= 0.0).collect();
  (ups.len() - 1) as f64 * RATE / (ups[ups.len() - 1] - ups[0]) as f64
}

#[test]
fn at_1x_it_never_steps_in() {
  let mut v = Varispeed::new(RATE, BLOCK, 0.5, 0.6);
  v.flywheel.set_target(1.0);
  assert!(v.straight());
}

#[test]
fn it_reads_at_its_speed_pitch_and_time_together() {
  for speed in [0.88, 1.12] {
    let mut v = Varispeed::new(RATE, BLOCK, 0.5, 0.6);
    let mut song = Song { next: 0, hz: 1_000.0 };
    v.flywheel.set_target(speed);
    // settled, after the glide
    play(&mut v, &mut song, 100, 512);
    let before = song.next;
    let out = play(&mut v, &mut song, 40, 512);
    let f = frequency(&out);
    assert!((f / 1_000.0 - speed).abs() < 1e-3, "{speed}: {f} hz");
    // the song went by at the speed
    let read = (song.next - before) as f64 / (40.0 * 512.0);
    assert!((read - speed).abs() < 2e-3, "{speed}: read {read}");
    // and loud as it was (the sinc is flat at 1 khz)
    let peak = out.iter().fold(0f32, |m, s| m.max(s.abs()));
    assert!((peak - 1.0).abs() < 0.01, "{peak}");
  }
}

#[test]
fn back_at_1x_it_steps_out_and_the_song_carries_on_in_order() {
  let mut v = Varispeed::new(RATE, BLOCK, 0.5, 0.6);
  // a song that counts its frames, so its order shows
  let mut next = 0u64;
  let mut fill = |out: &mut [f32]| {
    for frame in out.chunks_exact_mut(2) {
      frame[0] = next as f32;
      frame[1] = next as f32;
      next += 1;
    }
  };
  let mut out = vec![0.0; 256 * 2];
  for _ in 0..4 {
    fill(&mut out);
    v.remember(&out);
  }
  v.flywheel.set_target(1.05);
  for _ in 0..20 {
    v.render(&mut out, &mut fill);
  }
  v.flywheel.set_target(1.0);
  // the glide (about 1.2 s to settle) and the ease onto a whole frame
  // (under 0.1 s): out within 2 s
  let mut buffers = 0;
  while !v.straight() {
    v.render(&mut out, &mut fill);
    buffers += 1;
    assert!(buffers < 2 * 48_000 / 256, "still resampling");
  }
  // the last buffer before it stepped out ended on whole frames, one
  // after another, and straight playback picks up on the next
  let last = out[out.len() - 2];
  assert_eq!(last.fract(), 0.0);
  assert_eq!(out[out.len() - 4], last - 1.0);
  fill(&mut out);
  assert_eq!(out[0], last + 1.0);
}

#[test]
fn a_coast_fades_to_silence_and_stops_reading() {
  let mut v = Varispeed::new(RATE, BLOCK, 0.5, 0.6);
  let mut song = Song { next: 0, hz: 440.0 };
  v.flywheel.cut();
  let out = play(&mut v, &mut song, 60, 512);
  // 0.64 s: stopped, silent
  let tail = &out[out.len() - 200..];
  assert!(tail.iter().all(|s| s.abs() < 1e-6));
  let read = song.next;
  play(&mut v, &mut song, 4, 512);
  // only the queue's reach is read, no more
  assert!(song.next - read < 64, "{}", song.next - read);
  // a coast covers about half its time in song
  assert!((read as f64 / RATE - 0.3).abs() < 0.01, "{}", read as f64 / RATE);
}

#[test]
fn reading_allocates_nothing() {
  let mut v = Varispeed::new(RATE, BLOCK, 0.5, 0.6);
  let mut song = Song { next: 0, hz: 1_000.0 };
  let mut out = vec![0.0; 1024 * 2];
  let before = counting::made();
  v.flywheel.set_target(1.12);
  for _ in 0..50 {
    v.render(&mut out, &mut |chunk| song.fill(chunk));
  }
  v.flywheel.cut();
  for _ in 0..50 {
    v.render(&mut out, &mut |chunk| song.fill(chunk));
  }
  v.flywheel.run(true);
  for _ in 0..50 {
    v.render(&mut out, &mut |chunk| song.fill(chunk));
  }
  assert_eq!(counting::made() - before, 0, "allocated on the audio thread");
}

#[test]
fn a_stretch_is_the_song_at_the_speed_its_length_changed() {
  let speed = 1.1;
  let mut s = Stretch::new(speed);
  let mut song = Song { next: 0, hz: 1_000.0 };
  let mut out = Vec::new();
  let mut chunk = vec![0.0; 4096 * 2];
  for _ in 0..12 {
    song.fill(&mut chunk);
    s.push(&chunk, &mut out);
  }
  s.finish(&mut out);
  let frames = out.len() / 2;
  assert_eq!(frames, (12.0 * 4096.0 / speed).ceil() as usize);
  assert!((frequency(&out[2_000..out.len() - 2_000]) / 1_000.0 - speed).abs() < 1e-3);
}
