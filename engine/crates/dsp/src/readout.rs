// what an effect is doing now, for its panel to draw: a few numbers
// (an LFO's phase, where a delay's reading from, a notch's frequency)
// that the audio thread writes after every block and the app reads when
// it likes. each effect says what its slots hold (Effect::readout)
//
// one writer, any number of readers, no locks. a reader can catch a block
// half written (some slots from one block, some from the next). panels
// only draw from it, so a frame that's a block out doesn't matter

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// how many numbers an effect can report
pub const READOUT: usize = 32;

pub struct Readout {
  slots: [AtomicU32; READOUT],
  // frames run so far: when it stops going up, the effect isn't being
  // run (its chain is resting, or it's bypassed)
  written: AtomicU64,
  rate: AtomicU32,
}

impl Default for Readout {
  fn default() -> Self {
    Readout::new()
  }
}

impl Readout {
  pub fn new() -> Readout {
    Readout {
      slots: std::array::from_fn(|_| AtomicU32::new(0)),
      written: AtomicU64::new(0),
      rate: AtomicU32::new(0),
    }
  }

  // the audio thread, after a block of `frames`
  pub fn write(&self, slots: &[f32; READOUT], frames: usize, rate: u32) {
    for (slot, value) in self.slots.iter().zip(slots) {
      slot.store(value.to_bits(), Ordering::Relaxed);
    }
    self.rate.store(rate, Ordering::Relaxed);
    self.written.fetch_add(frames as u64, Ordering::Release);
  }

  // the slots, the frames run so far, and the rate (0 before any)
  pub fn read(&self) -> ([f32; READOUT], u64, u32) {
    let written = self.written.load(Ordering::Acquire);
    let slots = std::array::from_fn(|i| f32::from_bits(self.slots[i].load(Ordering::Relaxed)));
    (slots, written, self.rate.load(Ordering::Relaxed))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_it_gives_back_the_last_block_written() {
    let readout = Readout::new();
    assert_eq!(readout.read(), ([0.; READOUT], 0, 0));
    let mut slots = [0.; READOUT];
    slots[0] = 0.25;
    slots[31] = -3.;
    readout.write(&slots, 512, 48_000);
    slots[0] = 0.5;
    readout.write(&slots, 256, 48_000);
    let (got, written, rate) = readout.read();
    assert_eq!(got[0], 0.5);
    assert_eq!(got[31], -3.);
    assert_eq!(written, 768);
    assert_eq!(rate, 48_000);
  }
}
