// changing the mixer (see daw/daw_core/mixer.rs), each change recorded so
// it can be undone. commands/mixer.rs is the Mixer tab's side of these
//
// controls take effect straight away, even while playing. the setters return
// the value actually used, after it's kept in range
use crate::daw;
use crate::engine::mixer;
use crate::daw::history;

// every track's strip, up to the playlist's track count, and the master
pub fn mixer_settings(state: &daw::Session) -> mixer::MixerSettings {
  let track_count = state.playlist.audiograph.lock().unwrap().track_count();
  state.playlist.mixer.settings(track_count)
}

// change the mixer, and record the change so it can be undone
// - gesture: the fader or knob drag it's part of, if any. one drag is one
//   step, however many values it sends
fn edit<T>(state: &daw::Session, gesture: Option<u64>, change: impl FnOnce(&mixer::Mixer) -> T) -> T {
  let mixer = &state.playlist.mixer;
  let before = mixer.changed();
  let result = change(mixer);
  history::record(state, history::Step::new().in_gesture(gesture).with_mixer(before, mixer.changed()));
  result
}

pub fn change_track_volume(state: &daw::Session, track_number: u32, volume_db: f32, gesture: Option<u64>) -> f32 {
  edit(state, gesture, |mixer| mixer.set_volume_db(track_number, volume_db))
}

pub fn change_track_pan(state: &daw::Session, track_number: u32, pan: f32, gesture: Option<u64>) -> f32 {
  edit(state, gesture, |mixer| mixer.set_pan(track_number, pan))
}

pub fn change_track_mute(state: &daw::Session, track_number: u32, mute: bool) {
  edit(state, None, |mixer| mixer.set_mute(track_number, mute))
}

pub fn change_track_solo(state: &daw::Session, track_number: u32, solo: bool) {
  edit(state, None, |mixer| mixer.set_solo(track_number, solo))
}

pub fn change_master_volume(state: &daw::Session, volume_db: f32, gesture: Option<u64>) -> f32 {
  edit(state, gesture, |mixer| mixer.set_master_volume_db(volume_db))
}

pub fn change_track_name(state: &daw::Session, track_number: u32, name: &str) -> Option<String> {
  edit(state, None, |mixer| mixer.set_name(track_number, name))
}

pub fn change_track_color(state: &daw::Session, track_number: u32, color: Option<u8>) -> u8 {
  edit(state, None, |mixer| mixer.set_color(track_number, color))
}

pub fn reset(state: &daw::Session) -> mixer::MixerSettings {
  edit(state, None, |mixer| mixer.reset());
  mixer_settings(state)
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::daw::{defaults, playback};

  #[test]
  fn test_settings_cover_every_track_in_the_playlist() {
    let state = daw::Session::default();
    assert_eq!(mixer_settings(&state).tracks.len(), defaults::NUM_OF_TRACKS as usize);

    // a clip on a new track gives the mixer a strip for it too
    {
      let mut audiograph = state.playlist.audiograph.lock().unwrap();
      audiograph
        .construct_and_add_node(playback::METRONOME_TICK_PATH.to_string(), std::time::Duration::ZERO, 6)
        .unwrap();
    }
    let tracks = mixer_settings(&state).tracks;
    assert_eq!(tracks.len(), 7);
    assert_eq!(tracks.last().unwrap().track_number, 6);
  }
}
