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

// calculate the duration for n beats to execute
pub fn n_beat_duration_from_tempo(
  tempo: f32, 
  n: u32
) -> Duration {
  let interval = beat_interval_from_tempo(tempo);
  println!("beat interval: {}ms", interval.as_millis());

  interval * n
}

// calculate the duration for n subdivisions to execute
pub fn n_subdivision_duration_from_tempo<T: MusicalTiming>(
  tempo: f32, 
  n: u32,
  note: T,
) -> Duration {
  let interval = subdivision_interval_from_tempo(tempo, note);
  println!("beat interval: {}ms", interval.as_millis());

  interval * n
}

#[cfg(test)]
mod tests {
  use super::*;

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
