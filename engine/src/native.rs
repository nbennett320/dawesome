// the built-in effects (crates/dsp), as the racks hold them: each one is a
// Hosted like a VST3 plugin, with the id "dawesome:<its id>", so racks,
// bypass, undo, saving, mixdowns and resting treat it as they do any
// plugin. what differs is here:
// - it's loaded from dsp's registry, not a bundle (effects.rs sends ids
//   starting "dawesome:" here, whatever loader the racks use)
// - the catalog lists them first, with no path, so they're there before a
//   scan, and with nothing installed
// - its settings are saved as JSON: each parameter's plain value by its key
//   (so a range that changes later doesn't change how a project sounds)
// - it has no window of its own: the app draws its controls from its
//   parameters (get_effect_params, set_effect_param)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use dsp::{Entry, Kind, Runner, Store, Taper};

use crate::effects::Hosted;
use crate::plugin::{self, Event, PluginInfo, PluginKind, Processor, Transport};

pub const PREFIX: &str = "dawesome:";
pub const VENDOR: &str = "Dawesome";

pub fn is_native(plugin_id: &str) -> bool {
  plugin_id.starts_with(PREFIX)
}

fn entry(plugin_id: &str) -> Option<&'static Entry> {
  dsp::find(plugin_id.strip_prefix(PREFIX)?)
}

fn info_of(entry: &Entry) -> PluginInfo {
  PluginInfo {
    id: format!("{}{}", PREFIX, entry.info.id),
    path: String::new(),
    name: entry.info.name.into(),
    vendor: VENDOR.into(),
    version: env!("CARGO_PKG_VERSION").into(),
    kind: PluginKind::Effect,
    category: entry.info.category.into(),
    subcategories: Vec::new(),
  }
}

// every built-in effect, as the catalog lists them
pub fn catalog() -> Vec<PluginInfo> {
  dsp::REGISTRY.iter().map(|e| info_of(e)).collect()
}

pub fn find(plugin_id: &str) -> Option<PluginInfo> {
  entry(plugin_id).map(info_of)
}

// a new one, at its defaults
pub fn load(plugin_id: &str, rate: u32) -> Result<Box<dyn Hosted>, String> {
  let entry = entry(plugin_id)
    .ok_or_else(|| "it's a built-in effect this version of Dawesome doesn't have. A newer version may".to_string())?;
  let store = Store::new(entry);
  let processor = NativeProcessor(Runner::new(store.clone(), rate as f32));
  Ok(Box::new(Native { store, processor: Some(Box::new(processor)) }))
}

// ---- the main-thread half

pub struct Native {
  store: Arc<Store>,
  // until the racks take it for a chain
  processor: Option<Box<dyn Processor>>,
}

// a built-in effect's settings, as a project saves them
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Saved {
  // its id among the built-ins ("eq")
  effect: String,
  version: u32,
  // plain values (Hz, dB, ...) by key
  params: BTreeMap<String, f32>,
}

impl Native {
  #[cfg(test)]
  pub fn store(&self) -> &Arc<Store> {
    &self.store
  }

  // its parameters, and their values now
  pub fn params(&self) -> EffectParams {
    let entry = self.store.entry();
    EffectParams {
      effect: entry.info.id.into(),
      params: entry.params.iter().enumerate().map(|(id, p)| ParamWire {
        id: id as u32,
        key: p.key.into(),
        name: p.name.into(),
        group: p.group.into(),
        unit: p.unit.label().into(),
        min: p.min,
        max: p.max,
        default: p.default,
        default_normalized: p.normalize(p.default),
        taper: match p.taper { Taper::Linear => "linear", Taper::Log => "log" }.into(),
        step_count: p.step_count(),
        labels: match p.kind { Kind::Choice(labels) => labels.iter().map(|l| l.to_string()).collect(), _ => Vec::new() },
        value: self.value(id),
      }).collect(),
    }
  }

  fn value(&self, id: usize) -> ParamValue {
    let plain = self.store.plain(id);
    ParamValue {
      id: id as u32,
      value: plain,
      normalized: self.store.get(id),
      text: self.store.entry().params[id].format(plain),
    }
  }

  // every parameter's plain value, by id
  pub fn values(&self) -> Vec<f32> {
    self.store.plains()
  }

  // set every parameter's plain value (undo and redo)
  pub fn set_all(&self, values: &[f32]) {
    self.store.set_plains(values);
  }

  // set a parameter, by its normalized value: what it became
  pub fn set(&self, id: u32, normalized: f64) -> Result<ParamValue, String> {
    self.store.set(id as usize, normalized as f32).ok_or_else(no_such_param)?;
    Ok(self.value(id as usize))
  }

  // set a parameter from what was typed ("2.5k", "-3 dB", "Notch")
  pub fn set_text(&self, id: u32, text: &str) -> Result<ParamValue, String> {
    let desc = self.store.entry().params.get(id as usize).ok_or_else(no_such_param)?;
    let plain = desc.parse(text).ok_or_else(|| format!("\"{}\" isn't a value {} can have", text.trim(), desc.name))?;
    self.store.set_plain(id as usize, plain);
    Ok(self.value(id as usize))
  }
}

// what an analyzer draws: the effect's output now, in dB at log-spaced
// frequencies from 20 Hz to 20 kHz
pub const SPECTRUM_BINS: usize = 256;

impl Native {
  // its output's spectrum, taken now (on whichever thread asks: never the
  // audio thread's). none when the copy was overtaken: ask again
  pub fn spectrum(&self) -> Option<EffectSpectrum> {
    let tap = self.store.tap();
    let (samples, written) = tap.latest(dsp::tap::WINDOW)?;
    let rate = tap.rate();
    let levels = if rate == 0 {
      vec![dsp::spectrum::FLOOR; SPECTRUM_BINS]
    } else {
      dsp::spectrum::spectrum(&samples, rate as f32, SPECTRUM_BINS, 20., 20_000.)
    };
    Some(EffectSpectrum { written, rate, levels })
  }
}

fn no_such_param() -> String {
  "That effect has no such parameter".into()
}

impl Hosted for Native {
  fn take_processor(&mut self) -> Option<Box<dyn Processor>> {
    self.processor.take()
  }

  fn state(&self) -> Result<Vec<u8>, String> {
    let entry = self.store.entry();
    let params = entry.params.iter().zip(self.store.plains())
      .map(|(p, value)| (p.key.to_string(), value))
      .collect();
    let saved = Saved { effect: entry.info.id.into(), version: entry.info.version, params };
    serde_json::to_vec(&saved).map_err(|e| e.to_string())
  }

  // keys it doesn't know (from a newer version) are left out, and its own
  // that aren't there (an older one's) are at their defaults
  fn set_state(&mut self, blob: &[u8]) -> Result<(), String> {
    let saved: Saved = serde_json::from_slice(blob).map_err(|e| format!("its settings can't be read ({})", e))?;
    let entry = self.store.entry();
    if saved.effect != entry.info.id {
      return Err(format!("those are {}'s settings, not {}'s", saved.effect, entry.info.name));
    }
    let plains: Vec<f32> = entry.params.iter()
      .map(|p| saved.params.get(p.key).copied().unwrap_or(p.default))
      .collect();
    self.store.set_plains(&plains);
    Ok(())
  }

  // (its processor follows the rate each block is played at)
  fn set_sample_rate(&mut self, _rate: u32) -> Result<(), String> {
    Ok(())
  }

  // its tab shows its controls
  fn has_editor(&self) -> bool {
    true
  }

  fn idle(&mut self) {}

  fn editor_open(&self) -> bool {
    false
  }

  fn plugin(&mut self) -> Option<&mut plugin::Plugin> {
    None
  }

  fn native(&self) -> Option<&Native> {
    Some(self)
  }
}

// ---- the audio-thread half

struct NativeProcessor(Runner);

impl Processor for NativeProcessor {
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[Event], transport: &Transport) {
    self.0.process(input, output, transport);
  }

  fn tail_frames(&self) -> Option<u64> {
    self.0.tail_frames()
  }

  fn has_pending(&self) -> bool {
    self.0.has_pending()
  }
}

// ---- for the frontend (see context/effects-suite-project/04-api.md)

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EffectParams {
  // its id among the built-ins ("eq"): which panel the app draws
  pub effect: String,
  pub params: Vec<ParamWire>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParamWire {
  // its place in the list: what set_effect_param takes
  pub id: u32,
  pub key: String,
  pub name: String,
  pub group: String,
  // "Hz", "dB", "%", "ms", "beats" or ""
  pub unit: String,
  pub min: f32,
  pub max: f32,
  pub default: f32,
  pub default_normalized: f32,
  // "linear" or "log"
  pub taper: String,
  // 0 continuous, 1 a switch, else a choice of step_count + 1 labels
  pub step_count: i32,
  pub labels: Vec<String>,
  pub value: ParamValue,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EffectSpectrum {
  // frames the effect has put out so far: when it stops going up, the
  // effect isn't being run (resting after silence, or bypassed) and the
  // analyzer lets its picture fall away
  pub written: u64,
  pub rate: u32,
  // dB (a full-scale sine is 0, silence -120), SPECTRUM_BINS of them, from
  // 20 Hz to 20 kHz evenly by octave
  pub levels: Vec<f32>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParamValue {
  pub id: u32,
  // plain (Hz, dB, a choice's index)
  pub value: f32,
  pub normalized: f32,
  // as it's shown ("2.50 kHz")
  pub text: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  fn eq() -> Box<dyn Hosted> {
    load("dawesome:eq", 48_000).unwrap()
  }

  #[test]
  fn test_the_catalog_lists_the_built_ins() {
    let catalog = catalog();
    let eq = catalog.iter().find(|p| p.id == "dawesome:eq").unwrap();
    assert_eq!(eq.name, "EQ");
    assert_eq!(eq.vendor, "Dawesome");
    assert_eq!(eq.kind, PluginKind::Effect);
    assert_eq!(eq.category, "EQ");
    assert_eq!(find("dawesome:eq").as_ref(), Some(eq));
    assert!(find("dawesome:nothing").is_none());
    assert!(load("dawesome:nothing", 48_000).is_err());
  }

  #[test]
  fn test_settings_are_saved_by_key_and_come_back() {
    let a = eq();
    let native = a.native().unwrap();
    let freq = native.params().params.iter().find(|p| p.key == "band3.freq").unwrap().id;
    let set = native.set_text(freq, "2.5k").unwrap();
    assert_eq!(set.text, "2.50 kHz");
    let blob = a.state().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&blob).unwrap();
    assert_eq!(json["effect"], "eq");
    assert_eq!(json["version"], 1);
    assert_eq!(json["params"]["band3.freq"], 2500.0);

    let mut b = eq();
    b.set_state(&blob).unwrap();
    assert_eq!(b.native().unwrap().store().plain(freq as usize), 2500.);
  }

  #[test]
  fn test_saved_settings_from_another_version_load_what_they_can() {
    let mut a = eq();
    let blob = br#"{"effect":"eq","version":9,"params":{"band1.gain":-6,"band1.sparkle":1}}"#;
    a.set_state(blob).unwrap();
    let params = a.native().unwrap().params();
    let value = |key: &str| params.params.iter().find(|p| p.key == key).unwrap().value.value;
    assert_eq!(value("band1.gain"), -6.);
    // left at its default
    assert_eq!(value("band1.freq"), 50.);
  }

  #[test]
  fn test_settings_that_arent_its_own_are_refused() {
    let mut a = eq();
    assert!(a.set_state(br#"{"effect":"reverb","version":1,"params":{}}"#).is_err());
    assert!(a.set_state(b"not json").is_err());
  }

  #[test]
  fn test_parameters_are_described_for_the_app() {
    let a = eq();
    let params = a.native().unwrap().params();
    assert_eq!(params.effect, "eq");
    let shape = params.params.iter().find(|p| p.key == "band1.shape").unwrap();
    assert_eq!(shape.step_count, 6);
    assert_eq!(shape.labels[3], "Low cut");
    let freq = params.params.iter().find(|p| p.key == "band1.freq").unwrap();
    assert_eq!((freq.unit.as_str(), freq.taper.as_str()), ("Hz", "log"));
    let json = serde_json::to_value(&params).unwrap();
    assert!(json["params"][0]["stepCount"].is_number());
    assert!(json["params"][0]["value"]["normalized"].is_number());
    assert!(a.native().unwrap().set(9999, 0.5).is_err());
    assert!(a.native().unwrap().set_text(freq.id, "loud").is_err());
  }

  #[test]
  fn test_its_spectrum_shows_what_it_puts_out() {
    let mut a = eq();
    let mut processor = a.take_processor().unwrap();
    let before = a.native().unwrap().spectrum().unwrap();
    assert_eq!((before.written, before.rate), (0, 0));
    assert!(before.levels.iter().all(|l| *l == dsp::spectrum::FLOOR));

    let rate = 48_000;
    let transport = Transport { sample_rate: rate, ..Transport::default() };
    let sine: Vec<f32> = (0..8192).map(|i| (std::f32::consts::TAU * 1000. * i as f32 / rate as f32).sin()).collect();
    for block in sine.chunks(512) {
      let (mut l, mut r) = (vec![0f32; block.len()], vec![0f32; block.len()]);
      processor.process([block, block], [&mut l, &mut r], &[], &transport);
    }
    let after = a.native().unwrap().spectrum().unwrap();
    assert_eq!((after.written, after.rate, after.levels.len()), (8192, rate, SPECTRUM_BINS));
    let peak = after.levels.iter().cloned().fold(f32::MIN, f32::max);
    let at = after.levels.iter().position(|l| *l == peak).unwrap();
    let freq = 20. * 1000f32.powf(at as f32 / (SPECTRUM_BINS - 1) as f32);
    assert!((freq / 1000.).log2().abs() < 0.1, "the peak's at {} Hz", freq);
    assert!(peak > -1.5, "{}", peak);
  }

  #[test]
  fn test_its_processor_hears_a_change_at_its_next_block() {
    let mut a = eq();
    let mut processor = a.take_processor().unwrap();
    assert!(!processor.has_pending());
    a.native().unwrap().set(40, 0.).unwrap();
    assert!(processor.has_pending());
    let input = [0.5f32; 64];
    let (mut l, mut r) = ([0f32; 64], [0f32; 64]);
    processor.process([&input, &input], [&mut l, &mut r], &[], &Transport { sample_rate: 48_000, ..Transport::default() });
    assert!(!processor.has_pending());
  }
}
