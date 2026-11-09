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
use std::time::{Duration, Instant};

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

// what a run plays: the playlist, or one pattern looped on its own (Play in
// the Sequencer tab)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayMode {
  Playlist,
  Pattern(u64),
}

pub struct Playlist {
  playing: AtomicBool,
  recording: AtomicBool,
  pub started_time: Arc<Mutex<Option<Instant>>>,
  // where on the timeline the current run started, or where the next one
  // will start (the playhead while stopped or paused)
  pub start_offset: Mutex<Duration>,
  // bumped by every run and every stop, so a run's thread can tell it's been
  // replaced (see daw::run_playlist)
  pub run_id: AtomicU64,
  pub total_beats: AtomicU64,
  pub current_beat: AtomicU16,
  pub max_beats: AtomicU64,
  pub time_signature: Arc<Mutex<timing::TimeSignature>>,
  pub loop_enabled: AtomicBool,
  pub audiograph: Arc<Mutex<daw::AudioGraph<'static>>>,
  // the same mixer the audio graph plays clips through, reachable without
  // the graph's lock
  pub mixer: Arc<daw::Mixer>,
  // the same sequencer the audio graph plays pattern clips from. lock it
  // after the graph, never before
  pub sequencer: Arc<Mutex<daw::Sequencer>>,
  // what the current (or last) run plays. set before each run starts
  pub mode: Mutex<PlayMode>,
  pub ui: Arc<Mutex<PlaylistUI>>,
}

impl Playlist {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    let audiograph = daw::AudioGraph::new(
      daw::state::defaults::SAMPLE_RATE,
      daw::state::defaults::TEMPO,
      daw::state::defaults::MAX_BEATS
    );
    let mixer = audiograph.mixer.clone();
    let sequencer = audiograph.sequencer.clone();

    Playlist {
      playing: AtomicBool::from(false),
      recording: AtomicBool::from(false),
      started_time: Arc::new(Mutex::from(None)),
      start_offset: Mutex::new(Duration::ZERO),
      run_id: AtomicU64::from(0),
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
      audiograph: Arc::new(Mutex::new(audiograph)),
      mixer,
      sequencer,
      mode: Mutex::new(PlayMode::Playlist),
      ui: Arc::new(Mutex::from(PlaylistUI::new())),
    }
  }

  pub fn playing(&self) -> bool {
    self.playing.load(atomic::Ordering::SeqCst)
  }

  pub fn set_playing(&self, val: bool) {
    self.playing.store(val, atomic::Ordering::SeqCst)
  }

  pub fn mode(&self) -> PlayMode {
    *self.mode.lock().unwrap()
  }

  // the pattern playing on its own, if one is
  pub fn playing_pattern(&self) -> Option<u64> {
    match self.mode() {
      PlayMode::Pattern(id) if self.playing() => Some(id),
      _ => None,
    }
  }

  // whether the playlist itself is playing (not a pattern on its own)
  pub fn playing_playlist(&self) -> bool {
    self.playing() && self.mode() == PlayMode::Playlist
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

  // how long the playlist plays before jumping back to the start, or None
  // when looping is off
  pub fn loop_len(&self, beat_interval: Duration) -> Option<Duration> {
    if !self.loop_enabled.load(atomic::Ordering::SeqCst) { return None; }

    let beats = self.max_beats.load(atomic::Ordering::SeqCst);
    let len = beat_interval * beats as u32;
    if len.is_zero() { None } else { Some(len) }
  }
}

// positions closer than this to a beat count as on it, so a playhead set from
// a beat number (and float rounding) doesn't leave a sliver before the beat
const ON_BEAT_TOLERANCE: Duration = Duration::from_millis(1);

// the offset moved onto the beat it's within ON_BEAT_TOLERANCE of, if any
pub fn snap_to_beat(offset: Duration, beat_interval: Duration) -> Duration {
  let interval = beat_interval.as_nanos();
  if interval == 0 { return offset; }

  let into_beat = offset.as_nanos() % interval;
  let tolerance = ON_BEAT_TOLERANCE.as_nanos();
  if into_beat <= tolerance {
    offset - Duration::from_nanos(into_beat as u64)
  } else if interval - into_beat <= tolerance {
    offset + Duration::from_nanos((interval - into_beat) as u64)
  } else {
    offset
  }
}

// time from the offset to the next beat, zero when it's on a beat
pub fn until_next_beat(offset: Duration, beat_interval: Duration) -> Duration {
  let interval = beat_interval.as_nanos();
  if interval == 0 { return Duration::ZERO; }

  let into_beat = offset.as_nanos() % interval;
  if into_beat == 0 { Duration::ZERO } else { Duration::from_nanos((interval - into_beat) as u64) }
}

// where the timeline is `elapsed` into a run that started at `start`. a run
// always starts inside the loop (see daw::start_playlist), so with looping
// on, this is the time past the last jump back to the start
pub fn timeline_position(start: Duration, elapsed: Duration, loop_len: Option<Duration>) -> Duration {
  let t = start + elapsed;
  match loop_len {
    Some(len) if t >= len => Duration::from_nanos((t.as_nanos() % len.as_nanos()) as u64),
    _ => t,
  }
}

// the timeline offset after playing `slice` from `offset`, jumping back to the
// start at the loop's end (the same jump timeline_position works out)
pub fn advance(offset: Duration, slice: Duration, loop_len: Option<Duration>) -> Duration {
  let next = offset + slice;
  match loop_len {
    // started past the end (looping was turned on mid-run): back to the top
    Some(len) if offset >= len => Duration::ZERO,
    Some(len) if next >= len => next - len,
    _ => next,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const BEAT: Duration = Duration::from_millis(500);
  fn ms(n: u64) -> Duration { Duration::from_millis(n) }

  #[test]
  fn test_snap_to_beat() {
    assert_eq!(snap_to_beat(ms(1_000), BEAT), ms(1_000));
    // float noise either side of a beat lands on it
    assert_eq!(snap_to_beat(Duration::from_micros(1_000_400), BEAT), ms(1_000));
    assert_eq!(snap_to_beat(Duration::from_micros(999_600), BEAT), ms(1_000));
    // anywhere else is left alone
    assert_eq!(snap_to_beat(ms(1_250), BEAT), ms(1_250));
  }

  #[test]
  fn test_until_next_beat() {
    assert_eq!(until_next_beat(ms(0), BEAT), ms(0));
    assert_eq!(until_next_beat(ms(1_000), BEAT), ms(0));
    assert_eq!(until_next_beat(ms(1_200), BEAT), ms(300));
    assert_eq!(until_next_beat(ms(1_499), BEAT), ms(1));
  }

  #[test]
  fn test_timeline_position() {
    let lp = Some(ms(8_000));
    // no loop: just keeps going
    assert_eq!(timeline_position(ms(1_500), ms(10_000), None), ms(11_500));
    // inside the loop
    assert_eq!(timeline_position(ms(1_500), ms(2_000), lp), ms(3_500));
    // past the end, back from the top
    assert_eq!(timeline_position(ms(1_500), ms(7_000), lp), ms(500));
    assert_eq!(timeline_position(ms(0), ms(16_250), lp), ms(250));
  }

  #[test]
  fn test_advance_matches_timeline_position() {
    // a run from a paused point: a part beat up to the beat, then whole beats
    let lp = Some(ms(8_000));
    let start = ms(6_800);
    let mut offset = start;
    let mut elapsed = until_next_beat(start, BEAT);
    offset = advance(offset, elapsed, lp);
    assert_eq!(offset, ms(7_000));

    for _ in 0..40 {
      offset = advance(offset, BEAT, lp);
      elapsed += BEAT;
      assert_eq!(offset, timeline_position(start, elapsed, lp));
    }
  }

  #[test]
  fn test_advance_past_loop_end_starts_over() {
    assert_eq!(advance(ms(9_000), BEAT, Some(ms(8_000))), ms(0));
    assert_eq!(advance(ms(9_000), BEAT, None), ms(9_500));
  }
}