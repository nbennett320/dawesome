// the conformance suite: what every built-in effect must do, checked for
// each one in REGISTRY with no code of its own. an effect that passes is
// safe to put in a mixer channel: it makes no NaNs or denormals, falls
// silent within the tail it says, doesn't click when its knobs move, gives
// the same sound however the engine cuts it into blocks, and so on
//
// a second, test-only effect (Gain, below) is checked too, to show the
// suite is about effects in general and not the EQ

use std::sync::Arc;

use crate::effect::{Effect, EffectInfo, Entry, Io, Values};
use crate::param::{Kind, ParamDesc, Taper, Unit};
use crate::runner::{Runner, Store};
use crate::transport::Transport;
use crate::REGISTRY;

// ---- counting allocations (the audio thread mustn't make any)

mod counting {
  use std::alloc::{GlobalAlloc, Layout, System};
  use std::cell::Cell;

  thread_local! {
    static MADE: Cell<usize> = const { Cell::new(0) };
  }

  // the system's allocator, counting each thread's allocations
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

  // allocations made on this thread so far
  pub fn made() -> usize {
    MADE.with(|made| made.get())
  }
}

// ---- the test-only effect

static GAIN_PARAMS: [ParamDesc; 2] = [
  ParamDesc::continuous("gain", "Gain", "Gain", Unit::Db, (-60., 12.), 0., Taper::Linear),
  ParamDesc::toggle("invert", "Invert", "Gain", false),
];

static GAIN: Entry = Entry {
  info: EffectInfo { id: "gain", name: "Gain", category: "Utility", version: 1 },
  params: &GAIN_PARAMS,
  make: |_| Box::new(Gain),
};

struct Gain;

impl Effect for Gain {
  fn set_sample_rate(&mut self, _: f32) {}
  fn reset(&mut self) {}

  fn process(&mut self, io: Io, params: &Values, _: &Transport) {
    let sign = if params.on(1) { -1. } else { 1. };
    let gain = sign * 10f32.powf(params.get(0) / 20.);
    for side in 0..2 {
      for (y, x) in io.output[side].iter_mut().zip(io.input[side]) {
        *y = x * gain;
      }
    }
  }

  fn tail_frames(&self, _: &Values, _: f32) -> Option<u64> {
    Some(0)
  }
}

// every effect the suite checks
fn effects() -> Vec<&'static Entry> {
  REGISTRY.iter().copied().chain([&GAIN]).collect()
}

// ---- helpers

const RATE: u32 = 48_000;

fn transport(rate: u32) -> Transport {
  Transport { sample_rate: rate, ..Transport::default() }
}

// a small, repeatable random number source (xorshift)
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> f32 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    (self.0 >> 40) as f32 / (1u64 << 24) as f32
  }

  // -1 to 1
  fn noise(&mut self) -> f32 {
    self.next() * 2. - 1.
  }
}

fn noise(frames: usize, seed: u64) -> Vec<f32> {
  let mut rng = Rng(seed);
  (0..frames).map(|_| rng.noise() * 0.5).collect()
}

fn sine(frames: usize, freq: f32) -> Vec<f32> {
  (0..frames).map(|i| 0.5 * (std::f32::consts::TAU * freq * i as f32 / RATE as f32).sin()).collect()
}

// run `input` (the same on both sides) through `runner`, `block` frames at
// a time: both sides out
fn run(runner: &mut Runner, input: &[f32], block: usize, rate: u32) -> [Vec<f32>; 2] {
  let mut out = [vec![0.; input.len()], vec![0.; input.len()]];
  let [l, r] = &mut out;
  let mut at = 0;
  while at < input.len() {
    let end = (at + block).min(input.len());
    let t = Transport { position: at as i64, ..transport(rate) };
    runner.process([&input[at..end], &input[at..end]], [&mut l[at..end], &mut r[at..end]], &t);
    at = end;
  }
  out
}

// settings to try: the defaults, every parameter at its lowest, at its
// highest, and some at random
fn settings(entry: &Entry) -> Vec<Vec<f32>> {
  let params = entry.params;
  let mut all = vec![
    entry.defaults(),
    params.iter().map(|p| p.min).collect(),
    params.iter().map(|p| p.max).collect(),
  ];
  let mut rng = Rng(0x5eed);
  for _ in 0..12 {
    all.push(params.iter().map(|p| p.plain(rng.next())).collect());
  }
  all
}

fn store_with(entry: &'static Entry, plains: &[f32]) -> Arc<Store> {
  let store = Store::new(entry);
  store.set_plains(plains);
  store
}

fn peak(samples: &[f32]) -> f32 {
  samples.iter().fold(0f32, |m, x| m.max(x.abs()))
}

fn assert_clean(name: &str, out: &[Vec<f32>; 2], what: &str) {
  for x in out.iter().flatten() {
    assert!(x.is_finite(), "{}: {} gave {}", name, what, x);
    assert!(*x == 0. || x.abs() >= f32::MIN_POSITIVE, "{}: {} gave a denormal ({:e})", name, what, x);
  }
}

// ---- the suite

#[test]
fn test_parameters_are_well_formed() {
  for entry in effects() {
    let name = entry.info.name;
    let mut keys = std::collections::HashSet::new();
    for p in entry.params {
      assert!(keys.insert(p.key), "{}: two parameters are saved as {}", name, p.key);
      assert!(!p.key.is_empty() && !p.name.is_empty(), "{}: a parameter has no key or name", name);
      assert!(p.min < p.max, "{}: {} has no range", name, p.key);
      assert_eq!(p.clamp(p.default), p.default, "{}: {}'s default is one it can't have", name, p.key);
      if p.taper == Taper::Log { assert!(p.min > 0., "{}: {} is log from {}", name, p.key, p.min); }
      if let Kind::Choice(labels) = p.kind { assert!(labels.len() >= 2, "{}: {} has nothing to choose", name, p.key); }

      for i in 0..=20 {
        let plain = p.plain(i as f32 / 20.);
        // normalized and back
        let again = p.plain(p.normalize(plain));
        assert!((again - plain).abs() <= plain.abs() * 1e-4 + 1e-4, "{}: {} {} came back {}", name, p.key, plain, again);
        // shown, read back, and shown the same
        let shown = p.format(plain);
        let read = p.parse(&shown).unwrap_or_else(|| panic!("{}: {} can't read its own \"{}\"", name, p.key, shown));
        assert_eq!(p.format(read), shown, "{}: {} read \"{}\" as {}", name, p.key, shown, read);
      }
    }
  }
}

#[test]
fn test_no_settings_make_nans_or_denormals() {
  for entry in effects() {
    for (i, plains) in settings(entry).iter().enumerate() {
      let mut runner = Runner::new(store_with(entry, plains), RATE as f32);
      // noise, a full-scale click, then a long silence for anything to decay
      // into denormals
      let mut input = noise(RATE as usize / 4, i as u64 + 1);
      input.push(1.);
      input.extend(vec![0.; RATE as usize * 2]);
      let out = run(&mut runner, &input, 512, RATE);
      assert_clean(entry.info.name, &out, &format!("settings #{}", i));
    }
  }
}

#[test]
fn test_silence_comes_within_the_tail_it_says() {
  for entry in effects() {
    for (i, plains) in settings(entry).iter().enumerate() {
      let mut runner = Runner::new(store_with(entry, plains), RATE as f32);
      let input = noise(RATE as usize / 4, i as u64 + 1);
      run(&mut runner, &input, 512, RATE);
      let tail = runner.tail_frames().expect("every effect so far stops") as usize;
      // the tail (thrown away as it goes: a delay's can be minutes), then a
      // stretch after it that must be silent
      let silence = vec![0.; 1 << 16];
      let mut left = tail;
      while left > 0 {
        let chunk = left.min(silence.len());
        run(&mut runner, &silence[..chunk], 512, RATE);
        left -= chunk;
      }
      let out = run(&mut runner, &silence[..4096], 512, RATE);
      let after = peak(&out[0]).max(peak(&out[1]));
      assert!(after < 1e-4, "{}: settings #{} still sounds ({}) {} frames after its input stopped", entry.info.name, i, after, tail);
    }
  }
}

#[test]
fn test_how_the_sound_is_cut_into_blocks_doesnt_change_it() {
  // every size here starts a block at multiples of this, which is where
  // values are set (a block reads them at its start)
  const EVERY: usize = 7 * 4096;
  for entry in effects() {
    let all = settings(entry);
    let input = noise(EVERY * 3, 7);
    let mut outs = Vec::new();
    for block in [4096, 1, 7, 64, 333 * 7 * 4] {
      let store = Store::new(entry);
      let mut runner = Runner::new(store.clone(), RATE as f32);
      let mut out = [Vec::new(), Vec::new()];
      // new values (gliding from the last ones) every EVERY frames
      for (n, chunk) in input.chunks(EVERY).enumerate() {
        store.set_plains(&all[(n + 3) % all.len()]);
        let [l, r] = run(&mut runner, chunk, block, RATE);
        out[0].extend(l);
        out[1].extend(r);
      }
      outs.push((block, out));
    }
    let (_, first) = &outs[0];
    for (block, out) in &outs[1..] {
      assert!(out == first, "{}: {}-frame blocks sound different from 4096-frame ones", entry.info.name, block);
    }
  }
}

#[test]
fn test_turning_a_knob_doesnt_click() {
  // a low sine changes little from one sample to the next: a click is a
  // jump far bigger than that
  for entry in effects() {
    let name = entry.info.name;
    // (not one that bends pitch: see the next test)
    for (id, p) in entry.params.iter().enumerate().filter(|(_, p)| p.smoothed() && !p.bends_pitch) {
      for base in settings(entry).iter().take(6) {
        let store = store_with(entry, base);
        let mut runner = Runner::new(store.clone(), RATE as f32);
        let input = sine(RATE as usize, 110.);
        let (before, after) = input.split_at(RATE as usize / 2);
        // settle, then turn it from its lowest to its highest
        store.set_plain(id, p.min);
        run(&mut runner, before, 256, RATE);
        let settled = run(&mut runner, before, 256, RATE);
        store.set_plain(id, p.max);
        let turned = run(&mut runner, after, 256, RATE);
        let tail = run(&mut runner, after, 256, RATE);

        let step = |s: &[f32]| s.windows(2).fold(0f32, |m, w| m.max((w[1] - w[0]).abs()));
        let usual = step(&settled[0]).max(step(&tail[0]));
        let moving = step(&turned[0]);
        assert!(
          moving <= usual * 2. + 1e-3,
          "{}: turning {} with {:?} clicks (a jump of {} where the sound moves at most {})", name, p.key, base, moving, usual
        );
      }
    }
  }
}

#[test]
fn test_turning_a_knob_that_bends_pitch_keeps_it_clean() {
  // a delay's time glides like tape, bending its echoes' pitch, and each
  // echo bends again on its next pass: a sweep the test above would take
  // for clicks. so these are checked for staying clean and in bounds (the
  // effect's own tests check how they glide)
  for entry in effects() {
    let name = entry.info.name;
    for (id, p) in entry.params.iter().enumerate().filter(|(_, p)| p.bends_pitch) {
      for base in settings(entry).iter().take(6) {
        let store = store_with(entry, base);
        let mut runner = Runner::new(store.clone(), RATE as f32);
        let input = sine(RATE as usize, 110.);
        let (before, after) = input.split_at(RATE as usize / 2);
        store.set_plain(id, p.min);
        run(&mut runner, before, 256, RATE);
        store.set_plain(id, p.max);
        let turned = run(&mut runner, after, 256, RATE);
        let tail = run(&mut runner, after, 256, RATE);
        for out in [&turned, &tail] {
          assert_clean(name, out, &format!("turning {} with {:?}", p.key, base));
          // (a half-scale sine fed back at 95% builds to 10 at most, in step
          // with itself: past +30 dB, something's running away)
          let most = peak(&out[0]).max(peak(&out[1]));
          assert!(most < 16., "{}: turning {} with {:?} built up to {}", name, p.key, base, most);
        }
      }
    }
  }
}

#[test]
fn test_running_allocates_nothing() {
  for entry in effects() {
    for (i, plains) in settings(entry).iter().enumerate() {
      let store = Store::new(entry);
      let mut runner = Runner::new(store.clone(), RATE as f32);
      let input = noise(4096, i as u64 + 1);
      let (mut l, mut r) = (vec![0.; 4096], vec![0.; 4096]);
      let t = transport(RATE);
      // new values to glide to, then blocks of every size, as the engine
      // would run it (it asks has_pending and tail_frames each block too)
      store.set_plains(plains);
      let before = counting::made();
      for block in [4096, 1, 7, 64, 512] {
        assert!(runner.has_pending() || !runner.has_pending());
        runner.process([&input[..block], &input[..block]], [&mut l[..block], &mut r[..block]], &t);
        let _ = runner.tail_frames();
      }
      let made = counting::made() - before;
      assert_eq!(made, 0, "{}: settings #{} allocated {} times while running", entry.info.name, i, made);
    }
  }
}

#[test]
fn test_values_set_before_it_first_runs_are_where_it_starts() {
  // (a project's settings are put in after an effect is made: they mustn't
  // glide in from its defaults)
  for entry in effects() {
    for (i, plains) in settings(entry).iter().enumerate() {
      let mut before = Runner::new(store_with(entry, plains), RATE as f32);
      let store = Store::new(entry);
      let mut after = Runner::new(store.clone(), RATE as f32);
      store.set_plains(plains);
      let input = noise(8192, i as u64 + 1);
      assert!(
        run(&mut before, &input, 512, RATE) == run(&mut after, &input, 512, RATE),
        "{}: settings #{} glided in from its defaults", entry.info.name, i
      );
    }
  }
}

#[test]
fn test_reset_forgets_all_sound() {
  for entry in effects() {
    for (i, plains) in settings(entry).iter().enumerate() {
      let mut runner = Runner::new(store_with(entry, plains), RATE as f32);
      run(&mut runner, &noise(4096, i as u64 + 1), 512, RATE);
      runner.reset();
      let out = run(&mut runner, &vec![0.; 4096], 512, RATE);
      assert!(peak(&out[0]).max(peak(&out[1])) == 0., "{}: settings #{} still sounds after a reset", entry.info.name, i);
    }
  }
}

#[test]
fn test_a_new_sample_rate_sounds_as_one_it_started_at() {
  for entry in effects() {
    for (i, plains) in settings(entry).iter().enumerate() {
      let store = store_with(entry, plains);
      let mut moved = Runner::new(store.clone(), 44_100.);
      run(&mut moved, &noise(4096, i as u64 + 1), 512, 44_100);
      let mut fresh = Runner::new(store.clone(), 96_000.);
      let input = noise(8192, 99);
      let a = run(&mut moved, &input, 512, 96_000);
      let b = run(&mut fresh, &input, 512, 96_000);
      assert_clean(entry.info.name, &a, "a new sample rate");
      assert!(a == b, "{}: settings #{} sounds different after moving to 96 kHz", entry.info.name, i);
    }
  }
}

#[test]
fn test_what_it_puts_out_is_tapped_for_the_app() {
  for entry in effects() {
    let store = Store::new(entry);
    let mut runner = Runner::new(store.clone(), RATE as f32);
    let input = noise(5000, 3);
    let [l, r] = run(&mut runner, &input, 512, RATE);
    let tap = store.tap();
    assert_eq!((tap.written(), tap.rate()), (5000, RATE));
    let (latest, _) = tap.latest(64).unwrap();
    let expected: Vec<f32> = (5000 - 64..5000).map(|i| (l[i] + r[i]) * 0.5).collect();
    assert_eq!(latest, expected, "{}", entry.info.name);
  }
}

#[test]
fn test_values_set_together_come_back_as_they_were() {
  for entry in effects() {
    for plains in settings(entry) {
      let store = store_with(entry, &plains);
      let back = store.plains();
      for (p, (a, b)) in entry.params.iter().zip(plains.iter().zip(&back)) {
        assert!((a - b).abs() <= a.abs() * 1e-4 + 1e-4, "{}: {} was set to {} and came back {}", entry.info.name, p.key, a, b);
      }
    }
  }
}
