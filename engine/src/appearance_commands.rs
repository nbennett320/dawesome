// the commands the frontend uses to read the themes and the user's
// appearance settings, and to change the settings
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
  preferences: Preferences,
) -> Result<Preferences, String> {
  app.set(preferences)
}
