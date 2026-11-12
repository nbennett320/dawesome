// the Mixer tab's commands (see daw/daw_core/mixer.rs)
//
// controls take effect straight away, even while playing. the setters return
// the value actually used, after it's kept in range
use crate::daw;
use crate::history;

// every track's strip, up to the playlist's track count, and the master
pub fn mixer_settings(state: &daw::InnerState) -> daw::MixerSettings {
  let track_count = state.playlist.audiograph.lock().unwrap().track_count();
  state.playlist.mixer.settings(track_count)
}

#[tauri::command]
pub fn get_mixer(state: daw::TState<'_>) -> daw::MixerSettings {
  mixer_settings(&state)
}

// change the mixer, and record the change so it can be undone
// - gesture: the fader or knob drag it's part of, if any. one drag is one
//   step, however many values it sends
fn edit<T>(state: &daw::InnerState, gesture: Option<u64>, change: impl FnOnce(&daw::Mixer) -> T) -> T {
  let mixer = &state.playlist.mixer;
  let before = mixer.changed();
  let result = change(mixer);
  history::record(state, history::Step::new().in_gesture(gesture).with_mixer(before, mixer.changed()));
  result
}

pub fn change_track_volume(state: &daw::InnerState, track_number: u32, volume_db: f32, gesture: Option<u64>) -> f32 {
  edit(state, gesture, |mixer| mixer.set_volume_db(track_number, volume_db))
}

pub fn change_track_pan(state: &daw::InnerState, track_number: u32, pan: f32, gesture: Option<u64>) -> f32 {
  edit(state, gesture, |mixer| mixer.set_pan(track_number, pan))
}

pub fn change_track_mute(state: &daw::InnerState, track_number: u32, mute: bool) {
  edit(state, None, |mixer| mixer.set_mute(track_number, mute))
}

pub fn change_track_solo(state: &daw::InnerState, track_number: u32, solo: bool) {
  edit(state, None, |mixer| mixer.set_solo(track_number, solo))
}

pub fn change_master_volume(state: &daw::InnerState, volume_db: f32, gesture: Option<u64>) -> f32 {
  edit(state, gesture, |mixer| mixer.set_master_volume_db(volume_db))
}

pub fn change_track_name(state: &daw::InnerState, track_number: u32, name: &str) -> Option<String> {
  edit(state, None, |mixer| mixer.set_name(track_number, name))
}

pub fn reset(state: &daw::InnerState) -> daw::MixerSettings {
  edit(state, None, |mixer| mixer.reset());
  mixer_settings(state)
}

#[tauri::command]
pub fn set_track_volume(state: daw::TState<'_>, track_number: u32, volume_db: f32, gesture: Option<u64>) -> f32 {
  change_track_volume(&state, track_number, volume_db, gesture)
}

#[tauri::command]
pub fn set_track_pan(state: daw::TState<'_>, track_number: u32, pan: f32, gesture: Option<u64>) -> f32 {
  change_track_pan(&state, track_number, pan, gesture)
}

#[tauri::command]
pub fn set_track_mute(state: daw::TState<'_>, track_number: u32, mute: bool) {
  change_track_mute(&state, track_number, mute)
}

#[tauri::command]
pub fn set_track_solo(state: daw::TState<'_>, track_number: u32, solo: bool) {
  change_track_solo(&state, track_number, solo)
}

#[tauri::command]
pub fn set_master_volume(state: daw::TState<'_>, volume_db: f32, gesture: Option<u64>) -> f32 {
  change_master_volume(&state, volume_db, gesture)
}

// name a track, or clear its name with an empty one. returns the name kept
#[tauri::command]
pub fn set_track_name(state: daw::TState<'_>, track_number: u32, name: String) -> Option<String> {
  change_track_name(&state, track_number, &name)
}

// every strip back to its defaults (names are kept)
#[tauri::command]
pub fn reset_mixer(state: daw::TState<'_>) -> daw::MixerSettings {
  reset(&state)
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
