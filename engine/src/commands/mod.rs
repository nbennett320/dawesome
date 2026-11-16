// the Tauri commands the frontend calls, a file for each part of the app
use std::sync::Arc;

use crate::daw;

// the DAW's state, as Tauri hands it to a command (Tauri state). only
// commands take it: everything else takes &daw::InnerState, which tests
// can make
pub type TState<'a> = tauri::State<'a, Arc<daw::InnerState>>;

pub(crate) mod appearance;
pub(crate) mod audio;
pub(crate) mod clip;
pub(crate) mod clipboard;
pub(crate) mod effect;
pub(crate) mod history;
pub(crate) mod mixdown;
pub(crate) mod mixer;
pub(crate) mod playlist;
pub(crate) mod project;
pub(crate) mod sample;
pub(crate) mod sequencer;
pub(crate) mod synth;
pub(crate) mod transport;
