// files dragged in from outside the app (see daw/file_drop.rs)
//
// (async: off the main thread, as reading and copying files takes a while)
use crate::commands::TState;
use crate::daw::actions::clip::{add_clips, PlacedNode};
use crate::daw::file_drop::{self, DropScan, DroppedFile, Imported};

// what was dropped (files and folders), as the files in it that can be
// played and the things that can't
#[tauri::command(async)]
pub fn scan_dropped_files(paths: Vec<String>) -> DropScan {
  file_drop::scan(&paths)
}

// copy files into the project's clips, into the folder `into` there (the
// top when it's left out). files the clips already have aren't copied again
#[tauri::command(async)]
pub fn import_files(state: TState<'_>, files: Vec<DroppedFile>, into: Option<String>) -> Result<Imported, String> {
  file_drop::import(&state, &files, into.as_deref().unwrap_or(""))
}

// a clip of each file, one per track from `track_number` down, all where
// they were dropped. one step to undo
#[tauri::command(async)]
pub fn place_files(state: TState<'_>, paths: Vec<String>, track_number: u32, drop_beat: f32) -> Result<Vec<PlacedNode>, String> {
  add_clips(&state, &paths, track_number, Some(drop_beat))
}
