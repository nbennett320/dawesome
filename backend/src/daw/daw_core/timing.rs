use std::time::{Duration};

// pub static MAX_SUBDIVISIONS: u32 = 508_032_000;

pub struct TimeSignature {
  pub numerator: u16,
  pub denominator: u16
}

trait MusicalTiming {
  fn new(subdivision: u32) -> Self;
  fn ratio(&self) -> (u32, u32);
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct Note {
  subdivision: u32
}

impl MusicalTiming for Note {
  fn new(subdivision: u32) -> Self {
    Note {
      subdivision
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

impl QuarterNote {
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

impl EighthNote {
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

impl SixteenthNote {
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
  let min_ms = 60_000.;
  let ms_intrv = min_ms / tempo;
  let us_intv = ms_intrv * 1_000.;
  
  Duration::from_micros(us_intv.round() as u64)
}
