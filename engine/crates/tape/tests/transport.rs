// the transport against what measure.py would read from it: fitted to a
// profile (the fake deck's, as G5 measured it), its speed must measure as
// that profile. the bands here are measure.py's: the speed on a 2 khz
// grid, 29 s at a time, its mean removed, 2nd-order butterworth bands run
// forwards and back, a second off each end. (checked once against
// measure.py itself, on 200 windows: drift +0.4 %, wow -0.2 %, flutter
// +4.4 %, peaks to the bin)

use std::f64::consts::PI;

use tape::transport::{TapeTransport, TransportSettings};

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

// the fake deck's profile (out/g5/profile.json)
const SPEED_ERROR: f32 = 0.003_035;
const DRIFT: f32 = 4.8378e-4;
const WOW: f32 = 6.7356e-4;
const FLUTTER: f32 = 1.7063e-4;
const PEAKS: [(f32, f32); 2] = [(7.324_219, 2.990_747e-4), (20.996_094, 1.195_792e-4)];

// the speed's physics doesn't care about the audio rate (the slow parts
// run at 1 khz either way), and 8 khz keeps a debug build quick
const RATE: f64 = 8_000.0;
const GRID: f64 = 2_000.0;
const WINDOW: usize = 58_000;

fn profile() -> TransportSettings {
  TransportSettings::from_measured(SPEED_ERROR, DRIFT, WOW, FLUTTER, &PEAKS)
}

// `windows` back-to-back 29 s windows of speed on the 2 khz grid
fn windows(settings: TransportSettings, seed: u64, windows: usize, amount: f32) -> Vec<Vec<f64>> {
  let mut t = TapeTransport::new(settings, RATE as f32, seed);
  t.set_amount(amount);
  t.reset();
  let step = (RATE / GRID) as usize;
  let mut block = vec![0f32; WINDOW * step];
  (0..windows)
    .map(|_| {
      t.fill_speed(&mut block);
      block.iter().step_by(step).map(|&v| v as f64).collect()
    })
    .collect()
}

// ---- measure.py's bands

#[derive(Clone, Copy)]
struct Cx(f64, f64);

impl Cx {
  fn add(self, o: Cx) -> Cx {
    Cx(self.0 + o.0, self.1 + o.1)
  }
  fn sub(self, o: Cx) -> Cx {
    Cx(self.0 - o.0, self.1 - o.1)
  }
  fn mul(self, o: Cx) -> Cx {
    Cx(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
  }
  fn div(self, o: Cx) -> Cx {
    let d = o.0 * o.0 + o.1 * o.1;
    Cx((self.0 * o.0 + self.1 * o.1) / d, (self.1 * o.0 - self.0 * o.1) / d)
  }
  fn scale(self, k: f64) -> Cx {
    Cx(self.0 * k, self.1 * k)
  }
  fn sqrt(self) -> Cx {
    let r = (self.0 * self.0 + self.1 * self.1).sqrt();
    let re = ((r + self.0) / 2.0).sqrt();
    let im = ((r - self.0) / 2.0).sqrt().copysign(self.1);
    Cx(re, im)
  }
  fn abs(self) -> f64 {
    (self.0 * self.0 + self.1 * self.1).sqrt()
  }
}

// scipy's butter(2, ...) as second-order sections: [b0 b1 b2 a1 a2]
fn butter(lo: Option<f64>, hi: f64) -> Vec<[f64; 5]> {
  let fs = GRID;
  let warp = |f: f64| 2.0 * fs * (PI * f / fs).tan();
  let bilinear = |s: Cx| Cx(2.0 * fs, 0.0).add(s).div(Cx(2.0 * fs, 0.0).sub(s));
  let proto = [Cx((3.0 * PI / 4.0).cos(), (3.0 * PI / 4.0).sin())];
  let mut sections = Vec::new();
  let centre = match lo {
    None => {
      let wc = warp(hi);
      let p = bilinear(proto[0].scale(wc));
      // zeros at -1 (both at infinity in s)
      sections.push([1.0, 2.0, 1.0, -2.0 * p.0, p.0 * p.0 + p.1 * p.1]);
      0.0
    }
    Some(lo) => {
      let (wl, wh) = (warp(lo), warp(hi));
      let (bw, w0sq) = (wh - wl, wl * wh);
      // each prototype pole becomes two: s² - p bw s + w0² = 0
      let pb = proto[0].scale(bw);
      let root = pb.mul(pb).sub(Cx(4.0 * w0sq, 0.0)).sqrt();
      for s in [pb.add(root).scale(0.5), pb.sub(root).scale(0.5)] {
        let p = bilinear(s);
        // a zero at +1 (s = 0) and one at -1 (s = infinity)
        sections.push([1.0, 0.0, -1.0, -2.0 * p.0, p.0 * p.0 + p.1 * p.1]);
      }
      (wl * wh).sqrt()
    }
  };
  // unity where a butterworth has it: dc, or the band's centre
  let f = if centre == 0.0 { 0.0 } else { fs / PI * (centre / (2.0 * fs)).atan() };
  let z1 = Cx((-2.0 * PI * f / fs).cos(), (-2.0 * PI * f / fs).sin());
  let mut gain = 1.0;
  for s in &sections {
    let num = Cx(s[0], 0.0).add(z1.scale(s[1])).add(z1.mul(z1).scale(s[2]));
    let den = Cx(1.0, 0.0).add(z1.scale(s[3])).add(z1.mul(z1).scale(s[4]));
    gain *= num.div(den).abs();
  }
  for k in 0..3 {
    sections[0][k] /= gain;
  }
  sections
}

fn sosfilt(sos: &[[f64; 5]], x: &mut [f64]) {
  // from rest at x[0] held forever (scipy starts each pass the same way)
  let mut state = vec![[0.0f64; 2]; sos.len()];
  let first = x[0];
  let run = |v: f64, state: &mut Vec<[f64; 2]>| {
    let mut v = v;
    for (s, st) in sos.iter().zip(state.iter_mut()) {
      let y = s[0] * v + st[0];
      st[0] = s[1] * v - s[3] * y + st[1];
      st[1] = s[2] * v - s[4] * y;
      v = y;
    }
    v
  };
  for _ in 0..(20.0 * GRID) as usize {
    run(first, &mut state);
  }
  for v in x.iter_mut() {
    *v = run(*v, &mut state);
  }
}

fn filtfilt(sos: &[[f64; 5]], x: &[f64]) -> Vec<f64> {
  let mut y = x.to_vec();
  sosfilt(sos, &mut y);
  y.reverse();
  sosfilt(sos, &mut y);
  y.reverse();
  y
}

// one window's (drift, wow, flutter) powers
fn bands(speed: &[f64], filters: &[Vec<[f64; 5]>; 3]) -> [f64; 3] {
  let mean = speed.iter().sum::<f64>() / speed.len() as f64;
  let dev: Vec<f64> = speed.iter().map(|v| v / mean - 1.0).collect();
  let edge = GRID as usize;
  let mut out = [0.0; 3];
  for (o, sos) in out.iter_mut().zip(filters) {
    let y = filtfilt(sos, &dev);
    let mid = &y[edge..y.len() - edge];
    *o = mid.iter().map(|v| v * v).sum::<f64>() / mid.len() as f64;
  }
  out
}

fn filters() -> [Vec<[f64; 5]>; 3] {
  [butter(None, 0.5), butter(Some(0.5), 6.0), butter(Some(6.0), 200.0)]
}

// a hann-windowed periodogram of one window's deviation: (hz a bin, the
// power density)
fn spectrum(speed: &[f64]) -> (f64, Vec<f64>) {
  let n = 1 << 16;
  let mean = speed.iter().sum::<f64>() / speed.len() as f64;
  let (mut re, mut im) = (vec![0.0; n], vec![0.0; n]);
  let m = speed.len();
  let mut w2 = 0.0;
  for (i, v) in speed.iter().enumerate() {
    let w = 0.5 - 0.5 * (2.0 * PI * i as f64 / (m - 1) as f64).cos();
    re[i] = (v / mean - 1.0) * w;
    w2 += w * w;
  }
  dsp::fft::fft(&mut re, &mut im);
  let psd = (0..n / 2).map(|k| 2.0 * (re[k] * re[k] + im[k] * im[k]) / (GRID * w2)).collect();
  (GRID / n as f64, psd)
}

#[test]
fn fitted_to_a_profile_it_measures_as_that_profile() {
  let f = filters();
  let mut sum = [0.0; 3];
  let mut count = 0;
  // a single 29 s window reads drift only to about ±24 % (its slowest
  // parts barely fit in it; measured on 200 windows), wow ±6 %, flutter
  // ±1 %. 48 windows put drift's own scatter near ±3.5 %
  for seed in 0..4 {
    for w in windows(profile(), seed, 12, 1.0) {
      let b = bands(&w, &f);
      for (s, v) in sum.iter_mut().zip(b) {
        *s += v;
      }
      count += 1;
    }
  }
  let got: Vec<f64> = sum.iter().map(|s| (s / count as f64).sqrt()).collect();
  let want = [DRIFT as f64, WOW as f64, FLUTTER as f64];
  for (name, (g, w)) in ["drift", "wow", "flutter"].iter().zip(got.iter().zip(want)) {
    println!("{name}: {g:.4e} against {w:.4e} ({:+.1} %)", (g / w - 1.0) * 100.0);
    assert!((g / w - 1.0).abs() < 0.10, "{name}: {g:.4e} against {w:.4e}");
  }
}

#[test]
fn its_flutter_peaks_are_where_and_as_deep_as_measured() {
  for seed in [11, 12] {
    let w = &windows(profile(), seed, 1, 1.0)[0];
    let (df, psd) = spectrum(w);
    for (hz, depth) in PEAKS {
      let (hz, depth) = (hz as f64, depth as f64);
      let lo = ((hz - 1.0) / df) as usize;
      let hi = ((hz + 1.0) / df) as usize;
      let at = (lo..hi).max_by(|&a, &b| psd[a].total_cmp(&psd[b])).unwrap();
      let found = at as f64 * df;
      let got = (2.0 * psd[at - 3..=at + 3].iter().sum::<f64>() * df).sqrt();
      println!("seed {seed}: {found:.3} hz, depth {got:.3e} (measured {hz:.3}, {depth:.3e})");
      assert!((found - hz).abs() < 0.2, "{found} against {hz}");
      assert!((got / depth - 1.0).abs() < 0.15, "{got} against {depth}");
    }
  }
}

#[test]
fn twice_the_amount_is_twice_the_wobble() {
  let one = &windows(profile(), 5, 1, 1.0)[0];
  let two = &windows(profile(), 5, 1, 2.0)[0];
  let dev = |w: &[f64]| w.iter().map(|v| v - (1.0 + SPEED_ERROR as f64)).collect::<Vec<_>>();
  let (a, b) = (dev(one), dev(two));
  let ratio = (b.iter().map(|v| v * v).sum::<f64>() / a.iter().map(|v| v * v).sum::<f64>()).sqrt();
  assert!((ratio - 2.0).abs() < 0.02, "{ratio}");
}

#[test]
fn the_speed_stays_positive_and_near_one() {
  for settings in [profile(), TransportSettings::default()] {
    let w = windows(settings, 21, 1, 2.0);
    let (lo, hi) = w[0].iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert!(lo > 0.99 && hi < 1.01, "{lo} {hi}");
  }
  // sticky tape at twice the wobble too
  let mut sticky = profile();
  sticky.stick_slips_per_minute = 30.0;
  let w = windows(sticky, 21, 1, 2.0);
  assert!(w[0].iter().all(|&v| v > 0.5), "{:?}", w[0].iter().cloned().fold(f64::MAX, f64::min));
}

#[test]
fn filling_the_speed_allocates_nothing() {
  let mut t = TapeTransport::new(profile(), 48_000.0, 1);
  let mut out = vec![0f32; 4096];
  let other = TransportSettings { stretch_hz: 50.0, stick_slips_per_minute: 600.0, ..TransportSettings::default() };
  let before = counting::made();
  for n in [4096, 1, 7, 64, 512, 4096] {
    t.fill_speed(&mut out[..n]);
  }
  t.set_amount(1.5);
  t.set_settings(other);
  t.fill_speed(&mut out);
  t.stop();
  t.fill_speed(&mut out);
  t.start();
  t.fill_speed(&mut out);
  t.reset();
  assert_eq!(counting::made() - before, 0);
}
