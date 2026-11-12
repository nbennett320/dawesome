use super::*;
use crate::commands::clip::{add_clip, change_clip_settings, move_clip, remove_clips};
use crate::project;

fn state() -> Arc<daw::InnerState> {
  Arc::new(daw::InnerState::default())
}

// a clip of the metronome's tick on a beat
fn clip(state: &daw::InnerState, track: u32, beat: f32) -> u64 {
  add_clip(state, daw::METRONOME_TICK_PATH.to_string(), track, Some(beat)).unwrap().id
}

// the project as it would be saved: what undo and redo have to get back to
fn saved(state: &daw::InnerState) -> project::Project {
  project::capture(state, None)
}

fn beat_of(state: &daw::InnerState, id: u64) -> Option<f32> {
  crate::commands::playlist::node_beat(state, id).ok()
}

fn steps(state: &daw::InnerState) -> usize {
  state.history.lock().unwrap().len()
}

// undo gets the project back to how it was, and redo to how it became
fn there_and_back(state: &Arc<daw::InnerState>, before: &project::Project, after: &project::Project) {
  assert!(undo(state).unwrap().is_some());
  assert_eq!(&saved(state), before);
  assert!(redo(state).unwrap().is_some());
  assert_eq!(&saved(state), after);
}

#[test]
fn test_adding_a_clip() {
  let state = state();
  let empty = saved(&state);
  let id = clip(&state, 1, 4.);
  let added = saved(&state);

  assert_eq!(undo(&state).unwrap(), Some(Applied { clips: Some(vec![]) }));
  assert_eq!(saved(&state), empty);
  assert_eq!(beat_of(&state, id), None);

  // redone, it's the same clip: its id, and it's the one to select
  assert_eq!(redo(&state).unwrap(), Some(Applied { clips: Some(vec![id]) }));
  assert_eq!(saved(&state), added);
  assert_eq!(beat_of(&state, id), Some(4.));
}

#[test]
fn test_moving_a_clip() {
  let state = state();
  let id = clip(&state, 0, 2.);
  let before = saved(&state);
  move_clip(&state, id, 3, Some(9.)).unwrap();
  let after = saved(&state);
  assert_ne!(before, after);

  assert_eq!(undo(&state).unwrap(), Some(Applied { clips: Some(vec![id]) }));
  assert_eq!(beat_of(&state, id), Some(2.));
  assert_eq!(saved(&state), before);
  assert_eq!(redo(&state).unwrap(), Some(Applied { clips: Some(vec![id]) }));
  assert_eq!(saved(&state), after);
}

#[test]
fn test_moving_a_selection_is_one_step() {
  let state = state();
  let (a, b) = (clip(&state, 0, 2.), clip(&state, 1, 5.));
  let before = saved(&state);
  let was = steps(&state);

  // the command the playlist uses for a dragged selection
  let moved = crate::commands::clip::move_clips(&state, &[a, b], a, 10., 2);
  assert!(moved.is_ok());
  assert_eq!(steps(&state), was + 1);
  let after = saved(&state);

  there_and_back(&state, &before, &after);
  assert_eq!((beat_of(&state, a), beat_of(&state, b)), (Some(10.), Some(13.)));
}

#[test]
fn test_deleting_a_selection_is_one_step() {
  let state = state();
  let ids: Vec<u64> = (0..5).map(|n| clip(&state, n % 3, n as f32 * 2.)).collect();
  change_clip_settings(&state, ids[2], daw::ClipSettings { pitch: 7., ..Default::default() }, None).unwrap();
  let before = saved(&state);
  let was = steps(&state);

  remove_clips(&state, &ids).unwrap();
  assert_eq!(steps(&state), was + 1);
  let after = saved(&state);
  assert!(after.clips.is_empty());

  // one undo brings all five back, as they were, with their ids
  let applied = undo(&state).unwrap().unwrap();
  assert_eq!(applied.clips, Some(ids.clone()));
  assert_eq!(saved(&state), before);
  for &id in &ids {
    assert!(beat_of(&state, id).is_some());
  }
  assert!(redo(&state).unwrap().is_some());
  assert_eq!(saved(&state), after);
}

#[test]
fn test_deleting_is_all_or_nothing() {
  let state = state();
  let id = clip(&state, 0, 1.);
  let before = saved(&state);
  let was = steps(&state);

  assert!(remove_clips(&state, &[id, 9_999]).is_err());
  assert_eq!(saved(&state), before);
  // nothing to undo for it
  assert_eq!(steps(&state), was);
}

#[test]
fn test_a_gesture_is_one_step() {
  let state = state();
  let id = clip(&state, 0, 0.);
  let before = saved(&state);
  let was = steps(&state);

  // a knob dragged from 0 to -19dB sends many changes
  for n in 0..20 {
    let settings = daw::ClipSettings { gain_db: -(n as f32), ..Default::default() };
    change_clip_settings(&state, id, settings, Some(7)).unwrap();
  }
  assert_eq!(steps(&state), was + 1);
  let after = saved(&state);

  there_and_back(&state, &before, &after);

  // another gesture is another step, and so is a change with none
  change_clip_settings(&state, id, daw::ClipSettings { pan: 0.5, ..Default::default() }, Some(8)).unwrap();
  change_clip_settings(&state, id, daw::ClipSettings { pan: 0.7, ..Default::default() }, None).unwrap();
  change_clip_settings(&state, id, daw::ClipSettings { pan: 0.9, ..Default::default() }, None).unwrap();
  assert_eq!(steps(&state), was + 4);
}

#[test]
fn test_a_change_that_changes_nothing_is_no_step() {
  let state = state();
  let id = clip(&state, 0, 4.);
  let was = steps(&state);

  // dropped where it was
  move_clip(&state, id, 0, Some(4.)).unwrap();
  change_clip_settings(&state, id, daw::ClipSettings::default(), None).unwrap();
  assert_eq!(steps(&state), was);

  // a gesture that ends where it began leaves nothing to undo
  change_clip_settings(&state, id, daw::ClipSettings { pan: 0.5, ..Default::default() }, Some(3)).unwrap();
  assert_eq!(steps(&state), was + 1);
  change_clip_settings(&state, id, daw::ClipSettings::default(), Some(3)).unwrap();
  assert_eq!(steps(&state), was);
}

#[test]
fn test_the_last_hundred_steps_are_kept() {
  let state = state();
  let id = clip(&state, 0, 0.);
  for n in 1..=150 {
    move_clip(&state, id, 0, Some(n as f32)).unwrap();
  }
  assert_eq!(steps(&state), LIMIT);

  let mut undone = 0;
  while undo(&state).unwrap().is_some() { undone += 1; }
  assert_eq!(undone, 100);
  // the oldest 51 steps (the add, and 50 moves) are gone: it's on beat 50
  assert_eq!(beat_of(&state, id), Some(50.));
}

#[test]
fn test_a_new_change_ends_redo() {
  let state = state();
  let id = clip(&state, 0, 0.);
  for beat in [1., 2., 3.] {
    move_clip(&state, id, 0, Some(beat)).unwrap();
  }
  for _ in 0..3 { undo(&state).unwrap(); }
  assert!(state.history.lock().unwrap().can_redo());

  move_clip(&state, id, 2, Some(8.)).unwrap();
  assert!(!state.history.lock().unwrap().can_redo());
  assert_eq!(redo(&state).unwrap(), None);
  assert_eq!(beat_of(&state, id), Some(8.));
}

#[test]
fn test_nothing_to_undo() {
  let state = state();
  assert_eq!(undo(&state).unwrap(), None);
  assert_eq!(redo(&state).unwrap(), None);
}

#[test]
fn test_the_history_starts_again_with_a_project() {
  let state = state();
  let id = clip(&state, 0, 0.);
  move_clip(&state, id, 0, Some(3.)).unwrap();
  assert!(state.history.lock().unwrap().can_undo());

  // a new project, or one opened (both go through apply)
  project::apply(&state, &project::Project::default(), None);
  assert!(!state.history.lock().unwrap().can_undo());
  assert!(!state.history.lock().unwrap().can_redo());
  assert_eq!(undo(&state).unwrap(), None);
}

#[test]
fn test_saving_keeps_the_history() {
  let dir = std::env::temp_dir().join(format!("dawesome-history-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  let state = state();
  let id = clip(&state, 0, 0.);
  move_clip(&state, id, 0, Some(3.)).unwrap();
  let was = steps(&state);

  project::save_to(&state, &dir.join("song.dawesome")).unwrap();
  assert_eq!(steps(&state), was);
  assert!(undo(&state).unwrap().is_some());
  assert_eq!(beat_of(&state, id), Some(0.));
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_a_change_made_during_an_undo_ends_redo() {
  let state = state();
  let id = clip(&state, 0, 0.);
  move_clip(&state, id, 0, Some(3.)).unwrap();

  // as travel does, with a change recorded while the step is being applied
  let (mut step, changes) = {
    let mut history = state.history.lock().unwrap();
    (history.undo.pop().unwrap(), history.changes)
  };
  apply(&state, &mut step, Way::Undo);
  clip(&state, 1, 8.);
  assert_ne!(state.history.lock().unwrap().changes, changes);

  // and through the real thing: nothing to redo after a new change
  undo(&state).unwrap();
  clip(&state, 2, 4.);
  assert_eq!(redo(&state).unwrap(), None);
}

#[test]
fn test_undo_and_redo_leave_files_alone() {
  let dir = std::env::temp_dir().join(format!("dawesome-history-files-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  // a take, as a recording leaves it
  let take = dir.join("take.wav");
  std::fs::copy(daw::METRONOME_TICK_PATH, &take).unwrap();
  let bytes = std::fs::read(&take).unwrap();
  let listing = || {
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap()
      .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
      .collect();
    names.sort();
    names
  };
  let files = listing();

  let state = state();
  let id = add_clip(&state, take.to_string_lossy().into_owned(), 0, Some(1.)).unwrap().id;
  change_clip_settings(&state, id, daw::ClipSettings { pitch: 3., ..Default::default() }, None).unwrap();
  move_clip(&state, id, 2, Some(6.)).unwrap();
  remove_clips(&state, &[id]).unwrap();

  // all the way back, and all the way forward again, twice
  for _ in 0..2 {
    while undo(&state).unwrap().is_some() {}
    assert_eq!(listing(), files);
    assert_eq!(std::fs::read(&take).unwrap(), bytes);
    while redo(&state).unwrap().is_some() {}
    assert_eq!(listing(), files);
    assert_eq!(std::fs::read(&take).unwrap(), bytes);
  }
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_a_clip_comes_back_on_its_beat_after_a_tempo_change() {
  let state = state();
  let id = clip(&state, 0, 8.);
  remove_clips(&state, &[id]).unwrap();

  state.set_tempo(90.);
  undo(&state).unwrap();
  assert!((beat_of(&state, id).unwrap() - 8.).abs() < 1e-5);
}

#[test]
fn test_mixer_changes() {
  use crate::commands::mixer as mixer;
  let state = state();
  // something already set, which every undo has to leave alone
  mixer::change_track_pan(&state, 0, -0.5, None);
  mixer::change_track_name(&state, 3, "Bass");

  let changes: Vec<(&str, Box<dyn Fn(&daw::InnerState)>)> = vec![
    ("volume", Box::new(|s| { mixer::change_track_volume(s, 2, -12., None); })),
    ("pan", Box::new(|s| { mixer::change_track_pan(s, 2, 0.25, None); })),
    ("mute", Box::new(|s| mixer::change_track_mute(s, 1, true))),
    ("solo", Box::new(|s| mixer::change_track_solo(s, 1, true))),
    ("master", Box::new(|s| { mixer::change_master_volume(s, -3., None); })),
    ("name", Box::new(|s| { mixer::change_track_name(s, 2, "Drums"); })),
    ("rename", Box::new(|s| { mixer::change_track_name(s, 3, "Sub"); })),
    ("unname", Box::new(|s| { mixer::change_track_name(s, 3, ""); })),
    // a track that has no clips, past the playlist's last
    ("far track", Box::new(|s| { mixer::change_track_volume(s, 9, -20., None); })),
    ("reset", Box::new(|s| { mixer::reset(s); })),
  ];

  for (what, change) in &changes {
    let had = steps(&state);
    let before = saved(&state);
    change(&state);
    let after = saved(&state);
    assert_ne!(before, after, "{} changed nothing", what);
    assert_eq!(steps(&state), had + 1, "{} is one step", what);

    // a mixer step isn't about clips: the selection is left alone
    assert_eq!(undo(&state).unwrap(), Some(Applied { clips: None }), "{}", what);
    assert_eq!(saved(&state), before, "{} undone", what);
    assert_eq!(redo(&state).unwrap(), Some(Applied { clips: None }), "{}", what);
    assert_eq!(saved(&state), after, "{} redone", what);
  }

  // and all the way back, to how it started
  for _ in &changes { undo(&state).unwrap(); }
  let mix = state.playlist.mixer.changed();
  assert_eq!(mix.tracks.len(), 2);
  assert_eq!(mix.tracks[0].settings.pan, -0.5);
  assert_eq!(mix.tracks[1].name.as_deref(), Some("Bass"));
}

#[test]
fn test_undoing_a_solo_lets_the_other_tracks_sound_again() {
  use rodio::Source;
  let state = state();
  let mixer = &state.playlist.mixer;
  // what comes out of track 0's strip, given a steady signal
  let level = || {
    let source = rodio::source::Zero::<f32>::new(2, 44_100).map(|_: f32| 0.5).take(64);
    let source = rodio::buffer::SamplesBuffer::new(2, 44_100, source.collect::<Vec<f32>>());
    mixer.track_strip(0, source.convert_samples::<f32>()).last().unwrap().abs()
  };
  assert!(level() > 0.);

  crate::commands::mixer::change_track_solo(&state, 1, true);
  assert_eq!(level(), 0.);
  undo(&state).unwrap();
  assert!(level() > 0.);
  redo(&state).unwrap();
  assert_eq!(level(), 0.);
  // undone and redone again, the solo count hasn't drifted
  undo(&state).unwrap();
  undo(&state).unwrap();
  assert!(level() > 0.);
}

#[test]
fn test_a_fader_drag_is_one_step() {
  use crate::commands::mixer as mixer;
  let state = state();
  mixer::change_track_volume(&state, 0, -6., None);
  let before = saved(&state);

  // twenty values on the way from -6 to 0, as one drag sends them
  for i in 1..=20 {
    mixer::change_track_volume(&state, 0, -6. + 0.3 * i as f32, Some(7));
  }
  assert_eq!(steps(&state), 2);
  let after = saved(&state);
  assert_eq!(state.playlist.mixer.track(0).volume_db(), 0.);
  there_and_back(&state, &before, &after);

  // the next drag is its own step, as is one on another control in between
  mixer::change_track_volume(&state, 0, -3., Some(8));
  mixer::change_track_pan(&state, 0, 0.5, Some(8));
  mixer::change_master_volume(&state, -1., Some(9));
  assert_eq!(steps(&state), 4);
  undo(&state).unwrap();
  undo(&state).unwrap();
  assert_eq!(saved(&state), after);

  // a drag that ends where it began leaves nothing to undo
  mixer::change_track_pan(&state, 1, 0.5, Some(10));
  mixer::change_track_pan(&state, 1, 0., Some(10));
  assert_eq!(steps(&state), 2);
}

#[test]
fn test_tempo_changes() {
  use crate::commands::transport::change_tempo;
  let state = state();
  let id = clip(&state, 0, 8.);
  let ms = |id: u64| {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    audiograph.get_node_by_id(id).unwrap().start_offset.as_secs_f64() * 1000.
  };
  let before = saved(&state);
  assert_eq!(change_tempo(&state, 90., None), Ok(90.));
  let after = saved(&state);
  assert_eq!(steps(&state), 2);

  assert_eq!(undo(&state).unwrap(), Some(Applied { clips: None }));
  assert_eq!(state.tempo(), 120.);
  assert_eq!(saved(&state), before);
  // the clip is on beat 8 at either tempo: 4s in, or 5.33s
  assert!((ms(id) - 4_000.).abs() < 0.001);
  redo(&state).unwrap();
  assert_eq!(saved(&state), after);
  assert!((ms(id) - 8. * 60_000. / 90.).abs() < 0.01);

  // a refused tempo is no step
  assert!(change_tempo(&state, 0., None).is_err());
  assert!(change_tempo(&state, 90., None).is_ok());
  assert_eq!(steps(&state), 2);

  // a clip moved at one tempo goes back to its beat at another
  move_clip(&state, id, 0, Some(2.)).unwrap();
  change_tempo(&state, 150., None).unwrap();
  undo(&state).unwrap();
  undo(&state).unwrap();
  assert_eq!(state.tempo(), 90.);
  assert!((beat_of(&state, id).unwrap() - 8.).abs() < 1e-5);
  undo(&state).unwrap();
  assert_eq!(saved(&state), before);
}

#[test]
fn test_a_visit_to_the_tempo_field_is_one_step() {
  use crate::commands::transport::change_tempo;
  let state = state();
  let before = saved(&state);
  // 100 typed, then changed to 140, without leaving the field
  change_tempo(&state, 100., Some(3)).unwrap();
  change_tempo(&state, 140., Some(3)).unwrap();
  assert_eq!(steps(&state), 1);
  undo(&state).unwrap();
  assert_eq!(saved(&state), before);
  redo(&state).unwrap();
  assert_eq!(state.tempo(), 140.);

  // the next visit is another step
  change_tempo(&state, 80., Some(4)).unwrap();
  assert_eq!(steps(&state), 2);
  undo(&state).unwrap();
  assert_eq!(state.tempo(), 140.);
}

#[test]
fn test_clips_that_come_back_follow_moved_files() {
  let state = state();
  let id = clip(&state, 0, 0.);
  let path = state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().sample_path().to_string();
  remove_clips(&state, &[id]).unwrap();

  // its file was moved meanwhile (a take renamed in the recordings browser)
  state.history.lock().unwrap().move_paths(&[(path, "/somewhere/else.wav".to_string())]);
  undo(&state).unwrap();
  let audiograph = state.playlist.audiograph.lock().unwrap();
  assert_eq!(audiograph.get_node_by_id(id).unwrap().sample_path(), "/somewhere/else.wav");
}
