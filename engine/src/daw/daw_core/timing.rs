use std::time::{Duration};

// pub static MAX_SUBDIVISIONS: u32 = 508_032_000;

pub struct TimeSignature {
  pub numerator: u16,
  pub denominator: u16
}

pub trait MusicalTiming {
  // fn new() -> Self where Self: Sized;
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

impl WholeNote {
  pub fn new() -> Self {
    WholeNote {
      subdivision: 1
    }
  }
}

impl MusicalTiming for WholeNote {
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

impl HalfNote {
  pub fn new() -> Self {
    HalfNote {
      subdivision: 2
    }
  }
}

impl MusicalTiming for HalfNote {
  // fn new() -> Self {
  //   HalfNote {
  //     subdivision: 2
  //   }
  // }

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

impl QuarterNote {
  pub fn new() -> Self {
    QuarterNote {
      subdivision: 4
    }
  }
}

impl MusicalTiming for QuarterNote {
  // fn new() -> Self {
  //   QuarterNote {
  //     subdivision: 4
  //   }
  // }

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

impl EighthNote {
  pub fn new() -> Self {
    EighthNote {
      subdivision: 8
    }
  }
}

impl MusicalTiming for EighthNote {
  // fn new() -> Self {
  //   EighthNote {
  //     subdivision: 8
  //   }
  // }

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

impl SixteenthNote {
  pub fn new() -> Self {
    SixteenthNote {
      subdivision: 16
    }
  }
}

impl MusicalTiming for SixteenthNote {
  // fn new() -> Self {
  //   SixteenthNote {
  //     subdivision: 16
  //   }
  // }

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
  let beats_per_sec = 60. / tempo;
  let dur = Duration::from_secs_f32(beats_per_sec);

  dur
}

pub fn subdivision_interval_from_tempo<T: MusicalTiming>(
  tempo: f32, 
  note: T
) -> Duration {
  let interval = beat_interval_from_tempo(tempo);
  
  interval / note.subdivisions()
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
