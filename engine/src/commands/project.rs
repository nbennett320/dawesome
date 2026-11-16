// saving, loading and starting projects (see project.rs)
use std::path::Path;

use crate::commands::TState;
use crate::daw::project::{apply, has_unsaved_changes, load_from, mark_unchanged, save_project_to, LoadedProject, Project, SavedProject};
use crate::daw::project_files;

// (async, like load_project: off the main thread, as it can take a while)
#[tauri::command(async)]
pub fn save_project(state: TState<'_>, path: String, somewhere_new: bool) -> Result<SavedProject, String> {
  save_project_to(&state, Path::new(&path), somewhere_new)
}

// the file a project chosen to be saved at `path` would be saved to
// somewhere new (in a folder of its own)
#[tauri::command]
pub fn project_file_for(path: String) -> String {
  project_files::project_file_for(Path::new(&path)).to_string_lossy().into_owned()
}

// (async: off the main thread, as decoding every sample takes a while)
#[tauri::command(async)]
pub fn load_project(state: TState<'_>, path: String) -> Result<LoadedProject, String> {
  let loaded = load_from(&state, Path::new(&path));
  // its armed tracks keep the input open, ready to record
  crate::daw::recording::hold_input_for_armed(&state);
  loaded
}

// start an empty project with the default settings
#[tauri::command]
pub fn new_project(state: TState<'_>) -> LoadedProject {
  let loaded = apply(&state, &Project::default(), None);
  state.project.lock().unwrap().reset(None);
  mark_unchanged(&state);
  // nothing's armed: the input can close
  crate::daw::recording::hold_input_for_armed(&state);
  loaded
}

// whether quitting, or replacing the project, would lose changes
#[tauri::command]
pub fn project_has_unsaved_changes(state: TState<'_>) -> bool {
  has_unsaved_changes(&state)
}
