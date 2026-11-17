// running something nonlinear (a saturation curve) at 2x or 4x the rate,
// so the harmonics it makes above the base rate's nyquist are filtered off
// instead of folding back down as aliasing
//
// the filters are polyphase iir half-bands (valenzuela and constantinides;
// the design laurent de soras's hiir uses): each is two chains of
// first-order allpasses, one per phase, so up- and down-sampling cost a
// handful of multiplies a sample and nothing is computed only to be thrown
// away. there's no look-ahead, so no latency to report (D2), but they're
// not instant either: a round trip through 2x delays low frequencies by
// about 2.3 base-rate samples (3.1 through 4x), rising to about 6.4 (7.4)
// at 0.4 of the rate. that's a phase shift, smaller than tape's own; but
// mixed with the dry sound it combs (a first notch near 10 khz at 48 khz)
//
// 4x is two 2x stages: the outer one (base <-> 2x) does the hard work; the
// inner one (2x <-> 4x) only has to keep what's under 0.4 of the base rate
// clear of its images, so it's gentler and cheaper

// the outer stage: passband to 0.4 of the base rate (19.2 khz at 48 khz),
// stopband from 0.6, so nothing folds into the passband less than 87 db
// down. flat within 1e-8 db by design (an elliptic half-band's passband
// ripple is the square of its stopband's), 2e-7 db once rounded to f32
// (measured, 2x and 4x). designed with hiir's formula: 5
// coefficients, transition 0.1 of the 2x rate; 4 would be 70 db exactly,
// with no room for rounding
const OUTER_EVEN: [f32; 3] = [0.054_230_78, 0.398_796_97, 0.862_917_8];
const OUTER_ODD: [f32; 2] = [0.199_699_58, 0.621_096_85];

// the inner stage: passband to 0.1 of the 4x rate (0.4 of the base),
// stopband from 0.4 of it, 104 db down: 3 coefficients, transition 0.3
const INNER_EVEN: [f32; 2] = [0.062_735_75, 0.665_815_1];
const INNER_ODD: [f32; 1] = [0.263_732_33];

// what's left in a filter's memory below this is cleared (it would only
// decay into denormals, which are slow on some cpus)
const TINY: f32 = 1e-20;

// a chain of first-order allpasses, (a + z^-1) / (1 + a z^-1) each
#[derive(Clone, Copy)]
struct Allpasses<const N: usize> {
  coefs: [f32; N],
  // the last input to the chain, and each section's last output (which is
  // also the next section's last input)
  input: f32,
  outputs: [f32; N],
}

impl<const N: usize> Allpasses<N> {
  fn new(coefs: [f32; N]) -> Self {
    Allpasses { coefs, input: 0.0, outputs: [0.0; N] }
  }

  #[inline]
  fn run(&mut self, x: f32) -> f32 {
    let mut x = x;
    let mut before = self.input;
    self.input = x;
    for (a, out) in self.coefs.iter().zip(self.outputs.iter_mut()) {
      let y = a * (x - *out) + before;
      before = *out;
      *out = y;
      x = y;
    }
    x
  }

  fn reset(&mut self) {
    self.input = 0.0;
    self.outputs = [0.0; N];
  }

  fn flush(&mut self) {
    if self.input.abs() < TINY {
      self.input = 0.0;
    }
    for out in self.outputs.iter_mut() {
      if out.abs() < TINY {
        *out = 0.0;
      }
    }
  }
}

// one half-band, h(z) = (a0(z^2) + z^-1 a1(z^2)) / 2, with its own state
// for the way up and the way down
#[derive(Clone, Copy)]
struct HalfBand<const E: usize, const O: usize> {
  up: (Allpasses<E>, Allpasses<O>),
  down: (Allpasses<E>, Allpasses<O>),
}

impl<const E: usize, const O: usize> HalfBand<E, O> {
  fn new(even: [f32; E], odd: [f32; O]) -> Self {
    let paths = (Allpasses::new(even), Allpasses::new(odd));
    HalfBand { up: paths, down: paths }
  }

  // each sample in becomes two: one from each phase. (zero-stuffing halves
  // the level, the two phases' sum doubles it back: no gain to fix)
  fn up(&mut self, input: &[f32], output: &mut [f32]) {
    for (x, pair) in input.iter().zip(output.chunks_exact_mut(2)) {
      pair[0] = self.up.0.run(*x);
      pair[1] = self.up.1.run(*x);
    }
  }

  // each two samples in become one: the later goes through the even
  // phase, the earlier through the odd (its z^-1)
  fn down(&mut self, input: &[f32], output: &mut [f32]) {
    for (pair, y) in input.chunks_exact(2).zip(output.iter_mut()) {
      *y = 0.5 * (self.down.0.run(pair[1]) + self.down.1.run(pair[0]));
    }
  }

  fn reset(&mut self) {
    self.up.0.reset();
    self.up.1.reset();
    self.down.0.reset();
    self.down.1.reset();
  }

  fn flush(&mut self) {
    self.up.0.flush();
    self.up.1.flush();
    self.down.0.flush();
    self.down.1.flush();
  }
}

// one channel's oversampling. its buffers are made in new(), for the
// largest factor it will run at and the longest block it's handed at once;
// longer inputs are worked through in pieces of that size
pub struct Oversampler {
  factor: usize,
  most: usize,
  max_block: usize,
  outer: HalfBand<3, 2>,
  inner: HalfBand<2, 1>,
  two: Vec<f32>,
  four: Vec<f32>,
}

impl Oversampler {
  // `factor` is 1 (no oversampling), 2 or 4, and the most set_factor can
  // go to later
  pub fn new(factor: usize, max_block: usize) -> Oversampler {
    assert!(matches!(factor, 1 | 2 | 4), "oversampling is 1x, 2x or 4x, not {factor}x");
    let max_block = max_block.max(1);
    Oversampler {
      factor,
      most: factor,
      max_block,
      outer: HalfBand::new(OUTER_EVEN, OUTER_ODD),
      inner: HalfBand::new(INNER_EVEN, INNER_ODD),
      two: vec![0.0; if factor >= 2 { 2 * max_block } else { 0 }],
      four: vec![0.0; if factor >= 4 { 4 * max_block } else { 0 }],
    }
  }

  pub fn factor(&self) -> usize {
    self.factor
  }

  // the most it was made for
  pub fn max_factor(&self) -> usize {
    self.most
  }

  pub fn max_block(&self) -> usize {
    self.max_block
  }

  // change the factor, up to the one it was made with (its buffers are
  // that size, so this doesn't allocate). a change starts from silence:
  // the other factor's filters hold nothing of the sound so far
  pub fn set_factor(&mut self, factor: usize) {
    assert!(
      matches!(factor, 1 | 2 | 4) && factor <= self.most,
      "can't oversample {factor}x: made for up to {}x",
      self.most
    );
    if factor != self.factor {
      self.factor = factor;
      self.reset();
    }
  }

  // back to silence
  pub fn reset(&mut self) {
    self.outer.reset();
    self.inner.reset();
  }

  // `input` up to the oversampled rate, `f` on that, and back down into
  // `output` (the same length as `input`). `f` is called once for each
  // piece of up to max_block base-rate samples, with factor times as many
  pub fn process(&mut self, input: &[f32], output: &mut [f32], mut f: impl FnMut(&mut [f32])) {
    assert_eq!(input.len(), output.len(), "input and output differ in length");
    for (i, o) in input.chunks(self.max_block).zip(output.chunks_mut(self.max_block)) {
      o.copy_from_slice(i);
      self.run(o, &mut f);
    }
  }

  // the same, with the result written over the input
  pub fn process_in_place(&mut self, buffer: &mut [f32], mut f: impl FnMut(&mut [f32])) {
    for piece in buffer.chunks_mut(self.max_block) {
      self.run(piece, &mut f);
    }
  }

  fn run<F: FnMut(&mut [f32])>(&mut self, piece: &mut [f32], f: &mut F) {
    let n = piece.len();
    match self.factor {
      1 => f(piece),
      2 => {
        let two = &mut self.two[..2 * n];
        self.outer.up(piece, two);
        f(two);
        self.outer.down(two, piece);
        self.outer.flush();
      }
      _ => {
        let two = &mut self.two[..2 * n];
        let four = &mut self.four[..4 * n];
        self.outer.up(piece, two);
        self.inner.up(two, four);
        f(four);
        self.inner.down(four, two);
        self.outer.down(two, piece);
        self.outer.flush();
        self.inner.flush();
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::f64::consts::PI;

  const RATE: f64 = 48_000.0;

  // a sine's amplitude and phase at `freq` (cycles a sample) in `y`, and
  // what's left once it's taken out, as a share of its amplitude. `y`
  // must hold a whole number of cycles
  fn fit(y: &[f32], freq: f64) -> (f64, f64, f64) {
    let n = y.len() as f64;
    let (mut c, mut s) = (0.0, 0.0);
    for (i, &v) in y.iter().enumerate() {
      let p = 2.0 * PI * freq * i as f64;
      c += v as f64 * p.cos();
      s += v as f64 * p.sin();
    }
    let (c, s) = (2.0 * c / n, 2.0 * s / n);
    let amp = (c * c + s * s).sqrt();
    let mut rest = 0.0;
    for (i, &v) in y.iter().enumerate() {
      let p = 2.0 * PI * freq * i as f64;
      rest += (v as f64 - c * p.cos() - s * p.sin()).powi(2);
    }
    // the rest's rms against the sine's
    let rest = (rest / n).sqrt() / (amp / 2f64.sqrt());
    (amp, s.atan2(c), rest)
  }

  fn db(v: f64) -> f64 {
    20.0 * v.log10()
  }

  fn sine(freq: f64, frames: usize) -> Vec<f32> {
    (0..frames).map(|i| (2.0 * PI * freq * i as f64).sin() as f32 * 0.5).collect()
  }

  // settled: the filters' start-up has died away by then
  const SETTLE: usize = 4800;
  // 0.1 s: whole cycles of every multiple of 10 hz
  const WINDOW: usize = 4800;

  #[test]
  fn passband_is_flat_to_0_4_of_the_rate() {
    for factor in [2, 4] {
      let mut worst = 0.0f64;
      let mut freq = 20.0;
      while freq <= 19_200.0 {
        let mut os = Oversampler::new(factor, 512);
        let x = sine(freq / RATE, SETTLE + WINDOW);
        let mut y = vec![0.0; x.len()];
        os.process(&x, &mut y, |_| {});
        let (amp, _, _) = fit(&y[SETTLE..], freq / RATE);
        worst = worst.max(db(amp / 0.5).abs());
        freq = if freq < 1000.0 { freq * 2.0 } else { freq + 730.0 };
      }
      assert!(worst < 0.01, "{factor}x: {worst} db off flat");
    }
  }

  #[test]
  fn going_up_leaves_no_images() {
    // the oversampled sound holds the sine and nothing else (its image
    // near the base rate's nyquist is filtered off)
    for factor in [2, 4] {
      let mut worst = -200.0f64;
      for freq in [100.0, 1000.0, 5000.0, 12_000.0, 17_000.0, 19_200.0] {
        let mut os = Oversampler::new(factor, 4800);
        let x = sine(freq / RATE, SETTLE + WINDOW);
        let mut y = vec![0.0; x.len()];
        let mut up = Vec::new();
        os.process(&x, &mut y, |b| up.extend_from_slice(b));
        let (_, _, rest) = fit(&up[SETTLE * factor..], freq / RATE / factor as f64);
        worst = worst.max(db(rest));
      }
      assert!(worst < -80.0, "{factor}x: images {worst} db");
    }
  }

  #[test]
  fn going_down_rejects_what_would_fold_into_the_passband() {
    // a sine made at the oversampled rate, above 0.6 of the base rate:
    // whatever comes out (it would fold under 0.4 of the base rate) has to
    // be far down
    for factor in [2, 4] {
      let mut worst = -200.0f64;
      let top = RATE * factor as f64 / 2.0;
      let mut freq = 0.6 * RATE + 10.0;
      while freq < top {
        let folds_to = {
          let a = freq % RATE;
          if a > RATE / 2.0 { RATE - a } else { a }
        };
        if folds_to < 0.4 * RATE {
          let mut os = Oversampler::new(factor, 960);
          let step = freq / (RATE * factor as f64);
          let mut i = 0usize;
          let x = vec![0.0; SETTLE + WINDOW];
          let mut y = vec![0.0; x.len()];
          os.process(&x, &mut y, |b| {
            for v in b.iter_mut() {
              *v = (2.0 * PI * step * i as f64).sin() as f32 * 0.5;
              i += 1;
            }
          });
          let rms = (y[SETTLE..].iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / WINDOW as f64).sqrt();
          worst = worst.max(db(rms / (0.5 / 2f64.sqrt())));
        }
        freq += 977.0;
      }
      assert!(worst < -80.0, "{factor}x: folds back at {worst} db");
    }
  }

  #[test]
  fn a_2x_round_trip_is_its_allpasses_and_nothing_else() {
    // up, nothing, down is a0(z) a1(z) at the base rate: every allpass in
    // a row. so it's linear, keeps every level, and only shifts phase
    let mut os = Oversampler::new(2, 100);
    let mut chain_e = Allpasses::new(OUTER_EVEN);
    let mut chain_o = Allpasses::new(OUTER_ODD);
    let mut seed = 1u32;
    let x: Vec<f32> = (0..20_000)
      .map(|_| {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
      })
      .collect();
    let mut y = vec![0.0; x.len()];
    os.process(&x, &mut y, |_| {});
    let worst = x
      .iter()
      .zip(&y)
      .map(|(&a, &b)| (chain_o.run(chain_e.run(a)) - b).abs())
      .fold(0.0f32, f32::max);
    assert!(worst < 1e-6, "off by {worst}");
  }

  #[test]
  fn low_frequencies_come_back_a_few_samples_late() {
    // the phase shift's delay at 100 hz, in base-rate samples: what the
    // design says (no integer latency, but not none)
    for (factor, want) in [(2, 2.30), (4, 3.13)] {
      let freq = 100.0 / RATE;
      let mut os = Oversampler::new(factor, 256);
      let x = sine(freq, SETTLE + WINDOW);
      let mut y = vec![0.0; x.len()];
      os.process(&x, &mut y, |_| {});
      let (_, before, _) = fit(&x[SETTLE..], freq);
      let (_, after, _) = fit(&y[SETTLE..], freq);
      let late = (after - before) / (2.0 * PI * freq);
      assert!((late - want).abs() < 0.02, "{factor}x: {late} samples late");
    }
  }

  // ---- aliasing: a hard clip, with and without oversampling

  // the power that isn't the clipped sine's own harmonics, under 0.4 of
  // the rate, against the fundamental's, in db. `bin` is the sine's fft
  // bin, so it and its harmonics land on whole bins
  fn aliasing(factor: usize, bin: usize) -> f64 {
    const N: usize = 32_768;
    let freq = bin as f64 / N as f64;
    let x: Vec<f32> = (0..N + 8192).map(|i| (2.0 * PI * freq * i as f64).sin() as f32).collect();
    let mut y = vec![0.0; x.len()];
    let mut os = Oversampler::new(factor, 1024);
    os.process(&x, &mut y, |b| {
      for v in b.iter_mut() {
        *v = v.clamp(-0.25, 0.25);
      }
    });
    let mut re: Vec<f64> = y[8192..]
      .iter()
      .enumerate()
      .map(|(i, &v)| v as f64 * (0.5 - 0.5 * (2.0 * PI * i as f64 / N as f64).cos()))
      .collect();
    let mut im = vec![0.0; N];
    dsp::fft::fft(&mut re, &mut im);
    let power = |k: usize| re[k] * re[k] + im[k] * im[k];
    let near_harmonic = |k: usize| {
      let h = (k + bin / 2) / bin;
      h >= 1 && k.abs_diff(h * bin) <= 3
    };
    let fundamental: f64 = (bin - 3..=bin + 3).map(power).sum();
    let other: f64 = (4..(N as f64 * 0.4) as usize).filter(|&k| !near_harmonic(k)).map(power).sum();
    10.0 * (other / fundamental).log10()
  }

  #[test]
  fn oversampling_cuts_a_hard_clips_aliasing() {
    // about 2, 5 and 9 khz at 48 khz
    for bin in [1367, 3413, 6143] {
      let (one, two, four) = (aliasing(1, bin), aliasing(2, bin), aliasing(4, bin));
      eprintln!("bin {bin}: aliasing {one:.1} db, 2x {two:.1} db, 4x {four:.1} db");
      assert!(two < one - 6.0, "bin {bin}: 2x {two} db, 1x {one} db");
      assert!(four < two - 6.0, "bin {bin}: 4x {four} db, 2x {two} db");
    }
  }

  #[test]
  fn any_block_length_gives_the_same_sound() {
    let x = sine(3000.0 / RATE, 5000);
    let clip = |b: &mut [f32]| b.iter_mut().for_each(|v| *v = v.clamp(-0.2, 0.2));
    for factor in [1, 2, 4] {
      let mut whole = vec![0.0; x.len()];
      Oversampler::new(factor, 64).process(&x, &mut whole, clip);
      let mut os = Oversampler::new(factor, 64);
      let mut pieces = x.clone();
      let mut at = 0;
      for len in [1, 7, 64, 200, 3, 1000].iter().cycle() {
        let end = (at + len).min(x.len());
        os.process_in_place(&mut pieces[at..end], clip);
        at = end;
        if at == x.len() {
          break;
        }
      }
      assert_eq!(whole, pieces, "{factor}x");
    }
  }

  #[test]
  fn set_factor_switches_and_reset_starts_again() {
    let x = sine(1000.0 / RATE, 2000);
    let mut first = vec![0.0; x.len()];
    let mut again = vec![0.0; x.len()];
    let mut os = Oversampler::new(4, 256);
    os.set_factor(2);
    assert_eq!(os.factor(), 2);
    os.process(&x, &mut first, |_| {});
    os.reset();
    os.process(&x, &mut again, |_| {});
    assert_eq!(first, again);
    let mut fresh = vec![0.0; x.len()];
    Oversampler::new(2, 256).process(&x, &mut fresh, |_| {});
    assert_eq!(first, fresh);
  }

  #[test]
  #[should_panic]
  fn cant_go_past_what_it_was_made_for() {
    Oversampler::new(2, 64).set_factor(4);
  }
}
