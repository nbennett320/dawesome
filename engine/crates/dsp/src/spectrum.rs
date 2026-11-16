// a spectrum for an analyzer: the latest frames of a tap, windowed and
// transformed, as levels in dB at log-spaced frequencies (evenly by octave,
// as the EQ's curve is drawn). a full-scale sine reads 0 dB

use crate::fft::fft;

// the quietest level given (silence reads this)
pub const FLOOR: f32 = -120.;

// `bins` levels, in dB, at frequencies from `low` to `high` (evenly by
// ratio, the first at `low`, the last at `high`), from `samples` (a power of
// two long) taken at `rate`
pub fn spectrum(samples: &[f32], rate: f32, bins: usize, low: f32, high: f32) -> Vec<f32> {
  let n = samples.len();
  // a Hann window, and the gain that puts a full-scale sine at 1
  let window: Vec<f64> = (0..n).map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos()).collect();
  let scale = 2. / window.iter().sum::<f64>();
  let mut re: Vec<f64> = samples.iter().zip(&window).map(|(s, w)| *s as f64 * w).collect();
  let mut im = vec![0.; n];
  fft(&mut re, &mut im);
  let mags: Vec<f64> = (0..=n / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt() * scale).collect();

  // a frequency's place among the FFT's bins
  let per_bin = rate as f64 / n as f64;
  let place = |freq: f64| (freq / per_bin).clamp(0., (n / 2) as f64);
  let freq_of = |i: f64| low as f64 * (high as f64 / low as f64).powf(i / (bins - 1).max(1) as f64);

  (0..bins).map(|i| {
    // the FFT bins between this one's edges (half a step either side)...
    let (from, to) = (place(freq_of(i as f64 - 0.5)), place(freq_of(i as f64 + 0.5)));
    let (first, last) = (from.ceil() as usize, to.floor() as usize);
    let mag = if first <= last {
      mags[first..=last].iter().cloned().fold(0., f64::max)
    } else {
      // ...or, low down where there are none, between the two either side
      let at = place(freq_of(i as f64));
      let below = at.floor() as usize;
      let above = (below + 1).min(n / 2);
      let t = at - below as f64;
      mags[below] * (1. - t) + mags[above] * t
    };
    (20. * mag.max(1e-12).log10()).max(FLOOR as f64) as f32
  }).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const RATE: f32 = 48_000.;
  const N: usize = 2048;

  fn sine(freq: f32, amplitude: f32) -> Vec<f32> {
    (0..N).map(|i| amplitude * (std::f32::consts::TAU * freq * i as f32 / RATE).sin()).collect()
  }

  // the display bin nearest a frequency, of 256 from 20 Hz to 20 kHz
  fn bin_of(freq: f32) -> usize {
    ((freq / 20.).ln() / 1000f32.ln() * 255.).round() as usize
  }

  #[test]
  fn test_a_full_scale_sine_reads_0_db_at_its_frequency() {
    // (about 1 kHz, at an FFT bin's centre)
    let freq = (1000. / (RATE / N as f32)).round() * (RATE / N as f32);
    let levels = spectrum(&sine(freq, 1.), RATE, 256, 20., 20_000.);
    let peak = levels.iter().cloned().fold(f32::MIN, f32::max);
    assert!(peak.abs() < 0.1, "{}", peak);
    let at = levels.iter().position(|l| *l == peak).unwrap();
    assert!((at as i32 - bin_of(freq) as i32).abs() <= 1, "peak at {} not {}", at, bin_of(freq));
    // well down an octave and more away
    assert!(levels[bin_of(freq / 2.5)] < -40.);
    assert!(levels[bin_of(freq * 2.5)] < -40.);
  }

  #[test]
  fn test_a_quieter_sine_reads_quieter() {
    let freq = (3000. / (RATE / N as f32)).round() * (RATE / N as f32);
    let levels = spectrum(&sine(freq, 0.1), RATE, 256, 20., 20_000.);
    let peak = levels.iter().cloned().fold(f32::MIN, f32::max);
    assert!((peak + 20.).abs() < 0.1, "{}", peak);
  }

  #[test]
  fn test_every_level_is_a_number_for_silence_sines_and_noise() {
    let mut x = 1u32;
    let noise: Vec<f32> = (0..N).map(|_| { x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (x >> 8) as f32 / (1 << 24) as f32 - 0.5 }).collect();
    for samples in [vec![0.; N], sine(30., 1.), sine(19_000., 1.), noise] {
      let levels = spectrum(&samples, RATE, 256, 20., 20_000.);
      assert_eq!(levels.len(), 256);
      assert!(levels.iter().all(|l| l.is_finite() && *l >= FLOOR && *l <= 1.));
    }
    assert!(spectrum(&vec![0.; N], RATE, 256, 20., 20_000.).iter().all(|l| *l == FLOOR));
  }

  #[test]
  fn test_low_bins_between_the_ffts_follow_a_low_sine() {
    // 23 Hz apart, the FFT's bins: below 100 Hz most display bins fall
    // between them, and are drawn from their neighbours
    let levels = spectrum(&sine(50., 1.), RATE, 256, 20., 20_000.);
    let around = &levels[bin_of(40.)..=bin_of(60.)];
    assert!(around.iter().all(|l| *l > -12.), "{:?}", around);
  }
}
