// the Mixer tab's commands (see daw/daw_core/mixer.rs)
//
// controls take effect straight away, even while playing. the setters return
// the value actually used, after it's kept in range
use crate::daw;

// every track's strip, up to the playlist's track count, and the master
pub fn mixer_settings(state: &daw::InnerState) -> daw::MixerSettings {
  let track_count = state.playlist.audiograph.lock().unwrap().track_count();
  state.playlist.mixer.settings(track_count)
}

#[tauri::command]
pub fn get_mixer(state: daw::TState<'_>) -> daw::MixerSettings {
  mixer_settings(&state)
}

#[tauri::command]
pub fn set_track_volume(state: daw::TState<'_>, track_number: u32, volume_db: f32) -> f32 {
  state.playlist.mixer.set_volume_db(track_number, volume_db)
}

#[tauri::command]
pub fn set_track_pan(state: daw::TState<'_>, track_number: u32, pan: f32) -> f32 {
  state.playlist.mixer.set_pan(track_number, pan)
}

#[tauri::command]
pub fn set_track_mute(state: daw::TState<'_>, track_number: u32, mute: bool) {
  state.playlist.mixer.set_mute(track_number, mute)
}

#[tauri::command]
pub fn set_track_solo(state: daw::TState<'_>, track_number: u32, solo: bool) {
  state.playlist.mixer.set_solo(track_number, solo)
}

#[tauri::command]
pub fn set_master_volume(state: daw::TState<'_>, volume_db: f32) -> f32 {
  state.playlist.mixer.set_master_volume_db(volume_db)
}

// name a track, or clear its name with an empty one. returns the name kept
#[tauri::command]
pub fn set_track_name(state: daw::TState<'_>, track_number: u32, name: String) -> Option<String> {
  state.playlist.mixer.set_name(track_number, &name)
}

// every strip back to its defaults (names are kept)
#[tauri::command]
pub fn reset_mixer(state: daw::TState<'_>) -> daw::MixerSettings {
  state.playlist.mixer.reset();
  mixer_settings(&state)
}

// each strip's loudest sample since the last call, for the meters
#[tauri::command]
pub fn get_mixer_levels(state: daw::TState<'_>) -> daw::MixerLevels {
  state.playlist.mixer.take_levels()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_settings_cover_every_track_in_the_playlist() {
    let state = daw::InnerState::default();
    assert_eq!(mixer_settings(&state).tracks.len(), daw::defaults::NUM_OF_TRACKS as usize);

    // a clip on a new track gives the mixer a strip for it too
    {
      let mut audiograph = state.playlist.audiograph.lock().unwrap();
      audiograph
        .construct_and_add_node(daw::METRONOME_TICK_PATH.to_string(), std::time::Duration::ZERO, 6)
        .unwrap();
    }
    let tracks = mixer_settings(&state).tracks;
    assert_eq!(tracks.len(), 7);
    assert_eq!(tracks.last().unwrap().track_number, 6);
  }
}
