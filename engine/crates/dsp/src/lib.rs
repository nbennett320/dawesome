// dawesome's built-in effects: what they do to sound, and the parameters
// they're set with. nothing here knows about Tauri, VST3, the mixer or
// projects: the engine's glue (engine/src/native.rs) puts these effects in
// mixer channels' chains, and a headless build (a pedal) could use them
// without it. no dependencies, on purpose: the filters, smoothing and the
// rest are short, and the crate builds anywhere
//
// to add an effect (see README.md):
// - write it in effects/, implementing Effect, with its parameters listed
//   once as a static slice of ParamDesc
// - give it an Entry and add that to REGISTRY (effects/mod.rs)
// the conformance suite (testing.rs) then checks it with no more code, and
// the app lists it, saves it, and draws its controls from its parameters
//
// parameters belong to the wrapper (Runner, in runner.rs), not the effect:
// it holds their values, smooths them, and calls the effect with values
// that hold still for each call

pub mod delay_line;
pub mod effect;
pub mod effects;
pub mod fft;
pub mod param;
pub mod runner;
pub mod smooth;
pub mod spectrum;
pub mod svf;
pub mod tap;
pub mod transport;

#[cfg(test)]
mod testing;

pub use effect::{Effect, EffectInfo, Entry, Io, Values};
pub use effects::{find, REGISTRY};
pub use param::{Kind, ParamDesc, Taper, Unit};
pub use runner::{Runner, Store};
pub use tap::Tap;
pub use transport::Transport;
