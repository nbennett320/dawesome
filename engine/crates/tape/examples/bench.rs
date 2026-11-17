// what the network costs: ns a sample, and the share of one cpu core a
// stereo track takes at 48 khz. cargo run --release -p dawesome-tape --example bench

use std::sync::Arc;
use std::time::Instant;

use tape::{Gru, GruWeights};

fn weights(hidden: usize) -> GruWeights {
  let mut seed = 1u32;
  let mut next = move || {
    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (seed >> 8) as f32 / (1u32 << 24) as f32 * 0.2 - 0.1
  };
  GruWeights {
    hidden,
    w_ih: (0..6 * hidden).map(|_| next()).collect(),
    w_hh: (0..3 * hidden * hidden).map(|_| next()).collect(),
    b_ih: (0..3 * hidden).map(|_| next()).collect(),
    b_hh: (0..3 * hidden).map(|_| next()).collect(),
    dense_w: (0..hidden).map(|_| next()).collect(),
    dense_b: 0.0,
    drive_scale: 1.0 / 18.0,
    rate: 48_000,
  }
}

fn main() {
  let frames = 48_000 * 10;
  let input: Vec<f32> = (0..frames).map(|i| (i as f32 * 0.01).sin() * 0.3).collect();
  let mut out = vec![0.0f32; frames];
  for hidden in [8, 16, 32, 48] {
    let mut gru = Gru::new(Arc::new(weights(hidden))).unwrap();
    gru.process(&input[..48_000], 0.0, &mut out[..48_000]);
    let start = Instant::now();
    gru.process(&input, 3.0, &mut out);
    let secs = start.elapsed().as_secs_f64();
    let ns = secs * 1e9 / frames as f64;
    // a stereo track: two channels, 48000 samples a second each
    let core = 2.0 * 48_000.0 * ns * 1e-9 * 100.0;
    println!("hidden {hidden:3}: {ns:6.1} ns a sample, a stereo track {core:5.2} % of a core, 100 tracks {:5.1} cores  ({:.3})", core, out[frames - 1]);
  }
}
