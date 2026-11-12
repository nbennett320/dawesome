use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use crate::{daw, history};

#[tauri::command]
pub fn toggle_playlist(state: daw::TState<'_>) {
  if state.playlist.playing() {
    daw::pause_playlist(state.inner());
  } else {
    // start playlist
    daw::start_playlist(state.inner());
  }
}

// play from the playhead (nothing if already playing)
#[tauri::command]
pub fn play_playlist(state: daw::TState<'_>) {
  // a pattern playing on its own is replaced by the playlist
  if !state.playlist.playing_playlist() {
    daw::start_playlist(state.inner());
  }
}

// stop where it is, so play carries on from there
#[tauri::command]
pub fn pause_playlist(state: daw::TState<'_>) {
  if state.playlist.playing() {
    daw::pause_playlist(state.inner());
  }
}

// stop and go back to the start
#[tauri::command]
pub fn stop_playlist(state: daw::TState<'_>) {
  daw::stop_playlist(state.inner());
}

// move the playhead to a beat (kept within the playlist), carrying on playing
// from there if playing. returns the beat it moved to
#[tauri::command]
pub fn seek_playlist(state: daw::TState<'_>, beat: f64) -> f64 {
  daw::seek_playlist(state.inner(), beat)
}

#[tauri::command]
pub fn get_playlist_playing(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.playing())
}

// convert the engine's monotonic start time into wall clock ms
// (since the unix epoch), which the frontend can compare to Date.now()
fn wall_clock_ms(started: Instant) -> Result<f64, String> {
  let now = Instant::now();
  // UNIX_EPOCH is the epoch on every platform, including windows
  let wall = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .map_err(|e| e.to_string())?
    .as_secs_f64() * 1000.;

  // the start time is when the run's first frame is heard, which is just
  // after it starts (by the output's latency)
  Ok(if started >= now {
    wall + started.duration_since(now).as_secs_f64() * 1000.
  } else {
    wall - now.duration_since(started).as_secs_f64() * 1000.
  })
}

// everything the frontend needs to draw the playback cursor in sync with
// the engine's beat loop, without polling every frame
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTransport {
  playing: bool,
  // engine start time of the current run, ms since the unix epoch (None
  // before the first run)
  start_time_ms: Option<f64>,
  // where on the timeline the current run started, in ms. while playing,
  // the playhead is this plus the time since `start_time_ms` (wrapped at the
  // loop's end)
  start_offset_ms: f64,
  // where the playhead is now, in ms
  position_ms: f64,
  // length of one beat tick in the engine loop
  beat_interval_ms: f64,
  // beats played before the engine loops back to the start
  loop_beats: u64,
  // beats spanned by the full playlist width
  beats_displayed: u64,
  loop_enabled: bool,
  // where the last clip finishes, in ms (0 with no clips)
  song_end_ms: f64,
  // the time signature's numerator
  beats_per_bar: u16,
  // the pattern playing on its own (Play in the Sequencer tab), or None
  // when the playlist is playing or nothing is. while a pattern plays, the
  // playlist's playhead stays still
  pattern_id: Option<u64>,
  // how long that pattern is, in ms. it started at `start_time_ms` and
  // loops at this length
  pattern_length_ms: f64,
}

fn playlist_transport(state: &daw::InnerState) -> PlaylistTransport {
  let beat_interval = daw::beat_interval(state);
  // read as a pair: both move when the loop comes round
  let (start_offset, started) = daw::run_anchor(state);
  // (worked out before the struct, whose fields hold other locks as they're read)
  let loop_beats = daw::offset_beats(daw::loop_end(state), beat_interval).round() as u64;

  PlaylistTransport {
    playing: state.playlist.playing(),
    start_time_ms: started.and_then(|started| wall_clock_ms(started).ok()),
    start_offset_ms: start_offset.as_secs_f64() * 1000.,
    position_ms: daw::playhead(state).as_secs_f64() * 1000.,
    beat_interval_ms: beat_interval.as_secs_f64() * 1000.,
    loop_beats,
    beats_displayed: state.playlist.ui.lock().unwrap().max_beats_displayed,
    loop_enabled: state.playlist.loop_enabled.load(Ordering::SeqCst),
    song_end_ms: daw::song_end(state).unwrap_or_default().as_secs_f64() * 1000.,
    beats_per_bar: state.playlist.time_signature.lock().unwrap().numerator,
    pattern_id: state.playlist.playing_pattern(),
    pattern_length_ms: state.playlist.playing_pattern()
      .and_then(|id| daw::pattern_len(state, id))
      .map_or(0., |len| len.as_secs_f64() * 1000.),
  }
}

#[tauri::command]
pub fn get_playlist_transport(
  state: daw::TState<'_>
) -> PlaylistTransport {
  playlist_transport(state.inner())
}

// change the tempo (see daw::set_playlist_tempo), as a step that can be
// undone. refused while recording
// - gesture: the visit to the tempo field it's part of, if any
pub fn change_tempo(state: &Arc<daw::InnerState>, tempo: f32, gesture: Option<u64>) -> Result<f32, String> {
  let before = state.tempo();
  let after = daw::set_playlist_tempo(state, tempo)?;
  history::record(state, history::Step::new().in_gesture(gesture).with_tempo(before, after));
  Ok(after)
}

#[tauri::command]
pub fn set_playlist_tempo(
  state: daw::TState<'_>,
  val: f32,
  gesture: Option<u64>,
) -> Result<f32, String> {
  println!("playlist tempo updated: {}", val);
  change_tempo(state.inner(), val, gesture)
}

#[tauri::command]
pub fn toggle_metronome_enabled(
  state: daw::TState<'_>
) {
  let val = !state.metronome_enabled.load(Ordering::SeqCst);
  state.metronome_enabled.store(val, Ordering::SeqCst);
}

// whether recording counts in a bar first. returns the new setting
#[tauri::command]
pub fn toggle_count_in(state: daw::TState<'_>) -> bool {
  !state.playlist.count_in.fetch_xor(true, Ordering::SeqCst)
}

#[tauri::command]
pub fn get_metronome_enabled(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.metronome_enabled.load(Ordering::SeqCst))
}

#[tauri::command]
pub fn set_playlist_time_signature(
  state: daw::TState<'_>,
  numerator: u16,
  denominator: u16,
) {
  let updated: daw::timing::TimeSignature = daw::timing::TimeSignature {
    numerator,
    denominator,
  };

  *state.playlist.time_signature.lock().unwrap() = updated;
}

#[tauri::command]
pub fn toggle_loop_enabled(
  state: daw::TState<'_>
) {
  let val = !state
    .playlist
    .loop_enabled
    .load(Ordering::SeqCst);

  state
    .playlist
    .loop_enabled
    .store(val, Ordering::SeqCst);
}

#[tauri::command]
pub fn get_loop_enabled(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.loop_enabled.load(Ordering::SeqCst))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;
  use crate::clip_commands::{add_node_at_offset, move_audiograph_node_to};
  use crate::playlist_commands::{beat_to_offset, node_beat};

  #[test]
  fn test_start_time_can_be_just_ahead() {
    let state = daw::InnerState::default();
    let wall = |t: SystemTime| t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() * 1000.;

    // a run that's heard 20ms from now
    *state.playlist.started_time.lock().unwrap() = Some(Instant::now() + Duration::from_millis(20));
    let ahead = playlist_transport(&state).start_time_ms.unwrap() - wall(SystemTime::now());
    // (the wall clock and Instant are separate clocks, so allow a little)
    assert!(ahead > 15. && ahead < 21., "{}ms ahead", ahead);

    // and one heard 1.5s ago
    *state.playlist.started_time.lock().unwrap() = Some(Instant::now() - Duration::from_millis(1_500));
    let ago = wall(SystemTime::now()) - playlist_transport(&state).start_time_ms.unwrap();
    assert!(ago > 1_499. && ago < 1_505., "{}ms ago", ago);
  }

  #[test]
  fn test_playlist_transport() {
    let state = daw::InnerState::default();

    // before the playlist has been started there's no start time, but the
    // playhead is still reported
    let t = playlist_transport(&state);
    assert_eq!(t.start_time_ms, None);
    assert!(!t.playing);
    assert_eq!(t.position_ms, 0.);
    assert_eq!(t.song_end_ms, 0.);
    assert_eq!(t.beats_per_bar, 4);

    let started = Instant::now() - Duration::from_millis(1_500);
    *state.playlist.started_time.lock().unwrap() = Some(started);

    let t = playlist_transport(&state);
    assert_eq!(t.beat_interval_ms, 500.); // 120bpm
    // nothing in the playlist: the loop is four bars
    assert_eq!(t.loop_beats, 16);
    assert_eq!(t.beats_displayed, 256);
    assert!(t.loop_enabled);

    // start time is reported in wall clock ms, 1.5s in the past
    let now_ms = SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_secs_f64() * 1000.;
    assert!((now_ms - t.start_time_ms.unwrap() - 1_500.).abs() < 50.);

    state.set_tempo(60.);
    assert_eq!(playlist_transport(&state).beat_interval_ms, 1_000.);
  }

  // pretend a run started `ago_ms` back, from `from_ms` into the playlist
  // (starting a real run needs an audio device)
  fn fake_run(state: &daw::InnerState, from_ms: u64, ago_ms: u64) {
    *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(from_ms);
    *state.playlist.started_time.lock().unwrap() = Some(Instant::now() - Duration::from_millis(ago_ms));
    state.playlist.set_playing(true);
  }

  fn playhead_ms(state: &daw::InnerState) -> f64 {
    daw::playhead(state).as_secs_f64() * 1000.
  }

  #[test]
  fn test_playhead_follows_the_run() {
    let state = daw::InnerState::default();

    // stopped: wherever it was left
    *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(750);
    assert_eq!(playhead_ms(&state), 750.);

    // playing from 2s, 1.25s ago
    fake_run(&state, 2_000, 1_250);
    assert!((playhead_ms(&state) - 3_250.).abs() < 50.);
    assert!((playlist_transport(&state).position_ms - 3_250.).abs() < 50.);
    assert_eq!(playlist_transport(&state).start_offset_ms, 2_000.);

    // the 16 beat (8s) loop jumps back to the start
    fake_run(&state, 7_000, 2_000);
    assert!((playhead_ms(&state) - 1_000.).abs() < 50.);

    // without looping it runs on
    state.playlist.loop_enabled.store(false, Ordering::SeqCst);
    assert!((playhead_ms(&state) - 9_000.).abs() < 50.);
  }

  #[test]
  fn test_pause_keeps_the_playhead() {
    let state = Arc::new(daw::InnerState::default());
    fake_run(&state, 2_000, 1_250);

    daw::pause_playlist(&state);
    assert!(!state.playlist.playing());
    assert!((playhead_ms(&state) - 3_250.).abs() < 50.);
    // 3.25s is 6 beats in: the third beat of the second bar
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 6);
    assert_eq!(state.playlist.current_beat.load(Ordering::SeqCst), 2);
  }

  #[test]
  fn test_stop_goes_back_to_the_start() {
    let state = Arc::new(daw::InnerState::default());
    fake_run(&state, 2_000, 1_250);

    daw::stop_playlist(&state);
    assert!(!state.playlist.playing());
    assert_eq!(playhead_ms(&state), 0.);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 0);

    // stopping while stopped also rewinds
    *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(1_000);
    daw::stop_playlist(&state);
    assert_eq!(playhead_ms(&state), 0.);
  }

  #[test]
  fn test_stop_ends_the_run() {
    let state = Arc::new(daw::InnerState::default());
    let run = state.playlist.run_id.load(Ordering::SeqCst);

    // a run's thread checks the id before carrying on, so stopping (or
    // pausing) has to change it
    daw::stop_playlist(&state);
    assert_ne!(state.playlist.run_id.load(Ordering::SeqCst), run);
    let run = state.playlist.run_id.load(Ordering::SeqCst);
    daw::pause_playlist(&state);
    assert_ne!(state.playlist.run_id.load(Ordering::SeqCst), run);
  }

  #[test]
  fn test_seek_while_stopped() {
    let state = Arc::new(daw::InnerState::default());

    assert_eq!(daw::seek_playlist(&state, 4.), 4.);
    assert_eq!(playhead_ms(&state), 2_000.);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 4);
    assert_eq!(state.playlist.current_beat.load(Ordering::SeqCst), 0);
    assert!(!state.playlist.playing());

    // between beats
    assert_eq!(daw::seek_playlist(&state, 2.5), 2.5);
    assert_eq!(playhead_ms(&state), 1_250.);

    // float noise lands on the beat
    assert_eq!(daw::seek_playlist(&state, 3.000_000_1), 3.);

    // kept within the playlist (256 beats)
    assert_eq!(daw::seek_playlist(&state, -3.), 0.);
    assert_eq!(daw::seek_playlist(&state, 40.), 40.);
    assert_eq!(daw::seek_playlist(&state, 400.), 256.);
    assert_eq!(daw::seek_playlist(&state, f64::NAN), 0.);
  }

  #[test]
  fn test_clips_and_the_playhead_keep_their_beat_when_the_tempo_changes() {
    let state = Arc::new(daw::InnerState::default());
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let id = add_node_at_offset(&state, sample, 0, beat_to_offset(&state, Some(8.)), true).unwrap();
    let ms = |id: u64| state.playlist.audiograph.lock().unwrap()
      .get_node_by_id(id).unwrap().start_offset.as_secs_f64() * 1000.;
    let length = |id: u64| state.playlist.audiograph.lock().unwrap()
      .get_node_by_id(id).unwrap().duration();
    let played = length(id);
    daw::seek_playlist(&state, 9.);

    // 120bpm to 90: beat 8 is 5.33s in, rather than 4s
    assert_eq!(daw::set_playlist_tempo(&state, 90.), Ok(90.));
    assert!((node_beat(&state, id).unwrap() - 8.).abs() < 1e-5);
    assert!((ms(id) - 8. * 60_000. / 90.).abs() < 0.01);
    // the playhead is still on beat 9
    assert!((daw::offset_beats(daw::playhead(&state), daw::beat_interval(&state)) - 9.).abs() < 1e-5);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 9);
    // the clip plays the same audio, for as long
    assert_eq!(length(id), played);

    // and back again, without drifting
    daw::set_playlist_tempo(&state, 120.).unwrap();
    assert!((ms(id) - 4_000.).abs() < 0.001);
    assert_eq!(get_playlist_position_of(&state), 9.);

    // tempos that make no sense are refused, and nothing moves
    for bad in [0., 0.5, -30., f32::NAN, f32::INFINITY, 5_000., 1e-20] {
      assert!(daw::set_playlist_tempo(&state, bad).is_err());
    }
    assert_eq!(state.tempo(), 120.);
    assert!((ms(id) - 4_000.).abs() < 0.001);
    assert_eq!(get_playlist_position_of(&state), 9.);
  }

  #[test]
  fn test_playback_carries_on_from_its_beat_at_the_new_tempo() {
    let state = daw::InnerState::default();
    // a clip far enough along that the loop is long
    add_node_at_offset(&state, daw::METRONOME_TICK_PATH.to_string(), 0, beat_to_offset(&state, Some(60.)), true).unwrap();

    // playing from beat 4 (2s), 1.25s ago: beat 6.5
    fake_run(&state, 2_000, 1_250);
    assert_eq!(daw::retime(&state, 60.), Ok(daw::Retimed::Playlist));

    // the run was ended, and the playhead left on the beat it had got to, in
    // the new tempo's time (6.5s), ready to carry on from
    assert!(!state.playlist.playing());
    let beat = daw::offset_beats(daw::playhead(&state), daw::beat_interval(&state));
    assert!((beat - 6.5).abs() < 0.1, "beat {}", beat);
    assert!((playhead_ms(&state) - 6_500.).abs() < 100.);
    assert_eq!(state.playlist.total_beats.load(Ordering::SeqCst), 6);

    // stopped: nothing to carry on with
    assert_eq!(daw::retime(&state, 90.), Ok(daw::Retimed::Stopped));
    // the same tempo again changes nothing
    assert_eq!(daw::retime(&state, 90.), Ok(daw::Retimed::Stopped));

    // a pattern playing on its own is ended too, to start over. the
    // playlist's playhead stays on its beat
    *state.playlist.mode.lock().unwrap() = daw::PlayMode::Pattern(3);
    state.playlist.set_playing(true);
    assert_eq!(daw::retime(&state, 120.), Ok(daw::Retimed::Pattern(3)));
    assert!(!state.playlist.playing());
    let beat = daw::offset_beats(daw::playhead(&state), daw::beat_interval(&state));
    assert!((beat - 6.5).abs() < 0.1, "beat {}", beat);
  }

  // the playhead, in beats
  fn get_playlist_position_of(state: &daw::InnerState) -> f64 {
    let beats = daw::offset_beats(daw::playhead(state), daw::beat_interval(state));
    (beats * 1e6).round() / 1e6
  }

  #[test]
  fn test_the_loop_ends_with_the_last_clip() {
    let state = daw::InnerState::default();
    let sample = daw::METRONOME_TICK_PATH.to_string();
    let ms = |d: Duration| d.as_millis();

    // no clips: four bars (8s at 120bpm)
    assert_eq!(ms(daw::loop_end(&state)), 8_000);
    assert_eq!(daw::loop_len(&state).map(ms), Some(8_000));

    // a short clip at beat 9 (the start of bar 3) ends in bar 3
    let id = add_node_at_offset(&state, sample.clone(), 0, beat_to_offset(&state, Some(8.)), true).unwrap();
    assert_eq!(ms(daw::loop_end(&state)), 6_000);
    assert_eq!(playlist_transport(&state).loop_beats, 12);

    // moved into bar 60, the loop follows it
    move_audiograph_node_to(&state, id, 0, beat_to_offset(&state, Some(236.))).unwrap();
    assert_eq!(node_beat(&state, id).unwrap(), 236.);
    assert_eq!(playlist_transport(&state).loop_beats, 240);

    // looping off: no loop length, but where it would end is still known
    state.playlist.loop_enabled.store(false, Ordering::SeqCst);
    assert_eq!(daw::loop_len(&state), None);
    assert_eq!(playlist_transport(&state).loop_beats, 240);
  }

  #[test]
  fn test_the_playhead_wraps_at_the_last_clip() {
    let state = daw::InnerState::default();
    // a clip in bar 2: the loop is two bars (4s)
    add_node_at_offset(&state, daw::METRONOME_TICK_PATH.to_string(), 0, beat_to_offset(&state, Some(5.)), true).unwrap();
    assert_eq!(playlist_transport(&state).loop_beats, 8);

    // 5s into a run from the start: 1s into the second pass
    fake_run(&state, 0, 5_000);
    assert!((playhead_ms(&state) - 1_000.).abs() < 50.);
    let t = playlist_transport(&state);
    assert!((t.position_ms - 1_000.).abs() < 50.);
    assert_eq!(t.start_offset_ms, 0.);
  }

  #[test]
  fn test_song_end_in_transport() {
    let state = daw::InnerState::default();
    let id = state.playlist.audiograph.lock().unwrap()
      .construct_and_add_node(daw::METRONOME_TICK_PATH.to_string(), Duration::from_millis(1_000), 0)
      .unwrap();
    let tick = state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().duration();

    let expected = (Duration::from_millis(1_000) + tick).as_secs_f64() * 1000.;
    assert!((playlist_transport(&state).song_end_ms - expected).abs() < 0.001);
  }
}
