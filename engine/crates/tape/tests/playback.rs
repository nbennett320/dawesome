// the playback side, one channel: a pure delay when the tape runs
// steady; a wobbling pitch when it doesn't, by as much as the speed says
// (slow drift too); hiss at its set level; dropouts at their set rate;
// the dbx decode turning the hiss down; any block length; no allocation

use std::f64::consts::PI;

use tape::dbx::{DbxDecoder, DbxEncoder, VU0};
use tape::playback::{Playback, PlaybackSettings};
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

const RATE: f64 = 48_000.0;

// no wear: no hiss, no dropouts
fn clean() -> PlaybackSettings {
  PlaybackSettings { hiss_db: -200.0, dropouts_per_minute: 0.0, ..PlaybackSettings::default() }
}

fn sine(hz: f64, frames: usize, amp: f64) -> Vec<f32> {
  (0..frames).map(|n| (amp * (2.0 * PI * hz * n as f64 / RATE).sin()) as f32).collect()
}

fn run(p: &mut Playback, printed: &[f32], speed: &[f32], dbx: bool) -> Vec<f32> {
  let mut out = vec![0.0; printed.len()];
  p.process(printed, speed, dbx, &mut out);
  out
}

#[test]
fn steady_it_is_a_pure_delay_flat_to_16_khz() {
  // a steady speed, at the mean or off it (when told the mean)
  for (speed, mean) in [(1.0f32, 1.0), (1.003, 1.003f32 as f64)] {
    let mut settings = clean();
    settings.channel_delay = 0.37;
    let frames = 24_000;
    for hz in [50.0, 1_000.0, 5_000.0, 10_000.0, 15_000.0, 16_000.0] {
      let mut p = Playback::new(settings, RATE as f32, 1, 512);
      p.set_mean_speed(mean);
      let delay = p.latency();
      assert!((delay - (settings.base_delay_ms as f64 * 48.0 + 0.37)).abs() < 1e-4, "{delay}");
      let out = run(&mut p, &sine(hz, frames, 0.5), &vec![speed; frames], false);
      let mut worst = 0f64;
      for (n, &y) in out.iter().enumerate().skip(1_000) {
        let want = 0.5 * (2.0 * PI * hz * (n as f64 - delay) / RATE).sin();
        worst = worst.max((y as f64 - want).abs());
      }
      // within -60 db of the sine, at any of its frequencies
      assert!(worst < 5e-4, "{hz} hz at speed {speed}: off by {worst}");
    }
  }
}

// the frequency of a sine coming out, cycle by cycle: (seconds, hz)
fn cycles(out: &[f32]) -> Vec<(f64, f64)> {
  let mut ups = Vec::new();
  for n in 1..out.len() {
    let (a, b) = (out[n - 1] as f64, out[n] as f64);
    if a < 0.0 && b >= 0.0 {
      ups.push((n as f64 - 1.0 + a / (a - b)) / RATE);
    }
  }
  ups.windows(2).map(|w| ((w[0] + w[1]) / 2.0, 1.0 / (w[1] - w[0]))).collect()
}

// how deep a sinusoidal frequency change at `hz` is, as a fraction of
// `centre` (a least-squares fit of c + a sin + b cos)
fn modulation(cycles: &[(f64, f64)], hz: f64, centre: f64) -> f64 {
  let mut m = [[0.0; 4]; 3];
  for &(t, f) in cycles {
    let row = [1.0, (2.0 * PI * hz * t).sin(), (2.0 * PI * hz * t).cos()];
    for i in 0..3 {
      for j in 0..3 {
        m[i][j] += row[i] * row[j];
      }
      m[i][3] += row[i] * f;
    }
  }
  for c in 0..3 {
    for r in 0..3 {
      if r != c {
        let k = m[r][c] / m[c][c];
        for j in 0..4 {
          m[r][j] -= k * m[c][j];
        }
      }
    }
  }
  let (a, b) = (m[1][3] / m[1][1], m[2][3] / m[2][2]);
  (a * a + b * b).sqrt() / centre
}

#[test]
fn a_wobbling_speed_wobbles_the_pitch_by_as_much() {
  // flutter (4 hz, 1 %) and slow drift (0.1 hz, 0.03 %): the leak that
  // keeps the read point near home mustn't eat the drift
  for (hz, depth, seconds) in [(4.0, 0.01, 3.0), (0.1, 3e-4, 30.0)] {
    let frames = (seconds * RATE) as usize;
    let speed: Vec<f32> = (0..frames).map(|n| (1.0 + depth * (2.0 * PI * hz * n as f64 / RATE).sin()) as f32).collect();
    let mut p = Playback::new(clean(), RATE as f32, 1, 1024);
    let out = run(&mut p, &sine(1_000.0, frames, 0.5), &speed, false);
    let c = cycles(&out[2_000..]);
    let got = modulation(&c, hz, 1_000.0);
    println!("{hz} hz at {depth}: the pitch moves {:.4} of that", got / depth);
    assert!((got / depth - 1.0).abs() < 0.01, "{hz} hz: {got} against {depth}");
    assert_eq!(p.limited(), 0);
  }
}

// a band's power in a signal, from hann-windowed periodograms 2^15 long
fn band_rms(x: &[f32], lo: f64, hi: f64) -> f64 {
  let n = 1 << 15;
  let window: Vec<f64> = (0..n).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos()).collect();
  let w2: f64 = window.iter().map(|w| w * w).sum();
  let (mut total, mut count) = (0.0, 0);
  for chunk in x.chunks_exact(n) {
    let mut re: Vec<f64> = chunk.iter().zip(&window).map(|(v, w)| *v as f64 * w).collect();
    let mut im = vec![0.0; n];
    dsp::fft::fft(&mut re, &mut im);
    let df = RATE / n as f64;
    for k in 1..n / 2 {
      let f = k as f64 * df;
      if f >= lo && f < hi {
        total += 2.0 * (re[k] * re[k] + im[k] * im[k]) / w2 / n as f64;
      }
    }
    count += 1;
  }
  (total / count as f64).sqrt()
}

#[test]
fn hiss_comes_out_at_its_level() {
  for tilt in [0.0, 0.5, 1.0] {
    let mut settings = clean();
    settings.hiss_db = -50.0;
    settings.hiss_tilt = tilt;
    let frames = 1 << 19;
    let mut p = Playback::new(settings, RATE as f32, 4, 4096);
    let out = run(&mut p, &vec![0.0; frames], &vec![1.0; frames], false);
    let db = 20.0 * (band_rms(&out, 20.0, 20_000.0) / VU0 as f64).log10();
    println!("hiss, tilt {tilt}: {db:.2} db re 0 vu (set -50)");
    assert!((db + 50.0).abs() < 0.5, "{tilt}: {db}");
  }
}

#[test]
fn dropouts_come_at_their_rate() {
  // 10 minutes at 60 a minute, at a low rate to keep it quick; a steady
  // signal, so each dip shows as a fall in level
  let rate = 8_000.0;
  let mut settings = clean();
  settings.dropouts_per_minute = 60.0;
  settings.dropout_depth_db = (6.0, 20.0);
  let frames = (600.0 * rate) as usize;
  let mut p = Playback::new(settings, rate as f32, 9, 4096);
  let mut out = vec![0.0; frames];
  p.process(&vec![0.5; frames], &vec![1.0; frames], false, &mut out);
  let (mut count, mut inside, mut deepest, mut shallowest) = (0, false, 0f32, f32::MAX);
  let mut low = f32::MAX;
  for &y in &out[100..] {
    let dipping = y < 0.499;
    if dipping {
      low = low.min(y);
    }
    if dipping && !inside {
      count += 1;
    }
    if !dipping && inside {
      let db = -20.0 * (low / 0.5).log10();
      deepest = deepest.max(db);
      shallowest = shallowest.min(db);
      low = f32::MAX;
    }
    inside = dipping;
  }
  println!("dropouts: {count} in 10 minutes (set 600), {shallowest:.1} to {deepest:.1} db deep (set 6 to 20)");
  assert!((count as f64 / 600.0 - 1.0).abs() < 0.12, "{count}");
  assert!(shallowest > 5.5 && deepest < 20.01, "{shallowest} {deepest}");
}

#[test]
fn the_dbx_decode_turns_the_hiss_down() {
  let mut settings = clean();
  settings.hiss_db = -50.0;
  let frames = 1 << 18;
  let silence = vec![0.0; frames];
  let mut enc = DbxEncoder::new(RATE as f32);
  let mut printed = vec![0.0; frames];
  enc.process(&silence, &mut printed);
  let mut p = Playback::new(settings, RATE as f32, 4, 4096);
  let out = run(&mut p, &printed, &vec![1.0; frames], true);
  let db = 20.0 * (band_rms(&out[1 << 15..], 20.0, 20_000.0) / VU0 as f64).log10();
  println!("hiss at -50 db under silence, decoded: {db:.1} db");
  assert!(db < -75.0, "{db}");
}

#[test]
fn any_block_length_gives_the_same() {
  let frames = 48_000;
  let settings = PlaybackSettings { dropouts_per_minute: 600.0, channel_delay: -0.6, ..PlaybackSettings::default() };
  let mut t = TapeTransport::new(TransportSettings::default(), RATE as f32, 2);
  let mut speed = vec![0.0; frames];
  t.fill_speed(&mut speed);
  let mut enc = DbxEncoder::new(RATE as f32);
  let mut printed = vec![0.0; frames];
  enc.process(&sine(440.0, frames, 0.2), &mut printed);

  let mut p = Playback::new(settings, RATE as f32, 3, 256);
  let whole = run(&mut p, &printed, &speed, true);
  p.reset();
  let mut pieces = vec![0.0; frames];
  let mut at = 0;
  for n in [1, 7, 64, 333, 256, 257, 1000].iter().cycle() {
    if at == frames {
      break;
    }
    let end = (at + n).min(frames);
    p.process(&printed[at..end], &speed[at..end], true, &mut pieces[at..end]);
    at = end;
  }
  assert_eq!(whole, pieces);
}

#[test]
fn processing_allocates_nothing() {
  let settings = PlaybackSettings { dropouts_per_minute: 6_000.0, ..PlaybackSettings::default() };
  let mut p = Playback::new(settings, RATE as f32, 1, 512);
  let mut enc = DbxEncoder::new(RATE as f32);
  let mut dec = DbxDecoder::new(RATE as f32);
  let x = sine(440.0, 4096, 0.3);
  let speed = vec![1.001f32; 4096];
  let mut out = vec![0.0; 4096];
  let mut other = settings;
  other.hiss_tilt = 0.3;
  other.base_delay_ms = 8.0;
  let before = counting::made();
  for (n, dbx) in [(4096, true), (1, false), (7, true), (64, true), (512, false), (4096, true)] {
    p.process(&x[..n], &speed[..n], dbx, &mut out[..n]);
  }
  p.set_settings(other);
  p.set_mean_speed(1.001);
  p.process(&x, &speed, true, &mut out);
  p.reset();
  enc.process(&x, &mut out);
  dec.process_in_place(&mut out);
  enc.reset();
  dec.reset();
  assert_eq!(counting::made() - before, 0);
}
