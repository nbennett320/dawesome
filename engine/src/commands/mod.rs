// the Tauri commands the frontend calls, a file for each part of the app
use std::sync::Arc;

use crate::daw;

// the DAW's state, as Tauri hands it to a command (Tauri state). only
// commands take it: everything else takes &daw::Session, which tests
// can make
pub type TState<'a> = tauri::State<'a, Arc<daw::Session>>;

pub(crate) mod appearance;
pub(crate) mod audio;
pub(crate) mod clip;
pub(crate) mod clipboard;
pub(crate) mod effect;
pub(crate) mod file_drop;
pub(crate) mod history;
pub(crate) mod mixdown;
pub(crate) mod mixer;
pub(crate) mod playlist;
pub(crate) mod project;
pub(crate) mod sample;
pub(crate) mod sequencer;
pub(crate) mod share;
pub(crate) mod synth;
pub(crate) mod tape;
pub(crate) mod transport;
