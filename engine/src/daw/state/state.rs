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
}

impl InnerState {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    InnerState {
      metronome_enabled: AtomicBool::from(true),
      root_source: Arc::new(Mutex::new(Sink::new_idle())),
      playlist: Playlist::default()
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

  pub fn set_tempo(&self, tempo: f32) {
    self
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .set_tempo(tempo);
  }
}

pub type TState<'a> = tauri::State<'a, Arc<InnerState>>;
