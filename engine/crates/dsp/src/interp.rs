// reading between samples: a value part of the way from one sample to the
// next, for anything that reads sound at a moving or fractional position (a
// delay line's read, and later a chorus's, or a sample played at a pitch)

// 4-point, 3rd-order Hermite (Catmull-Rom): the value `t` of the way (0 to 1)
// from `x0` to `x1`, curving through the samples either side, `before` x0
// and `after` x1. at t = 0 it's x0 exactly, at t = 1 x1. it keeps the highs
// far better than a straight line between x0 and x1: at 48 kHz, halfway
// between samples, it's 0.5 dB down at 10 kHz where a straight line is 2 dB
#[inline]
pub fn hermite(before: f32, x0: f32, x1: f32, after: f32, t: f32) -> f32 {
  let c1 = 0.5 * (x1 - before);
  let c2 = before - 2.5 * x0 + 2. * x1 - 0.5 * after;
  let c3 = 0.5 * (after - before) + 1.5 * (x0 - x1);
  ((c3 * t + c2) * t + c1) * t + x0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_it_lands_on_the_samples_at_either_end() {
    assert_eq!(hermite(3., -1., 2., 5., 0.), -1.);
    assert_eq!(hermite(3., -1., 2., 5., 1.), 2.);
  }

  #[test]
  fn test_a_straight_line_stays_straight() {
    for t in [0.1, 0.25, 0.5, 0.9] {
      assert!((hermite(0., 1., 2., 3., t) - (1. + t)).abs() < 1e-6);
    }
  }

  #[test]
  fn test_it_follows_a_curve_closely() {
    // a slow sine, between its samples
    let wave = |n: f32| (n * 0.05).sin();
    for t in [0.2, 0.5, 0.7] {
      let got = hermite(wave(9.), wave(10.), wave(11.), wave(12.), t);
      assert!((got - wave(10. + t)).abs() < 1e-5, "at {}: {} vs {}", t, got, wave(10. + t));
    }
  }

  #[test]
  fn test_halfway_it_keeps_the_highs_as_said() {
    // a sine at 10 kHz (48 kHz), read halfway between every pair of
    // samples: its level against the sine itself
    let w = std::f32::consts::TAU * 10_000. / 48_000.;
    let wave = |n: f32| (w * n).sin();
    let (mut heard, mut real) = (0f64, 0f64);
    for n in 1..4000 {
      let x = n as f32;
      let got = hermite(wave(x - 1.), wave(x), wave(x + 1.), wave(x + 2.), 0.5);
      heard += (got as f64).powi(2);
      real += (wave(x + 0.5) as f64).powi(2);
    }
    let db = 10. * (heard / real).log10();
    assert!((db + 0.53).abs() < 0.05, "{} dB", db);
  }
}
