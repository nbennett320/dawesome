// always-live mode (T23): the tape built for the engine to run on every
// taped track, each block. remove this file (and its uses in mod.rs) to
// drop the mode: see context/tape-project/08-mode-removal.md
//
// the record side is the hand-built model of a deck (Computer modeled) or
// a trained network (Neural), as Settings → Controls → Tape says. for now
// both stand in for the real decks: the model has the fake deck's
// settings, and the network bundled is one trained on the fake deck
// (crates/tape/assets/fake.model.json). DAWESOME_TAPE_MODEL, pointing at
// another model.json, overrides the bundled network, to try others. a
// network runs only at its own rate (no resampling yet, C2): at another,
// the model stands in
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tape::{wear, GruWeights, MachineSettings, NeuralCore, RecordSide, SimpleCore, SimpleCoreSettings, TapeCore, TapeMachine};

use crate::daw::tape_settings::TapeSettings;
use crate::engine::processor::MAX_BLOCK;
use crate::engine::tape::EngineTape;
use crate::preferences::{TapeMethod, TapeMode, TapeTap};

// what the engine's tape is built for: when this changes, it's built
// again
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
  pub rate: u32,
  pub before_effects: bool,
  pub machine: u8,
  pub neural: bool,
  // always-live mode: each track's record side runs in the engine; print
  // mode: the tracks wait for their prints (print.rs)
  pub live: bool,
  // the tracks on it, in order
  pub tracks: Vec<u32>,
}

// the tape the engine should have, if any
pub fn shape(settings: &TapeSettings, mode: TapeMode, tap: TapeTap, method: TapeMethod, rate: u32) -> Option<Shape> {
  if !settings.machine.enabled {
    return None;
  }
  let mut tracks: Vec<u32> = settings.tracks.iter().filter(|t| t.on_tape).map(|t| t.track_number).collect();
  if tracks.is_empty() {
    return None;
  }
  tracks.sort_unstable();
  Some(Shape { rate, before_effects: tap == TapeTap::BeforeEffects, machine: settings.machine.machine, neural: method == TapeMethod::Neural, live: mode == TapeMode::Live, tracks })
}

// the Transport tab's knobs on the machine's profile: Age wears the tape
// from fresh to a 1950s reel (crates/tape's wear.rs, the fake deck's
// presets for now)
pub fn machine_settings(settings: &TapeSettings) -> MachineSettings {
  let m = &settings.machine;
  let worn = wear::at(m.age / 100.0);
  MachineSettings {
    transport: worn.transport,
    playback: worn.playback,
    wow_flutter: m.wobble / 100.0,
    hiss: m.hiss / 100.0,
    dbx: m.dbx,
    output_db: m.output_db,
  }
}

// a seed for the tape's wobble, hiss and dropouts: the same every time
// it's built, so a project sounds the same each time it plays
const SEED: u64 = 0x7a9e_5eed;

// the network bundled with the app: one trained on the fake deck
const BUNDLED: &str = include_str!("../../../crates/tape/assets/fake.model.json");

// the network: DAWESOME_TAPE_MODEL's if it's set (and reads), else the
// bundled one. read once
fn network() -> Option<Arc<GruWeights>> {
  static NETWORK: OnceLock<Option<Arc<GruWeights>>> = OnceLock::new();
  NETWORK.get_or_init(|| {
    let (name, read) = match std::env::var("DAWESOME_TAPE_MODEL") {
      Ok(path) => (path.clone(), std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|text| read_model(&text))),
      Err(_) => ("the bundled network".to_string(), read_model(BUNDLED)),
    };
    match read {
      Ok(weights) => {
        println!("tape: neural, {} ({} units, {} Hz)", name, weights.hidden, weights.rate);
        Some(Arc::new(weights))
      }
      Err(e) => {
        println!("tape: couldn't read {}: {}", name, e);
        None
      }
    }
  }).clone()
}

// model.json (tools/tape-model's model.py, schema 1) as plain arrays
fn read_model(text: &str) -> Result<GruWeights, String> {
  let spec: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
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

// a track side's core: the network when it's chosen and can run at this
// rate, else the hand-built model (print.rs uses it too)
pub fn core(neural: bool, rate: f32) -> Box<dyn TapeCore> {
  if neural {
    match network().map(|weights| NeuralCore::new(weights, rate)) {
      Some(Ok(core)) => return Box::new(core),
      Some(Err(e)) => println!("tape: computer modeled instead of neural: {}", e),
      None => println!("tape: computer modeled instead of neural: no network"),
    }
  }
  Box::new(SimpleCore::new(SimpleCoreSettings::default(), rate, MAX_BLOCK))
}

pub fn build(shape: &Shape, settings: &TapeSettings) -> Box<EngineTape> {
  build_with(shape, settings, &|_| shape.live)
}

// the same, each track recorded live or not as `live` says (an export runs
// live the tracks that can't be printed yet: export.rs)
pub fn build_with(shape: &Shape, settings: &TapeSettings, live: &dyn Fn(u32) -> bool) -> Box<EngineTape> {
  let rate = shape.rate as f32;
  let mut machine = TapeMachine::new(machine_settings(settings), rate, SEED, MAX_BLOCK);
  let mut sides = Vec::with_capacity(shape.tracks.len());
  for &number in &shape.tracks {
    // live: the record side runs here; printed: it ran ahead (print.rs)
    let live = live(number);
    let mut side = || {
      let record = live.then(|| RecordSide::new(core(shape.neural, rate), rate, MAX_BLOCK));
      machine.add_track(record, 0.0)
    };
    let pair = [side(), side()];
    let drive = settings.track(number).drive_db;
    machine.set_drive(pair[0], drive);
    machine.set_drive(pair[1], drive);
    sides.push((number, pair, live));
  }
  EngineTape::new(machine, shape.before_effects, shape.rate, sides)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_a_trained_network_is_read_as_trained() {
    let weights = read_model(BUNDLED).unwrap();
    assert_eq!((weights.hidden, weights.rate), (32, 48_000));
    assert!(NeuralCore::new(Arc::new(weights.clone()), 48_000.0).is_ok());
    // at another rate it can't run yet (no resampling): the caller falls back
    assert!(NeuralCore::new(Arc::new(weights), 44_100.0).is_err());
    assert!(read_model("{}").is_err());
  }

  #[test]
  fn test_age_wears_the_tape() {
    use crate::daw::tape_settings::TapeMachine as Knobs;
    let at = |age: f32| machine_settings(&TapeSettings { machine: Knobs { age, ..Knobs::default() }, ..TapeSettings::default() });
    let (fresh, old) = (at(0.0), at(100.0));
    assert_eq!(fresh.playback, wear::fresh().playback);
    assert_eq!(old.playback, wear::ancient().playback);
    assert!(old.playback.hiss_db > fresh.playback.hiss_db);
    assert!(old.transport.stick_slips_per_minute > 0.0);
  }
}
