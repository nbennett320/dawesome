// the DAW: a song's model (its playlist, patterns, mixer and effects, its
// history), what can be done to it (actions/), playing it (playback.rs),
// recording into it, and saving it. a Session holds all of it, shared by
// whatever runs the DAW (the app's commands, the tests)
pub(crate) mod playback;

pub mod defaults;
pub mod playlist;
pub(crate) mod audiograph;
pub(crate) mod sequencer;
pub(crate) mod timing;
pub(crate) mod waveform;

pub(crate) mod actions;
pub(crate) mod bounce;
pub(crate) mod clipboard;
pub(crate) mod effects;
pub(crate) mod history;
pub(crate) mod join;
pub(crate) mod main_thread;
pub(crate) mod mixdown;
pub(crate) mod mixdown_file;
pub(crate) mod native;
pub(crate) mod plugin_catalog;
pub(crate) mod project;
pub(crate) mod project_files;
pub(crate) mod recording;
#[cfg(test)]
pub(crate) mod test_support;

use playlist::Playlist;
use std::sync::{
  Mutex,
};
use std::sync::atomic::{
  AtomicBool, 
};

pub struct Session {
  pub metronome_enabled: AtomicBool,
  pub playlist: Playlist,
  // where the project is saved, and where its takes go
  pub project: Mutex<crate::daw::project_files::ProjectLocation>,
  // the armed tracks, and the recording in progress
  pub recorder: Mutex<crate::daw::recording::Recorder>,
  // what's been done to the project, to undo and redo. locked on its own,
  // never while another of the project's locks is held
  pub history: Mutex<crate::daw::history::History>,
  // the clips last copied or cut (see clipboard.rs)
  pub clipboard: Mutex<crate::daw::clipboard::Clipboard>,
}

impl Session {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    Session {
      metronome_enabled: AtomicBool::from(true),
      playlist: Playlist::default(),
      project: Mutex::new(Default::default()),
      recorder: Mutex::new(Default::default()),
      history: Mutex::new(Default::default()),
      clipboard: Mutex::new(Default::default()),
    }
  }

  pub fn tempo(&self) -> f32 {
    self
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .tempo()
  }

  // change the tempo. clips and the playhead keep the beats they're on
  //
  // only with nothing playing: a run keeps the beat length it started with
  // (see playback::set_playlist_tempo, which ends the run first)
  pub(crate) fn set_tempo(&self, tempo: f32) {
    // the playhead moves under the graph's lock, so nothing sees the clips
    // moved and the playhead not
    let mut audiograph = self.playlist.audiograph.lock().unwrap();
    let before = audiograph.beat_interval();
    audiograph.set_tempo(tempo);
    let after = audiograph.beat_interval();

    let mut playhead = self.playlist.start_offset.lock().unwrap();
    *playhead = timing::keep_beat(*playhead, before, after);
  }
}

