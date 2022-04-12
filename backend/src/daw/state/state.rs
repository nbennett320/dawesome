use std::sync;
use std::sync::atomic;
use crate::daw::timing;

pub struct InnerState {
  pub global_tempo_bpm: sync::Arc<sync::Mutex<f32>>,
  pub playlist_is_playing: atomic::AtomicBool,
  pub playlist_started_time: atomic::AtomicI64,
  pub playlist_total_beats: atomic::AtomicU64,
  pub playlist_current_beat: atomic::AtomicU16,
  pub playlist_time_signature: sync::Arc<sync::Mutex<timing::TimeSignature>>,
  pub metronome_enabled: atomic::AtomicBool,
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
      playlist_time_signature: sync::Arc::new(sync::Mutex::new(timing::TimeSignature {
        numerator: 4,
        denominator: 4
      })),
      metronome_enabled: atomic::AtomicBool::from(true),
    }
  }
}
