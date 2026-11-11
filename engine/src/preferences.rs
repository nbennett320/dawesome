// the user's settings, saved in the config folder as preferences.json
//
// for now that's which theme to use. a missing or unreadable file is the
// defaults, and fields this version doesn't know are ignored, so a newer
// version's file still loads
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::theme::{self, Appearance, ResolvedTheme};

// follow the system, or always light or dark
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceSetting {
  #[default]
  System,
  Light,
  Dark,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
  pub appearance: AppearanceSetting,
  // the theme used when the appearance is light, and when it's dark
  pub light_theme: String,
  pub dark_theme: String,
}

impl Default for Preferences {
  fn default() -> Self {
    Preferences {
      appearance: AppearanceSetting::System,
      light_theme: theme::DEFAULT_LIGHT.to_string(),
      dark_theme: theme::DEFAULT_DARK.to_string(),
    }
  }
}

// where the settings are saved, e.g. ~/Library/Application Support/dawesome
fn default_file() -> Option<PathBuf> {
  dirs::config_dir().map(|dir| dir.join("dawesome").join("preferences.json"))
}

impl Preferences {
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

  // a theme that doesn't exist, or is the wrong appearance for its slot,
  // becomes that slot's default. the file isn't rewritten, so a theme that
  // is only missing for now (say a user theme) comes back when it does
  pub fn sanitize(&self, themes: &[ResolvedTheme]) -> Preferences {
    let usable = |id: &str, appearance: Appearance| {
      themes.iter().any(|t| t.id == id && t.appearance == appearance)
    };
    let pick = |id: &String, appearance: Appearance, default: &str| {
      if usable(id, appearance) { id.clone() } else { default.to_string() }
    };
    Preferences {
      appearance: self.appearance,
      light_theme: pick(&self.light_theme, Appearance::Light, theme::DEFAULT_LIGHT),
      dark_theme: pick(&self.dark_theme, Appearance::Dark, theme::DEFAULT_DARK),
    }
  }
}

// the settings and themes the commands work on
pub struct AppPreferences {
  file: Option<PathBuf>,
  pub prefs: Mutex<Preferences>,
  pub themes: Vec<ResolvedTheme>,
}

impl AppPreferences {
  // read the saved settings from the user's config folder
  pub fn load() -> Self {
    let file = default_file();
    let prefs = file.as_deref().map(Preferences::load).unwrap_or_default();

    AppPreferences {
      file,
      prefs: Mutex::new(prefs),
      themes: theme::stock(),
    }
  }

  pub fn current(&self) -> Preferences {
    self.prefs.lock().unwrap().sanitize(&self.themes)
  }

  // sanitize, keep and save new settings, and return what was kept
  pub fn set(&self, prefs: Preferences) -> Result<Preferences, String> {
    let prefs = prefs.sanitize(&self.themes);
    if let Some(file) = &self.file {
      prefs.save(file)?;
    }
    *self.prefs.lock().unwrap() = prefs.clone();
    Ok(prefs)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir()
      .join(format!("dawesome-prefs-test-{}-{}", std::process::id(), name))
      .join("preferences.json")
  }

  #[test]
  fn test_round_trip() {
    let file = temp_file("round-trip");
    let prefs = Preferences {
      appearance: AppearanceSetting::Dark,
      light_theme: "charcoal-light".to_string(),
      dark_theme: "slate-dark".to_string(),
    };
    prefs.save(&file).unwrap();
    assert_eq!(Preferences::load(&file), prefs);
    let json = fs::read_to_string(&file).unwrap();
    assert!(json.contains("\"lightTheme\"") && json.contains("\"dark\""), "{}", json);
    fs::remove_dir_all(file.parent().unwrap()).ok();
  }

  #[test]
  fn test_missing_and_garbage_files_are_defaults() {
    let file = temp_file("garbage");
    assert_eq!(Preferences::load(&file), Preferences::default());
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "{ not json").unwrap();
    assert_eq!(Preferences::load(&file), Preferences::default());
    fs::remove_dir_all(file.parent().unwrap()).ok();
  }

  #[test]
  fn test_unknown_and_missing_fields() {
    let file = temp_file("extra");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, r#"{ "appearance": "light", "fontSize": 14 }"#).unwrap();
    let prefs = Preferences::load(&file);
    assert_eq!(prefs.appearance, AppearanceSetting::Light);
    assert_eq!(prefs.dark_theme, theme::DEFAULT_DARK);
    fs::remove_dir_all(file.parent().unwrap()).ok();
  }

  #[test]
  fn test_sanitize() {
    let themes = theme::stock();
    let ok = Preferences {
      appearance: AppearanceSetting::System,
      light_theme: "charcoal-light".to_string(),
      dark_theme: "slate-dark".to_string(),
    };
    assert_eq!(ok.sanitize(&themes), ok);

    let bad = Preferences {
      appearance: AppearanceSetting::Dark,
      light_theme: "no-such-theme".to_string(),
      dark_theme: "slate-light".to_string(),
    };
    let fixed = bad.sanitize(&themes);
    assert_eq!(fixed.light_theme, theme::DEFAULT_LIGHT);
    assert_eq!(fixed.dark_theme, theme::DEFAULT_DARK);
    assert_eq!(fixed.appearance, AppearanceSetting::Dark);

    let dark_in_light = Preferences { light_theme: "charcoal".to_string(), ..ok };
    assert_eq!(dark_in_light.sanitize(&themes).light_theme, theme::DEFAULT_LIGHT);
  }
}
