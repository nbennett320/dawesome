// the user's settings, saved in the config folder as preferences.json
//
// for now that's which theme to use. a missing or unreadable file is the
// defaults, and fields this version doesn't know are ignored, so a newer
// version's file still loads
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::daw::recording::LoopRecording;
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

// where a plugin's own interface opens: a floating window, or a tab
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginView {
  #[default]
  Window,
  Tab,
}

// how the Pattern Sequencer's step keys look, after Roland's drum machines:
// coloured caps (TR-808), plain caps over a coloured strip (TR-909), or
// pads that light up (TR-8S)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum StepKeys {
  #[serde(rename = "tr808")]
  Tr808,
  #[default]
  #[serde(rename = "tr909")]
  Tr909,
  #[serde(rename = "tr8s")]
  Tr8s,
}

// how tracks on tape are worked out (T23 in context/tape-project): their
// record side printed once per change, in the background, or run live all
// the time. see 08-mode-removal.md for removing either
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TapeMode {
  #[default]
  Print,
  Live,
}

// how the tape's record side is worked out: the hand-built model of a
// deck, or a network trained on recordings of one
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TapeMethod {
  #[default]
  Modeled,
  Neural,
}

// when the tape's speed runs down and up as a deck's motor does (T30 in
// context/tape-project): a pitch dive on Stop, and a spin up on Play, from
// the flywheel's inertia. off (the default), they're at once, as ever
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TapeInertia {
  Both,
  Stop,
  Start,
  #[default]
  Off,
}

impl TapeInertia {
  pub fn on_stop(self) -> bool {
    matches!(self, TapeInertia::Both | TapeInertia::Stop)
  }

  pub fn on_start(self) -> bool {
    matches!(self, TapeInertia::Both | TapeInertia::Start)
  }
}

// which drags are heard scrubbing (T32 in context/tape-project): all of
// them (the playhead in the Timeline and the playlist, the Transport tab's
// reels, and the wind keys), all but the playlist's ("Timeline and
// transport", the default), or none, when no scrub proxy is made or kept
// at all
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scrubbing {
  On,
  #[default]
  Transport,
  Off,
}

// where on a track the tape sits (W1): before its first effect, as on a
// Portastudio (the effects are on the mixer, after the tape), or after its
// whole chain (the effects are recorded onto tape too)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum TapeTap {
  #[default]
  #[serde(rename = "before")]
  BeforeEffects,
  #[serde(rename = "after")]
  AfterEffects,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
  pub appearance: AppearanceSetting,
  // the theme used when the appearance is light, and when it's dark
  pub light_theme: String,
  pub dark_theme: String,
  pub plugin_view: PluginView,
  pub step_keys: StepKeys,
  // whether typing `:` opens the vim-style command bar, along the bottom
  pub command_bar: bool,
  // what a recording does at the loop's end
  pub loop_recording: LoopRecording,
  // whether animations still being tried out are shown (a dragged clip
  // lifting off the playlist, for now). on unless turned off
  pub experimental_animations: bool,
  // how tracks on tape are worked out
  pub tape_mode: TapeMode,
  // where on a track the tape sits
  pub tape_tap: TapeTap,
  // how the tape's record side is worked out
  pub tape_method: TapeMethod,
  // whether Stop and Play run the tape down and up
  pub tape_inertia: TapeInertia,
  // which drags are heard scrubbing
  pub scrub: Scrubbing,
  // the folders of samples added in Preferences › File Paths, besides the
  // ones Dawesome comes with (see daw/library.rs)
  pub sample_paths: Vec<String>,
}

impl Default for Preferences {
  fn default() -> Self {
    Preferences {
      appearance: AppearanceSetting::System,
      light_theme: theme::DEFAULT_LIGHT.to_string(),
      dark_theme: theme::DEFAULT_DARK.to_string(),
      plugin_view: PluginView::Window,
      step_keys: StepKeys::default(),
      command_bar: false,
      loop_recording: LoopRecording::default(),
      experimental_animations: true,
      tape_mode: TapeMode::default(),
      tape_tap: TapeTap::default(),
      tape_method: TapeMethod::default(),
      tape_inertia: TapeInertia::default(),
      scrub: Scrubbing::default(),
      sample_paths: Vec::new(),
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
      plugin_view: self.plugin_view,
      step_keys: self.step_keys,
      command_bar: self.command_bar,
      loop_recording: self.loop_recording,
      experimental_animations: self.experimental_animations,
      tape_mode: self.tape_mode,
      tape_tap: self.tape_tap,
      tape_method: self.tape_method,
      tape_inertia: self.tape_inertia,
      scrub: self.scrub,
      sample_paths: self.sample_paths.clone(),
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

  // keep and save the folders of samples added in File Paths. only these
  // change them: set keeps the ones there are
  pub fn set_sample_paths(&self, sample_paths: Vec<String>) -> Result<Preferences, String> {
    let prefs = Preferences { sample_paths, ..self.prefs.lock().unwrap().clone() };
    if let Some(file) = &self.file {
      prefs.save(file)?;
    }
    *self.prefs.lock().unwrap() = prefs.clone();
    Ok(prefs)
  }

  // sanitize, keep and save new settings, and return what was kept. the
  // sample folders stay as they are (the frontend's copy of the settings
  // may be from before one was added)
  pub fn set(&self, prefs: Preferences) -> Result<Preferences, String> {
    let prefs = Preferences { sample_paths: self.prefs.lock().unwrap().sample_paths.clone(), ..prefs };
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
      plugin_view: PluginView::Tab,
      step_keys: StepKeys::Tr8s,
      command_bar: true,
      loop_recording: LoopRecording::Stop,
      experimental_animations: true,
      tape_mode: TapeMode::Live,
      tape_tap: TapeTap::default(),
      tape_method: TapeMethod::default(),
      tape_inertia: TapeInertia::Stop,
      scrub: Scrubbing::Off,
      sample_paths: vec!["/Users/me/Samples".to_string()],
    };
    prefs.save(&file).unwrap();
    assert_eq!(Preferences::load(&file), prefs);
    let json = fs::read_to_string(&file).unwrap();
    assert!(json.contains("\"lightTheme\"") && json.contains("\"dark\""), "{}", json);
    assert!(json.contains("\"pluginView\": \"tab\""), "{}", json);
    assert!(json.contains("\"stepKeys\": \"tr8s\""), "{}", json);
    assert!(json.contains("\"commandBar\": true"), "{}", json);
    assert!(json.contains("\"loopRecording\": \"stop\""), "{}", json);
    assert!(json.contains("\"experimentalAnimations\": true"), "{}", json);
    assert!(json.contains("\"tapeMode\": \"live\""), "{}", json);
    assert!(json.contains("\"tapeInertia\": \"stop\""), "{}", json);
    assert!(json.contains("\"scrub\": \"off\""), "{}", json);
    assert!(json.contains("\"samplePaths\": [\n    \"/Users/me/Samples\"\n  ]"), "{}", json);
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
    // plugins open in windows unless a tab is asked for
    assert_eq!(prefs.plugin_view, PluginView::Window);
    // and step keys look like a TR-909's
    assert_eq!(prefs.step_keys, StepKeys::Tr909);
    // and the command bar is off
    assert!(!prefs.command_bar);
    // and a loop recording carries on, layering its passes
    assert_eq!(prefs.loop_recording, LoopRecording::Layer);
    // and experimental animations are on
    assert!(prefs.experimental_animations);
    // and tracks on tape are printed in the background
    assert_eq!(prefs.tape_mode, TapeMode::Print);
    assert_eq!(prefs.tape_tap, TapeTap::BeforeEffects);
    assert_eq!(prefs.tape_method, TapeMethod::Modeled);
    // and Stop and Play are at once
    assert_eq!(prefs.tape_inertia, TapeInertia::Off);
    // and everything but the playlist's playhead scrubs
    assert_eq!(prefs.scrub, Scrubbing::Transport);
    fs::remove_dir_all(file.parent().unwrap()).ok();
  }

  #[test]
  fn test_tape_inertia_reads_each_way_and_says_what_it_covers() {
    for (json, inertia, stop, start) in [
      ("both", TapeInertia::Both, true, true),
      ("stop", TapeInertia::Stop, true, false),
      ("start", TapeInertia::Start, false, true),
      ("off", TapeInertia::Off, false, false),
    ] {
      let prefs: Preferences = serde_json::from_str(&format!(r#"{{ "tapeInertia": "{json}" }}"#)).unwrap();
      assert_eq!(prefs.tape_inertia, inertia);
      assert_eq!((inertia.on_stop(), inertia.on_start()), (stop, start));
    }
  }

  #[test]
  fn test_scrubbing_reads_each_way() {
    for (json, scrub) in [("on", Scrubbing::On), ("transport", Scrubbing::Transport), ("off", Scrubbing::Off)] {
      let prefs: Preferences = serde_json::from_str(&format!(r#"{{ "scrub": "{json}" }}"#)).unwrap();
      assert_eq!(prefs.scrub, scrub);
    }
  }

  #[test]
  fn test_only_set_sample_paths_changes_them() {
    let file = temp_file("sample-paths");
    let app = AppPreferences { file: Some(file.clone()), prefs: Mutex::new(Preferences::default()), themes: theme::stock() };
    app.set_sample_paths(vec!["/Users/me/Samples".into()]).unwrap();
    // settings from before the folder was added don't take it away
    let kept = app.set(Preferences { command_bar: true, ..Preferences::default() }).unwrap();
    assert!(kept.command_bar);
    assert_eq!(kept.sample_paths, vec!["/Users/me/Samples".to_string()]);
    assert_eq!(Preferences::load(&file).sample_paths, vec!["/Users/me/Samples".to_string()]);
    fs::remove_dir_all(file.parent().unwrap()).ok();
  }

  #[test]
  fn test_sanitize() {
    let themes = theme::stock();
    let ok = Preferences {
      appearance: AppearanceSetting::System,
      light_theme: "charcoal-light".to_string(),
      dark_theme: "slate-dark".to_string(),
      plugin_view: PluginView::Tab,
      step_keys: StepKeys::Tr808,
      command_bar: true,
      loop_recording: LoopRecording::Layer,
      experimental_animations: true,
      tape_mode: TapeMode::Live,
      tape_tap: TapeTap::default(),
      tape_method: TapeMethod::default(),
      tape_inertia: TapeInertia::Both,
      scrub: Scrubbing::On,
      sample_paths: vec!["/Users/me/Samples".to_string()],
    };
    assert_eq!(ok.sanitize(&themes), ok);

    let bad = Preferences {
      appearance: AppearanceSetting::Dark,
      light_theme: "no-such-theme".to_string(),
      dark_theme: "slate-light".to_string(),
      plugin_view: PluginView::Window,
      step_keys: StepKeys::Tr909,
      command_bar: false,
      loop_recording: LoopRecording::Layer,
      experimental_animations: false,
      tape_mode: TapeMode::Print,
      tape_tap: TapeTap::default(),
      tape_method: TapeMethod::default(),
      tape_inertia: TapeInertia::Start,
      scrub: Scrubbing::Transport,
      sample_paths: Vec::new(),
    };
    let fixed = bad.sanitize(&themes);
    assert_eq!(fixed.light_theme, theme::DEFAULT_LIGHT);
    assert_eq!(fixed.dark_theme, theme::DEFAULT_DARK);
    assert_eq!(fixed.appearance, AppearanceSetting::Dark);
    assert_eq!(fixed.tape_inertia, TapeInertia::Start);

    let dark_in_light = Preferences { light_theme: "charcoal".to_string(), ..ok };
    assert_eq!(dark_in_light.sanitize(&themes).light_theme, theme::DEFAULT_LIGHT);
  }
}
