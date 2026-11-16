// the Mixer tab's commands (see actions/mixer.rs)
use crate::daw::actions::mixer::{
  change_master_volume, change_track_color, change_track_mute, change_track_name, change_track_pan,
  change_track_solo, change_track_volume, mixer_settings, reset,
};
use crate::engine::mixer;
use crate::commands::TState;

#[tauri::command]
pub fn get_mixer(state: TState<'_>) -> mixer::MixerSettings {
  mixer_settings(&state)
}

#[tauri::command]
pub fn set_track_volume(state: TState<'_>, track_number: u32, volume_db: f32, gesture: Option<u64>) -> f32 {
  change_track_volume(&state, track_number, volume_db, gesture)
}

#[tauri::command]
pub fn set_track_pan(state: TState<'_>, track_number: u32, pan: f32, gesture: Option<u64>) -> f32 {
  change_track_pan(&state, track_number, pan, gesture)
}

#[tauri::command]
pub fn set_track_mute(state: TState<'_>, track_number: u32, mute: bool) {
  change_track_mute(&state, track_number, mute)
}

#[tauri::command]
pub fn set_track_solo(state: TState<'_>, track_number: u32, solo: bool) {
  change_track_solo(&state, track_number, solo)
}

#[tauri::command]
pub fn set_master_volume(state: TState<'_>, volume_db: f32, gesture: Option<u64>) -> f32 {
  change_master_volume(&state, volume_db, gesture)
}

// name a track, or clear its name with an empty one. returns the name kept
#[tauri::command]
pub fn set_track_name(state: TState<'_>, track_number: u32, name: String) -> Option<String> {
  change_track_name(&state, track_number, &name)
}

// give a track a color from the theme's track palette, or none to go back to
// its default. returns the color it's drawn in
#[tauri::command]
pub fn set_track_color(state: TState<'_>, track_number: u32, color: Option<u8>) -> u8 {
  change_track_color(&state, track_number, color)
}

// every strip back to its defaults (names and colors are kept)
#[tauri::command]
pub fn reset_mixer(state: TState<'_>) -> mixer::MixerSettings {
  reset(&state)
}

// each strip's loudest sample since the last call, for the meters
#[tauri::command]
pub fn get_mixer_levels(state: TState<'_>) -> mixer::MixerLevels {
  state.playlist.mixer.take_levels()
}
