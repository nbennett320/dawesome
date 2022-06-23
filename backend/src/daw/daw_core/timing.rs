use std::time::{Duration};

// pub static MAX_SUBDIVISIONS: u32 = 508_032_000;

pub struct TimeSignature {
  pub numerator: u16,
  pub denominator: u16
}

pub trait MusicalTiming {
  fn new() -> Self;
  fn ratio(&self) -> (u32, u32);
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct Note {
  subdivision: u32
}

impl MusicalTiming for Note {
  fn new() -> Self {
    Note {
      subdivision: 1
    }
  }

  fn ratio(&self) -> (u32, u32) {
    (1, self.subdivision)
  }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct QuarterNote {
  note: Note
}

impl MusicalTiming for QuarterNote {
  fn new() -> Self {
    QuarterNote {
      note: Note {
        subdivision: 4
      }
    }
  }

  fn ratio(&self) -> (u32, u32) {
   (1, self.note.subdivision)
  }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct EighthNote {
  note: Note
}

impl MusicalTiming for EighthNote {
  fn new() -> Self {
    EighthNote {
      note: Note {
        subdivision: 8
      }
    }
  }

  fn ratio(&self) -> (u32, u32) {
   (1, self.note.subdivision)
  }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct SixteenthNote {
  note: Note
}

impl MusicalTiming for SixteenthNote {
  fn new() -> Self {
    SixteenthNote {
      note: Note {
        subdivision: 16
      }
    }
  }

  fn ratio(&self) -> (u32, u32) {
   (1, self.note.subdivision)
  }
}

// util functions
pub fn tempo_to_interval(tempo: f32) -> Duration {
  let beats_per_sec = tempo / 60. / 4.;
  let dur = Duration::from_secs_f32(beats_per_sec);

  println!("dir: {}s", dur.as_secs_f32());
  
  dur
}
