// plugins: effects and instruments that run in a mixer channel's chain
//
// a plugin has two halves. its Processor runs on the audio thread, a block at
// a time, and nothing else touches it. the rest (its parameters, its saved
// state, its window) stays with whoever loaded it
//
// vst3 is the VST3 host (a plugin's module, its two halves, its window), and
// scan finds the plugins installed on this machine

// (a plugin keeps some of the host's objects only to keep them alive)
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub mod scan;
pub mod vst3;

#[allow(unused_imports)]
pub use self::vst3::Plugin;

// the most frames a processor is ever given at once
pub const MAX_BLOCK: usize = 4096;

// what a processor is told about the moment it's playing
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transport {
  // whether a run is playing (chains are processed while stopped too)
  pub playing: bool,
  // beats a minute
  pub tempo: f64,
  // the time signature
  pub beats_per_bar: u16,
  pub beat_unit: u16,
  // where the block starts in the song, in frames. while stopped it stays
  // where the last run got to
  pub position: i64,
  pub sample_rate: u32,
}

impl Default for Transport {
  fn default() -> Self {
    Transport {
      playing: false,
      tempo: 120.,
      beats_per_bar: 4,
      beat_unit: 4,
      position: 0,
      sample_rate: 44_100,
    }
  }
}

// something for an instrument to play, `offset` frames into the block
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
  // velocity from 0 to 1
  NoteOn { offset: u32, pitch: u8, velocity: f32 },
  NoteOff { offset: u32, pitch: u8 },
}

impl Event {
  pub fn offset(&self) -> u32 {
    match *self {
      Event::NoteOn { offset, .. } | Event::NoteOff { offset, .. } => offset,
    }
  }
}

// the half of a plugin the audio thread runs
pub trait Processor: Send {
  // run one block: stereo in (left, right), stereo out, all four the same
  // length (at most MAX_BLOCK). `output` holds whatever was there before, and
  // must be filled. `events` are in order of their offset
  //
  // called on the audio thread: it mustn't lock, allocate, or touch the disk
  fn process(
    &mut self,
    input: [&[f32]; 2],
    output: [&mut [f32]; 2],
    events: &[Event],
    transport: &Transport,
  );
}

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
