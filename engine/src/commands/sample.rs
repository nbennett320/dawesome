use std::thread;

use crate::{app, daw};

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

#[tauri::command]
pub fn preview_sample(path: String) {
  // the first sound opens the output, which takes a moment
  thread::spawn(move || {
    if let Err(e) = daw::play_sample(&path) {
      println!("could not preview sample: {}", e);
    }
  });
}

// waveform data and duration (seconds) of a sample that isn't in the
// playlist yet, in the same shape as get_node_data
// used to draw a clip while it's dragged in from the sidebar
// (async: off the main thread, as decoding the sample takes a while)
#[tauri::command(async)]
pub fn get_sample_preview(
  sample_path: String,
) -> Result<(String, f32), String> {
  let (dur, waveform) = daw::preview_sample(&sample_path)?;

  Ok((waveform, dur.as_secs_f32()))
}
