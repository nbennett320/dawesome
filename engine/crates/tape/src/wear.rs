// how a tape wears: what the Age knob does (T6, T13). Age runs from a
// fresh cassette (0), through a worn one (0.5), to a cheap 1950s reel that's
// sat in a junk shop for seventy years (1): more wow and drift, a sticky
// tape catching on the heads, more hiss, dropouts, and highs worn away
//
// for now the three are the fake deck's presets (tools/tape-model,
// faketape.py: Deck, WORN, ANCIENT), as the transport and playback sides
// take them. measured worn cassettes replace the middle once they're
// recorded (B3); nobody will measure the 1950s end
//
// only the playback side and the transport wear here. the record side
// (saturation, the deck's own eq) is the deck's, not the tape's, so it
// stays as it is: a worn tape's loss of highs is the playback side's
// loss_hz instead
//
// what isn't here yet from the ancient preset: mono, print-through, noise
// riding on the signal, the level wandering (D74)

use crate::playback::{PlaybackSettings, LOSS_OFF_HZ};
use crate::transport::{Rotor, TransportSettings, MAX_ROTORS};

// one point on the Age knob
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wear {
  pub transport: TransportSettings,
  pub playback: PlaybackSettings,
}

// a fresh cassette: the transport's and playback's defaults (the fake
// deck's measured values)
pub fn fresh() -> Wear {
  Wear { transport: TransportSettings::default(), playback: PlaybackSettings::default() }
}

// a worn one (faketape.WORN): twice the wow, a little more drift, 6 db more
// hiss, 20 dropouts a minute, the highs going from 9 khz
pub fn worn() -> Wear {
  let mut transport = TransportSettings::from_measured(0.0, 4.0e-4, 1.4e-3, 2.0e-4, &[(7.3, 3.0e-4), (21.0, 1.2e-4)]);
  transport.stick_slips_per_minute = 0.0;
  let playback = PlaybackSettings {
    hiss_db: PlaybackSettings::default().hiss_db + 6.0,
    dropouts_per_minute: 20.0,
    dropout_depth_db: (3.0, 15.0),
    dropout_seconds: (0.01, 0.08),
    dropout_dulling: 0.6,
    loss_hz: 9_000.0,
    ..PlaybackSettings::default()
  };
  Wear { transport, playback }
}

// the 1950s reel (faketape.ANCIENT): heavy, slow wow from lopsided old
// parts (peaks at 5.1 and 13 hz), the sticky tape catching 8 times a
// minute, hiss 22 db up, 45 deep, long dropouts a minute, the highs gone
// from 4.5 khz. its drift is toned down from the fake deck's 3e-3 to 6e-4:
// a slow wander that big can't fit an insert's delay line (O6); a lasting
// speed change is the engine's varispeed (part E)
pub fn ancient() -> Wear {
  let mut transport = TransportSettings::from_measured(0.0, 6.0e-4, 4.2e-3, 1.2e-3, &[(5.1, 1.5e-3), (13.0, 8.0e-4)]);
  transport.stick_slips_per_minute = 8.0;
  let playback = PlaybackSettings {
    hiss_db: PlaybackSettings::default().hiss_db + 22.0,
    dropouts_per_minute: 45.0,
    dropout_depth_db: (6.0, 30.0),
    dropout_seconds: (0.02, 0.3),
    dropout_dulling: 0.8,
    loss_hz: 4_500.0,
    ..PlaybackSettings::default()
  };
  Wear { transport, playback }
}

// where `age` (0 to 1) puts the tape: fresh to worn over the first half,
// worn to ancient over the second
pub fn at(age: f32) -> Wear {
  let age = if age.is_finite() { age.clamp(0.0, 1.0) } else { 0.0 };
  if age <= 0.5 {
    blend(&fresh(), &worn(), age * 2.0)
  } else {
    blend(&worn(), &ancient(), (age - 0.5) * 2.0)
  }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
  a + (b - a) * t
}

// between two frequencies by ratio, so a sweep sounds even
// (the ends exactly: exp of ln rounds, and a hair under LOSS_OFF_HZ would
// switch the loss on for a fresh tape)
fn lerp_hz(a: f32, b: f32, t: f32) -> f32 {
  if t <= 0.0 { return a; }
  if t >= 1.0 { return b; }
  (a.max(1.0).ln() + (b.max(1.0).ln() - a.max(1.0).ln()) * t).exp()
}

fn pair(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
  (lerp(a.0, b.0, t), lerp(a.1, b.1, t))
}

// `t` of the way from a to b. strengths blend linearly; a's turning parts
// fade out as b's fade in (they're different parts, not one sliding in
// pitch)
pub fn blend(a: &Wear, b: &Wear, t: f32) -> Wear {
  // the ends exactly (a + (b - a) * 1 isn't always b in floats)
  if t <= 0.0 { return *a; }
  if t >= 1.0 { return *b; }
  let (x, y) = (&a.transport, &b.transport);
  let mut transport = *x;
  transport.speed_error = lerp(x.speed_error, y.speed_error, t);
  transport.reel_depth = lerp(x.reel_depth, y.reel_depth, t);
  transport.drift = lerp(x.drift, y.drift, t);
  transport.friction = lerp(x.friction, y.friction, t);
  transport.stretch = lerp(x.stretch, y.stretch, t);
  transport.scrape = lerp(x.scrape, y.scrape, t);
  transport.stick_slips_per_minute = lerp(x.stick_slips_per_minute, y.stick_slips_per_minute, t);
  transport.stick_depth = pair(x.stick_depth, y.stick_depth, t);
  transport.stick_seconds = pair(x.stick_seconds, y.stick_seconds, t);
  // the turning parts: a's at (1 - t), then b's at t, as many as fit
  let mut rotors = [Rotor::default(); MAX_ROTORS];
  let parts = x.rotors.iter().map(|r| (r, 1.0 - t)).chain(y.rotors.iter().map(|r| (r, t)));
  for (slot, (r, w)) in rotors.iter_mut().zip(parts.filter(|(r, w)| r.depth > 0.0 && *w > 0.0)) {
    *slot = Rotor { hz: r.hz, depth: r.depth * w };
  }
  transport.rotors = rotors;

  let (p, q) = (&a.playback, &b.playback);
  let playback = PlaybackSettings {
    hiss_db: lerp(p.hiss_db, q.hiss_db, t),
    hiss_tilt: lerp(p.hiss_tilt, q.hiss_tilt, t),
    dropouts_per_minute: lerp(p.dropouts_per_minute, q.dropouts_per_minute, t),
    dropout_depth_db: pair(p.dropout_depth_db, q.dropout_depth_db, t),
    dropout_seconds: pair(p.dropout_seconds, q.dropout_seconds, t),
    dropout_dulling: lerp(p.dropout_dulling, q.dropout_dulling, t),
    // from no loss at all, the cut comes in from the top of the band
    loss_hz: lerp_hz(p.loss_hz.min(LOSS_OFF_HZ), q.loss_hz.min(LOSS_OFF_HZ), t),
    ..*p
  };
  Wear { transport, playback }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::playback::Playback;
  use crate::transport::TapeTransport;

  #[test]
  fn the_ends_are_the_presets() {
    assert_eq!(at(0.0), fresh());
    assert_eq!(at(0.5), worn());
    assert_eq!(at(1.0), ancient());
    assert_eq!(at(f32::NAN), fresh());
    assert_eq!(at(7.0), ancient());
  }

  #[test]
  fn age_wears_the_tape_steadily() {
    // more hiss, dropouts and wobble, and fewer highs, all the way up
    let mut last = at(0.0);
    for i in 1..=20 {
      let w = at(i as f32 / 20.0);
      assert!(w.playback.hiss_db >= last.playback.hiss_db);
      assert!(w.playback.dropouts_per_minute >= last.playback.dropouts_per_minute);
      assert!(w.playback.loss_hz <= last.playback.loss_hz);
      last = w;
    }
  }

  #[test]
  fn parts_fade_rather_than_slide() {
    let w = at(0.75);
    let hz: Vec<f32> = w.transport.rotors.iter().filter(|r| r.depth > 0.0).map(|r| r.hz).collect();
    // the worn tape's parts at half strength, and the ancient reel's
    assert!(hz.iter().any(|&f| (f - 7.3).abs() < 0.1) && hz.iter().any(|&f| (f - 5.1).abs() < 0.1), "{hz:?}");
  }

  #[test]
  fn the_oldest_tape_still_fits_the_delay_line() {
    // O6: the ancient reel's wobble, at the Wow & Flutter knob's 100 %,
    // rarely needs easing at the default 8 ms (the fake deck's own drift
    // eased it a third of the time)
    let rate = 48_000.0;
    let w = ancient();
    let mut transport = TapeTransport::new(w.transport, rate, 3);
    let mut playback = Playback::new(w.playback, rate, 3, 512);
    playback.set_mean_speed(transport.mean_speed());
    let (mut speed, input, mut out) = (vec![0.0; 512], vec![0.0; 512], vec![0.0; 512]);
    let blocks = 60 * 48_000 / 512;
    for _ in 0..blocks {
      transport.fill_speed(&mut speed);
      playback.process(&input, &speed, false, &mut out);
    }
    let eased = playback.limited() as f64 / (blocks * 512) as f64;
    assert!(eased < 0.02, "eased {:.1} % of the time", eased * 100.0);
  }
}
