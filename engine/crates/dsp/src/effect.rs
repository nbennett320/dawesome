// what a built-in effect is: an Entry (its name, its parameters, and how to
// make one) and the Effect the audio thread runs
//
// an effect only makes sound. its parameters are held and smoothed by the
// Runner that calls it, which hands it their plain values (Hz, dB, ...) as
// they are for each call. while one is moving, the Runner calls it every
// few frames (see runner.rs), so an effect just works out what it needs
// from the values it's given, each call

use crate::param::ParamDesc;
use crate::readout::READOUT;
use crate::transport::Transport;

// the effect's sound: runs on the audio thread, so it mustn't lock,
// allocate, or touch the disk
pub trait Effect: Send {
  // the sample rate changed (the effect is reset after)
  fn set_sample_rate(&mut self, rate: f32);

  // forget all sound it holds (filters' memory, delay lines, tails)
  fn reset(&mut self);

  // one stretch of frames. `io`'s slices are all the same length (at least
  // one frame); `params` holds every parameter's plain value, by id
  fn process(&mut self, io: Io, params: &Values, transport: &Transport);

  // how long it can go on sounding after its input stops, in frames, set as
  // `params` says: none if it never stops. the engine doesn't rest a chain
  // until it's been silent this long
  fn tail_frames(&self, params: &Values, rate: f32) -> Option<u64>;

  // what it's doing now, for its panel (readout.rs): called after each
  // block, `out` all 0s. each effect says what its slots hold. most have
  // nothing to say
  fn readout(&self, _out: &mut [f32; READOUT]) {}
}

// a stretch of sound in and out, stereo
pub struct Io<'a> {
  pub input: [&'a [f32]; 2],
  pub output: [&'a mut [f32]; 2],
  // another channel's sound, keying it (a compressor's). always none for
  // now: kept here so effects don't change shape when it comes
  pub sidechain: Option<[&'a [f32]; 2]>,
}

impl Io<'_> {
  pub fn frames(&self) -> usize {
    self.input[0].len()
  }
}

// every parameter's plain value, by id (its place in the effect's list)
#[derive(Clone, Copy)]
pub struct Values<'a>(pub &'a [f32]);

impl Values<'_> {
  pub fn get(&self, id: usize) -> f32 {
    self.0[id]
  }

  pub fn on(&self, id: usize) -> bool {
    self.0[id] >= 0.5
  }

  pub fn choice(&self, id: usize) -> usize {
    self.0[id].max(0.) as usize
  }
}

// what the app lists it as
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectInfo {
  // its id among the built-ins, e.g. "eq" (projects save it as
  // "dawesome:eq"). never changes
  pub id: &'static str,
  pub name: &'static str,
  // where the browser files it: EQ, Dynamics, Reverb, Delay, Modulation
  pub category: &'static str,
  // its saved settings' version: raise it when what a saved value means
  // changes (so an older one can be read differently)
  pub version: u32,
}

// a built-in effect, as REGISTRY lists it
pub struct Entry {
  pub info: EffectInfo,
  // never reordered or taken from: only added to at the end (see param.rs)
  pub params: &'static [ParamDesc],
  // a new one at this sample rate
  pub make: fn(rate: f32) -> Box<dyn Effect>,
}

impl Entry {
  // every parameter's default, by id
  pub fn defaults(&self) -> Vec<f32> {
    self.params.iter().map(|p| p.default).collect()
  }

  // a parameter's id, by its saved key
  pub fn id_of(&self, key: &str) -> Option<usize> {
    self.params.iter().position(|p| p.key == key)
  }
}
