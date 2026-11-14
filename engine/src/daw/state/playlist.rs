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
  // the playlist's length, in beats (read with daw::playlist_beats, worked
  // out by daw::fit_playlist)
  pub beats: u64,
  pub snap_enabled: bool,
  pub snap_subdivision: daw::timing::Note,
}

impl UI for PlaylistUI {
  fn new() -> Self {
    PlaylistUI {
      viewport: app::workspaces::WorkspaceViewport::new(),
      // a new playlist's: in 4/4, with no clips
      beats: daw::state::defaults::PLAYLIST_BARS * 4,
      snap_enabled: daw::state::defaults::SNAP_ENABLED,
      snap_subdivision: daw::state::defaults::SNAP_SUBDIVISION,
    }
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
  pub started_time: Arc<Mutex<Option<Instant>>>,
  // where on the timeline the current run started, or where the next one
  // will start (the playhead while stopped or paused)
  pub start_offset: Mutex<Duration>,
  // bumped by every run and every stop, so a run's thread can tell it's been
  // replaced (see daw::run_playlist)
  pub run_id: AtomicU64,
  pub total_beats: AtomicU64,
  pub current_beat: AtomicU16,
  pub time_signature: Arc<Mutex<timing::TimeSignature>>,
  pub loop_enabled: AtomicBool,
  // the part of the playlist that loops, drawn in the ruler (None: from the
  // start up to the end flag)
  pub loop_region: Mutex<Option<LoopRegion>>,
  // where the end flag was dragged to, in beats (None: it follows the last
  // clip). a region loops instead while there is one
  pub loop_end_pin: Mutex<Option<f64>>,
  // whether recording counts in a bar first
  pub count_in: AtomicBool,
  pub audiograph: Arc<Mutex<daw::AudioGraph<'static>>>,
  // the same mixer the audio graph plays clips through, reachable without
  // the graph's lock
  pub mixer: Arc<daw::Mixer>,
  // the same sequencer the audio graph plays pattern clips from. lock it
  // after the graph, never before
  pub sequencer: Arc<Mutex<daw::Sequencer>>,
  // each mixer channel's effects (see effects.rs)
  pub effects: Arc<crate::effects::Effects>,
  // keys held on sample rows, auditioned (see commands/synth.rs)
  pub held_keys: daw::HeldKeys,
  // what the current (or last) run plays. set before each run starts
  pub mode: Mutex<PlayMode>,
  // the step a pattern playing on its own counts from, with
  // `started_time` when it's heard: the step its run started from (the
  // Piano Roll's start marker), then the one it's on each time it comes
  // round or its length changes (see daw::next_pattern_slice). in steps, so
  // it stays on its step if the tempo changes
  pub pattern_start: Mutex<f64>,
  pub ui: Arc<Mutex<PlaylistUI>>,
}

impl Playlist {
  pub fn _new() -> Option<Self> {
    None
  }

  pub fn default() -> Self {
    let audiograph = daw::AudioGraph::new(daw::state::defaults::SAMPLE_RATE, daw::state::defaults::TEMPO);
    let mixer = audiograph.mixer.clone();
    let sequencer = audiograph.sequencer.clone();
    let effects = Arc::new(crate::effects::Effects::new(mixer.clone()));

    Playlist {
      playing: AtomicBool::from(false),
      started_time: Arc::new(Mutex::from(None)),
      start_offset: Mutex::new(Duration::ZERO),
      run_id: AtomicU64::from(0),
      total_beats: AtomicU64::from(0),
      current_beat: AtomicU16::from(0),
      time_signature: Arc::new(Mutex::new(
        timing::TimeSignature {
          numerator: 4,
          denominator: 4,
        },
      )),
      loop_enabled: AtomicBool::from(true),
      loop_region: Mutex::new(None),
      loop_end_pin: Mutex::new(None),
      count_in: AtomicBool::from(false),
      audiograph: Arc::new(Mutex::new(audiograph)),
      mixer,
      sequencer,
      effects,
      held_keys: daw::HeldKeys::default(),
      mode: Mutex::new(PlayMode::Playlist),
      pattern_start: Mutex::new(0.),

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

}

// where the loop ends: the end of the last clip, on the next bar line (or on
// it, if it ends on one). with no clips, a few bars in
// - song_end: where the last clip finishes (zero with no clips)
pub fn loop_end_after(song_end: Duration, beat_interval: Duration, beats_per_bar: u16) -> Duration {
  let bar = beat_interval * beats_per_bar.max(1) as u32;
  if bar.is_zero() { return Duration::ZERO; }
  if song_end.is_zero() { return bar * daw::state::defaults::EMPTY_LOOP_BARS; }

  // a clip ending a hair past a bar line (float noise) ends on it
  let end = song_end.saturating_sub(ON_BEAT_TOLERANCE).as_nanos();
  let bars = (end / bar.as_nanos() + 1) as u32;
  bar * bars
}

// how many beats long the playlist is: PLAYLIST_BARS bars, or up to the beat
// the last clip ends on (or in) when that's later, so a change to a shorter
// bar leaves no clip past the end
// - song_end: where the last clip finishes (zero with no clips)
pub fn playlist_beats_for(song_end: Duration, beat_interval: Duration, beats_per_bar: u16) -> u64 {
  let bars = daw::state::defaults::PLAYLIST_BARS * beats_per_bar.max(1) as u64;
  if beat_interval.is_zero() { return bars; }

  // a clip ending a hair past a beat (float noise) ends on it
  let end = song_end.saturating_sub(ON_BEAT_TOLERANCE).as_nanos();
  let song_beats = end.div_ceil(beat_interval.as_nanos()) as u64;
  bars.max(song_beats)
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

// a part of the playlist drawn in the ruler, that playback loops while
// looping is on: from `start` up to `end`, in beats, so a tempo change keeps
// it on the same beats. start is always before end
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct LoopRegion {
  pub start: f64,
  pub end: f64,
}

// edges closer than this, in beats, are on the same beat
pub const SAME_BEAT: f64 = 1e-6;

impl LoopRegion {
  // the region between two edges dragged in the ruler, in either order,
  // kept to the playlist's `beats`. None when they're on the same beat:
  // that's no region
  pub fn between(a: f64, b: f64, beats: f64) -> Option<LoopRegion> {
    if !a.is_finite() || !b.is_finite() { return None; }
    let beats = beats.max(0.);
    LoopRegion { start: a.min(b).clamp(0., beats), end: a.max(b).clamp(0., beats) }.valid()
  }

  // the region, if it makes sense (one read from a file may not)
  pub fn valid(self) -> Option<LoopRegion> {
    let fine = self.start.is_finite() && self.end.is_finite() && self.start >= 0. && self.end - self.start >= SAME_BEAT;
    fine.then_some(self)
  }
}

// where the loop's end flag can be pinned: at least a beat in, so the loop
// is never empty. None for a beat that makes no sense
pub fn valid_loop_end_pin(beat: f64) -> Option<f64> {
  beat.is_finite().then(|| beat.max(1.))
}

// where the loop starts and ends on the timeline (see daw::loop_span)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopSpan {
  pub start: Duration,
  pub end: Duration,
}

impl LoopSpan {
  pub fn len(&self) -> Duration {
    self.end.saturating_sub(self.start)
  }
}

// where the timeline is `elapsed` into a run that started at `start`. a run
// starts inside the loop or before it (see daw::start_playlist), so with
// looping on, this plays in up to the loop's end, then goes round from its
// start
pub fn timeline_position(start: Duration, elapsed: Duration, span: Option<LoopSpan>) -> Duration {
  let t = start + elapsed;
  match span {
    Some(span) if t >= span.end && !span.len().is_zero() =>
      span.start + Duration::from_nanos(((t - span.end).as_nanos() % span.len().as_nanos()) as u64),
    _ => t,
  }
}

// the timeline offset after playing `slice` from `offset`, going back to the
// loop's start at its end (the same jump timeline_position works out), and
// whether it went back
pub fn advance(offset: Duration, slice: Duration, span: Option<LoopSpan>) -> (Duration, bool) {
  let next = offset + slice;
  match span {
    // started past the end (looping was turned on mid-run, or the loop moved
    // behind the playhead): back to its start
    Some(span) if offset >= span.end => (span.start, true),
    Some(span) if next >= span.end => {
      let over = next - span.end;
      let len = span.len().as_nanos().max(1);
      (span.start + Duration::from_nanos((over.as_nanos() % len) as u64), true)
    }
    _ => (next, false),
  }
}

// how long the slice of a run from `offset` is: up to the next beat (a whole
// beat from one), and no further than the loop's end, so the loop comes
// round where it ends, on a beat or not
pub fn slice_len(offset: Duration, beat_interval: Duration, span: Option<LoopSpan>) -> Duration {
  let lead = until_next_beat(offset, beat_interval);
  let len = if lead.is_zero() { beat_interval } else { lead };
  match span {
    Some(span) if offset < span.end => len.min(span.end - offset),
    _ => len,
  }
}

// where a pattern playing on its own, `len` long now, has got to after
// playing `slice` from `offset`: round to its start at its end. made longer,
// it carries on into its new bars; made shorter than `offset`, it carries on
// where that falls in its new length, as the cursors (which count its steps
// the same way) have it
pub fn advance_round(offset: Duration, slice: Duration, len: Duration) -> Duration {
  if len.is_zero() { return Duration::ZERO; }
  Duration::from_nanos(((offset + slice).as_nanos() % len.as_nanos()) as u64)
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
  fn test_loop_end_after() {
    // 120bpm, 4/4: a bar is 2s
    let end = |song_ms: u64| loop_end_after(ms(song_ms), BEAT, 4);

    // no clips: four bars
    assert_eq!(end(0), ms(8_000));
    // the last clip ends mid-bar: the next bar line
    assert_eq!(end(5_000), ms(6_000));
    assert_eq!(end(100), ms(2_000));
    // on a bar line (or a hair past it): that line
    assert_eq!(end(4_000), ms(4_000));
    assert_eq!(loop_end_after(Duration::from_micros(4_000_400), BEAT, 4), ms(4_000));
    assert_eq!(end(4_002), ms(6_000));
    // three beats to the bar
    assert_eq!(loop_end_after(ms(5_000), BEAT, 3), ms(6_000));
    assert_eq!(loop_end_after(ms(0), BEAT, 3), ms(6_000));
  }

  #[test]
  fn test_playlist_beats_for() {
    // 120bpm: a beat is 500ms
    let beats = |song_ms: u64, beats_per_bar: u16| playlist_beats_for(ms(song_ms), BEAT, beats_per_bar);

    // 128 bars, of four beats or three
    assert_eq!(beats(0, 4), 512);
    assert_eq!(beats(0, 3), 384);
    // a clip ending inside the 128 bars changes nothing
    assert_eq!(beats(150_000, 3), 384);
    // one ending past them: up to the beat it ends in, or on
    assert_eq!(beats(200_100, 3), 401);
    assert_eq!(beats(200_000, 3), 400);
    assert_eq!(playlist_beats_for(Duration::from_micros(200_000_400), BEAT, 3), 400);
    // (no beat length to measure the clips by: just the bars)
    assert_eq!(playlist_beats_for(ms(900_000), Duration::ZERO, 4), 512);
  }

  #[test]
  fn test_until_next_beat() {
    assert_eq!(until_next_beat(ms(0), BEAT), ms(0));
    assert_eq!(until_next_beat(ms(1_000), BEAT), ms(0));
    assert_eq!(until_next_beat(ms(1_200), BEAT), ms(300));
    assert_eq!(until_next_beat(ms(1_499), BEAT), ms(1));
  }

  fn span(start: u64, end: u64) -> Option<LoopSpan> {
    Some(LoopSpan { start: ms(start), end: ms(end) })
  }

  #[test]
  fn test_timeline_position() {
    let lp = span(0, 8_000);
    // no loop: just keeps going
    assert_eq!(timeline_position(ms(1_500), ms(10_000), None), ms(11_500));
    // inside the loop
    assert_eq!(timeline_position(ms(1_500), ms(2_000), lp), ms(3_500));
    // past the end, back from the top
    assert_eq!(timeline_position(ms(1_500), ms(7_000), lp), ms(500));
    assert_eq!(timeline_position(ms(0), ms(16_250), lp), ms(250));
  }

  #[test]
  fn test_timeline_position_in_a_region() {
    let lp = span(2_000, 4_000);
    // from before the region: plays in, then goes round inside it
    assert_eq!(timeline_position(ms(500), ms(1_000), lp), ms(1_500));
    assert_eq!(timeline_position(ms(500), ms(3_400), lp), ms(3_900));
    assert_eq!(timeline_position(ms(500), ms(3_500), lp), ms(2_000));
    assert_eq!(timeline_position(ms(500), ms(6_250), lp), ms(2_750));
    // from inside it
    assert_eq!(timeline_position(ms(3_000), ms(1_500), lp), ms(2_500));
  }

  #[test]
  fn test_advance_matches_timeline_position() {
    // a run from a paused point: a part beat up to the beat, then whole beats
    for lp in [span(0, 8_000), span(2_000, 8_000), span(1_250, 7_100)] {
      let start = ms(6_800);
      let mut offset = start;
      let mut elapsed = Duration::ZERO;

      for _ in 0..40 {
        let slice = slice_len(offset, BEAT, lp);
        offset = advance(offset, slice, lp).0;
        elapsed += slice;
        assert_eq!(offset, timeline_position(start, elapsed, lp));
      }
    }
  }

  #[test]
  fn test_advance_says_when_it_goes_round() {
    assert_eq!(advance(ms(1_000), BEAT, span(0, 8_000)), (ms(1_500), false));
    assert_eq!(advance(ms(7_500), BEAT, span(0, 8_000)), (ms(0), true));
    assert_eq!(advance(ms(3_500), BEAT, span(2_000, 4_000)), (ms(2_000), true));
    // a loop one slice long goes round every slice (back where it was)
    assert_eq!(advance(ms(2_000), BEAT, span(2_000, 2_500)), (ms(2_000), true));
    // without looping it never does
    assert_eq!(advance(ms(7_500), BEAT, None), (ms(8_000), false));
  }

  #[test]
  fn test_advance_past_loop_end_starts_over() {
    assert_eq!(advance(ms(9_000), BEAT, span(0, 8_000)), (ms(0), true));
    assert_eq!(advance(ms(9_000), BEAT, span(2_000, 8_000)), (ms(2_000), true));
    assert_eq!(advance(ms(9_000), BEAT, None), (ms(9_500), false));
  }

  #[test]
  fn test_a_slice_ends_on_the_next_beat_or_the_loop_end() {
    // a beat from a beat, up to the next from between them
    assert_eq!(slice_len(ms(1_000), BEAT, None), BEAT);
    assert_eq!(slice_len(ms(1_200), BEAT, None), ms(300));
    // no further than the loop's end, when that's off the beat
    assert_eq!(slice_len(ms(3_000), BEAT, span(2_000, 3_250)), ms(250));
    assert_eq!(slice_len(ms(1_000), BEAT, span(2_000, 3_250)), BEAT);
    // past the end (it's about to go round), as without one
    assert_eq!(slice_len(ms(4_000), BEAT, span(2_000, 3_250)), BEAT);
  }

  #[test]
  fn test_a_region_between_two_edges() {
    let region = |start, end| Some(LoopRegion { start, end });
    assert_eq!(LoopRegion::between(4., 8., 64.), region(4., 8.));
    // in either order
    assert_eq!(LoopRegion::between(8., 4., 64.), region(4., 8.));
    // kept to the playlist
    assert_eq!(LoopRegion::between(-2., 70., 64.), region(0., 64.));
    // on the same beat (or past the end on both sides): none
    assert_eq!(LoopRegion::between(4., 4., 64.), None);
    assert_eq!(LoopRegion::between(4., 4. + SAME_BEAT / 2., 64.), None);
    assert_eq!(LoopRegion::between(70., 80., 64.), None);
    assert_eq!(LoopRegion::between(f64::NAN, 4., 64.), None);
  }

  #[test]
  fn test_a_loop_end_pin_is_at_least_a_beat_in() {
    assert_eq!(valid_loop_end_pin(12.), Some(12.));
    assert_eq!(valid_loop_end_pin(0.25), Some(1.));
    assert_eq!(valid_loop_end_pin(f64::INFINITY), None);
  }

  #[test]
  fn test_a_pattern_goes_round_at_its_length_now() {
    // a bar is 2s: round at its end
    assert_eq!(advance_round(ms(1_000), BEAT, ms(2_000)), ms(1_500));
    assert_eq!(advance_round(ms(1_500), BEAT, ms(2_000)), ms(0));
    // made two bars long, it carries on into the second
    assert_eq!(advance_round(ms(1_500), BEAT, ms(4_000)), ms(2_000));
    // made a bar long from the second: where that falls in the first
    assert_eq!(advance_round(ms(2_500), BEAT, ms(2_000)), ms(1_000));
    assert_eq!(advance_round(ms(500), BEAT, Duration::ZERO), ms(0));
  }
}