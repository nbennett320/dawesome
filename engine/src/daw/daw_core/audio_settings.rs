// which devices the engine plays (and later records) through, and how big a
// buffer it asks the output for. saved in the user's config folder, so they
// survive restarts
use rodio::cpal::SupportedBufferSize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// buffer sizes offered, in frames. smaller is less latency but more work for
// the computer (too small and the audio crackles)
pub const BUFFER_CHOICES: [u32; 5] = [64, 128, 256, 512, 1024];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioSettings {
  // by name. none for the system's default
  pub output_device: Option<String>,
  pub input_device: Option<String>,
  // frames per buffer. none for the device's default
  pub buffer_frames: Option<u32>,
  // how much earlier recordings are placed than the devices' reported
  // latency says, in milliseconds. for devices that don't report all of it
  // (Bluetooth headphones add a lot). negative places them later
  pub record_offset_ms: f64,
}

// where the settings are saved, e.g. ~/Library/Application Support/dawesome
pub fn default_file() -> Option<PathBuf> {
  tauri::api::path::config_dir().map(|dir| dir.join("dawesome").join("audio-settings.json"))
}

impl AudioSettings {
  // read saved settings. a missing or unreadable file is the defaults
  pub fn load(file: &Path) -> Self {
    fs::read_to_string(file)
      .ok()
      .and_then(|json| serde_json::from_str(&json).ok())
      .unwrap_or_default()
  }

  pub fn save(&self, file: &Path) -> Result<(), String> {
    if let Some(dir) = file.parent() {
      fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
    fs::write(file, json).map_err(|e| e.to_string())
  }

  // the saved settings, or the defaults
  pub fn load_default() -> Self {
    default_file().map(|file| AudioSettings::load(&file)).unwrap_or_default()
  }

  pub fn save_default(&self) -> Result<(), String> {
    let file = default_file().ok_or("there's no config folder to save audio settings in")?;
    self.save(&file)
  }
}

// the buffer sizes a device takes, of those offered
pub fn buffer_choices(supported: &SupportedBufferSize) -> Vec<u32> {
  BUFFER_CHOICES
    .iter()
    .copied()
    .filter(|&frames| match supported {
      SupportedBufferSize::Range { min, max } => frames >= *min && frames <= *max,
      SupportedBufferSize::Unknown => true,
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_file(name: &str) -> PathBuf {
    std::env::temp_dir()
      .join(format!("dawesome-audio-settings-{}-{}", name, std::process::id()))
      .join("audio-settings.json")
  }

  #[test]
  fn test_save_and_load() {
    let file = test_file("round-trip");
    let settings = AudioSettings {
      output_device: Some("Studio Interface".into()),
      input_device: Some("Studio Interface".into()),
      buffer_frames: Some(128),
      record_offset_ms: 150.,
    };

    settings.save(&file).unwrap();
    assert_eq!(AudioSettings::load(&file), settings);

    let _ = fs::remove_dir_all(file.parent().unwrap());
  }

  #[test]
  fn test_missing_or_broken_files_are_the_defaults() {
    let file = test_file("broken");
    assert_eq!(AudioSettings::load(&file), AudioSettings::default());

    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "not json").unwrap();
    assert_eq!(AudioSettings::load(&file), AudioSettings::default());

    // settings saved before a field existed load with it at its default
    fs::write(&file, r#"{ "bufferFrames": 256 }"#).unwrap();
    assert_eq!(AudioSettings::load(&file), AudioSettings { buffer_frames: Some(256), ..Default::default() });

    let _ = fs::remove_dir_all(file.parent().unwrap());
  }

  #[test]
  fn test_buffer_choices_fit_the_device() {
    assert_eq!(buffer_choices(&SupportedBufferSize::Range { min: 100, max: 600 }), vec![128, 256, 512]);
    assert_eq!(buffer_choices(&SupportedBufferSize::Range { min: 14, max: 4096 }), BUFFER_CHOICES.to_vec());
    assert_eq!(buffer_choices(&SupportedBufferSize::Unknown), BUFFER_CHOICES.to_vec());
  }
}
