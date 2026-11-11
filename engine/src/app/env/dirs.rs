use std::vec::{Vec};
use std::path::{Path, PathBuf};
use std::fs::{
  self,
  DirEntry,
};
use tauri::Manager;

// where the sample browser starts. release builds list the samples bundled
// into the app (assets/dawesome, see resources in tauri.conf.json); debug
// builds list everything in ./assets
pub fn sample_browser_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
  if cfg!(debug_assertions) {
    return Ok(PathBuf::from("./assets/"));
  }
  app.path().resource_dir()
    .map(|dir| dir.join("samples"))
    .map_err(|e| format!("could not find the app's resources: {}", e))
}

pub fn get_sample_browser_root(root: &Path) -> Result<(Vec<String>, Vec<String>, Vec<String>, Vec<String>), String> {
  let entries: Vec<DirEntry> = fs::read_dir(root)
    .map_err(|e| format!("could not list {}: {}", root.display(), e))?
    .filter_map(|e| e.ok())
    .filter(|e| !is_hidden(e))
    .collect();
  let mut dirs: Vec<String> = Vec::new();
  let mut dirs_paths: Vec<String> = Vec::new();
  let mut samples: Vec<String> = Vec::new();
  let mut samples_paths: Vec<String> = Vec::new();

  for entry in entries {
    if entry.path().is_dir() {
      let val = String::from(entry.path().to_str().unwrap());
      let path = String::from(entry.path().to_str().unwrap());
      dirs.push(val);
      dirs_paths.push(path);
    } else {
      let val = String::from(entry.path().file_name().unwrap().to_str().unwrap());
      let path = String::from(entry.path().to_str().unwrap());
      samples.push(val);
      samples_paths.push(path);
    }
  }

  Ok((samples, samples_paths, dirs, dirs_paths))
}

pub fn enumerate_files_in_dir(dir_path: String) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
  let entries: Vec<DirEntry> = fs::read_dir(dir_path)
    .unwrap()
    .map(|e| { e.unwrap() })
    .filter(|e| !is_hidden(e))
    .collect();
  let mut dirs: Vec<String> = Vec::new();
  let mut dirs_paths: Vec<String> = Vec::new();
  let mut samples: Vec<String> = Vec::new();
  let mut samples_paths: Vec<String> = Vec::new();

  for entry in entries {
    if entry.path().is_dir() {
      let val = String::from(entry.path().to_str().unwrap());
      let path = String::from(entry.path().to_str().unwrap());
      dirs.push(val);
      dirs_paths.push(path);
    } else {
      let val = String::from(entry.path().file_name().unwrap().to_str().unwrap());
      let path = String::from(entry.path().to_str().unwrap());
      samples.push(val);
      samples_paths.push(path);
    }
  }

  (samples, samples_paths, dirs, dirs_paths)
}

// dotfiles like .DS_Store aren't samples
fn is_hidden(entry: &DirEntry) -> bool {
  entry.file_name().to_string_lossy().starts_with('.')
}
