// the commands the frontend uses to read the themes and the user's
// appearance settings, and to change the settings
use tauri::{Manager, WebviewWindow};

use crate::app::window::{parse_color, pinned_theme};
use crate::preferences::{AppPreferences, Preferences};
use crate::theme::ResolvedTheme;

#[tauri::command]
pub fn themes(app: tauri::State<'_, AppPreferences>) -> Vec<ResolvedTheme> {
  app.themes.clone()
}

#[tauri::command]
pub fn preferences(app: tauri::State<'_, AppPreferences>) -> Preferences {
  app.current()
}

// keep the settings (with unusable themes put right) and return what was kept
#[tauri::command]
pub fn set_preferences(
  app: tauri::State<'_, AppPreferences>,
  window: tauri::Window,
  preferences: Preferences,
) -> Result<Preferences, String> {
  let kept = app.set(preferences)?;
  // the title bar follows: the system's, or pinned. the settings are saved
  // either way, so a failure here is only reported
  window.set_theme(pinned_theme(kept.appearance)).map_err(|e| e.to_string())?;
  Ok(kept)
}

// the frontend calls this after a theme change, so a resize or overscroll
// doesn't show the old color
#[tauri::command]
pub fn set_window_background(app: tauri::AppHandle, color: String) -> Result<(), String> {
  let color = parse_color(&color)?;
  let window: WebviewWindow = app.get_webview_window("main").ok_or("no main window")?;
  window.set_background_color(Some(color)).map_err(|e| e.to_string())
}
