// project files opened from outside the app: double-clicked in Finder, or
// dropped on its Dock icon (macOS sends them as RunEvent::Opened). the
// frontend opens them as File › Open would (see openProjectFile in
// src/state/slices/projectSlice.ts)
//
// a file that opens the app arrives before the window is ready to hear
// about it, so it's held until the frontend asks (opened_files). after
// that, files are sent as they come ("open-file")
use std::path::Path;
use std::sync::Mutex;

use tauri::{Emitter, Manager};

use crate::daw::{project_files, share};

#[derive(Default)]
pub struct OpenedFiles {
  // files held for the frontend, and whether it's asked yet
  held: Mutex<(Vec<String>, bool)>,
}

// whether a path is a project or a shareable project
fn is_project(path: &Path) -> bool {
  path.extension().is_some_and(|ext| {
    ext.eq_ignore_ascii_case(project_files::PROJECT_EXTENSION) || ext.eq_ignore_ascii_case(share::SHARE_EXTENSION)
  })
}

impl OpenedFiles {
  // files the system asked the app to open: held, or sent to the frontend
  pub fn opened(&self, app: &tauri::AppHandle, paths: impl IntoIterator<Item = std::path::PathBuf>) {
    let paths: Vec<String> = paths.into_iter()
      .filter(|path| is_project(path))
      .map(|path| path.to_string_lossy().into_owned())
      .collect();
    let mut held = self.held.lock().unwrap();
    if held.1 {
      drop(held);
      for path in paths {
        let _ = app.emit("open-file", path);
      }
      // brought forward, as the file was opened
      if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
      }
    } else {
      held.0.extend(paths);
    }
  }

  // the files held for the frontend; from now on they're sent
  pub fn take(&self) -> Vec<String> {
    let mut held = self.held.lock().unwrap();
    held.1 = true;
    std::mem::take(&mut held.0)
  }
}

// the files opened before the frontend was ready (the frontend asks once,
// as it starts)
#[tauri::command]
pub fn opened_files(opened: tauri::State<'_, OpenedFiles>) -> Vec<String> {
  opened.take()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_only_projects_are_opened() {
    assert!(is_project(Path::new("/a/song.dawesome")));
    assert!(is_project(Path::new("/a/song.SHAREDAWESOME")));
    assert!(!is_project(Path::new("/a/kick.wav")));
    assert!(!is_project(Path::new("/a/song")));
  }

  #[test]
  fn test_files_are_held_until_asked_for() {
    let opened = OpenedFiles::default();
    opened.held.lock().unwrap().0.push("/a/song.dawesome".into());
    assert_eq!(opened.take(), vec!["/a/song.dawesome".to_string()]);
    assert_eq!(opened.take(), Vec::<String>::new());
    assert!(opened.held.lock().unwrap().1);
  }
}
