use crate::daw;
use crate::daw::timing;
use rodio;
use std::sync;
use std::sync::atomic;
use tauri;

pub struct InnerState {
  pub global_tempo_bpm: sync::Arc<sync::Mutex<f32>>,
  pub playlist_is_playing: atomic::AtomicBool,
  pub playlist_started_time: atomic::AtomicI64,
  pub playlist_total_beats: atomic::AtomicU64,
  pub playlist_current_beat: atomic::AtomicU16,
  pub playlist_time_signature: sync::Arc<sync::Mutex<timing::TimeSignature>>,
  pub playlist_audiograph: sync::Arc<sync::Mutex<daw::AudioGraph<'static>>>,
  pub metronome_enabled: atomic::AtomicBool,
  pub root_source: sync::Arc<sync::Mutex<(rodio::Sink, rodio::queue::SourcesQueueOutput<f32>)>>,
}

impl InnerState {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    InnerState {
      playlist_is_playing: atomic::AtomicBool::from(false),
      global_tempo_bpm: sync::Arc::new(sync::Mutex::new(120.)),
      playlist_started_time: atomic::AtomicI64::from(0),
      playlist_total_beats: atomic::AtomicU64::from(0),
      playlist_current_beat: atomic::AtomicU16::from(0),
      playlist_time_signature: sync::Arc::new(sync::Mutex::new(
        timing::TimeSignature {
          numerator: 4,
          denominator: 4,
        },
      )),
      playlist_audiograph: sync::Arc::new(sync::Mutex::new(
        daw::AudioGraph::new(),
      )),
      metronome_enabled: atomic::AtomicBool::from(true),
      root_source: sync::Arc::new(sync::Mutex::new(rodio::Sink::new_idle())),
    }
  }
}
