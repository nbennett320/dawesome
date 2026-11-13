use super::*;

// an empty folder for one test's files
fn test_dir(name: &str) -> PathBuf {
  let dir = std::env::temp_dir()
    .join(format!("dawesome-project-{}-{}", name, std::process::id()));
  let _ = fs::remove_dir_all(&dir);
  fs::create_dir_all(&dir).unwrap();
  fs::canonicalize(dir).unwrap()
}

fn add_clip(state: &daw::InnerState, path: &str, track: u32, beat: f64) -> u64 {
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let offset = audiograph.beat_interval().mul_f64(beat);
  audiograph.construct_and_add_node(path.to_string(), offset, track).unwrap()
}

// a take (any short sample will do), as it would be recorded
fn record_take(dir: &Path, name: &str) -> String {
  fs::create_dir_all(dir).unwrap();
  let path = dir.join(name);
  fs::copy(daw::METRONOME_TICK_PATH, &path).unwrap();
  path.to_string_lossy().into_owned()
}

fn saved_json(path: &str) -> Project {
  Project::from_json(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn test_count_in_is_saved() {
  let dir = test_dir("count-in");
  let state = daw::InnerState::default();
  state.playlist.count_in.store(true, Ordering::SeqCst);
  let file = dir.join("song.dawesome");
  save_to(&state, &file).unwrap();

  let loaded = daw::InnerState::default();
  assert!(load_from(&loaded, &file).unwrap().settings.count_in);
  assert!(loaded.playlist.count_in.load(Ordering::SeqCst));
  // projects from before it was saved don't count in
  assert!(!Project::from_json("{}").unwrap().settings.count_in);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_added_tracks_are_saved() {
  let dir = test_dir("added_tracks");
  let file = dir.join("song.dawesome");
  let default = daw::defaults::NUM_OF_TRACKS;

  // the default number isn't written
  let state = daw::InnerState::default();
  save_to(&state, &file).unwrap();
  let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
  assert!(json.get("tracks").is_none());

  crate::commands::playlist::add_track_to(&state);
  save_to(&state, &file).unwrap();

  let loaded = daw::InnerState::default();
  load_from(&loaded, &file).unwrap();
  assert_eq!(loaded.playlist.audiograph.lock().unwrap().track_count(), default + 1);

  // and a new project starts with the default number again
  apply(&loaded, &Project::default(), None);
  assert_eq!(loaded.playlist.audiograph.lock().unwrap().track_count(), default);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_armed_tracks_are_saved() {
  use crate::recording::{ArmedTrack, InputChoice};
  let dir = test_dir("armed");
  let state = daw::InnerState::default();
  let armed = vec![
    ArmedTrack { track_number: 0, input: InputChoice { first: 0, stereo: true } },
    ArmedTrack { track_number: 2, input: InputChoice { first: 3, stereo: false } },
  ];
  state.recorder.lock().unwrap().set_armed(&armed);
  let file = dir.join("song.dawesome");
  save_to(&state, &file).unwrap();

  let loaded = daw::InnerState::default();
  assert_eq!(load_from(&loaded, &file).unwrap().armed, armed);
  assert_eq!(loaded.recorder.lock().unwrap().armed(), armed);

  // a new project has nothing armed
  apply(&loaded, &Project::default(), None);
  assert!(loaded.recorder.lock().unwrap().armed().is_empty());

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_first_save_makes_a_folder_and_brings_in_takes() {
  let dir = test_dir("first-save");
  let state = daw::InnerState::default();
  // recorded before the project had a file
  let session = dir.join("unsaved-session");
  state.project.lock().unwrap().session = Some(session.clone());
  let take = record_take(&session, "untitled_11092026_0.wav");
  let clip = add_clip(&state, &take, 0, 0.);
  let sample = add_clip(&state, daw::METRONOME_TICK_PATH, 1, 4.);
  let row = state.playlist.sequencer.lock().unwrap().add_row(&take).unwrap();

  let saved = save_project_to(&state, &dir.join("song.dawesome"), true).unwrap();

  let project_file = dir.join("song").join("song.dawesome");
  let recorded = dir.join("song").join("recordings").join("song_11092026_0.wav");
  assert_eq!(saved.path, project_file.to_string_lossy());
  assert!(project_file.is_file());
  assert!(recorded.is_file());
  assert!(!Path::new(&take).exists());
  // the unsaved folder is tidied away once it's empty
  assert!(!session.exists());
  assert!(saved.problems.is_empty());

  // the clip and the row follow their take; the other sample stays put
  assert_eq!(saved.moved_clips, vec![MovedClip { id: clip, path: recorded.to_string_lossy().into_owned() }]);
  let audiograph = state.playlist.audiograph.lock().unwrap();
  assert_eq!(audiograph.get_node_by_id(clip).unwrap().sample_path(), recorded.to_string_lossy());
  assert_eq!(audiograph.get_node_by_id(sample).unwrap().sample_path(), daw::METRONOME_TICK_PATH);
  drop(audiograph);
  let rows = saved.sequencer.expect("the sequencer, with its row moved").rows;
  assert_eq!(rows.iter().find(|r| r.id == row).unwrap().path, recorded.to_string_lossy());

  // saved relative to the project, so the folder can be moved
  let project = saved_json(&saved.path);
  assert_eq!(project.clips[0].path, "recordings/song_11092026_0.wav");
  assert_eq!(project.sequencer.rows[0].path, "recordings/song_11092026_0.wav");

  // and takes recorded from now on go in its recordings folder
  let mut location = state.project.lock().unwrap();
  assert_eq!(location.recordings_dir().unwrap(), dir.join("song").join("recordings"));
  assert_eq!(location.name(), "song");
  drop(location);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_first_save_keeps_folders_and_unused_takes() {
  let dir = test_dir("first-save-folders");
  let state = daw::InnerState::default();
  let session = dir.join("unsaved-session");
  state.project.lock().unwrap().session = Some(session.clone());
  let filed = record_take(&session.join("Vocals"), "untitled_11092026_0.wav");
  let unused = record_take(&session, "untitled_11092026_1.wav");
  let clip = add_clip(&state, &filed, 0, 0.);

  let saved = save_project_to(&state, &dir.join("song.dawesome"), true).unwrap();

  let recordings = dir.join("song").join("recordings");
  let moved = recordings.join("Vocals").join("song_11092026_0.wav");
  assert!(saved.problems.is_empty(), "{:?}", saved.problems);
  assert!(moved.is_file());
  assert!(recordings.join("song_11092026_1.wav").is_file());
  assert!(!Path::new(&filed).exists() && !Path::new(&unused).exists());
  assert!(!session.exists());

  assert_eq!(saved.moved_clips, vec![MovedClip { id: clip, path: moved.to_string_lossy().into_owned() }]);
  assert_eq!(saved_json(&saved.path).clips[0].path, "recordings/Vocals/song_11092026_0.wav");

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_save_as_copies_takes_into_the_new_project() {
  let dir = test_dir("save-as");
  let state = daw::InnerState::default();
  let old_file = dir.join("song").join("song.dawesome");
  let take = record_take(&dir.join("song").join("recordings"), "song_11092026_0.wav");
  add_clip(&state, &take, 0, 0.);
  save_project_to(&state, &old_file, false).unwrap();

  let saved = save_project_to(&state, &dir.join("remix.dawesome"), true).unwrap();

  let copy = dir.join("remix").join("recordings").join("remix_11092026_0.wav");
  assert_eq!(saved.path, dir.join("remix").join("remix.dawesome").to_string_lossy());
  assert!(copy.is_file());
  // the old project still has its take, and still plays it
  assert!(Path::new(&take).is_file());
  assert_eq!(saved_json(old_file.to_str().unwrap()).clips[0].path, "recordings/song_11092026_0.wav");
  assert_eq!(saved_json(&saved.path).clips[0].path, "recordings/remix_11092026_0.wav");

  // saving again leaves the takes where they are
  let again = save_project_to(&state, Path::new(&saved.path), false).unwrap();
  assert!(again.moved_clips.is_empty());

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_a_project_saved_before_folders_stays_where_it_is() {
  let dir = test_dir("bare");
  let state = daw::InnerState::default();
  add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
  let file = dir.join("old.dawesome");

  let saved = save_project_to(&state, &file, false).unwrap();

  assert_eq!(saved.path, file.to_string_lossy());
  assert!(!dir.join("old").exists());
  // its takes go next to it
  assert_eq!(state.project.lock().unwrap().recordings_dir().unwrap(), dir.join("recordings"));

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_opening_and_new_projects_move_where_takes_go() {
  let dir = test_dir("open");
  let state = daw::InnerState::default();
  let file = dir.join("song").join("song.dawesome");
  save_project_to(&state, &file, false).unwrap();

  let other = daw::InnerState::default();
  load_from(&other, &file).unwrap();
  assert_eq!(other.project.lock().unwrap().path.as_deref(), Some(file.as_path()));

  apply(&other, &Project::default(), None);
  other.project.lock().unwrap().reset(None);
  assert_eq!(other.project.lock().unwrap().name(), project_files::UNTITLED);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_clips_are_saved_on_their_beats_after_a_tempo_change() {
  let sample = fs::canonicalize(daw::METRONOME_TICK_PATH).unwrap();
  let state = daw::InnerState::default();
  add_clip(&state, sample.to_str().unwrap(), 1, 4.5);
  add_clip(&state, sample.to_str().unwrap(), 0, 200.);

  // placed at 120bpm, saved at 93 (then 77): the same beats, to the digit
  state.set_tempo(93.);
  state.set_tempo(77.);
  let beats: Vec<f64> = capture(&state, None).clips.iter().map(|clip| clip.start_beat).collect();
  assert_eq!(beats, vec![4.5, 200.]);
}

#[test]
fn test_save_and_load_round_trip() {
  let dir = test_dir("round-trip");
  let project_path = dir.join("song.dawesome");
  let sample = fs::canonicalize(daw::METRONOME_TICK_PATH).unwrap();
  let sample = sample.to_str().unwrap();

  let state = daw::InnerState::default();
  state.set_tempo(90.);
  *state.playlist.time_signature.lock().unwrap() =
    daw::timing::TimeSignature { numerator: 3, denominator: 4 };
  state.playlist.ui.lock().unwrap().snap_subdivision = daw::timing::Note::EighthNote;
  state.metronome_enabled.store(false, Ordering::SeqCst);

  add_clip(&state, sample, 0, 0.);
  let id = add_clip(&state, sample, 2, 4.5);
  state.playlist.audiograph.lock().unwrap()
    .get_mut_node_by_id(id).unwrap()
    .set_settings(daw::ClipSettings { gain_db: -6., pitch: 12., attack_ms: 5., ..Default::default() });

  save_to(&state, &project_path).unwrap();
  assert!(!dir.join("song.dawesome.tmp").exists());

  // a fresh app with a clip of its own, which loading replaces
  let loaded_state = daw::InnerState::default();
  let old_id = add_clip(&loaded_state, sample, 1, 1.);
  let loaded = load_from(&loaded_state, &project_path).unwrap();

  assert!(loaded.missing.is_empty());
  assert_eq!(loaded.settings.tempo, 90.);
  assert_eq!(loaded.settings.time_signature.numerator, 3);
  assert_eq!(loaded.settings.snap_note, daw::timing::Note::EighthNote);
  assert!(!loaded.settings.metronome_enabled);
  assert_eq!(loaded.clips.len(), 2);
  assert!(loaded.clips.iter().all(|c| c.id != old_id));

  let clip = loaded.clips.iter().find(|c| c.track_number == 2).unwrap();
  assert!((clip.start_beat - 4.5).abs() < 1e-4);
  assert_eq!(clip.info.settings.pitch, 12.);
  assert_eq!(clip.info.settings.gain_db, -6.);

  // saving the loaded project again gives the same project
  assert_eq!(capture(&loaded_state, Some(&dir)), capture(&state, Some(&dir)));
  assert_eq!(loaded_state.playlist.audiograph.lock().unwrap().nodes.len(), 2);

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_sample_paths_relative_to_the_project() {
  let dir = test_dir("relative");
  fs::create_dir_all(dir.join("samples")).unwrap();
  let sample = dir.join("samples").join("tick.wav");
  fs::copy(daw::METRONOME_TICK_PATH, &sample).unwrap();

  let state = daw::InnerState::default();
  add_clip(&state, sample.to_str().unwrap(), 0, 0.);
  // added by a relative path, which is saved as absolute
  add_clip(&state, daw::METRONOME_TICK_PATH, 1, 0.);
  save_to(&state, &dir.join("song.dawesome")).unwrap();

  let project = Project::from_json(
    &fs::read_to_string(dir.join("song.dawesome")).unwrap()).unwrap();
  let paths: Vec<&str> = project.clips.iter().map(|c| c.path.as_str()).collect();
  assert!(paths.contains(&"samples/tick.wav"));
  assert!(paths.iter().any(|p| Path::new(p).is_absolute()));

  // the project folder still loads after it's moved
  let moved = test_dir("relative-moved");
  fs::remove_dir_all(&moved).unwrap();
  fs::rename(&dir, &moved).unwrap();

  let loaded = load_from(&daw::InnerState::default(), &moved.join("song.dawesome")).unwrap();
  assert!(loaded.missing.is_empty());
  assert!(loaded.clips.iter().any(|c| Path::new(&c.path) == moved.join("samples/tick.wav")));

  fs::remove_dir_all(moved).unwrap();
}

#[test]
fn test_missing_samples_are_left_out() {
  let dir = test_dir("missing");
  let project_path = dir.join("song.dawesome");
  let sample = fs::canonicalize(daw::METRONOME_TICK_PATH).unwrap();
  fs::write(&project_path, format!(r#"{{
      "version": 1,
      "tempo": 100,
      "clips": [
        {{ "path": "gone.wav", "trackNumber": 0, "startBeat": 0 }},
        {{ "path": {:?}, "trackNumber": 1, "startBeat": 2 }}
      ]
    }}"#, sample.to_str().unwrap())).unwrap();

  let loaded = load_from(&daw::InnerState::default(), &project_path).unwrap();

  // the rest of the project still loads, with defaults for what's left out
  assert_eq!(loaded.settings.tempo, 100.);
  assert!(loaded.settings.loop_enabled);
  assert_eq!(loaded.clips.len(), 1);
  assert_eq!(loaded.clips[0].info.settings, daw::ClipSettings::default());
  assert_eq!(loaded.missing.len(), 1);
  assert_eq!(Path::new(&loaded.missing[0].path), dir.join("gone.wav"));

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_bad_projects_change_nothing() {
  let dir = test_dir("bad");
  let state = daw::InnerState::default();
  add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);

  let bad = [
    "not json",
    r#"{"version": 99}"#,
    r#"{"tempo": 0}"#,
    r#"{"timeSignature": {"numerator": 4, "denominator": 0}}"#,
    r#"{"clips": [{"trackNumber": 0}]}"#,
  ];
  for (i, json) in bad.iter().enumerate() {
    let path = dir.join(format!("bad-{}.dawesome", i));
    fs::write(&path, json).unwrap();
    assert!(load_from(&state, &path).is_err(), "{} should not load", json);
  }
  assert!(load_from(&state, &dir.join("nowhere.dawesome")).is_err());

  assert_eq!(state.playlist.audiograph.lock().unwrap().nodes.len(), 1);
  assert_eq!(state.tempo(), 120.);

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_new_project_clears_the_playlist() {
  let state = daw::InnerState::default();
  let old_id = add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
  state.set_tempo(80.);

  let loaded = apply(&state, &Project::default(), None);

  assert!(loaded.clips.is_empty());
  assert_eq!(loaded.settings, ProjectSettings::default());
  assert_eq!(state.tempo(), 120.);
  assert!(state.playlist.audiograph.lock().unwrap().nodes.is_empty());

  // ids aren't reused, so a stale id from the old project names nothing
  let new_id = add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
  assert!(new_id > old_id);
}

#[test]
fn test_mixer_saved_and_loaded() {
  let dir = test_dir("mixer");
  let project_path = dir.join("mix.dawesome");

  let state = daw::InnerState::default();
  state.playlist.mixer.set_volume_db(2, -12.);
  state.playlist.mixer.set_solo(2, true);
  state.playlist.mixer.set_master_volume_db(-3.);
  state.playlist.mixer.set_name(3, "Bass");
  save_to(&state, &project_path).unwrap();

  // only the changed and named tracks are saved
  let saved = Project::from_json(&fs::read_to_string(&project_path).unwrap()).unwrap();
  let saved_tracks: Vec<u32> = saved.mixer.tracks.iter().map(|t| t.track_number).collect();
  assert_eq!(saved_tracks, vec![2, 3]);

  let other = daw::InnerState::default();
  other.playlist.mixer.set_mute(0, true);
  let loaded = load_from(&other, &project_path).unwrap();

  let track = |n: u32| loaded.mixer.tracks.iter().find(|t| t.track_number == n).unwrap().settings;
  let name = |n: u32| loaded.mixer.tracks.iter().find(|t| t.track_number == n).unwrap().name.clone();
  assert_eq!(name(3).as_deref(), Some("Bass"));
  assert_eq!(name(2), None);
  assert_eq!(track(2), daw::ChannelSettings { volume_db: -12., pan: 0., mute: false, solo: true });
  // the loading app's own changes don't carry over
  assert_eq!(track(0), daw::ChannelSettings::default());
  assert_eq!(loaded.mixer.master.volume_db, -3.);
  // every track the playlist shows is listed
  assert_eq!(loaded.mixer.tracks.len(), daw::defaults::NUM_OF_TRACKS as usize);

  // an older project without a mixer loads with a flat one
  let old = Project::from_json(r#"{"version": 1, "tempo": 100}"#).unwrap();
  let loaded = apply(&other, &old, None);
  assert!(loaded.mixer.tracks.iter().all(|t| t.settings == daw::ChannelSettings::default()));
  assert_eq!(loaded.mixer.master.volume_db, 0.);

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_patterns_saved_and_loaded() {
  let dir = test_dir("patterns");
  let project_path = dir.join("beat.dawesome");
  fs::create_dir_all(dir.join("drums")).unwrap();
  let kick = dir.join("drums").join("kick.wav");
  fs::copy("assets/assets_66-bd-01.wav", &kick).unwrap();

  let state = daw::InnerState::default();
  let pattern = {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let (row, pattern) = {
      let mut seq = audiograph.sequencer.lock().unwrap();
      let row = seq.add_row(kick.to_str().unwrap()).unwrap();
      let pattern = seq.add_pattern(2, 4, daw::PatternKind::Drum).unwrap();
      seq.set_step(pattern, row, 0, true).unwrap();
      seq.set_step(pattern, row, 12, true).unwrap();
      seq.rename_pattern(pattern, "intro").unwrap();
      seq.set_row_volume(row, 0.5).unwrap();
      (row, pattern)
    };
    let _ = row;
    let beat = audiograph.beat_interval();
    audiograph.add_pattern_node::<daw::timing::Note>(pattern, beat * 4, 1, None).unwrap();
    audiograph.add_pattern_node::<daw::timing::Note>(pattern, beat * 0, 0, None).unwrap();
    pattern
  };
  save_to(&state, &project_path).unwrap();

  let saved = Project::from_json(&fs::read_to_string(&project_path).unwrap()).unwrap();
  assert_eq!(saved.version, PROJECT_VERSION_WITHOUT_WINDOWS);
  // inside the project's folder, so saved relative to it
  assert_eq!(saved.sequencer.rows[0].path, "drums/kick.wav");
  // in the order they start
  assert_eq!(saved.pattern_clips.iter().map(|c| c.start_beat).collect::<Vec<_>>(), vec![0., 4.]);

  let other = daw::InnerState::default();
  let loaded = load_from(&other, &project_path).unwrap();
  assert!(loaded.missing.is_empty());
  assert_eq!(loaded.sequencer.rows.len(), 1);
  assert_eq!(loaded.sequencer.rows[0].volume, 0.5);
  assert_eq!(loaded.sequencer.patterns[0].name.as_deref(), Some("intro"));
  assert_eq!(loaded.sequencer.patterns[0].length, 32);
  assert_eq!(loaded.pattern_clips.len(), 2);
  assert!(loaded.pattern_clips.iter().all(|c| c.pattern_id == pattern));
  assert_eq!(other.playlist.audiograph.lock().unwrap().pattern_nodes.len(), 2);

  // and saved again, it's the same project
  assert_eq!(capture(&other, Some(&dir)), capture(&state, Some(&dir)));

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_projects_without_patterns_stay_version_1() {
  let state = daw::InnerState::default();
  add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
  let project = capture(&state, None);
  assert_eq!(project.version, PROJECT_VERSION_WITHOUT_PATTERNS);

  // and leave the new fields out of the file
  let json = serde_json::to_value(&project).unwrap();
  assert!(json.get("sequencer").is_none());
  assert!(json.get("patternClips").is_none());
}

fn edit(id: u64, start: f64, end: Option<f64>) -> crate::commands::clip::WindowEdit {
  crate::commands::clip::WindowEdit { id, window: ClipWindow { start, end } }
}

// a one bar pattern with a kick step, and a clip of it on a beat
fn add_pattern_clip(state: &daw::InnerState, bars: u32, beat: f64) -> (u64, u64) {
  let pattern = {
    let mut seq = state.playlist.sequencer.lock().unwrap();
    let row = seq.add_row("assets/assets_66-bd-01.wav").unwrap();
    let pattern = seq.add_pattern(bars, 4, daw::PatternKind::Drum).unwrap();
    seq.set_step(pattern, row, 0, true).unwrap();
    pattern
  };
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let offset = audiograph.beat_interval().mul_f64(beat);
  let clip = audiograph.add_pattern_node::<daw::timing::Note>(pattern, offset, 1, None).unwrap();
  (pattern, clip)
}

fn sorted_nodes(state: &daw::InnerState) -> Vec<daw::AudioNode> {
  let mut nodes = state.playlist.audiograph.lock().unwrap().nodes.clone();
  nodes.sort_by_key(|node| node.start_offset);
  nodes
}

#[test]
fn test_a_trimmed_clip_is_saved_and_loaded() {
  let dir = test_dir("trimmed");
  let file = dir.join("song.dawesome");
  let state = daw::InnerState::default();
  let id = add_clip(&state, "assets/assets_66-bd-01.wav", 0, 2.);
  crate::commands::clip::set_windows(&state, &[edit(id, 0.02, Some(0.08))]).unwrap();
  let before = sorted_nodes(&state).remove(0);
  assert!(before.window().is_some());
  save_to(&state, &file).unwrap();

  let other = daw::InnerState::default();
  let loaded = load_from(&other, &file).unwrap();
  let after = sorted_nodes(&other).remove(0);
  assert_eq!(after.window(), before.window());
  assert_eq!(after.start_offset, before.start_offset);
  assert_eq!(after.settings(), before.settings());
  assert_eq!(loaded.clips.len(), 1);
  assert_eq!(loaded.clips[0].info.window, before.window().map(ClipWindow::of_sample));

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_the_two_halves_of_a_cut_clip_come_back_to_the_nanosecond() {
  let dir = test_dir("cut");
  let file = dir.join("song.dawesome");
  let state = daw::InnerState::default();
  let id = add_clip(&state, "assets/assets_66-bd-01.wav", 0, 2.337);
  crate::commands::clip::change_clip_settings(
    &state, id, daw::ClipSettings { pitch: -7., ..Default::default() }, None,
  ).unwrap();
  // a beat inside the clip
  let beat = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let node = audiograph.get_node_by_id(id).unwrap();
    (node.start_offset + node.duration() / 2).as_secs_f64() / audiograph.beat_interval().as_secs_f64()
  };
  crate::commands::clip::split(&state, &[id], beat).unwrap();
  let before = sorted_nodes(&state);
  assert_eq!(before.len(), 2);
  save_to(&state, &file).unwrap();

  let other = daw::InnerState::default();
  load_from(&other, &file).unwrap();
  let after = sorted_nodes(&other);
  assert_eq!(after.len(), 2);
  let (left, right) = (&after[0], &after[1]);
  assert_eq!(left.origin(), right.origin());
  assert_eq!(left.start_offset + left.duration(), right.start_offset);
  for (loaded, saved) in after.iter().zip(&before) {
    assert_eq!(loaded.start_offset, saved.start_offset);
    assert_eq!(loaded.window(), saved.window());
  }

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_the_version_follows_what_a_project_uses() {
  // nothing trimmed or patterned: version 1, with no window in the file
  let state = daw::InnerState::default();
  let id = add_clip(&state, "assets/assets_66-bd-01.wav", 0, 0.);
  let plain = capture(&state, None);
  assert_eq!(plain.version, 1);
  assert!(!serde_json::to_string(&plain).unwrap().contains("\"window\""));

  // a trimmed sample clip, without patterns
  crate::commands::clip::set_windows(&state, &[edit(id, 0.02, Some(0.08))]).unwrap();
  let trimmed = capture(&state, None);
  assert_eq!(trimmed.version, 3);
  assert!(serde_json::to_string(&trimmed).unwrap().contains("\"window\""));

  // patterns, not trimmed
  let state = daw::InnerState::default();
  let (_, clip) = add_pattern_clip(&state, 1, 0.);
  assert_eq!(capture(&state, None).version, 2);

  // a trimmed pattern clip
  crate::commands::clip::set_windows(&state, &[edit(clip, 4., Some(12.))]).unwrap();
  let trimmed = capture(&state, None);
  assert_eq!(trimmed.version, 3);
  assert_eq!(trimmed.pattern_clips[0].window, Some(ClipWindow { start: 4., end: Some(12.) }));
}

#[test]
fn test_a_pattern_clips_window_is_kept_past_the_patterns_end() {
  let dir = test_dir("pattern-window");
  let file = dir.join("song.dawesome");
  let state = daw::InnerState::default();
  let (pattern, clip) = add_pattern_clip(&state, 2, 0.);
  crate::commands::clip::set_windows(&state, &[edit(clip, 4., Some(24.))]).unwrap();
  state.playlist.sequencer.lock().unwrap().set_pattern_bars(pattern, 1, 4).unwrap();
  save_to(&state, &file).unwrap();

  let other = daw::InnerState::default();
  let loaded = load_from(&other, &file).unwrap();
  let window = Some(daw::PatternWindow { start: 4., end: Some(24.) });
  assert_eq!(other.playlist.audiograph.lock().unwrap().pattern_nodes[0].window, window);
  assert_eq!(loaded.pattern_clips[0].window, Some(ClipWindow { start: 4., end: Some(24.) }));

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_an_older_file_loads_whole_and_is_left_alone() {
  let dir = test_dir("older");
  let file = dir.join("old.dawesome");
  let sample = fs::canonicalize("assets/assets_66-bd-01.wav").unwrap();
  let sample = sample.to_str().unwrap();
  let json = format!(r#"{{
  "version": 2,
  "clips": [
    {{ "path": "{sample}", "trackNumber": 0, "startBeat": 1.5 }}
  ],
  "sequencer": {{
    "rows": [{{ "id": 0, "path": "{sample}" }}],
    "patterns": [{{ "id": 1, "name": null, "length": 16, "steps": {{ "0": [0, 4] }} }}]
  }},
  "patternClips": [
    {{ "patternId": 1, "trackNumber": 1, "startBeat": 4 }}
  ]
}}
"#);
  fs::write(&file, &json).unwrap();

  let state = daw::InnerState::default();
  let loaded = load_from(&state, &file).unwrap();
  assert!(loaded.missing.is_empty());
  assert_eq!(loaded.clips.len(), 1);
  assert_eq!(loaded.pattern_clips.len(), 1);
  assert_eq!(loaded.clips[0].info.window, None);
  assert_eq!(loaded.pattern_clips[0].window, None);
  let audiograph = state.playlist.audiograph.lock().unwrap();
  assert!(audiograph.nodes.iter().all(|node| node.window().is_none()));
  assert!(audiograph.pattern_nodes.iter().all(|node| node.window.is_none()));
  drop(audiograph);
  assert_eq!(fs::read(&file).unwrap(), json.as_bytes());

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_a_newer_file_is_refused() {
  let error = Project::from_json(r#"{"version": 4}"#).unwrap_err();
  assert!(error.contains("saved by a newer version"), "{}", error);
  assert!(Project::from_json(r#"{"version": 3}"#).is_ok());
}

#[test]
fn test_missing_row_samples_and_unknown_patterns() {
  let state = daw::InnerState::default();
  let project = Project::from_json(r#"{
      "version": 2,
      "sequencer": {
        "rows": [{ "id": 0, "path": "/nowhere/kick.wav" }],
        "patterns": [{ "id": 3, "name": null, "length": 16, "steps": { "0": [0, 4] } }]
      },
      "patternClips": [
        { "patternId": 3, "trackNumber": 0, "startBeat": 2 },
        { "patternId": 8, "trackNumber": 0, "startBeat": 0 }
      ]
    }"#).unwrap();

  let loaded = apply(&state, &project, None);
  assert_eq!(loaded.missing.len(), 1);
  assert_eq!(loaded.missing[0].path, "/nowhere/kick.wav");
  // the pattern is kept, without the missing row's steps
  assert_eq!(loaded.sequencer.patterns.len(), 1);
  assert!(loaded.sequencer.patterns[0].steps.is_empty());
  // only the clip of a pattern that exists
  assert_eq!(loaded.pattern_clips.len(), 1);
  assert_eq!(loaded.pattern_clips[0].start_beat, 2.);

  // a new project clears the sequencer
  let fresh = apply(&state, &Project::default(), None);
  assert!(fresh.sequencer.patterns.is_empty());
  assert!(fresh.pattern_clips.is_empty());
}

// each channel's effects, without their ids: (track, [(name, bypassed, missing)])
use crate::effects::SavedEffect;

fn rack_shape(racks: &crate::effects::Racks) -> Vec<(Option<u32>, Vec<(String, bool, bool)>)> {
  racks.iter()
    .map(|r| (r.track, r.slots.iter().map(|s| (s.name.clone(), s.bypassed, s.missing)).collect()))
    .collect()
}

#[test]
fn test_effects_saved_and_loaded() {
  use crate::daw::Channel;
  use crate::effects::testing::*;
  let dir = test_dir("effects");
  let file = dir.join("fx.dawesome");

  let mut rig = rig();
  let state = rig.state.clone();
  let effects = &state.playlist.effects;
  let a = effects.add(Channel::Track(1), &gain().id, None).unwrap();
  let b = effects.add(Channel::Track(1), &gain().id, None).unwrap();
  let m = effects.add(Channel::Master, &gain().id, None).unwrap();
  // the second ahead of the first, so order shows
  effects.move_to(b, Channel::Track(1), 0).unwrap();
  rig.turn(a, 0.25);
  rig.turn(m, 0.75);
  effects.set_bypassed(b, true).unwrap();
  let heard = rig.level(Some(1));
  save_to(&state, &file).unwrap();

  let saved = saved_json(file.to_str().unwrap());
  assert_eq!(saved.effects.len(), 2);
  assert_eq!(saved.effects[0].track, Some(1));
  assert_eq!(saved.effects[1].track, None);
  let first = &saved.effects[0].slots[1];
  assert_eq!((first.plugin_id.as_str(), first.path.as_str(), first.bypassed), (gain().id.as_str(), "/fake/Gain.vst3", false));
  assert_eq!(base64::decode(&first.state).unwrap(), 0.25f32.to_le_bytes());
  assert!(saved.effects[0].slots[0].bypassed);

  let mut other = rig_with_an_effect();
  let loaded = load_from(&other.state, &file).unwrap();
  assert!(loaded.missing_effects.is_empty());
  assert_eq!(rack_shape(&loaded.effects), vec![
    (Some(1), vec![("Gain".into(), true, false), ("Gain".into(), false, false)]),
    (None, vec![("Gain".into(), false, false)]),
  ]);
  assert_eq!(loaded.effects, other.state.playlist.effects.racks());
  assert!((other.level(Some(1)) - heard).abs() < 1e-6);
  assert!((heard - 0.2 * 0.25 * 0.75).abs() < 1e-6);
  // the old project's effects went
  assert_eq!(other.loader.alive.load(std::sync::atomic::Ordering::SeqCst), 3);
  assert_eq!(capture(&other.state, Some(&dir)).effects, capture(&state, Some(&dir)).effects);

  fs::remove_dir_all(dir).unwrap();
}

// a rig that already has an effect, which loading a project replaces
fn rig_with_an_effect() -> crate::effects::testing::Rig {
  let rig = crate::effects::testing::rig();
  rig.effects().add(crate::daw::Channel::Track(7), &crate::effects::testing::gain().id, None).unwrap();
  rig
}

#[test]
fn test_missing_effects_are_kept_and_saved_as_they_were() {
  use crate::effects::testing::*;
  let dir = test_dir("missing-effects");
  let file = dir.join("old.dawesome");
  let absent = absent();
  // an effect that isn't installed, one found by its saved path though
  // the catalog doesn't know it, and one with damaged settings
  let json = serde_json::json!({
    "effects": [
      { "track": 0, "slots": [
        { "pluginId": absent.id, "name": "Absent", "vendor": "Fake Audio", "path": "/gone/Absent.vst3",
          "bypassed": true, "state": base64::encode([1u8, 2, 3]) },
        { "pluginId": "vst3:00000000000000000000000000000099", "name": "Elsewhere", "vendor": "Them",
          "path": "/fake/Elsewhere.vst3", "state": base64::encode(0.5f32.to_le_bytes()) },
      ] },
      { "track": null, "slots": [
        { "pluginId": gain().id, "name": "Gain", "vendor": "Fake Audio", "path": "/fake/Gain.vst3", "state": "!!" },
      ] },
    ],
  });
  fs::write(&file, json.to_string()).unwrap();

  let mut rig = rig();
  let loaded = load_from(&rig.state, &file).unwrap();
  assert_eq!(rack_shape(&loaded.effects), vec![
    (Some(0), vec![("Absent".into(), true, true), ("Elsewhere".into(), false, false)]),
    (None, vec![("Gain".into(), false, true)]),
  ]);
  let names: Vec<(Option<u32>, String)> = loaded.missing_effects.iter().map(|m| (m.track, m.name.clone())).collect();
  assert_eq!(names, vec![(Some(0), "Absent".to_string()), (None, "Gain".to_string())]);
  assert_eq!(loaded.missing_effects[0].reason, "not installed");
  assert_eq!(loaded.missing_effects[0].path, "/gone/Absent.vst3");
  // only the one that loaded is heard
  assert!((rig.level(Some(0)) - 0.1).abs() < 1e-6);
  let wire = serde_json::to_value(&loaded).unwrap();
  assert_eq!(wire["missingEffects"][0]["name"], "Absent");
  assert_eq!(wire["effects"][0]["slots"][0]["missing"], true);

  let again = dir.join("again.dawesome");
  save_to(&rig.state, &again).unwrap();
  let before = saved_json(file.to_str().unwrap()).effects;
  let after = saved_json(again.to_str().unwrap()).effects;
  assert_eq!(after[0].slots[0], before[0].slots[0]);
  // (settings that can't be read at all aren't kept)
  assert_eq!(after[1].slots[0], SavedEffect { state: String::new(), ..before[1].slots[0].clone() });
  assert_eq!(loaded.missing_effects[1].reason, "its saved settings are damaged");
  assert_eq!(after[0].slots[1].state, before[0].slots[1].state);

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_projects_without_effects_load_with_none() {
  let rig = rig_with_an_effect();
  assert!(Project::from_json("{}").unwrap().effects.is_empty());
  let loaded = apply(&rig.state, &Project::from_json(r#"{"tempo": 100}"#).unwrap(), None);
  assert!(loaded.effects.is_empty());
  assert!(rig.effects().racks().is_empty());
  assert_eq!(rig.loader.alive.load(std::sync::atomic::Ordering::SeqCst), 0);
  // and a project without effects is saved without them
  let json = serde_json::to_value(capture(&rig.state, None)).unwrap();
  assert!(json.get("effects").is_none());
}

#[test]
fn test_new_project_clears_the_effects() {
  let rig = rig_with_an_effect();
  let loaded = apply(&rig.state, &Project::default(), None);
  assert!(loaded.effects.is_empty());
  assert!(rig.effects().racks().is_empty());
}

#[test]
fn test_synths_and_note_patterns_saved_and_loaded() {
  use crate::effects::testing::*;
  use std::sync::atomic::Ordering::SeqCst;
  let dir = test_dir("synths");
  let file = dir.join("keys.dawesome");

  let rig = rig();
  let state = rig.state.clone();
  let synth = crate::commands::synth::add(&state, &synth().id).unwrap().synth_id;
  let quiet = crate::commands::synth::add(&state, &crate::effects::testing::synth().id).unwrap().synth_id;
  rig.turn(synth, 0.3);
  crate::commands::synth::set_volume(&state, synth, 0.5, None).unwrap();
  crate::commands::synth::set_muted(&state, quiet, true).unwrap();
  state.playlist.effects.keep_window_position(synth, crate::effects::WindowPosition { x: 5., y: 6. });
  let pattern = state.playlist.sequencer.lock().unwrap().add_pattern(8, 4, daw::PatternKind::Note).unwrap();
  let note = daw::PatternNote { pitch: 64, start: 2., length: 3., velocity: 0.5 };
  crate::commands::sequencer::set_notes(&state, pattern, synth, vec![note], None).unwrap();
  let folder = state.playlist.sequencer.lock().unwrap().add_folder("Keys", None, daw::PatternKind::Note).unwrap();
  state.playlist.sequencer.lock().unwrap().move_pattern(pattern, Some(folder)).unwrap();
  crate::commands::sequencer::add_pattern_clip_for_tests(&state, pattern, 3, Some(0.));
  save_to(&state, &file).unwrap();

  let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
  let saved = &json["sequencer"]["synths"][0];
  assert_eq!((saved["name"].as_str(), saved["volume"].as_f64(), saved["muted"].as_bool()), (Some("Synth"), Some(0.5), Some(false)));
  assert_eq!(base64::decode(saved["state"].as_str().unwrap()).unwrap(), 0.3f32.to_le_bytes());
  assert_eq!(saved["window"], serde_json::json!({ "x": 5., "y": 6. }));
  assert_eq!(json["sequencer"]["synths"][1]["name"], "Synth 2");
  assert_eq!(json["sequencer"]["patterns"][0]["kind"], "note");
  assert_eq!(json["sequencer"]["folders"][0]["kind"], "note");

  let mut other = rig_with_an_effect();
  let loaded = load_from(&other.state, &file).unwrap();
  assert!(loaded.missing_effects.is_empty() && loaded.dropped.is_empty());
  let synths = &loaded.sequencer.synths;
  assert_eq!(synths.iter().map(|s| (s.name.as_str(), s.volume, s.muted, s.missing)).collect::<Vec<_>>(),
    vec![("Synth", 0.5, false, false), ("Synth 2", 1., true, false)]);
  // with their notes (under the ids they have now)
  let id = synths[0].id;
  let made = loaded.sequencer.pattern(pattern).unwrap();
  assert_eq!((made.kind, made.length, made.folder), (daw::PatternKind::Note, 128, Some(folder)));
  assert_eq!(made.notes[&id], vec![note]);
  assert!((other.gain_of(id) - 0.3).abs() < 1e-6);
  assert_eq!(other.effects().window_position(id), Some(crate::effects::WindowPosition { x: 5., y: 6. }));
  // the track's copy is there, playing the master copy's settings
  assert_eq!(other.copy_gain(id, Some(3)), Some(0.3));
  assert!((other.hear_notes(id, Some(3), &[(60, 0, 300)], 400)[100] - 0.3 * 0.5).abs() < 1e-6);
  // (the old project's effect went: two masters and a copy)
  assert_eq!(other.loader.alive.load(SeqCst), 3);

  // saved again, the same
  let again = dir.join("again.dawesome");
  save_to(&other.state, &again).unwrap();
  let mut first = saved_json(file.to_str().unwrap()).sequencer;
  let mut second = saved_json(again.to_str().unwrap()).sequencer;
  first.renumber_synths({ let mut n = 0; move || { n += 1; n } });
  second.renumber_synths({ let mut n = 0; move || { n += 1; n } });
  assert_eq!(first, second);

  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_a_missing_synth_keeps_its_notes_and_settings() {
  use crate::effects::testing::*;
  let gone = gone_synth();
  let json = serde_json::json!({
    "version": 2,
    "sequencer": {
      "rows": [],
      "synths": [{ "id": 4, "pluginId": gone.id, "name": "Gone Synth", "vendor": "Fake Audio",
        "path": "/gone/Synth.vst3", "volume": 0.75, "state": base64::encode([7u8, 8]) }],
      "patterns": [{ "id": 0, "name": null, "kind": "note", "length": 16, "steps": {},
        "notes": { "4": [{ "pitch": 60, "start": 0, "length": 1, "velocity": 1 }] } }],
    },
    "patternClips": [{ "patternId": 0, "trackNumber": 1, "startBeat": 0 }],
  });
  let rig = rig();
  let loaded = apply(&rig.state, &Project::from_json(&json.to_string()).unwrap(), None);
  assert_eq!(loaded.missing_effects.len(), 1);
  assert_eq!((loaded.missing_effects[0].track, loaded.missing_effects[0].name.as_str()), (None, "Gone Synth"));
  let synth = &loaded.sequencer.synths[0];
  assert!(synth.missing && !synth.has_editor);
  assert_eq!(loaded.sequencer.patterns[0].notes[&synth.id].len(), 1);
  // silent: no copies
  assert!(rig.effects().copy_ids().is_empty());

  let saved = capture(&rig.state, None).sequencer;
  assert_eq!(base64::decode(&saved.synths[0].state).unwrap(), vec![7u8, 8]);
  assert_eq!((saved.synths[0].path.as_str(), saved.synths[0].volume), ("/gone/Synth.vst3", 0.75));
  assert_eq!(saved.patterns[0].notes[&saved.synths[0].id].len(), 1);
}

#[test]
fn test_a_stage_one_project_loads_without_its_instruments_and_notes() {
  use crate::effects::testing::*;
  let json = serde_json::json!({
    "version": 2,
    "sequencer": {
      "rows": [],
      "patterns": [
        { "id": 0, "name": null, "length": 16, "steps": {},
          "notes": [{ "pitch": 60, "start": 0, "length": 1, "velocity": 1 }] },
        { "id": 1, "name": null, "length": 16, "steps": {},
          "notes": [{ "pitch": 62, "start": 0, "length": 1, "velocity": 1 }] },
        { "id": 2, "name": null, "length": 16, "steps": {} },
      ],
    },
    "effects": [
      { "track": 0, "slots": [
        { "pluginId": synth().id, "name": "Synth", "kind": "instrument", "path": "/fake/Synth.vst3" },
        { "pluginId": gain().id, "name": "Gain", "path": "/fake/Gain.vst3" },
      ] },
    ],
  });
  let rig = rig();
  let loaded = apply(&rig.state, &Project::from_json(&json.to_string()).unwrap(), None);
  assert_eq!(loaded.dropped, vec![
    "1 instrument in a mixer channel's effects was left out: instruments are synths in the Note Sequencer now".to_string(),
    "The notes in 2 drum patterns were left out: notes are in note patterns now".to_string(),
  ]);
  assert_eq!(serde_json::to_value(&loaded).unwrap()["dropped"].as_array().unwrap().len(), 2);
  assert_eq!(rack_shape(&loaded.effects), vec![(Some(0), vec![("Gain".into(), false, false)])]);
  assert_eq!(loaded.sequencer.patterns.len(), 3);
  assert!(loaded.sequencer.patterns.iter().all(|p| p.kind == daw::PatternKind::Drum && p.notes.is_empty()));
  assert!(loaded.sequencer.synths.is_empty());
  // nothing dropped: nothing said
  assert!(Project::from_json(r#"{"tempo": 100}"#).unwrap().dropped.is_empty());
}

// unsaved changes: the project against how it was last started, opened or
// saved

fn tick_clip(state: &daw::InnerState, track: u32, beat: f32) -> u64 {
  crate::commands::clip::add_clip(state, daw::METRONOME_TICK_PATH.to_string(), track, Some(beat))
    .unwrap()
    .id
}

#[test]
fn test_a_new_project_has_no_unsaved_changes() {
  let state = daw::InnerState::default();
  mark_unchanged(&state);
  assert!(!has_unsaved_changes(&state));
}

#[test]
fn test_unsaved_until_marked() {
  // (only before the app has started)
  assert!(has_unsaved_changes(&daw::InnerState::default()));
}

#[test]
fn test_a_clip_is_a_change_until_undone() {
  let state = std::sync::Arc::new(daw::InnerState::default());
  mark_unchanged(&state);

  tick_clip(&state, 0, 4.);
  assert!(has_unsaved_changes(&state));
  crate::history::undo(&state).unwrap();
  assert!(!has_unsaved_changes(&state));
  crate::history::redo(&state).unwrap();
  assert!(has_unsaved_changes(&state));
}

#[test]
fn test_settings_outside_the_history_are_changes() {
  let state = daw::InnerState::default();
  mark_unchanged(&state);
  state.playlist.loop_enabled.store(!state.playlist.loop_enabled.load(Ordering::SeqCst), Ordering::SeqCst);
  assert!(has_unsaved_changes(&state));

  let state = daw::InnerState::default();
  mark_unchanged(&state);
  state.set_tempo(daw::defaults::TEMPO + 10.);
  assert!(has_unsaved_changes(&state));
}

#[test]
fn test_saved_and_opened_projects_have_no_unsaved_changes() {
  let dir = test_dir("unsaved-changes");
  let state = daw::InnerState::default();
  mark_unchanged(&state);
  tick_clip(&state, 0, 0.);
  tick_clip(&state, 1, 2.);
  let saved = save_project_to(&state, &dir.join("song.dawesome"), true).unwrap();
  assert!(!has_unsaved_changes(&state));

  let opened = daw::InnerState::default();
  load_from(&opened, Path::new(&saved.path)).unwrap();
  assert!(!has_unsaved_changes(&opened));
  tick_clip(&opened, 2, 1.);
  assert!(has_unsaved_changes(&opened));

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_clips_in_another_order_are_no_change() {
  let state = std::sync::Arc::new(daw::InnerState::default());
  let early = tick_clip(&state, 0, 0.);
  tick_clip(&state, 0, 4.);
  mark_unchanged(&state);

  // undoing a move puts the clip back at the end of the graph's list
  crate::commands::clip::move_clip(&state, early, 0, Some(8.)).unwrap();
  assert!(has_unsaved_changes(&state));
  crate::history::undo(&state).unwrap();
  assert!(!has_unsaved_changes(&state));
}
