use crate::daw::{audiograph, playback};
use crate::app;

// (async: off the main thread, as listing a folder can take a while)
#[tauri::command(async)]
pub fn get_sidebar_samples(app: tauri::AppHandle) -> Result<(
  Vec<String>,
  Vec<String>,
  Vec<String>,
  Vec<String>
), String> {
  let root = app::env::sample_browser_root(&app)?;
  app::env::get_sample_browser_root(&root)
}

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
