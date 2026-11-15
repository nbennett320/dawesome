// a value gliding toward where it was last set, so turning a knob (or,
// later, automation) doesn't click. one-pole: it covers most of the way in
// TIME and slows as it nears, and lands exactly once it's close enough
//
// it moves in steps of a fixed number of frames (see Runner), and works in
// normalized values, so a frequency glides evenly by octave and a gain
// evenly by dB

// how long it takes to cover about 63% of the way
pub const TIME: f32 = 0.020;
// close enough to land (normalized: well below anything audible)
const LAND: f32 = 1e-5;

#[derive(Clone, Copy, Debug)]
pub struct Smoother {
  value: f32,
  target: f32,
  // how much of the gap is left after each step
  keep: f32,
}

impl Smoother {
  // stepping `frames` at a time at `rate`
  pub fn new(value: f32, frames: usize, rate: f32) -> Smoother {
    Smoother { value, target: value, keep: Self::keep(frames, rate) }
  }

  fn keep(frames: usize, rate: f32) -> f32 {
    (-(frames as f32) / (TIME * rate.max(1.))).exp()
  }

  pub fn set_rate(&mut self, frames: usize, rate: f32) {
    self.keep = Self::keep(frames, rate);
  }

  pub fn value(&self) -> f32 {
    self.value
  }

  pub fn target(&self) -> f32 {
    self.target
  }

  pub fn moving(&self) -> bool {
    self.value != self.target
  }

  pub fn glide_to(&mut self, target: f32) {
    self.target = target;
  }

  pub fn jump_to(&mut self, target: f32) {
    self.target = target;
    self.value = target;
  }

  // one step's worth nearer
  pub fn step(&mut self) {
    if !self.moving() { return; }
    self.value = self.target + (self.value - self.target) * self.keep;
    if (self.value - self.target).abs() < LAND {
      self.value = self.target;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_it_glides_and_then_lands() {
    let rate = 48_000.;
    let mut s = Smoother::new(0., 32, rate);
    s.glide_to(1.);
    let steps_in = |seconds: f32| (seconds * rate / 32.) as usize;
    for _ in 0..steps_in(TIME) { s.step(); }
    assert!((s.value() - 0.63).abs() < 0.03, "{}", s.value());
    for _ in 0..steps_in(TIME * 20.) { s.step(); }
    assert_eq!(s.value(), 1.);
    assert!(!s.moving());
  }
}
