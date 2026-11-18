// shareable projects (.sharedawesome, see daw/share.rs)
use std::path::Path;

use serde::Serialize;
use tauri::Emitter;

use crate::commands::TState;
use crate::daw::share::{self, Exported};
use crate::preferences::AppPreferences;

// how far along an export is, sent as it goes ("share-progress")
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SharingProgress {
  done: u64,
  total: u64,
}

// File › Save as Shareable Project…: zip the open project, as it is now,
// with every sample it uses, into `path`. the project itself isn't saved
// (async: off the main thread, as copying samples takes a while)
#[tauri::command(async)]
pub fn export_shareable(
  app: tauri::AppHandle,
  state: TState<'_>,
  prefs: tauri::State<'_, AppPreferences>,
  path: String,
  layout: Option<serde_json::Value>,
) -> Result<Exported, String> {
  // a few hundred updates at most, not one per megabyte
  let mut sent = None;
  let mut progress = |done: u64, total: u64| {
    let step = if total == 0 { 0 } else { done * 200 / total };
    if sent == Some(step) { return; }
    sent = Some(step);
    let _ = app.emit("share-progress", SharingProgress { done, total });
  };
  share::export(&state, Path::new(&path), &prefs.current().sample_paths, layout, &mut progress)
}

