// the project's tape (T14, T20, T22 in context/tape-project): the switch,
// the machine's controls (the Transport tab) and which tracks are on tape,
// each with its own drive (the Mixer tab). saved with the project, and
// left out of the file while it's all at its defaults, so projects that
// don't use tape save just as before
//
// only the settings for now: nothing here changes the sound yet. the engine
// wiring (daw/tape/, see 02-plan.md) is meant to take this over
use serde::{Deserialize, Serialize};

// the machines to choose from, by their place in this list (what's saved).
// the second deck's real name comes after the machine check (step 0, see
// 04-recording.md)
pub const MACHINES: u8 = 2;

pub const AGE_MAX: f32 = 100.;
pub const WOBBLE_MAX: f32 = 200.;
pub const HISS_MAX: f32 = 200.;
pub const OUTPUT_DB_MIN: f32 = -24.;
pub const OUTPUT_DB_MAX: f32 = 12.;
pub const DRIVE_DB_MIN: f32 = -12.;
pub const DRIVE_DB_MAX: f32 = 18.;

// the machine's controls, as on the Transport tab
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TapeMachine {
  // the project switch: off, the engine plays as it would without tape
  pub enabled: bool,
  // 0 is the Porta 414, 1 the second deck
  pub machine: u8,
  // how worn the tape is, 0 (fresh) to 100 %
  pub age: f32,
  // wow and flutter, 100 % is the real deck's, up to 200 %
  pub wobble: f32,
  // hiss, 100 % is the real deck's, up to 200 %
  pub hiss: f32,
  // the deck's noise reduction
  pub dbx: bool,
  pub output_db: f32,
}

impl Default for TapeMachine {
  fn default() -> Self {
    TapeMachine {
      enabled: false,
      machine: 0,
      age: 0.,
      wobble: 100.,
      hiss: 100.,
      dbx: true,
      output_db: 0.,
    }
  }
}

// a number kept in range. one that isn't a number is the default
fn kept(value: f32, min: f32, max: f32, default: f32) -> f32 {
  if value.is_nan() { default } else { value.clamp(min, max) }
}

impl TapeMachine {
  // in range: a hand-edited file, or a newer app's machine, can't go past
  // what the controls can
  pub fn sanitized(self) -> Self {
    let default = TapeMachine::default();
    TapeMachine {
      enabled: self.enabled,
      machine: if self.machine < MACHINES { self.machine } else { default.machine },
      age: kept(self.age, 0., AGE_MAX, default.age),
      wobble: kept(self.wobble, 0., WOBBLE_MAX, default.wobble),
      hiss: kept(self.hiss, 0., HISS_MAX, default.hiss),
      dbx: self.dbx,
      output_db: kept(self.output_db, OUTPUT_DB_MIN, OUTPUT_DB_MAX, default.output_db),
    }
  }
}

// a track's place on the tape
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackTape {
  pub track_number: u32,
  // whether its audio goes through the tape before its mixer channel
  pub on_tape: bool,
  // its record level onto the tape, like a Portastudio's input trim
  pub drive_db: f32,
}

impl Default for TrackTape {
  fn default() -> Self {
    TrackTape { track_number: 0, on_tape: false, drive_db: 0. }
  }
}

impl TrackTape {
  pub fn sanitized(self) -> Self {
    TrackTape { drive_db: kept(self.drive_db, DRIVE_DB_MIN, DRIVE_DB_MAX, 0.), ..self }
  }

  // off the tape at 0 dB, as every track starts
  fn is_default(&self) -> bool {
    !self.on_tape && self.drive_db == 0.
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TapeSettings {
  #[serde(flatten)]
  pub machine: TapeMachine,
  // only tracks changed from their defaults, by track number
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub tracks: Vec<TrackTape>,
}

impl TapeSettings {
  // all at its defaults: tape off, and no track on it or driven
  pub fn is_default(&self) -> bool {
    *self == TapeSettings::default()
  }

  pub fn sanitized(&self) -> Self {
    let mut tape = TapeSettings { machine: self.machine.sanitized(), tracks: Vec::new() };
    for track in &self.tracks {
      tape.set_track(track.sanitized());
    }
    tape
  }

  // a track's place on the tape (off it, at 0 dB, if it's not listed).
  // (for the engine wiring, which doesn't exist yet)
  #[allow(dead_code)]
  pub fn track(&self, track_number: u32) -> TrackTape {
    self.tracks.iter().find(|t| t.track_number == track_number).copied()
      .unwrap_or(TrackTape { track_number, ..TrackTape::default() })
  }

  // change a track's, keeping the list in track order and without tracks at
  // their defaults (so changing one back is no change to the project)
  pub fn set_track(&mut self, track: TrackTape) {
    self.tracks.retain(|t| t.track_number != track.track_number);
    if !track.is_default() {
      let at = self.tracks.partition_point(|t| t.track_number < track.track_number);
      self.tracks.insert(at, track);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_defaults_are_tape_off_and_left_out() {
    let tape = TapeSettings::default();
    assert!(!tape.machine.enabled);
    assert_eq!(tape.machine.machine, 0);
    assert_eq!((tape.machine.age, tape.machine.wobble, tape.machine.hiss), (0., 100., 100.));
    assert!(tape.machine.dbx);
    assert_eq!(tape.machine.output_db, 0.);
    assert!(tape.is_default());
    // and missing fields are their defaults
    assert_eq!(serde_json::from_str::<TapeSettings>("{}").unwrap(), tape);
  }

  #[test]
  fn test_values_are_kept_in_range() {
    let wild = TapeSettings {
      machine: TapeMachine {
        enabled: true, machine: 9, age: 400., wobble: -3., hiss: f32::NAN, dbx: false, output_db: 99.,
      },
      tracks: vec![
        TrackTape { track_number: 3, on_tape: true, drive_db: 40. },
        TrackTape { track_number: 1, on_tape: false, drive_db: -40. },
        // at its defaults: left out
        TrackTape { track_number: 2, on_tape: false, drive_db: 0. },
      ],
    };
    let tape = wild.sanitized();
    assert_eq!(tape.machine, TapeMachine {
      enabled: true, machine: 0, age: 100., wobble: 0., hiss: 100., dbx: false, output_db: 12.,
    });
    assert_eq!(tape.tracks, vec![
      TrackTape { track_number: 1, on_tape: false, drive_db: -12. },
      TrackTape { track_number: 3, on_tape: true, drive_db: 18. },
    ]);
  }

  #[test]
  fn test_a_track_back_at_its_defaults_is_dropped() {
    let mut tape = TapeSettings::default();
    tape.set_track(TrackTape { track_number: 4, on_tape: true, drive_db: 3. });
    assert_eq!(tape.track(4), TrackTape { track_number: 4, on_tape: true, drive_db: 3. });
    assert_eq!(tape.track(5), TrackTape { track_number: 5, ..TrackTape::default() });
    tape.set_track(TrackTape { track_number: 4, on_tape: false, drive_db: 0. });
    assert!(tape.is_default());
  }
}
