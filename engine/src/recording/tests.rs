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
  Session {
    tracks: vec![],
    start: Duration::ZERO,
    start_beat: 0.,
    heard: Instant::now(),
    keep_from: Instant::now(),
    at_loop_end: LoopRecording::Layer,
    live: None,
    placing: None,
    playing_pass: None,
    loop_len: None,
    loop_end: Duration::ZERO,
  }
}

#[test]
fn test_still_recording_until_the_takes_are_in() {
  let state = daw::InnerState::default();
  state.recorder.lock().unwrap().session = Some(Session { tracks: vec![(2, PathBuf::new())], start_beat: 4.5, ..session() });
  let now = collect(&state);
  assert!(now.recording);
  // the playlist shows where it's recording
  assert_eq!(now.take, Some(TakeInProgress { start_beat: 4.5, tracks: vec![2] }));

  // stopped, but its takes aren't on their tracks yet: asking now used to
  // say it was over, with no clips, so the frontend stopped asking and
  // the clips only showed up with the next recording
  let stopped = state.recorder.lock().unwrap().begin_finish();
  assert!(stopped.is_some());
  assert!(!is_recording(&state));
  let meanwhile = collect(&state);
  assert!(meanwhile.recording);
  assert!(meanwhile.clips.is_empty());
  // and nothing more is being recorded
  assert_eq!(meanwhile.take, None);

  state.recorder.lock().unwrap().end_finish(vec![], vec!["a problem".into()]);
  assert_eq!(collect(&state), Recorded { recording: false, clips: vec![], problems: vec!["a problem".into()], take: None });
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
  assert!(start(&state, LoopRecording::Layer).unwrap_err().contains("Arm a track"));
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
  assert!(start(&state, LoopRecording::Layer).unwrap_err().contains("Arm a track"));
  assert!(!is_recording(&state));
}

#[test]
fn test_finishing_without_a_recording_does_nothing() {
  let state = daw::InnerState::default();
  finish(&state, false);
  assert_eq!(collect(&state), Recorded { recording: false, clips: vec![], problems: vec![], take: None });
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

  start(&state, LoopRecording::Layer).unwrap();
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

// a take as the writer leaves it: a copy of the metronome's tick in a file
// of its own (one of `count` taken for this test)
fn take_file(name: &str) -> PathBuf {
  let path = std::env::temp_dir().join(format!("dawesome-pass-{}-{}.wav", name, std::process::id()));
  std::fs::copy(daw::METRONOME_TICK_PATH, &path).unwrap();
  path
}

fn takes(path: &std::path::Path, pass: u32) -> daw::Takes {
  daw::Takes {
    files: vec![daw::Take { path: path.to_path_buf(), channels: 1, sample_rate: 44_100, frames: 100 }],
    started: None,
    dropped: 0,
    pass,
  }
}

#[test]
fn test_passes_go_where_the_loop_comes_round() {
  let at = Instant::now();
  let start = Duration::from_secs(3);
  // the first where the recording started, the rest at the loop's start
  assert_eq!(pass_start(&takes(std::path::Path::new("a"), 0), start, at), start);
  assert_eq!(pass_start(&takes(std::path::Path::new("a"), 2), start, at), Duration::ZERO);
  // a first that started late starts as late
  let late = daw::Takes { started: Some(at + Duration::from_millis(20)), ..takes(std::path::Path::new("a"), 0) };
  assert_eq!(pass_start(&late, start, at), start + Duration::from_millis(20));
}

#[test]
fn test_a_recording_stops_at_the_loop_end_unless_it_layers() {
  let state = daw::InnerState::default();
  assert!(!stops_at_loop_end(&state));

  state.recorder.lock().unwrap().session = Some(Session { at_loop_end: LoopRecording::Stop, ..session() });
  assert!(stops_at_loop_end(&state));
  // layering, but not looping when it started: it can't layer
  state.recorder.lock().unwrap().session = Some(session());
  assert!(stops_at_loop_end(&state));
  state.recorder.lock().unwrap().session = Some(Session { live: Some(Default::default()), ..session() });
  assert!(!stops_at_loop_end(&state));
}

#[test]
fn test_the_pass_playing_as_it_was_recorded_is_held_back() {
  let state = daw::InnerState::default();
  state.recorder.lock().unwrap().session = Some(Session { playing_pass: Some(1), ..session() });
  let (first, second) = (take_file("held-1"), take_file("held-2"));

  // pass 1 is playing on pass 2 as it was recorded: its clip doesn't start
  // too, until the loop comes round again
  let (held, problems) = place(&state, &takes(&second, 1), &[3], Duration::ZERO);
  assert!(problems.is_empty());
  assert_eq!((held[0].track_number, held[0].start_beat), (3, 0.));
  // the pass before it plays from the playlist
  let (playing, _) = place(&state, &takes(&first, 0), &[3], Duration::ZERO);
  {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    assert!(audiograph.held_back.contains(&held[0].id));
    assert!(!audiograph.held_back.contains(&playing[0].id));
  }
  let starting = |state: &daw::InnerState| {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    audiograph.init();
    audiograph.set_current_offset(Some(Duration::ZERO));
    audiograph.run_sounds_at(Duration::ZERO, 44_100, Duration::from_millis(10), false, false).len()
  };
  assert_eq!(starting(&state), 1);

  // the recording's over: they both play (no input is recording here)
  finish(&state, false);
  assert_eq!(starting(&state), 2);

  let _ = std::fs::remove_file(&first);
  let _ = std::fs::remove_file(&second);
}

#[test]
fn test_one_undo_takes_a_whole_loop_recording_off() {
  let state = Arc::new(daw::InnerState::default());
  let paths: Vec<PathBuf> = (0..3).map(|n| take_file(&format!("undo-{}", n))).collect();

  // three passes, put on their tracks as they're finished
  let (finished_tx, finished_rx) = mpsc::channel();
  let placer = state.clone();
  let placing = thread::spawn(move || place_passes(&placer, finished_rx, &[2], Duration::ZERO, Instant::now()));
  for (n, path) in paths.iter().enumerate() { finished_tx.send(takes(path, n as u32)).unwrap(); }
  drop(finished_tx);
  state.recorder.lock().unwrap().session = Some(Session { placing: Some(placing), ..session() });

  // (no input is recording here, which is said)
  finish(&state, false);
  let recorded = collect(&state);
  assert_eq!(recorded.clips.len(), 3);
  assert!(!recorded.recording);
  let ids: Vec<u64> = recorded.clips.iter().map(|clip| clip.id).collect();
  let on = |state: &daw::InnerState| ids.iter()
    .filter(|&&id| state.playlist.audiograph.lock().unwrap().get_node_by_id(id).is_some())
    .count();
  assert_eq!(on(&state), 3);

  assert!(crate::history::undo(&state).unwrap().is_some());
  assert_eq!(on(&state), 0);
  assert!(crate::history::redo(&state).unwrap().is_some());
  assert_eq!(on(&state), 3);

  for path in &paths { let _ = std::fs::remove_file(path); }
}

#[test]
fn test_which_pass_ends_where_the_loop_comes_round() {
  let s = |secs: f64| Duration::from_secs_f64(secs);
  // started 1s before the end of a 4s loop
  assert_eq!(ended_pass(s(1.), s(4.), s(1.)), Some(0));
  assert_eq!(ended_pass(s(1.), s(4.), s(5.)), Some(1));
  assert_eq!(ended_pass(s(1.), s(4.), s(9.)), Some(2));
  // a moment either side of where it comes round is the same pass
  assert_eq!(ended_pass(s(1.), s(4.), s(9.002)), Some(2));
  assert_eq!(ended_pass(s(1.), s(4.), s(8.998)), Some(2));
  // it hasn't come round since the recording started
  assert_eq!(ended_pass(s(3.), s(4.), s(0.5)), None);
  assert_eq!(ended_pass(s(1.), Duration::ZERO, s(1.)), None);
}

#[test]
fn test_the_take_in_progress_starts_at_the_loop_start_once_it_comes_round() {
  let state = daw::InnerState::default();
  let looping = |heard: Instant| Session {
    tracks: vec![(1, PathBuf::new())],
    start: Duration::from_secs(2),
    start_beat: 4.,
    heard,
    live: Some(Default::default()),
    loop_end: Duration::from_secs(4),
    ..session()
  };

  // still on its first pass, from where it started
  state.recorder.lock().unwrap().session = Some(looping(Instant::now()));
  assert_eq!(collect(&state).take.unwrap().start_beat, 4.);
  // the loop's come round: the pass being recorded is from the loop's start
  state.recorder.lock().unwrap().session = Some(looping(Instant::now() - Duration::from_secs(3)));
  assert_eq!(collect(&state).take.unwrap().start_beat, 0.);
}

// the pass that's ending plays on the next pass, from where it was recorded
#[test]
fn test_the_pass_that_ends_plays_on_the_next() {
  let rate = 1_000;
  let state = daw::InnerState::default();
  let (engine, mut renderer) = daw::Engine::new(rate);

  // a recording from 1s into a 4s loop: its first pass is 3s, which is
  // still being recorded (its last second isn't in yet)
  let samples = Arc::new(daw::LiveSamples::new(3_000, 1, rate));
  for _ in 0..2_000 { samples.push([0.5]); }
  let live = Arc::new(daw::LivePasses::default());
  live.add(daw::LivePass { index: 0, files: vec![samples] });
  let heard = Instant::now();
  state.recorder.lock().unwrap().session = Some(Session {
    tracks: vec![(1, PathBuf::new())],
    start: Duration::from_secs(1),
    heard,
    live: Some(live),
    loop_end: Duration::from_secs(4),
    ..session()
  });
  // and a clip of the pass before (as if there were one) held back
  state.playlist.audiograph.lock().unwrap().held_back.insert(7);

  // the loop comes round 3s in (half a second into the run)
  let at = Duration::from_millis(500);
  let voices = layer_voices(&state, &engine, 1, at, heard + Duration::from_secs(3));
  assert_eq!(voices.len(), 1);
  assert_eq!(state.recorder.lock().unwrap().session.as_ref().unwrap().playing_pass, Some(0));
  // what was held back plays from the playlist now
  assert!(state.playlist.audiograph.lock().unwrap().held_back.is_empty());

  engine.start_run(1, state.playlist.mixer.master(), voices).unwrap();
  let mut out = vec![0f32; 2 * 2_000];
  renderer.render(&mut out, Instant::now(), Duration::ZERO);
  // silent until 1s into the loop (where the recording started), then the
  // pass, through its track's strip
  let frame = |n: usize| out[n * 2];
  assert_eq!(frame(1_400), 0.);
  assert_eq!(frame(1_499), 0.);
  assert!(frame(1_500) > 0.1 && frame(1_999) > 0.1, "{} {}", frame(1_500), frame(1_999));

  // nothing's sent for a loop that comes round before the recording's
  // first pass is over (it started after that was scheduled)
  let again = layer_voices(&state, &engine, 1, at, heard + Duration::from_millis(500));
  assert!(again.is_empty());
}

// layers a loop recording from the real input through the real output: run
// with `cargo test -- --ignored` (macOS asks for the microphone first; what
// the microphone hears is played back on the passes after)
#[test]
#[ignore = "needs audio input and output devices"]
fn test_a_loop_recording_layers_its_passes() {
  let dir = std::env::temp_dir().join(format!("dawesome-layering-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  let state = Arc::new(daw::InnerState::default());
  state.project.lock().unwrap().reset(Some(dir.join("song").join("song.dawesome")));
  state.playlist.loop_enabled.store(true, std::sync::atomic::Ordering::SeqCst);
  // the empty playlist loops 4 bars: 3.2s at 300 bpm
  daw::set_playlist_tempo(&state, 300.).unwrap();
  let channels = daw::input_channel_count().unwrap();
  state.recorder.lock().unwrap().arm(1, Some(InputChoice::default_for(channels)));
  // from the second bar
  *state.playlist.start_offset.lock().unwrap() = daw::beat_interval(&state) * 4;

  start(&state, LoopRecording::Layer).unwrap();
  // the rest of the first pass, two whole passes, and some of a third
  std::thread::sleep(Duration::from_millis(2_400 + 2 * 3_200 + 1_000));
  // each finished pass is on its track while it's still recording
  let meanwhile = collect(&state);
  assert!(meanwhile.recording);
  assert_eq!(meanwhile.clips.len(), 3, "{:?}", meanwhile);
  daw::pause_playlist(&state);
  let last = collect(&state);

  let clips: Vec<_> = meanwhile.clips.iter().chain(&last.clips).collect();
  assert_eq!(clips.len(), 4, "{:?}", last);
  assert_eq!(clips[0].start_beat, 4.);
  assert!(clips[1..].iter().all(|clip| clip.start_beat == 0.));
  // whole passes are as long as the loop
  assert!((clips[1].info.sample_duration - 3.2).abs() < 0.01, "{}", clips[1].info.sample_duration);
  assert!(state.playlist.audiograph.lock().unwrap().held_back.is_empty());

  // and one undo takes them all off
  assert!(crate::history::undo(&state).unwrap().is_some());
  let graph = state.playlist.audiograph.lock().unwrap();
  assert!(clips.iter().all(|clip| graph.get_node_by_id(clip.id).is_none()));
  drop(graph);

  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_passes_play_in_a_run_that_started_with_an_empty_playlist() {
  // (an empty playlist still loops, so it can be loop recorded)
  let state = daw::InnerState::default();
  state.playlist.audiograph.lock().unwrap().init();
  state.playlist.set_playing(true);
  let path = take_file("empty-run");

  let (clips, _) = place(&state, &takes(&path, 1), &[1], Duration::ZERO);
  assert_eq!(clips.len(), 1);
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  audiograph.set_current_offset(Some(Duration::ZERO));
  assert_eq!(audiograph.run_sounds_at(Duration::ZERO, 44_100, Duration::from_millis(10), false, false).len(), 1);
  drop(audiograph);

  let _ = std::fs::remove_file(&path);
}
