use crate::app::ui::{UI};
use crate::daw;
use crate::daw::timing;
use crate::app;
use std::sync::{
  Arc, 
  Mutex,
};
use std::sync::atomic::{
  AtomicBool, 
  AtomicU16, 
  AtomicU64,
  self,
};
use std::time::{Instant};

#[derive(Clone, Copy)]
pub struct PlaylistUI {
  pub viewport: app::workspaces::WorkspaceViewport,
  pub max_beats: u64,
  pub max_beats_displayed: u64,
  pub snap_enabled: bool,
  pub snap_subdivision: daw::timing::Note,
}

impl UI for PlaylistUI {
  fn new() -> Self {
    PlaylistUI {
      viewport: app::workspaces::WorkspaceViewport::new(),
      max_beats: daw::state::defaults::MAX_BEATS,
      max_beats_displayed: daw::state::defaults::MAX_BEATS_DISPLAYED,
      snap_enabled: daw::state::defaults::SNAP_ENABLED,
      snap_subdivision: daw::state::defaults::SNAP_SUBDIVISION,
    }
  }

  fn vp_width(&self) -> Option<f32> {
    self.viewport.width
  }

  fn vp_height(&self) -> Option<f32> {
    self.viewport.height
  }
}

impl PlaylistUI {
  pub fn toggle_snap_enabled(&mut self) {
    self.snap_enabled = !self.snap_enabled;
  }
}

pub struct Playlist {
  playing: AtomicBool,
  recording: AtomicBool,
  pub started_time: Arc<Mutex<Option<Instant>>>,
  pub total_beats: AtomicU64,
  pub current_beat: AtomicU16,
  pub max_beats: AtomicU64,
  pub time_signature: Arc<Mutex<timing::TimeSignature>>,
  pub loop_enabled: AtomicBool,
  pub audiograph: Arc<Mutex<daw::AudioGraph<'static>>>,
  pub ui: Arc<Mutex<PlaylistUI>>,
}

impl Playlist {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    Playlist {
      playing: AtomicBool::from(false),
      recording: AtomicBool::from(false),
      started_time: Arc::new(Mutex::from(None)),
      total_beats: AtomicU64::from(0),
      current_beat: AtomicU16::from(0),
      max_beats: AtomicU64::from(daw::state::defaults::MAX_BEATS),
      time_signature: Arc::new(Mutex::new(
        timing::TimeSignature {
          numerator: 4,
          denominator: 4,
        },
      )),
      loop_enabled: AtomicBool::from(true),
      audiograph: Arc::new(Mutex::new(
        daw::AudioGraph::new(
          daw::state::defaults::SAMPLE_RATE,
          daw::state::defaults::TEMPO,
          daw::state::defaults::MAX_BEATS
        ),
      )),
      ui: Arc::new(Mutex::from(PlaylistUI::new())),
    }
  }

  pub fn playing(&self) -> bool {
    self.playing.load(atomic::Ordering::SeqCst)
  }

  pub fn set_playing(&self, val: bool) {
    self.playing.store(val, atomic::Ordering::SeqCst)
  }

  pub fn recording(&self) -> bool {
    self.recording.load(atomic::Ordering::SeqCst)
  }

  pub fn set_recording(&self, val: bool) {
    self.recording.store(val, atomic::Ordering::SeqCst)
  }

  pub fn len_in_beats(&self) -> u64 {
    self.total_beats.load(atomic::Ordering::SeqCst)
  }

  pub fn duration(&self) -> u64 {
    self.total_beats.load(atomic::Ordering::SeqCst)
  }
}