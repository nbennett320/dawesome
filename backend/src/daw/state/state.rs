use crate::daw;
use crate::daw::timing;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{
  AtomicBool, 
  AtomicU16, 
  AtomicU64
};
use std::time::{Instant};
use rodio::{Sink};
use rodio::queue::{SourcesQueueOutput};
use tauri;

pub struct Playlist {
  pub playing: AtomicBool,
  pub started_time: Arc<Mutex<Option<Instant>>>,
  pub total_beats: AtomicU64,
  pub current_beat: AtomicU16,
  pub time_signature: Arc<Mutex<timing::TimeSignature>>,
  pub audiograph: Arc<Mutex<daw::AudioGraph<'static>>>,
}

impl Playlist {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    Playlist {
      playing: AtomicBool::from(false),
      started_time: Arc::new(Mutex::from(None)),
      total_beats: AtomicU64::from(0),
      current_beat: AtomicU16::from(0),
      time_signature: Arc::new(Mutex::new(
        timing::TimeSignature {
          numerator: 4,
          denominator: 4,
        },
      )),
      audiograph: Arc::new(Mutex::new(
        daw::AudioGraph::new(),
      )),
    }
  }
}

pub struct InnerState {
  pub global_tempo_bpm: Arc<Mutex<f32>>,
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
      global_tempo_bpm: Arc::new(Mutex::new(120.)),
      metronome_enabled: AtomicBool::from(true),
      root_source: Arc::new(Mutex::new(Sink::new_idle())),
      playlist: Playlist::default()
    }
  }
}
