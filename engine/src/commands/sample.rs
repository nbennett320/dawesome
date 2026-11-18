use std::path::Path;

use crate::daw::{audiograph, library, playback, project_files};
use crate::app;
use crate::commands::TState;
use crate::preferences::AppPreferences;

// every folder of samples, as File Paths and the Samples browser list it:
// the ones Dawesome comes with, the user's, and the open project's
fn listed_sample_paths(app: &tauri::AppHandle, state: &TState<'_>, prefs: &AppPreferences) -> Vec<library::SamplePath> {
  let bundled = app::env::sample_browser_root(app).ok();
  let project = state.project.lock().unwrap().path.as_deref().map(project_files::samples_dir_for);
  library::sample_paths(bundled.as_deref(), &prefs.current().sample_paths, project.as_deref())
}

#[tauri::command]
pub fn sample_paths(
  app: tauri::AppHandle,
  state: TState<'_>,
  prefs: tauri::State<'_, AppPreferences>,
) -> Vec<library::SamplePath> {
  listed_sample_paths(&app, &state, &prefs)
}

// add a folder of samples (File Paths' Add Folder…), returning the list
#[tauri::command]
pub fn add_sample_path(
  app: tauri::AppHandle,
  state: TState<'_>,
  prefs: tauri::State<'_, AppPreferences>,
  path: String,
) -> Result<Vec<library::SamplePath>, String> {
  let user = library::with_added(&prefs.current().sample_paths, Path::new(&path))?;
  prefs.set_sample_paths(user)?;
  Ok(listed_sample_paths(&app, &state, &prefs))
}

// take a folder out of File Paths (nothing on disk changes), returning the
// list
#[tauri::command]
pub fn remove_sample_path(
  app: tauri::AppHandle,
  state: TState<'_>,
  prefs: tauri::State<'_, AppPreferences>,
  path: String,
) -> Result<Vec<library::SamplePath>, String> {
  let user = library::without(&prefs.current().sample_paths, &path);
  prefs.set_sample_paths(user)?;
  Ok(listed_sample_paths(&app, &state, &prefs))
}

// (async: off the main thread, as listing a folder can take a while)
#[tauri::command(async)]
pub fn enumerate_directory(dir_path: String) -> Result<(
  Vec<String>, 
  Vec<String>, 
  Vec<String>, 
  Vec<String>
), String> {
  let (samples, samples_paths, dirs, dirs_paths) = 
    app::env::enumerate_files_in_dir(dir_path);
  Ok((samples, samples_paths, dirs, dirs_paths))
}

// play a sample to hear it, returning how long it plays (seconds), so the
// sidebar can show its stop button for that long
// (async: off the main thread, as decoding the sample and opening the
// output, for the first sound, take a moment)
#[tauri::command(async)]
pub fn preview_sample(path: String) -> Result<f64, String> {
  playback::play_sample(&path).map(|length| length.as_secs_f64())
}

// stop every sample or clip being previewed
// (async: if the output's being opened, stopping waits for it, and the
// window shouldn't)
#[tauri::command(async)]
pub fn stop_previews() -> Result<(), String> {
  playback::stop_previews()
}

// waveform data and duration (seconds) of a sample that isn't in the
// playlist yet, in the same shape as get_node_data
// used to draw a clip while it's dragged in from the sidebar
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn get_sample_preview(
  sample_path: String,
) -> Result<(String, f32), String> {
  let (dur, waveform) = audiograph::preview_sample(&sample_path)?;

  Ok((waveform, dur.as_secs_f32()))
}
