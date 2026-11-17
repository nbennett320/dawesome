// dawesome's tape: what a cassette 4-track does to sound, on the way onto
// the tape (the record side) and off it again (the playback side). nothing
// here knows about Tauri, the mixer or projects: the DAW's glue
// (engine/src/daw/tape) puts tracks on tape, an insert effect can wrap one
// track of it, and a headless build (a pedal) could use it alone. its
// only dependency is dsp (filters, delay line, smoothing), which has none:
// the network's weights come in as plain arrays, and reading model.json is
// the glue's job
//
// the plan, and why it's split this way: context/tape-project/02-plan.md
// ("Architecture: one crate, three uses")
//
// nothing here allocates once it's set up: every buffer is made in new()

// the record side: what goes onto the tape
pub mod core;
pub mod dbx;
pub mod gru;
pub mod oversample;
// the playback side: what comes off it
pub mod playback;
pub mod transport;
// a tape: one transport and the tracks on it
pub mod machine;
// how a tape wears: the Age knob
pub mod wear;

pub use core::{NeuralCore, SimpleCore, SimpleCoreSettings, TapeCore};
pub use dbx::{DbxDecoder, DbxEncoder};
pub use gru::{Gru, GruError, GruWeights};
pub use machine::{MachineSettings, RecordSide, TapeMachine};
pub use playback::{Playback, PlaybackSettings};
pub use transport::{TapeTransport, TransportSettings};
pub use wear::Wear;
