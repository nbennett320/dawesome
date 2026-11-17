// always-live mode (T23): the tape built for the engine to run on every
// taped track, each block. remove this file (and its uses in mod.rs) to
// drop the mode: see context/tape-project/08-mode-removal.md
//
// for now every machine is the hand-built core with the fake deck's
// settings (crates/tape's defaults): the stand-in until the real decks are
// measured and trained (profile.json, model.json)
use tape::{MachineSettings, PlaybackSettings, RecordSide, SimpleCore, SimpleCoreSettings, TapeMachine, TransportSettings};

use crate::daw::tape_settings::TapeSettings;
use crate::engine::processor::MAX_BLOCK;
use crate::engine::tape::LiveTape;
use crate::preferences::{TapeMode, TapeTap};

// what a live tape is built for: when this changes, it's built again
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
  pub rate: u32,
  pub before_effects: bool,
  pub machine: u8,
  // the tracks on it, in order
  pub tracks: Vec<u32>,
}

// the live tape the engine should have, if any
pub fn shape(settings: &TapeSettings, mode: TapeMode, tap: TapeTap, rate: u32) -> Option<Shape> {
  if !settings.machine.enabled || mode != TapeMode::Live {
    return None;
  }
  let mut tracks: Vec<u32> = settings.tracks.iter().filter(|t| t.on_tape).map(|t| t.track_number).collect();
  if tracks.is_empty() {
    return None;
  }
  tracks.sort_unstable();
  Some(Shape { rate, before_effects: tap == TapeTap::BeforeEffects, machine: settings.machine.machine, tracks })
}

// the Transport tab's knobs on the machine's profile
// - Age isn't applied yet: it blends between measured profiles (D57)
pub fn machine_settings(settings: &TapeSettings) -> MachineSettings {
  let m = &settings.machine;
  MachineSettings {
    transport: TransportSettings::default(),
    playback: PlaybackSettings::default(),
    wow_flutter: m.wobble / 100.0,
    hiss: m.hiss / 100.0,
    dbx: m.dbx,
    output_db: m.output_db,
  }
}

// a seed for the tape's wobble, hiss and dropouts: the same every time
// it's built, so a project sounds the same each time it plays
const SEED: u64 = 0x7a9e_5eed;

pub fn build(shape: &Shape, settings: &TapeSettings) -> Box<LiveTape> {
  let rate = shape.rate as f32;
  let mut machine = TapeMachine::new(machine_settings(settings), rate, SEED, MAX_BLOCK);
  let mut sides = Vec::with_capacity(shape.tracks.len());
  for &number in &shape.tracks {
    let mut side = || {
      let core = Box::new(SimpleCore::new(SimpleCoreSettings::default(), rate, MAX_BLOCK));
      machine.add_track(Some(RecordSide::new(core, rate, MAX_BLOCK)), 0.0)
    };
    let pair = [side(), side()];
    let drive = settings.track(number).drive_db;
    machine.set_drive(pair[0], drive);
    machine.set_drive(pair[1], drive);
    sides.push((number, pair));
  }
  LiveTape::new(machine, shape.before_effects, sides)
}
