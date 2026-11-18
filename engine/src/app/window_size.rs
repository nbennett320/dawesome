// the main window's size, kept between launches in window.json beside the
// preferences (not in them: the frontend saves those, and would write over
// it). logical pixels, so a retina screen doesn't double it each launch
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{PhysicalSize, Window};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct WindowSize {
  pub width: f64,
  pub height: f64,
}

// sizes outside these are a broken file, not a window
const SMALLEST: f64 = 200.;
const LARGEST: f64 = 20_000.;

impl WindowSize {
  fn usable(self) -> Option<Self> {
    let ok = |n: f64| n.is_finite() && (SMALLEST..=LARGEST).contains(&n);
    (ok(self.width) && ok(self.height)).then_some(self)
  }

  // the saved size, or none when there isn't a usable one
  pub fn load(file: &Path) -> Option<Self> {
    fs::read_to_string(file)
      .ok()
      .and_then(|json| serde_json::from_str::<WindowSize>(&json).ok())
      .and_then(WindowSize::usable)
  }

  pub fn save(self, file: &Path) -> Result<(), String> {
    if let Some(dir) = file.parent() {
      fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&self).map_err(|e| e.to_string())?;
    fs::write(file, json).map_err(|e| e.to_string())
  }

  // no bigger than `room`, e.g. the screen, for a size saved on a larger one
  pub fn within(self, room: WindowSize) -> Self {
    WindowSize { width: self.width.min(room.width), height: self.height.min(room.height) }
  }
}

fn default_file() -> Option<PathBuf> {
  dirs::config_dir().map(|dir| dir.join("dawesome").join("window.json"))
}

// the size the window opened at, then followed as it's resized, and saved
// as the app exits
pub struct KeptWindowSize {
  file: Option<PathBuf>,
  size: Mutex<Option<WindowSize>>,
}

impl KeptWindowSize {
  pub fn load() -> Self {
    let file = default_file();
    let size = file.as_deref().and_then(WindowSize::load);
    KeptWindowSize { file, size: Mutex::new(size) }
  }

  pub fn current(&self) -> Option<WindowSize> {
    *self.size.lock().unwrap()
  }

  // the window was resized. minimized and fullscreen sizes aren't kept, so
  // the window reopens at its last ordinary size
  pub fn follow(&self, window: &Window, size: PhysicalSize<u32>) {
    if size.width == 0 || size.height == 0 { return; }
    if window.is_minimized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false) { return; }
    let Ok(scale) = window.scale_factor() else { return };

    let logical = size.to_logical::<f64>(scale);
    let size = WindowSize { width: logical.width, height: logical.height };
    if let Some(size) = size.usable() {
      *self.size.lock().unwrap() = Some(size);
    }
  }

  pub fn save(&self) {
    let (Some(file), Some(size)) = (self.file.as_deref(), self.current()) else { return };
    if let Err(e) = size.save(file) {
      println!("could not save the window's size: {}", e);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn size(width: f64, height: f64) -> WindowSize {
    WindowSize { width, height }
  }

  fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dawesome-window-size-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir.join("window.json")
  }

  #[test]
  fn test_saves_and_loads() {
    let file = temp_file("round-trip");
    size(1440., 820.5).save(&file).unwrap();
    assert_eq!(WindowSize::load(&file), Some(size(1440., 820.5)));
    let _ = fs::remove_dir_all(file.parent().unwrap());
  }

  #[test]
  fn test_missing_or_broken_file_is_none() {
    let file = temp_file("broken");
    assert_eq!(WindowSize::load(&file), None);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "not json").unwrap();
    assert_eq!(WindowSize::load(&file), None);
    fs::write(&file, r#"{ "width": 800 }"#).unwrap();
    assert_eq!(WindowSize::load(&file), None);
    let _ = fs::remove_dir_all(file.parent().unwrap());
  }

  #[test]
  fn test_unusable_sizes_are_none() {
    assert_eq!(size(0., 600.).usable(), None);
    assert_eq!(size(-800., 600.).usable(), None);
    assert_eq!(size(800., f64::NAN).usable(), None);
    assert_eq!(size(800., 1e9).usable(), None);
    assert_eq!(size(800., 600.).usable(), Some(size(800., 600.)));
  }

  #[test]
  fn test_within_the_screen() {
    let screen = size(1512., 982.);
    assert_eq!(size(2560., 900.).within(screen), size(1512., 900.));
    assert_eq!(size(1200., 600.).within(screen), size(1200., 600.));
  }
}
