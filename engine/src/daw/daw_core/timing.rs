use std::time::{Duration};

// pub static MAX_SUBDIVISIONS: u32 = 508_032_000;

// saved in project files as {"numerator": 4, "denominator": 4}
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct TimeSignature {
  pub numerator: u16,
  pub denominator: u16
}

pub trait MusicalTiming {
  // length of the note in beats, as (numerator, denominator)
  fn ratio(&self) -> (u32, u32);
}

// serialized by name ("QuarterNote", ...) for the frontend
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, serde::Serialize, serde::Deserialize)]
pub enum Note {
  WholeNote,
  HalfNote,
  QuarterNote,
  EighthNote,
  SixteenthNote,
}

impl MusicalTiming for Note {
  fn ratio(&self) -> (u32, u32) {
    match self {
      Note::WholeNote => (4, 1),
      Note::HalfNote => (2, 1),
      Note::QuarterNote => (1, 1),
      Note::EighthNote => (1, 2),
      Note::SixteenthNote => (1, 4),
    }
  }
}

// a concrete type per note, so call sites read like
// `interval_of_subdivision(QuarterNote::new())`
macro_rules! note_type {
  ($name:ident) => {
    #[derive(Clone, Copy, PartialEq, Debug, Default)]
    pub struct $name;

    #[allow(dead_code)]
    impl $name {
      pub fn new() -> Self { $name }
    }

    impl MusicalTiming for $name {
      fn ratio(&self) -> (u32, u32) { Note::$name.ratio() }
    }
  };
}

note_type!(WholeNote);
note_type!(HalfNote);
note_type!(QuarterNote);
note_type!(EighthNote);
note_type!(SixteenthNote);

// util functions

// calculate the interval of one beat, given a tempo
pub fn beat_interval_from_tempo(tempo: f32) -> Duration {
  let beats_per_sec = 60. / tempo;
  let dur = Duration::from_secs_f32(beats_per_sec);

  dur
}

pub fn subdivision_interval_from_tempo<T: MusicalTiming>(
  tempo: f32, 
  note: T
) -> Duration {
  let (beats, per) = note.ratio();

  beat_interval_from_tempo(tempo) * beats / per
}

// where an offset is once the beat's length changes from `before` to `after`,
// keeping the beat it's on
pub fn keep_beat(offset: Duration, before: Duration, after: Duration) -> Duration {
  if before.is_zero() { return offset; }

  // in whole nanoseconds, to the nearest, so a round trip (120 to 90 and
  // back) doesn't drift
  let nanos = (offset.as_nanos() * after.as_nanos() + before.as_nanos() / 2) / before.as_nanos();
  Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

// the tempos the playlist can have, in beats a minute
pub const MIN_TEMPO: f32 = 1.;
pub const MAX_TEMPO: f32 = 1000.;

pub fn valid_tempo(tempo: f32) -> bool {
  tempo.is_finite() && tempo >= MIN_TEMPO && tempo <= MAX_TEMPO
}

// calculate the duration for n beats to execute
pub fn n_beat_duration_from_tempo(
  tempo: f32, 
  n: u32
) -> Duration {
  let interval = beat_interval_from_tempo(tempo);
  println!("beat interval: {}ms", interval.as_millis());

  interval * n
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_keep_beat() {
    let ms = Duration::from_millis;
    // beat 8 at 120bpm (500ms a beat) is beat 8 at 60bpm (1s a beat)
    assert_eq!(keep_beat(ms(4_000), ms(500), ms(1_000)), ms(8_000));
    assert_eq!(keep_beat(ms(4_000), ms(1_000), ms(500)), ms(2_000));
    assert_eq!(keep_beat(ms(0), ms(500), ms(1_000)), ms(0));
    // between beats too
    assert_eq!(keep_beat(ms(750), ms(500), ms(1_000)), ms(1_500));

    // there and back doesn't drift
    let (a, b) = (beat_interval_from_tempo(120.), beat_interval_from_tempo(93.));
    let start = a * 9;
    let there = keep_beat(start, a, b);
    assert!(there.abs_diff(b * 9) < Duration::from_nanos(10));
    assert!(keep_beat(there, b, a).abs_diff(start) < Duration::from_nanos(10));

    // off the grid, near the end of the playlist, through a few odd tempos
    // and back: still within a nanosecond or two
    let off_grid = a.mul_f64(255.3);
    let tempos = [93., 61.5, 177., 33., 120.];
    let mut offset = off_grid;
    let mut interval = a;
    for _ in 0..200 {
      for tempo in tempos {
        let next = beat_interval_from_tempo(tempo);
        offset = keep_beat(offset, interval, next);
        interval = next;
      }
    }
    assert!(offset.abs_diff(off_grid) < Duration::from_nanos(1_000), "{:?}", offset.abs_diff(off_grid));
  }

  #[test]
  fn test_valid_tempo() {
    assert!([1., 20., 120., 999.5, 1000.].into_iter().all(valid_tempo));
    assert!(![0., 0.5, -30., 1000.5, f32::NAN, f32::INFINITY, 1e-20].into_iter().any(valid_tempo));
  }

  #[test]
  fn test_note_lengths() {
    // 120bpm: one beat (a quarter note) is 500ms
    let ms = |note: Note| subdivision_interval_from_tempo(120., note).as_millis();
    assert_eq!(ms(Note::WholeNote), 2_000);
    assert_eq!(ms(Note::HalfNote), 1_000);
    assert_eq!(ms(Note::QuarterNote), 500);
    assert_eq!(ms(Note::EighthNote), 250);
    assert_eq!(ms(Note::SixteenthNote), 125);

    // the note types match the enum
    assert_eq!(WholeNote::new().ratio(), Note::WholeNote.ratio());
    assert_eq!(HalfNote::new().ratio(), Note::HalfNote.ratio());
    assert_eq!(QuarterNote::new().ratio(), Note::QuarterNote.ratio());
    assert_eq!(EighthNote::new().ratio(), Note::EighthNote.ratio());
    assert_eq!(SixteenthNote::new().ratio(), Note::SixteenthNote.ratio());
  }
}
