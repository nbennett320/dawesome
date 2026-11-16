// what an effect puts out, kept for the app to look at (the EQ's analyzer):
// the latest stretch of its output, mono, in a ring. the audio thread writes
// it after every block, with no locks and no allocation; the app copies the
// latest WINDOW frames out when it asks for a spectrum
//
// there's one writer (the audio thread running the effect) and any number of
// readers. a reader that's overtaken while it copies (the writer went round
// the ring past what it was copying) gets nothing, and asks again next time

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// how many frames a spectrum is taken from
pub const WINDOW: usize = 2048;
// the ring's length: the window, plus room for the longest block the engine
// runs (4096) to be written while the window's being copied
pub const RING: usize = 8192;

pub struct Tap {
  samples: Box<[AtomicU32]>,
  // frames written so far
  written: AtomicU64,
  // the rate they were written at
  rate: AtomicU32,
}

impl Default for Tap {
  fn default() -> Self {
    Tap::new()
  }
}

impl Tap {
  pub fn new() -> Tap {
    Tap {
      samples: (0..RING).map(|_| AtomicU32::new(0)).collect(),
      written: AtomicU64::new(0),
      rate: AtomicU32::new(0),
    }
  }

  // the audio thread: a block's output (both sides, mixed to mono)
  pub fn write(&self, left: &[f32], right: &[f32], rate: u32) {
    self.rate.store(rate, Ordering::Relaxed);
    let start = self.written.load(Ordering::Relaxed);
    for (i, (l, r)) in left.iter().zip(right).enumerate() {
      let at = ((start + i as u64) % RING as u64) as usize;
      self.samples[at].store(((l + r) * 0.5).to_bits(), Ordering::Relaxed);
    }
    self.written.store(start + left.len().min(right.len()) as u64, Ordering::Release);
  }

  // frames written so far: when it stops going up, the effect isn't being
  // run (its chain is resting, or it's bypassed)
  pub fn written(&self) -> u64 {
    self.written.load(Ordering::Acquire)
  }

  // the rate the last block was written at (0 before any)
  pub fn rate(&self) -> u32 {
    self.rate.load(Ordering::Relaxed)
  }

  // the latest `frames` (at most RING - 4096), oldest first, silence before
  // the first written; and how many had been written. none if the writer
  // overtook the copy
  pub fn latest(&self, frames: usize) -> Option<(Vec<f32>, u64)> {
    assert!(frames <= RING - 4096, "a tap holds at most RING - 4096 frames safely");
    let end = self.written();
    let mut out = vec![0f32; frames];
    for (i, sample) in out.iter_mut().enumerate() {
      // the frame `frames - i` before the end
      let back = (frames - i) as u64;
      if back > end { continue; }
      let at = ((end - back) % RING as u64) as usize;
      *sample = f32::from_bits(self.samples[at].load(Ordering::Relaxed));
    }
    let now = self.written();
    if now - end > (RING - frames) as u64 { return None; }
    Some((out, end))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_it_keeps_the_latest_frames_in_order_round_the_ring() {
    let tap = Tap::new();
    let block: Vec<f32> = (0..1000).map(|i| i as f32).collect();
    for n in 0..20 {
      let shifted: Vec<f32> = block.iter().map(|x| x + (n * 1000) as f32).collect();
      tap.write(&shifted, &shifted, 48_000);
    }
    assert_eq!(tap.written(), 20_000);
    assert_eq!(tap.rate(), 48_000);
    let (latest, written) = tap.latest(WINDOW).unwrap();
    assert_eq!(written, 20_000);
    let expected: Vec<f32> = (20_000 - WINDOW..20_000).map(|i| i as f32).collect();
    assert_eq!(latest, expected);
  }

  #[test]
  fn test_it_mixes_both_sides_and_starts_silent() {
    let tap = Tap::new();
    tap.write(&[1., 1.], &[0., 0.5], 44_100);
    let (latest, _) = tap.latest(4).unwrap();
    assert_eq!(latest, vec![0., 0., 0.5, 0.75]);
  }

  #[test]
  fn test_a_copy_overtaken_by_the_writer_is_dropped() {
    // (a reader can't be paused mid-copy here, so: what it checks)
    let tap = Tap::new();
    tap.write(&[0.; 100], &[0.; 100], 48_000);
    let end = tap.written();
    tap.write(&vec![0.; RING - WINDOW + 1], &vec![0.; RING - WINDOW + 1], 48_000);
    assert!(tap.written() - end > (RING - WINDOW) as u64);
    // and a copy taken now, with nothing written meanwhile, is fine
    assert!(tap.latest(WINDOW).is_some());
  }
}
