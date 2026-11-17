// always-live mode (T23): the tape built for the engine to run on every
// taped track, each block. remove this file (and its uses in mod.rs) to
// drop the mode: see context/tape-project/08-mode-removal.md
//
// for now every machine is the hand-built core with the fake deck's
// settings (crates/tape's defaults): the stand-in until the real decks are
// measured and trained (profile.json, model.json). to try a trained
// network instead, point DAWESOME_TAPE_MODEL at its model.json (from
// tools/tape-model's `train`): every taped track then records through it,
// when the output runs at the network's rate (no resampling yet, C2)
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tape::{GruWeights, MachineSettings, NeuralCore, PlaybackSettings, RecordSide, SimpleCore, SimpleCoreSettings, TapeCore, TapeMachine, TransportSettings};

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

// a trained network to try, named by DAWESOME_TAPE_MODEL, read once
fn network() -> Option<Arc<GruWeights>> {
  static NETWORK: OnceLock<Option<Arc<GruWeights>>> = OnceLock::new();
  NETWORK.get_or_init(|| {
    let path = std::env::var("DAWESOME_TAPE_MODEL").ok()?;
    match read_model(&path) {
      Ok(weights) => {
        println!("tape: recording through the network in {} ({} units, {} Hz)", path, weights.hidden, weights.rate);
        Some(Arc::new(weights))
      }
      Err(e) => {
        println!("tape: couldn't read the network in {}: {}", path, e);
        None
      }
    }
  }).clone()
}

// model.json (tools/tape-model's model.py, schema 1) as plain arrays
fn read_model(path: &str) -> Result<GruWeights, String> {
  let spec: Value = serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
  if spec["schema"] != 1 || spec["architecture"] != "gru" {
    return Err("not a schema 1 gru model".into());
  }
  let floats = |v: &Value| -> Result<Vec<f32>, String> {
    v.as_array().ok_or("missing weights")?.iter().map(|x| x.as_f64().map(|x| x as f32).ok_or_else(|| "not a number".to_string())).collect()
  };
  let w = &spec["weights"];
  let weights = GruWeights {
    hidden: spec["hidden"].as_u64().ok_or("no hidden size")? as usize,
    w_ih: floats(&w["w_ih"]["values"])?,
    w_hh: floats(&w["w_hh"]["values"])?,
    b_ih: floats(&w["b_ih"]["values"])?,
    b_hh: floats(&w["b_hh"]["values"])?,
    dense_w: floats(&w["dense_w"]["values"])?,
    dense_b: *floats(&w["dense_b"]["values"])?.first().ok_or("no dense bias")?,
    drive_scale: spec["inputs"][1]["scale"].as_f64().ok_or("no drive scale")? as f32,
    rate: spec["rate"].as_u64().ok_or("no rate")? as u32,
  };
  weights.check().map_err(|e| e.to_string())?;
  Ok(weights)
}

// a track side's core: the network if one's set and runs at this rate,
// else the hand-built one
fn core(rate: f32) -> Box<dyn TapeCore> {
  if let Some(weights) = network() {
    match NeuralCore::new(weights, rate) {
      Ok(core) => return Box::new(core),
      Err(e) => println!("tape: the hand-built core instead of the network: {}", e),
    }
  }
  Box::new(SimpleCore::new(SimpleCoreSettings::default(), rate, MAX_BLOCK))
}

pub fn build(shape: &Shape, settings: &TapeSettings) -> Box<LiveTape> {
  let rate = shape.rate as f32;
  let mut machine = TapeMachine::new(machine_settings(settings), rate, SEED, MAX_BLOCK);
  let mut sides = Vec::with_capacity(shape.tracks.len());
  for &number in &shape.tracks {
    let mut side = || machine.add_track(Some(RecordSide::new(core(rate), rate, MAX_BLOCK)), 0.0);
    let pair = [side(), side()];
    let drive = settings.track(number).drive_db;
    machine.set_drive(pair[0], drive);
    machine.set_drive(pair[1], drive);
    sides.push((number, pair));
  }
  LiveTape::new(machine, shape.before_effects, sides)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_a_trained_network_is_read_as_trained() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/crates/tape/tests/assets/fake.model.json");
    let weights = read_model(path).unwrap();
    assert_eq!((weights.hidden, weights.rate), (32, 48_000));
    assert!(NeuralCore::new(Arc::new(weights.clone()), 48_000.0).is_ok());
    // at another rate it can't run yet (no resampling): the caller falls back
    assert!(NeuralCore::new(Arc::new(weights), 44_100.0).is_err());
    assert!(read_model("/nowhere/model.json").is_err());
  }
}
