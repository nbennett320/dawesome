use crate::daw::state::{Playlist};
use std::sync::{
  Arc, 
  Mutex,
};
use std::sync::atomic::{
  AtomicBool, 
};
use rodio::{Sink};
use rodio::queue::{SourcesQueueOutput};

pub struct InnerState {
  pub metronome_enabled: AtomicBool,
  pub root_source: Arc<Mutex<(Sink, SourcesQueueOutput<f32>)>>,
  pub playlist: Playlist,
  // where the project is saved, and where its takes go
  pub project: Mutex<crate::project_files::ProjectLocation>,
  // the armed tracks, and the recording in progress
  pub recorder: Mutex<crate::recording::Recorder>,
  // what's been done to the project, to undo and redo. locked on its own,
  // never while another of the project's locks is held
  pub history: Mutex<crate::history::History>,
}

impl InnerState {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    InnerState {
      metronome_enabled: AtomicBool::from(true),
      root_source: Arc::new(Mutex::new(Sink::new_idle())),
      playlist: Playlist::default(),
      project: Mutex::new(Default::default()),
      recorder: Mutex::new(Default::default()),
      history: Mutex::new(Default::default()),
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
  // (see daw::set_playlist_tempo, which ends the run first)
  pub(crate) fn set_tempo(&self, tempo: f32) {
    // the playhead moves under the graph's lock, so nothing sees the clips
    // moved and the playhead not
    let mut audiograph = self.playlist.audiograph.lock().unwrap();
    let before = audiograph.beat_interval();
    audiograph.set_tempo(tempo);
    let after = audiograph.beat_interval();

    let mut playhead = self.playlist.start_offset.lock().unwrap();
    *playhead = crate::daw::timing::keep_beat(*playhead, before, after);
  }
}

pub type TState<'a> = tauri::State<'a, Arc<InnerState>>;
