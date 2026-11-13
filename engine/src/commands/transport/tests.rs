use super::*;
use std::time::Duration;
use crate::commands::clip::{add_node_at_offset, move_audiograph_node_to};
use crate::commands::playlist::{beat_to_offset, node_beat};

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

#[test]
fn test_a_pattern_started_part_way_in_reports_when_its_start_was_heard() {
  let state = daw::InnerState::default();
  let pattern = state.playlist.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
  *state.playlist.mode.lock().unwrap() = daw::PlayMode::Pattern(pattern);
  *state.playlist.started_time.lock().unwrap() = Some(Instant::now());
  state.playlist.set_playing(true);

  let started = |step: f64| {
    *state.playlist.pattern_start.lock().unwrap() = step;
    playlist_transport(&state).start_time_ms.unwrap()
  };
  // from step 4 (a beat, 500ms in): as if step 0 had been heard 500ms
  // before the run started
  let from_start = started(0.);
  assert!((from_start - started(4.) - 500.).abs() < 1.);
}
