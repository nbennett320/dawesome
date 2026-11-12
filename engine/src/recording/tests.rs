use super::*;

#[test]
fn test_input_choices() {
  assert_eq!(InputChoice::default_for(1), InputChoice { first: 0, stereo: false });
  assert_eq!(InputChoice::default_for(8), InputChoice { first: 0, stereo: true });
  assert_eq!(InputChoice { first: 2, stereo: true }.channels(), vec![2, 3]);
  assert_eq!(InputChoice { first: 5, stereo: false }.channels(), vec![5]);
}

#[test]
fn test_choices_fit_the_input() {
  let mono = |first| InputChoice { first, stereo: false };
  let pair = |first| InputChoice { first, stereo: true };

  // ones that fit stay as they are
  assert_eq!(pair(0).fit(2), pair(0));
  assert_eq!(pair(2).fit(4), pair(2));
  assert_eq!(mono(3).fit(4), mono(3));
  // a pair on a mono input records its channel
  assert_eq!(pair(0).fit(1), mono(0));
  // a pair past an input's channels is its first pair
  assert_eq!(pair(2).fit(2), pair(0));
  assert_eq!(pair(1).fit(2), pair(0));
  // a channel past them is the first
  assert_eq!(mono(1).fit(1), mono(0));
  assert_eq!(mono(5).fit(4), mono(0));
  // nothing known about the input: as chosen
  assert_eq!(pair(4).fit(0), pair(4));
}

#[test]
fn test_arming() {
  let mut recorder = Recorder::default();
  recorder.arm(3, Some(InputChoice { first: 0, stereo: true }));
  recorder.arm(1, Some(InputChoice { first: 1, stereo: false }));
  recorder.arm(3, Some(InputChoice { first: 2, stereo: false }));

  // by track, the latest choice
  assert_eq!(recorder.armed(), vec![
    ArmedTrack { track_number: 1, input: InputChoice { first: 1, stereo: false } },
    ArmedTrack { track_number: 3, input: InputChoice { first: 2, stereo: false } },
  ]);

  recorder.arm(1, None);
  assert_eq!(recorder.armed().len(), 1);
  recorder.set_armed(&[]);
  assert!(recorder.armed().is_empty());
}

// a stand-in for a recording in progress
fn session() -> Session {
  Session { tracks: vec![], start: Duration::ZERO, keep_from: Instant::now(), loop_len: None, loop_end: Duration::ZERO }
}

#[test]
fn test_still_recording_until_the_takes_are_in() {
  let state = daw::InnerState::default();
  state.recorder.lock().unwrap().session = Some(session());
  assert!(collect(&state).recording);

  // stopped, but its takes aren't on their tracks yet: asking now used to
  // say it was over, with no clips, so the frontend stopped asking and
  // the clips only showed up with the next recording
  let stopped = state.recorder.lock().unwrap().begin_finish();
  assert!(stopped.is_some());
  assert!(!is_recording(&state));
  let meanwhile = collect(&state);
  assert!(meanwhile.recording);
  assert!(meanwhile.clips.is_empty());

  state.recorder.lock().unwrap().end_finish(vec![], vec!["a problem".into()]);
  assert_eq!(collect(&state), Recorded { recording: false, clips: vec![], problems: vec!["a problem".into()] });
}

#[test]
fn test_no_undo_or_redo_while_recording() {
  use crate::history;
  let state = Arc::new(daw::InnerState::default());
  let clip = crate::commands::clip::add_clip(&state, daw::METRONOME_TICK_PATH.to_string(), 0, Some(2.)).unwrap().id;
  let has_clip = || state.playlist.audiograph.lock().unwrap().get_node_by_id(clip).is_some();

  // while a take is being recorded, nothing is undone, and the step waits
  state.recorder.lock().unwrap().session = Some(session());
  assert_eq!(history::undo(&state), Ok(None));
  assert!(has_clip());
  assert_eq!(state.history.lock().unwrap().len(), 1);

  // nor while its takes are being put on their tracks
  state.recorder.lock().unwrap().begin_finish();
  assert_eq!(history::undo(&state), Ok(None));
  assert!(has_clip());

  // once it's over, the step is still there to undo
  state.recorder.lock().unwrap().end_finish(vec![], vec![]);
  assert!(history::undo(&state).unwrap().is_some());
  assert!(!has_clip());

  // and redo waits the same way
  state.recorder.lock().unwrap().session = Some(session());
  assert_eq!(history::redo(&state), Ok(None));
  assert!(!has_clip());
  state.recorder.lock().unwrap().session = None;
  assert!(history::redo(&state).unwrap().is_some());
  assert!(has_clip());
}

#[test]
fn test_a_new_recording_waits_for_the_last_to_finish() {
  let state = Arc::new(daw::InnerState::default());
  state.recorder.lock().unwrap().session = Some(session());
  state.recorder.lock().unwrap().begin_finish();

  let finisher = state.clone();
  let done = std::thread::spawn(move || {
    std::thread::sleep(Duration::from_millis(50));
    finisher.recorder.lock().unwrap().end_finish(vec![], vec![]);
  });

  // nothing's armed, so it's refused, but only once the last one's in
  let asked = Instant::now();
  assert!(start(&state).unwrap_err().contains("Arm a track"));
  assert!(asked.elapsed() >= Duration::from_millis(45));
  done.join().unwrap();
}

#[test]
fn test_the_loop_stays_put_while_recording() {
  let state = daw::InnerState::default();
  let ms = |d: Duration| d.as_millis();
  // four bars with nothing in the playlist
  assert_eq!(ms(daw::loop_end(&state)), 8_000);

  // (started with looping off: the loop's end is kept all the same)
  state.playlist.loop_enabled.store(false, std::sync::atomic::Ordering::SeqCst);
  state.recorder.lock().unwrap().session = Some(Session {
    loop_len: daw::loop_len(&state),
    loop_end: daw::loop_end(&state),
    ..session()
  });
  state.playlist.loop_enabled.store(true, std::sync::atomic::Ordering::SeqCst);

  // a clip in bar 6 would move the loop's end, but not under a take
  state.playlist.audiograph.lock().unwrap()
    .construct_and_add_node(daw::METRONOME_TICK_PATH.to_string(), Duration::from_millis(10_500), 0)
    .unwrap();
  assert_eq!(ms(daw::loop_end(&state)), 8_000);

  // once it's over, the loop reaches the new clip
  state.recorder.lock().unwrap().session = None;
  assert_eq!(ms(daw::loop_end(&state)), 12_000);
}

#[test]
fn test_the_tempo_is_locked_while_recording() {
  let state = Arc::new(daw::InnerState::default());
  state.recorder.lock().unwrap().session = Some(session());

  *state.playlist.start_offset.lock().unwrap() = Duration::from_millis(1_500);
  assert!(daw::set_playlist_tempo(&state, 90.).is_err());
  assert_eq!(state.tempo(), 120.);
  // nothing moved
  assert_eq!(*state.playlist.start_offset.lock().unwrap(), Duration::from_millis(1_500));

  // nor while its takes are being put on their tracks, at the tempo they
  // were recorded in
  let stopped = state.recorder.lock().unwrap().begin_finish();
  assert!(stopped.is_some());
  assert!(daw::set_playlist_tempo(&state, 90.).is_err());

  state.recorder.lock().unwrap().end_finish(vec![], vec![]);
  assert_eq!(daw::set_playlist_tempo(&state, 90.), Ok(90.));
}

#[test]
fn test_nothing_is_undone_while_recording() {
  let state = Arc::new(daw::InnerState::default());
  let id = crate::commands::clip::add_clip(&state, daw::METRONOME_TICK_PATH.to_string(), 0, Some(2.)).unwrap().id;
  state.recorder.lock().unwrap().session = Some(session());

  assert_eq!(crate::history::undo(&state).unwrap(), None);
  assert!(state.playlist.audiograph.lock().unwrap().get_node_by_id(id).is_some());

  // nor while its takes are being put on their tracks
  let _ = state.recorder.lock().unwrap().begin_finish();
  assert_eq!(crate::history::undo(&state).unwrap(), None);

  state.recorder.lock().unwrap().end_finish(vec![], vec![]);
  assert!(crate::history::undo(&state).unwrap().is_some());
  assert!(state.playlist.audiograph.lock().unwrap().get_node_by_id(id).is_none());
}

#[test]
fn test_a_count_in_is_a_bar() {
  let state = daw::InnerState::default();
  // off unless it's turned on
  assert_eq!(count_in(&state), Duration::ZERO);

  state.playlist.count_in.store(true, std::sync::atomic::Ordering::SeqCst);
  // 4/4 at 120bpm
  assert_eq!(count_in(&state), Duration::from_secs(2));
  state.playlist.time_signature.lock().unwrap().numerator = 3;
  assert_eq!(count_in(&state), Duration::from_millis(1_500));
}

#[test]
fn test_nothing_armed_records_nothing() {
  let state = Arc::new(daw::InnerState::default());
  assert!(start(&state).unwrap_err().contains("Arm a track"));
  assert!(!is_recording(&state));
}

#[test]
fn test_finishing_without_a_recording_does_nothing() {
  let state = daw::InnerState::default();
  finish(&state, false);
  assert_eq!(collect(&state), Recorded { recording: false, clips: vec![], problems: vec![] });
}

// records from the real input onto an armed track, playing through the
// real output: run with `cargo test -- --ignored` (macOS asks for the
// microphone first)
#[test]
#[ignore = "needs audio input and output devices"]
fn test_recording_onto_the_playlist() {
  let dir = std::env::temp_dir().join(format!("dawesome-recording-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  let state = Arc::new(daw::InnerState::default());
  state.project.lock().unwrap().reset(Some(dir.join("song").join("song.dawesome")));
  state.playlist.loop_enabled.store(false, std::sync::atomic::Ordering::SeqCst);
  let channels = daw::input_channel_count().unwrap();
  state.recorder.lock().unwrap().arm(1, Some(InputChoice::default_for(channels)));
  // from beat 2
  let beat = daw::beat_interval(&state);
  *state.playlist.start_offset.lock().unwrap() = beat * 2;

  start(&state).unwrap();
  assert!(is_recording(&state) && state.playlist.playing());
  std::thread::sleep(Duration::from_millis(1_000));
  daw::pause_playlist(&state);

  let recorded = collect(&state);
  println!("{:?}", recorded);
  assert!(!recorded.recording);
  // (a saved input that isn't connected is said, and the default used)
  assert!(recorded.problems.iter().all(|p| p.contains("default input")), "{:?}", recorded.problems);
  let clip = &recorded.clips[0];
  assert_eq!(clip.track_number, 1);
  assert_eq!(clip.start_beat, 2.);
  assert!(clip.path.ends_with("recordings/song_") || clip.path.contains("/song/recordings/song_"));
  // about a second (less the time to get going)
  assert!(clip.info.sample_duration > 0.8 && clip.info.sample_duration < 1.1, "{}", clip.info.sample_duration);
  assert!(state.playlist.audiograph.lock().unwrap().get_node_by_id(clip.id).is_some());

  let _ = std::fs::remove_dir_all(&dir);
}
