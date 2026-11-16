// the Mixdown tab's commands (see actions/mixdown.rs), and showing the file
// in the Finder
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::actions::mixdown::{defaults_for, start, MixdownDefaults, MixdownOptions, Mixdowns, Status};
use crate::commands::TState;
use crate::daw;

pub type MixdownState<'a> = tauri::State<'a, Arc<Mixdowns>>;

#[tauri::command]
pub fn mixdown_defaults(state: TState<'_>) -> MixdownDefaults {
  defaults_for(state.project.lock().unwrap().path.as_deref())
}

// start a mixdown of the song as it is now. anything that stops it before
// it starts (nothing to mix, no name, a folder that can't be written to) is
// the error; anything after, the status says
// (off the main thread: it writes to the disk)
#[tauri::command(async)]
pub fn start_mixdown(state: TState<'_>, mixdowns: MixdownState<'_>, options: MixdownOptions) -> Result<(), String> {
  start(&state, mixdowns.inner(), options, daw::open_engine_rate().unwrap_or(44_100)).map(|_| ())
}

#[tauri::command]
pub fn mixdown_status(mixdowns: MixdownState<'_>) -> Status {
  mixdowns.status()
}

// stop the mixdown going, if there is one. nothing of it is kept
#[tauri::command]
pub fn cancel_mixdown(mixdowns: MixdownState<'_>) {
  mixdowns.cancel.store(true, Ordering::Relaxed);
}

// show a file in the Finder (or the file manager), selected
#[tauri::command]
pub fn reveal_file(path: String) -> Result<(), String> {
  let path = PathBuf::from(path);
  if !path.exists() { return Err(format!("{} isn't there any more", path.display())); }

  #[cfg(target_os = "macos")]
  let command = std::process::Command::new("open").arg("-R").arg(&path).spawn();
  #[cfg(target_os = "windows")]
  let command = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
  #[cfg(not(any(target_os = "macos", target_os = "windows")))]
  let command = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(&path)).spawn();

  command.map(|_| ()).map_err(|e| e.to_string())
}
