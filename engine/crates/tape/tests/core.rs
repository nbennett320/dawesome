// the hand-built core against the fake deck it's a port of (faketape.core
// in tools/tape-model, run on one channel in float64), and what neither
// core nor the oversampler may do on the audio thread: allocate
//
// the fixture (assets/core_simple.json) holds the python's output, every
// `every`th sample, for a test signal both sides build from the same
// recipe (below), so the file stays small

use std::f64::consts::PI;
use std::fs;
use std::sync::Arc;

use serde_json::Value;
use tape::core::{NeuralCore, SimpleCore, SimpleCoreSettings, TapeCore};
use tape::oversample::Oversampler;
use tape::GruWeights;

// ---- counting allocations (only this test binary's allocator; counted
// per thread, as tests run side by side)

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

// ---- the test signal

const RATE: usize = 48_000;
const REF: f64 = 0.125_892_541_179_416_73;

// uniform in [-1, 1), from the top 24 bits of a plain lcg: the same
// numbers in python
struct Lcg(u32);

impl Lcg {
  fn next(&mut self) -> f64 {
    self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (self.0 >> 8) as f64 / 16_777_216.0 * 2.0 - 1.0
  }
}

// a sweep, noise bursts, and something like music: the fixture script's
// recipe, step for step
fn recipe() -> Vec<f32> {
  let mut out: Vec<f64> = Vec::new();
  let mut noise = Lcg(12345);
  let rate = RATE as f64;
  let n = RATE;
  let k = (20_000.0f64 / 20.0).ln();
  for i in 0..n {
    let t = i as f64 / rate;
    let ph = 2.0 * PI * 20.0 * ((t * k).exp() - 1.0) / k;
    let fade = (1.0f64).min(i as f64 / 240.0).min((n - 1 - i) as f64 / 240.0);
    out.push(2.0 * REF * ph.sin() * fade);
  }
  out.extend(std::iter::repeat_n(0.0, RATE / 20));
  for level in [0.5, 1.0, 2.0, 4.0] {
    for _ in 0..RATE / 10 {
      out.push(level * REF * noise.next());
    }
    out.extend(std::iter::repeat_n(0.0, RATE / 20));
  }
  for i in 0..RATE {
    let t = i as f64 / rate;
    let mut v = 1.5 * REF * (2.0 * PI * 55.0 * t).sin() * (1.0f64).min(t / 0.02);
    for f in [220.0, 277.18, 329.63] {
      v += 0.5 * REF * (2.0 * PI * f * t).sin();
    }
    let beat = (t / 0.25) as usize;
    let tp = t - beat as f64 * 0.25;
    let f = [880.0, 1320.0, 660.0, 990.0][beat];
    let p = 2.0 * PI * f * tp;
    v += 2.0 * REF * (-tp / 0.08).exp() * (p.sin() + 0.5 * (2.0 * p).sin() + 0.25 * (3.0 * p).sin());
    let th = t - (t / 0.125) as usize as f64 * 0.125;
    v += REF * (-th / 0.01).exp() * noise.next();
    out.push(v);
  }
  out.extend(std::iter::repeat_n(0.0, RATE / 10));
  out.into_iter().map(|v| v as f32).collect()
}

fn floats(v: &Value) -> Vec<f64> {
  v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

fn load(name: &str) -> Value {
  let path = format!("{}/tests/assets/{name}", env!("CARGO_MANIFEST_DIR"));
  serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

// the python deck's record side, as SimpleCore's settings
fn settings(deck: &Value, oversample: usize) -> SimpleCoreSettings {
  let f = |name: &str| deck[name].as_f64().unwrap() as f32;
  SimpleCoreSettings {
    emphasis_hz: f("emphasis_hz"),
    emphasis_db: f("emphasis_db"),
    headroom: f("headroom"),
    asymmetry: f("asymmetry"),
    compress: f("compress"),
    compress_seconds: f("compress_seconds"),
    head_bump_hz: f("head_bump_hz"),
    head_bump_db: f("head_bump_db"),
    hf_loss_hz: f("hf_loss_hz"),
    hf_loss_stages: deck["hf_loss_stages"].as_u64().unwrap() as usize,
    low_cut_hz: f("low_cut_hz"),
    oversample,
  }
}

#[test]
fn the_simple_core_matches_the_fake_deck() {
  let fixture = load("core_simple.json");
  assert_eq!(fixture["rate"].as_u64().unwrap() as usize, RATE);
  let input = recipe();

  // the same signal on both sides, or the rest means nothing
  assert_eq!(input.len(), fixture["frames"].as_u64().unwrap() as usize, "recipe mismatch: length");
  let check = &fixture["check"];
  let sum: f64 = input.iter().map(|v| v.abs() as f64).sum();
  let want = check["sum_abs"].as_f64().unwrap();
  assert!((sum - want).abs() < 1e-9 * want, "recipe mismatch: sum {sum}, python {want}");
  for (at, v) in floats(&check["at"]).iter().zip(floats(&check["values"])) {
    assert_eq!(input[*at as usize] as f64, v, "recipe mismatch at {at}");
  }

  // the defaults are the fresh deck's
  let deck = &fixture["deck"];
  assert_eq!(settings(deck, 2), SimpleCoreSettings::default());

  let every = fixture["every"].as_u64().unwrap() as usize;
  for case in fixture["cases"].as_array().unwrap() {
    let drive = case["drive_db"].as_f64().unwrap() as f32;
    let want = floats(&case["output"]);
    let mut core = SimpleCore::new(settings(deck, 1), RATE as f32, 1024);
    let mut got = vec![0.0; input.len()];
    core.process(&input, drive, &mut got);
    let got: Vec<f32> = got.into_iter().step_by(every).collect();
    assert_eq!(got.len(), want.len());
    let peak = want.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let worst = got.iter().zip(&want).map(|(a, b)| (*a as f64 - b).abs()).fold(0.0, f64::max);
    eprintln!("drive {drive:+}: worst {worst:.2e}, {:.1} db under the peak ({peak:.3})", 20.0 * (worst / peak).log10());
    assert!(worst < 1e-5, "drive {drive}: off by {worst} (peak {peak})");
  }
}

#[test]
fn oversampled_it_stays_close_to_the_fake_deck() {
  // not sample for sample (the oversampler shifts phase): the level of
  // what comes out over the sweep to 10 khz and the music, at 2x and 4x,
  // against the python's
  let fixture = load("core_simple.json");
  let input = recipe();
  let every = fixture["every"].as_u64().unwrap() as usize;
  let rms = |v: &[f64]| (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt();
  for case in fixture["cases"].as_array().unwrap() {
    let drive = case["drive_db"].as_f64().unwrap() as f32;
    let want = floats(&case["output"]);
    for oversample in [2, 4] {
      let mut core = SimpleCore::new(settings(&fixture["deck"], oversample), RATE as f32, 1024);
      let mut got = vec![0.0; input.len()];
      core.process(&input, drive, &mut got);
      let got: Vec<f64> = got.into_iter().step_by(every).map(|v| v as f64).collect();
      for (name, from, to) in [("sweep", 0, 43_200), ("music", 79_200, 127_200)] {
        let (a, b) = (rms(&got[from / every..to / every]), rms(&want[from / every..to / every]));
        let off = 20.0 * (a / b).log10();
        assert!(off.abs() < 0.05, "drive {drive}, {oversample}x, {name}: {off} db");
      }
    }
  }
}

#[test]
fn nothing_allocates_once_made() {
  let input: Vec<f32> = (0..5000).map(|i| (i as f32 * 0.05).sin() * 0.4).collect();
  let mut out = vec![0.0f32; input.len()];

  let mut os = Oversampler::new(4, 256);
  let mut simple = SimpleCore::new(SimpleCoreSettings::default(), 48_000.0, 256);
  let h = 8;
  let weights = Arc::new(GruWeights {
    hidden: h,
    w_ih: vec![0.1; 6 * h],
    w_hh: vec![0.05; 3 * h * h],
    b_ih: vec![0.0; 3 * h],
    b_hh: vec![0.0; 3 * h],
    dense_w: vec![0.2; h],
    dense_b: 0.0,
    drive_scale: 1.0 / 18.0,
    rate: 48_000,
  });
  let mut neural = NeuralCore::new(weights, 48_000.0).unwrap();
  let other = SimpleCoreSettings { oversample: 1, hf_loss_stages: 4, ..Default::default() };

  let before = counting::made();
  for factor in [1, 2, 4] {
    os.set_factor(factor);
    os.process(&input, &mut out, |b| b.iter_mut().for_each(|v| *v = v.tanh()));
    os.process_in_place(&mut out, |b| b.iter_mut().for_each(|v| *v *= 0.5));
  }
  os.reset();
  for (drive, len) in [(0.0, 5000), (6.0, 17), (12.0, 256), (-3.0, 1)] {
    simple.process(&input[..len], drive, &mut out[..len]);
    neural.process(&input[..len], drive, &mut out[..len]);
  }
  simple.set_settings(other);
  simple.process(&input, 3.0, &mut out);
  simple.reset();
  neural.reset();
  assert_eq!(counting::made() - before, 0, "allocated on the audio thread");
}
