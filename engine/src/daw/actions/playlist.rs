// the playlist's tracks and timeline: where a beat falls, where a clip
// starts, the snap setting, and adding a track. commands/playlist.rs is the
// frontend's side of these
use std::time::Duration;

use crate::daw::{playback, timing};
use crate::daw;

// the snap setting, so the frontend can preview where a dragged clip will land
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapSetting {
  enabled: bool,
  // the note value clips snap to
  note: timing::Note,
  // length of one snap step, in beats
  beats: f32,
}

pub(crate) fn snap_setting(state: &daw::InnerState) -> SnapSetting {
  let ui = state.playlist.ui.lock().unwrap();
  let (num, den) = timing::MusicalTiming::ratio(&ui.snap_subdivision);

  SnapSetting {
    enabled: ui.snap_enabled,
    note: ui.snap_subdivision,
    beats: num as f32 / den as f32,
  }
}

// the number of beats the playlist shows, and the time they span
// at the current tempo
pub(crate) fn displayed_beats_and_duration(
  state: &daw::InnerState,
) -> (f32, std::time::Duration) {
  let tempo = state.playlist.audiograph.lock().unwrap().tempo();
  let beats = playback::playlist_beats(state);
  let dur = timing::n_beat_duration_from_tempo(tempo, beats as u32);

  (beats as f32, dur)
}

// convert a drop position, in beats from the start of the playlist,
// into a start offset
// positions are in beats rather than pixels, so they don't depend on the
// canvas size, zoom, or pan, which the frontend accounts for
pub(crate) fn beat_to_offset(
  state: &daw::InnerState,
  beat: Option<f32>,
) -> std::time::Duration {
  let (beats, dur) = displayed_beats_and_duration(state);

  calc_sample_offset(beat.unwrap_or(0.), beats, dur)
}

// convert a position along the playlist into a start offset
// - position: distance from the playlist's start (e.g. pixels or beats)
// - playlist_length: length of the whole playlist, in the same unit
// - max_playlist_dur: time spanned by the whole playlist
// positions outside the playlist are clamped to its start/end
fn calc_sample_offset(
  position: f32,
  playlist_length: f32,
  max_playlist_dur: Duration,
) -> Duration {
  if !(playlist_length > 0.) || !position.is_finite() {
    return Duration::ZERO;
  }

  let ratio = (position / playlist_length).clamp(0., 1.);
  let dur = max_playlist_dur.mul_f64(ratio as f64);

  println!(
    "position: {}, playlist length: {}, ratio: {}, max dur: {}ms, offset: {}ms",
    position, playlist_length, ratio, max_playlist_dur.as_millis(), dur.as_millis());

  dur
}

// the inverse of beat_to_offset: how many beats into the playlist
// a start offset is
pub(crate) fn offset_to_beat(
  state: &daw::InnerState,
  offset: std::time::Duration,
) -> f32 {
  let interval = state.playlist.audiograph.lock().unwrap().beat_interval();

  if interval.is_zero() { return 0.; }

  (offset.as_secs_f64() / interval.as_secs_f64()) as f32
}

// where a node starts, in beats from the start of the playlist, after snapping
pub(crate) fn node_beat(state: &daw::InnerState, id: u64) -> Result<f32, String> {
  let (offset, _) = state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .placement(id)
    .ok_or_else(|| format!("no node with id {}", id))?;

  Ok(offset_to_beat(state, offset))
}

pub fn add_track_to(state: &daw::InnerState) -> u32 {
  let (before, count) = {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let before = audiograph.min_tracks;
    (before, audiograph.add_track())
  };
  crate::daw::history::record(state, crate::daw::history::Step::new().with_tracks(before, count));
  count
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_snap_setting() {
    let state = daw::InnerState::default();

    use timing::Note;

    // default: snap on, one beat
    assert_eq!(
      snap_setting(&state),
      SnapSetting { enabled: true, note: Note::QuarterNote, beats: 1. });

    state.playlist.ui.lock().unwrap().snap_subdivision = Note::SixteenthNote;
    state.playlist.ui.lock().unwrap().toggle_snap_enabled();
    assert_eq!(
      snap_setting(&state),
      SnapSetting { enabled: false, note: Note::SixteenthNote, beats: 0.25 });

    // the frontend sends and receives notes by name
    let note: Note = serde_json::from_str("\"EighthNote\"").unwrap();
    assert_eq!(note, Note::EighthNote);
    assert_eq!(serde_json::to_string(&Note::WholeNote).unwrap(), "\"WholeNote\"");
  }

  #[test]
  fn test_beat_to_offset() {
    let state = daw::InnerState::default();

    // default: 120bpm, a playlist of 512 beats (128 bars)
    let ms = |b: Option<f32>| beat_to_offset(&state, b).as_millis();
    assert_eq!(ms(Some(0.)), 0);
    assert_eq!(ms(Some(1.)), 500);
    assert_eq!(ms(Some(8.)), 4_000);
    assert_eq!(ms(Some(1.5)), 750);
    assert_eq!(ms(None), 0);

    // drops outside the playlist are clamped to its start/end
    assert_eq!(ms(Some(-2.)), 0);
    assert_eq!(ms(Some(40.)), 20_000);
    assert_eq!(ms(Some(600.)), 256_000);

    // a beat is longer at a slower tempo
    state.set_tempo(60.);
    assert_eq!(ms(Some(8.)), 8_000);
  }

  #[test]
  fn test_calc_sample_offset() {
    let max = Duration::from_millis(8_000); // 16 beats at 120bpm
    let width = 1_000.;

    assert_eq!(calc_sample_offset(0., width, max).as_millis(), 0);
    assert_eq!(calc_sample_offset(500., width, max).as_millis(), 4_000);
    assert_eq!(calc_sample_offset(250., width, max).as_millis(), 2_000);
    assert_eq!(calc_sample_offset(1_000., width, max).as_millis(), 8_000);
  }

  #[test]
  fn test_calc_sample_offset_clamps_and_rejects_bad_input() {
    let max = Duration::from_millis(8_000);

    // outside the playlist is clamped to its edges
    assert_eq!(calc_sample_offset(-50., 1_000., max), Duration::ZERO);
    assert_eq!(calc_sample_offset(1_500., 1_000., max), max);

    // zero/negative width or non-finite positions don't divide by zero
    assert_eq!(calc_sample_offset(100., 0., max), Duration::ZERO);
    assert_eq!(calc_sample_offset(100., -10., max), Duration::ZERO);
    assert_eq!(calc_sample_offset(f32::NAN, 1_000., max), Duration::ZERO);
  }
}
