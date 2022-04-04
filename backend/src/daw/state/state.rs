use std::sync::atomic;

pub struct InnerState {
  pub global_tempo_bpm: f32,
  pub playlist_is_playing: atomic::AtomicBool,
  pub playlist_started_time: atomic::AtomicI64,
  pub metronome_enabled: atomic::AtomicBool,
}

impl InnerState {
  pub fn new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    InnerState {
      playlist_is_playing: atomic::AtomicBool::from(false),
      global_tempo_bpm: 120.,
      playlist_started_time: atomic::AtomicI64::from(0),
      metronome_enabled: atomic::AtomicBool::from(true),
    }
  }
}
