use std::time::{Duration};

// pub static MAX_SUBDIVISIONS: u32 = 508_032_000;

pub struct TimeSignature {
  pub numerator: u16,
  pub denominator: u16
}

pub trait MusicalTiming {
  fn new() -> Self;
  fn ratio(&self) -> (u32, u32);
  fn subdivisions(&self) -> u32;
}

pub type MusicalSubdivision = Box<dyn MusicalTiming + Send>;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Note {
  WholeNote,
  HalfNote,
  QuarterNote,
  EighthNote,
  SixteenthNote,
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct WholeNote {
  subdivision: u32,
}

impl MusicalTiming for WholeNote {
  fn new() -> Self {
    WholeNote {
      subdivision: 1
    }
  }

  fn ratio(&self) -> (u32, u32) {
    (1, self.subdivision)
  }

  fn subdivisions(&self) -> u32 {
    self.subdivision
  }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct HalfNote {
  subdivision: u32,
}

impl MusicalTiming for HalfNote {
  fn new() -> Self {
    HalfNote {
      subdivision: 2
    }
  }

  fn ratio(&self) -> (u32, u32) {
   (1, self.subdivision)
  }

  fn subdivisions(&self) -> u32 {
    self.subdivision
  }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct QuarterNote {
  subdivision: u32,
}

impl MusicalTiming for QuarterNote {
  fn new() -> Self {
    QuarterNote {
      subdivision: 4
    }
  }

  fn ratio(&self) -> (u32, u32) {
   (1, self.subdivision)
  }

  fn subdivisions(&self) -> u32 {
    self.subdivision
  }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct EighthNote {
  subdivision: u32,
}

impl MusicalTiming for EighthNote {
  fn new() -> Self {
    EighthNote {
      subdivision: 8
    }
  }

  fn ratio(&self) -> (u32, u32) {
   (1, self.subdivision)
  }

  fn subdivisions(&self) -> u32 {
    self.subdivision
  }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct SixteenthNote {
  subdivision: u32,
}

impl MusicalTiming for SixteenthNote {
  fn new() -> Self {
    SixteenthNote {
      subdivision: 16
    }
  }

  fn ratio(&self) -> (u32, u32) {
   (1, self.subdivision)
  }

  fn subdivisions(&self) -> u32 {
    self.subdivision
  }
}

// util functions

// calculate the interval of one beat, given a tempo
pub fn beat_interval_from_tempo(tempo: f32) -> Duration {
  let beats_per_sec = tempo / 60. / 4.;
  let dur = Duration::from_secs_f32(beats_per_sec);

  dur
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
