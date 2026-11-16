// a fast Fourier transform: radix 2, in place, for analyzers. small and
// plain: it runs where the app asks for a spectrum, never on the audio
// thread

use std::f64::consts::PI;

// transform `re` and `im` (a power of two long) in place
pub fn fft(re: &mut [f64], im: &mut [f64]) {
  let n = re.len();
  assert!(n.is_power_of_two() && im.len() == n, "an FFT takes two halves the same power of two long");

  // bit-reversed order
  let mut j = 0;
  for i in 1..n {
    let mut bit = n >> 1;
    while j & bit != 0 {
      j ^= bit;
      bit >>= 1;
    }
    j |= bit;
    if i < j {
      re.swap(i, j);
      im.swap(i, j);
    }
  }

  let mut len = 2;
  while len <= n {
    let angle = -2. * PI / len as f64;
    let (w_re, w_im) = (angle.cos(), angle.sin());
    for start in (0..n).step_by(len) {
      let (mut c_re, mut c_im) = (1f64, 0f64);
      for k in 0..len / 2 {
        let (a, b) = (start + k, start + k + len / 2);
        let t_re = re[b] * c_re - im[b] * c_im;
        let t_im = re[b] * c_im + im[b] * c_re;
        re[b] = re[a] - t_re;
        im[b] = im[a] - t_im;
        re[a] += t_re;
        im[a] += t_im;
        let next = c_re * w_re - c_im * w_im;
        c_im = c_re * w_im + c_im * w_re;
        c_re = next;
      }
    }
    len <<= 1;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_an_impulse_has_every_frequency_equally() {
    let mut re = vec![0.; 16];
    let mut im = vec![0.; 16];
    re[0] = 1.;
    fft(&mut re, &mut im);
    for k in 0..16 {
      assert!((re[k] - 1.).abs() < 1e-12 && im[k].abs() < 1e-12);
    }
  }

  #[test]
  fn test_a_sine_lands_in_its_bin() {
    let n = 64;
    let mut re: Vec<f64> = (0..n).map(|i| (2. * PI * 5. * i as f64 / n as f64).sin()).collect();
    let mut im = vec![0.; n];
    fft(&mut re, &mut im);
    let mags: Vec<f64> = (0..n).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect();
    // half its energy at bin 5, half mirrored at n - 5
    assert!((mags[5] - n as f64 / 2.).abs() < 1e-9);
    assert!((mags[n - 5] - n as f64 / 2.).abs() < 1e-9);
    assert!(mags.iter().enumerate().filter(|(k, _)| *k != 5 && *k != n - 5).all(|(_, m)| *m < 1e-9));
  }
}
