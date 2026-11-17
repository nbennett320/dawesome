// what the hand-built core costs at each oversampling factor: ns a sample,
// and the share of one cpu core a stereo track takes at 48 khz.
// cargo run --release -p dawesome-tape --example core_bench

use std::time::Instant;

use tape::core::{SimpleCore, SimpleCoreSettings, TapeCore};

fn main() {
  let frames = 48_000 * 10;
  let input: Vec<f32> = (0..frames).map(|i| (i as f32 * 0.01).sin() * 0.3).collect();
  let mut out = vec![0.0f32; frames];
  for oversample in [1, 2, 4] {
    let settings = SimpleCoreSettings { oversample, ..Default::default() };
    let mut core = SimpleCore::new(settings, 48_000.0, 512);
    // the engine's blocks: 512 frames at a time
    core.process(&input[..48_000], 0.0, &mut out[..48_000]);
    let start = Instant::now();
    for (i, o) in input.chunks(512).zip(out.chunks_mut(512)) {
      core.process(i, 6.0, o);
    }
    let secs = start.elapsed().as_secs_f64();
    let ns = secs * 1e9 / frames as f64;
    let core_share = 2.0 * 48_000.0 * ns * 1e-9 * 100.0;
    println!("simple core {oversample}x: {ns:5.1} ns a sample, a stereo track {core_share:5.2} % of a core, 100 tracks {:5.2} cores  ({:.3})", core_share, out[frames - 1]);
  }
}
