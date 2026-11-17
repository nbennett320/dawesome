// what the playback side costs: the transport (once a tape) and the
// playback (once a channel), in ns a sample at 48 khz; what reset() and
// new() take; and how often the read point runs short of room (and the
// wobble's flattened) at each base delay, for a fresh tape and an
// ancient-like one. cargo run --release -p dawesome-tape --example playback_bench

use std::time::Instant;

use tape::dbx::DbxEncoder;
use tape::playback::{Playback, PlaybackSettings};
use tape::transport::{TapeTransport, TransportSettings};

const RATE: f32 = 48_000.0;
const BLOCK: usize = 256;

// the fake deck's ancient reel, as from_measured would get it (wow from
// its 0.45 hz, 6e-3 sine; flutter and drift as set), sticky
fn ancient() -> TransportSettings {
  let mut s = TransportSettings::from_measured(-0.012, 3.0e-3, 4.2e-3, 1.2e-3, &[(5.1, 1.5e-3), (13.0, 8.0e-4)]);
  s.scrape = 6.0e-4;
  s.stick_slips_per_minute = 8.0;
  s
}

fn main() {
  let frames = 48_000 * 20;

  // the transport
  let mut t = TapeTransport::new(TransportSettings::default(), RATE, 1);
  let mut speed = vec![0f32; frames];
  t.fill_speed(&mut speed[..48_000]);
  let start = Instant::now();
  for chunk in speed.chunks_mut(BLOCK) {
    t.fill_speed(chunk);
  }
  let ns = start.elapsed().as_secs_f64() * 1e9 / frames as f64;
  println!("transport: {ns:5.1} ns a sample (one a tape)");
  let start = Instant::now();
  for _ in 0..10 {
    t.reset();
  }
  println!("transport reset (5 s warm-up): {:6.0} us", start.elapsed().as_secs_f64() * 1e6 / 10.0);
  let start = Instant::now();
  let fresh = TapeTransport::new(TransportSettings::default(), RATE, 2);
  println!("transport new: {:6.0} us", start.elapsed().as_secs_f64() * 1e6);
  drop(fresh);

  // the playback, worn and decoded: everything on
  let mut enc = DbxEncoder::new(RATE);
  let input: Vec<f32> = (0..frames).map(|i| (i as f32 * 0.03).sin() * 0.2).collect();
  let mut printed = vec![0f32; frames];
  enc.process(&input, &mut printed);
  let start = Instant::now();
  let mut enc2 = DbxEncoder::new(RATE);
  enc2.process(&input, &mut printed);
  let enc_ns = start.elapsed().as_secs_f64() * 1e9 / frames as f64;
  let mut settings = PlaybackSettings::default();
  settings.dropouts_per_minute = 20.0;
  let start = Instant::now();
  let mut p = Playback::new(settings, RATE, 1, BLOCK);
  println!("playback new: {:6.0} us (the first makes the shared sinc table)", start.elapsed().as_secs_f64() * 1e6);
  let mut out = vec![0f32; frames];
  for dbx in [false, true] {
    p.reset();
    let start = Instant::now();
    for ((x, v), o) in printed.chunks(BLOCK).zip(speed.chunks(BLOCK)).zip(out.chunks_mut(BLOCK)) {
      p.process(x, v, dbx, o);
    }
    let ns = start.elapsed().as_secs_f64() * 1e9 / frames as f64;
    println!("playback, dbx {}: {ns:5.1} ns a sample (one a channel)", if dbx { "on " } else { "off" });
  }
  println!("dbx encode: {enc_ns:5.1} ns a sample");
  let a = 2.0 * 48_000.0 * 1e-9 * 100.0;
  println!("(a stereo track's playback at 48 khz is {:.2} % of a core per 10 ns a sample)", a * 10.0);
  println!("({})", out[frames - 1]);

  // room: ten minutes of each tape at each base delay, the share of frames
  // eased off near the end of the read point's room
  let ten = 48_000 * 600;
  for (name, transport) in [("fresh (default)", TransportSettings::default()), ("ancient-like", ancient())] {
    for amount in [1.0, 2.0] {
      let mut line = format!("{name:>16}, wow & flutter {:3.0} %:", amount * 100.0);
      for base in [3.0, 5.0, 8.0, 12.0, 20.0] {
        let mut t = TapeTransport::new(transport, RATE, 7);
        t.set_amount(amount);
        t.reset();
        let mut s = PlaybackSettings::default();
        s.base_delay_ms = base;
        s.hiss_db = -200.0;
        let mut p = Playback::new(s, RATE, 7, 4096);
        p.set_mean_speed(t.mean_speed());
        let (mut v, mut x, mut o) = (vec![0f32; 4096], vec![0f32; 4096], vec![0f32; 4096]);
        x.fill(0.1);
        for _ in 0..ten / 4096 {
          t.fill_speed(&mut v);
          p.process(&x, &v, false, &mut o);
        }
        line += &format!("  {base:>2} ms {:6.2} %", p.limited() as f64 / ten as f64 * 100.0);
      }
      println!("{line}");
    }
  }
}
