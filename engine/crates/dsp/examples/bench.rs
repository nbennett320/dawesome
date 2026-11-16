// what the built-in effects cost: each registered effect run in blocks as
// the engine runs them, timed, in a few situations, and one analyzer frame.
// run it built for release:
//
//   cargo run --release -p dawesome-dsp --example bench
//
// (pnpm dsp:bench). the numbers are this machine's: a Raspberry Pi will be
// several times slower

use std::hint::black_box;
use std::time::{Duration, Instant};

use dsp::effects::eq;
use dsp::{Runner, Store, Transport, REGISTRY};

const RATE: u32 = 48_000;
const BLOCK: usize = 512;
// how long each situation is timed for
const RUN: Duration = Duration::from_secs(2);

// a small repeatable noise
fn noise(frames: usize) -> Vec<f32> {
  let mut x = 0x2545_f491u32;
  (0..frames).map(|_| {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    (x >> 8) as f32 / (1 << 24) as f32 - 0.5
  }).collect()
}

// time `runner` on blocks of noise, calling `each` before every block (to
// move its knobs): the average time a block takes
fn time(runner: &mut Runner, mut each: impl FnMut(usize)) -> Duration {
  let input = noise(BLOCK);
  let (mut l, mut r) = (vec![0f32; BLOCK], vec![0f32; BLOCK]);
  let transport = Transport { sample_rate: RATE, ..Transport::default() };
  // warm up
  for n in 0..200 {
    each(n);
    runner.process([&input, &input], [&mut l, &mut r], &transport);
  }
  let started = Instant::now();
  let mut blocks = 0;
  while started.elapsed() < RUN {
    for _ in 0..100 {
      each(blocks);
      runner.process([black_box(&input), black_box(&input)], [&mut l, &mut r], &transport);
      blocks += 1;
    }
  }
  black_box(&l);
  started.elapsed() / blocks as u32
}

fn report(what: &str, per_block: Duration) {
  let real = Duration::from_secs_f64(BLOCK as f64 / RATE as f64);
  println!(
    "  {:<44} {:>8.2} µs a block  {:>6.3}% of one core  ({:.0} at once)",
    what,
    per_block.as_secs_f64() * 1e6,
    per_block.as_secs_f64() / real.as_secs_f64() * 100.,
    real.as_secs_f64() / per_block.as_secs_f64(),
  );
}

fn main() {
  if cfg!(debug_assertions) {
    println!("(a debug build: use --release for numbers that mean anything)\n");
  }
  println!("{}-frame blocks at {} Hz (a block lasts {:.2} ms)\n", BLOCK, RATE, BLOCK as f64 / RATE as f64 * 1e3);

  for entry in REGISTRY {
    println!("{}", entry.info.name);
    let store = Store::new(entry);
    report("at its defaults", time(&mut Runner::new(store.clone(), RATE as f32), |_| {}));
  }

  // the EQ in the situations that cost it most
  println!("\nEQ, all 8 bands on");
  let on = |store: &Store| {
    for band in 0..eq::BANDS {
      store.set_plain(eq::id(band, eq::ON), 1.);
      store.set_plain(eq::id(band, eq::GAIN), 6.);
      store.set_plain(eq::id(band, eq::Q), 2.);
    }
  };
  let store = Store::new(&eq::ENTRY);
  on(&store);
  report("still", time(&mut Runner::new(store.clone(), RATE as f32), |_| {}));

  // every band's frequency moving all the time: the glide never lands, so
  // the EQ is run 4 frames at a time with its filters remade each time
  let store = Store::new(&eq::ENTRY);
  on(&store);
  let mut runner = Runner::new(store.clone(), RATE as f32);
  let moving = time(&mut runner, |n| {
    for band in 0..eq::BANDS {
      store.set(eq::id(band, eq::FREQ), ((n + band * 37) % 200) as f32 / 200.);
    }
  });
  report("every frequency gliding, all the time", moving);

  // one band's knob turned (the usual case): one of the 8 gliding
  let store = Store::new(&eq::ENTRY);
  on(&store);
  let mut runner = Runner::new(store.clone(), RATE as f32);
  let one = time(&mut runner, |n| { store.set(eq::id(0, eq::GAIN), (n % 100) as f32 / 100.); });
  report("one band's gain gliding, all the time", one);

  // an analyzer frame, as the app asks for one ~30 times a second
  println!("\nAnalyzer");
  let store = Store::new(&eq::ENTRY);
  let mut runner = Runner::new(store.clone(), RATE as f32);
  time(&mut runner, |_| {});
  let started = Instant::now();
  let mut frames = 0;
  while started.elapsed() < RUN {
    let (samples, _) = store.tap().latest(dsp::tap::WINDOW).unwrap();
    black_box(dsp::spectrum::spectrum(&samples, RATE as f32, 256, 20., 20_000.));
    frames += 1;
  }
  let each = started.elapsed() / frames;
  println!(
    "  {:<44} {:>8.2} µs a frame   {:>6.3}% of one core at 30 frames a second",
    "a 2048-point spectrum, 256 levels",
    each.as_secs_f64() * 1e6,
    each.as_secs_f64() * 30. * 100.,
  );
}
