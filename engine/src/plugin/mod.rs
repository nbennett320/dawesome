// plugins: effects and instruments that run in a mixer channel's chain
//
// a plugin has two halves. its Processor (processor.rs) runs on the
// audio thread, a block at a time, and nothing else touches it. the rest (its parameters, its saved
// state, its window) stays with whoever loaded it
//
// vst3 is the VST3 host (a plugin's module, its two halves, its window),
// scan finds the plugins installed on this machine, and window puts a
// plugin's editor on screen (a floating window, or docked over the page)

// (a plugin keeps some of the host's objects only to keep them alive)
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub mod scan;
pub mod vst3;
pub mod window;

#[allow(unused_imports)]
pub use self::vst3::Plugin;

// what kind of plugin a class is
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
  Effect,
  Instrument,
}

// one plugin found on this machine, as the browser lists it and a project
// refers to it
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
  // "vst3:" and the 32 hex digits of its class id. what projects save
  pub id: String,
  // the bundle it's in
  pub path: String,
  pub name: String,
  pub vendor: String,
  pub version: String,
  pub kind: PluginKind,
  // where the browser files it: Dynamics, EQ, Reverb, ... for effects, and
  // Synth, Sampler, Drum, Piano or Other for instruments
  pub category: String,
  // the plugin's own VST3 subcategories ("Fx", "Dynamics", ...)
  pub subcategories: Vec<String>,
}

// one of a plugin's parameters. values are normalized (0 to 1)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParamInfo {
  pub id: u32,
  pub name: String,
  pub units: String,
  pub default: f64,
  // 0 for a continuous parameter, otherwise the number of steps (1 for a
  // switch)
  pub step_count: i32,
  pub automatable: bool,
  pub read_only: bool,
  // the plugin's own bypass switch
  pub bypass: bool,
  // not meant to be shown
  pub hidden: bool,
}
