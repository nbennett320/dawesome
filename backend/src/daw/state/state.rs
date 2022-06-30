use crate::app::ui::{UI};
use crate::daw;
use crate::daw::timing;
use crate::app;
use std::sync::{
  Arc, 
  Mutex
};
use std::sync::atomic::{
  AtomicBool, 
  AtomicU16, 
  AtomicU64
};
use std::time::{Instant};
use rodio::{Sink};
use rodio::queue::{SourcesQueueOutput};

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

pub struct Playlist {
  pub playing: AtomicBool,
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
}

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
