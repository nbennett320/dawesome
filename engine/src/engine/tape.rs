// the tape on the audio thread (always-live mode, T23): one tape machine
// (crates/tape), and which of its tracks each mixer track records onto. a
// stereo track takes two: its left and right sides, side by side on the
// tape like two of a 4-track's tracks
//
// it's built whole off the audio thread (adding a track to a machine
// allocates) and sent over; the knobs and drives after that are small
// messages that don't allocate, so turning a knob doesn't restart the
// tape's wobble. with no tape, the engine runs exactly as before (T14)
//
// print mode (the default) doesn't use this: a track there is heard direct
// until its print is ready, as a 4-track's input is before it's on tape
// (W2). see context/tape-project/08-mode-removal.md
use tape::{MachineSettings, TapeMachine};

use super::processor;

pub struct LiveTape {
  pub machine: TapeMachine,
  // where on a track the tape sits: before its first effect (true), as on
  // a Portastudio, or after its whole chain
  pub before_effects: bool,
  // (track number, its left and right tracks on the machine)
  tracks: Vec<(u32, [usize; 2])>,
  // what comes off the tape this block, before it's put back on the bus
  scratch: [Vec<f32>; 2],
}

impl LiveTape {
  pub fn new(machine: TapeMachine, before_effects: bool, tracks: Vec<(u32, [usize; 2])>) -> Box<LiveTape> {
    let scratch = [vec![0.0; processor::MAX_BLOCK], vec![0.0; processor::MAX_BLOCK]];
    Box::new(LiveTape { machine, before_effects, tracks, scratch })
  }

  // a track's place on the tape, if it's on it
  pub fn sides(&self, track_number: u32) -> Option<[usize; 2]> {
    self.tracks.iter().find(|(n, _)| *n == track_number).map(|(_, sides)| *sides)
  }

  // the tape's knobs (no allocation)
  pub fn set_settings(&mut self, settings: MachineSettings) {
    self.machine.set_settings(settings);
  }

  // a track's drive (no allocation)
  pub fn set_drive(&mut self, track_number: u32, drive_db: f32) {
    if let Some([left, right]) = self.sides(track_number) {
      self.machine.set_drive(left, drive_db);
      self.machine.set_drive(right, drive_db);
    }
  }

  // a track's stereo block onto the tape and off it again, in place.
  // `advance` must have moved the tape on for this block first
  pub fn run(&mut self, sides: [usize; 2], left: &mut [f32], right: &mut [f32]) {
    for (side, buffer) in [left, right].into_iter().enumerate() {
      let frames = buffer.len();
      let out = &mut self.scratch[side][..frames];
      self.machine.record_and_play(sides[side], buffer, out);
      buffer.copy_from_slice(out);
    }
  }
}
