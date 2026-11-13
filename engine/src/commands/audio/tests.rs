use super::*;

fn test_dir(name: &str) -> PathBuf {
  let dir = std::env::temp_dir()
    .join(format!("dawesome-audio-commands-{}-{}", name, std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::canonicalize(dir).unwrap()
}

// a saved project with a take on the playlist and in a sequencer row
fn saved_project(name: &str) -> (daw::InnerState, PathBuf, PathBuf, String) {
  let dir = test_dir(name);
  let state = daw::InnerState::default();
  let file = dir.join("song").join("song.dawesome");
  let recordings = dir.join("song").join("recordings");
  std::fs::create_dir_all(&recordings).unwrap();
  let take = recordings.join("song_11092026_0.wav").to_string_lossy().into_owned();
  std::fs::copy(daw::METRONOME_TICK_PATH, &take).unwrap();
  std::fs::create_dir_all(recordings.join("Vocals")).unwrap();

  state.playlist.audiograph.lock().unwrap()
    .construct_and_add_node(take.clone(), std::time::Duration::ZERO, 0).unwrap();
  state.playlist.sequencer.lock().unwrap().add_row(&take).unwrap();
  crate::project::save_project_to(&state, &file, false).unwrap();
  (state, dir, file, take)
}

// the project as it opens next time, and the samples it couldn't find
fn reopen(file: &Path) -> (crate::project::LoadedProject, daw::InnerState) {
  let state = daw::InnerState::default();
  let loaded = crate::project::load_from(&state, file).unwrap();
  (loaded, state)
}

#[test]
fn test_a_moved_take_is_found_when_the_project_is_opened_again() {
  let (state, dir, file, take) = saved_project("reopen-moved");
  let moved = move_recording_in(&state, &take, "Vocals", Library::Recordings).unwrap();

  let (loaded, _) = reopen(&file);
  assert!(loaded.missing.is_empty(), "{:?}", loaded.missing);
  assert_eq!(loaded.clips[0].path, moved.path);
  assert_eq!(loaded.sequencer.rows[0].sample_path(), Some(moved.path.as_str()));
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_renamed_takes_and_folders_are_found_when_the_project_is_opened_again() {
  let (state, dir, file, take) = saved_project("reopen-renamed");
  let renamed = rename_recording_in(&state, &take, "lead", Library::Recordings).unwrap();
  move_recording_in(&state, &renamed.path, "Vocals", Library::Recordings).unwrap();
  rename_recording_folder_in(&state, "Vocals", "Lead vocals", Library::Recordings).unwrap();
  std::fs::create_dir_all(dir.join("song").join("recordings").join("Old")).unwrap();
  let last = move_recording_folder_in(&state, "Lead vocals", "Old", Library::Recordings).unwrap();
  assert_eq!(last.path, "Old/Lead vocals");

  let (loaded, _) = reopen(&file);
  let expected = dir.join("song").join("recordings").join("Old").join("Lead vocals").join("lead.wav");
  assert!(loaded.missing.is_empty(), "{:?}", loaded.missing);
  assert_eq!(loaded.clips[0].path, expected.to_string_lossy());
  assert_eq!(loaded.sequencer.rows[0].sample_path(), Some(&*expected.to_string_lossy()));
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_moving_a_take_saves_nothing_else() {
  let (state, dir, file, take) = saved_project("reopen-unsaved");
  // a change made after saving, and not saved
  state.playlist.audiograph.lock().unwrap()
    .construct_and_add_node(daw::METRONOME_TICK_PATH.to_string(), std::time::Duration::ZERO, 1).unwrap();
  move_recording_in(&state, &take, "Vocals", Library::Recordings).unwrap();

  let (loaded, _) = reopen(&file);
  assert_eq!(loaded.clips.len(), 1);
  let _ = std::fs::remove_dir_all(&dir);
}

// a saved project with a clip file in clips/ used by a clip on the playlist
fn project_with_a_clip_file(name: &str) -> (daw::InnerState, PathBuf, PathBuf, String) {
  let (state, dir, file, _) = saved_project(name);
  let clips = dir.join("song").join("clips");
  std::fs::create_dir_all(&clips).unwrap();
  let clip = clips.join("song_join_0.wav").to_string_lossy().into_owned();
  std::fs::copy(daw::METRONOME_TICK_PATH, &clip).unwrap();
  state.playlist.audiograph.lock().unwrap()
    .construct_and_add_node(clip.clone(), std::time::Duration::ZERO, 1).unwrap();
  (state, dir, file, clip)
}

fn clip_paths(state: &daw::InnerState) -> Vec<String> {
  state.playlist.audiograph.lock().unwrap().nodes.iter().map(|n| n.sample_path().to_string()).collect()
}

#[test]
fn test_the_clips_library_lists_clips_and_not_recordings() {
  let (state, dir, _, clip) = project_with_a_clip_file("clips-list");
  let clips = crate::project_files::list_recordings(&existing_dir(&state, Library::Clips).unwrap());
  assert_eq!(clips.takes.len(), 1);
  assert_eq!(clips.takes[0].path, clip);
  let recordings = crate::project_files::list_recordings(&existing_dir(&state, Library::Recordings).unwrap());
  assert!(recordings.takes.iter().all(|take| take.path != clip));
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_a_renamed_clip_file_is_followed_by_its_clip() {
  let (state, dir, _, clip) = project_with_a_clip_file("clips-rename");
  let renamed = rename_recording_in(&state, &clip, "mix", Library::Clips).unwrap();
  assert!(renamed.path.ends_with("clips/mix.wav"), "{}", renamed.path);
  assert_eq!(renamed.moved_clips.len(), 1);
  assert_eq!(renamed.moved_clips[0].path, renamed.path);
  assert!(clip_paths(&state).contains(&renamed.path));
  assert!(!clip_paths(&state).contains(&clip));
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_a_clip_file_moved_into_a_new_folder_is_followed_by_its_clip() {
  let (state, dir, _, clip) = project_with_a_clip_file("clips-move");
  let dir_clips = dir.join("song").join("clips");
  let made = dir_to_change(&state, Library::Clips, true).unwrap();
  assert_eq!(made, dir_clips);
  crate::project_files::create_folder(&made, "", "Joins").unwrap();
  let moved = move_recording_in(&state, &clip, "Joins", Library::Clips).unwrap();
  assert!(moved.path.ends_with("clips/Joins/song_join_0.wav"), "{}", moved.path);
  assert_eq!(moved.moved_clips.len(), 1);
  assert!(clip_paths(&state).contains(&moved.path));
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_a_clip_file_in_use_is_not_deleted() {
  let (state, dir, _, clip) = project_with_a_clip_file("clips-delete");
  let error = delete_recording_in(&state, &clip, Library::Clips).unwrap_err();
  assert!(error.contains("clip"), "{}", error);
  assert!(Path::new(&clip).is_file());
  let error = delete_recording_folder_in(&state, "", Library::Clips).unwrap_err();
  assert!(error.contains("clips"), "{}", error);
  // a take isn't a clip file
  let take = dir.join("song").join("recordings").join("song_11092026_0.wav").to_string_lossy().into_owned();
  let error = delete_recording_in(&state, &take, Library::Clips).unwrap_err();
  assert!(error.contains("clips"), "{}", error);
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_a_file_that_undo_could_bring_back_is_in_use() {
  let (state, dir, _, clip) = project_with_a_clip_file("clips-history");
  // a second clip of the file, dropped in and then deleted (which can be
  // undone), and the first taken away without a trace
  let added = crate::commands::clip::add_clip(&state, clip.clone(), 2, Some(4.)).unwrap().id;
  crate::commands::clip::remove_clips(&state, &[added]).unwrap();
  let first = state.playlist.audiograph.lock().unwrap().nodes.iter()
    .find(|node| node.sample_path() == clip).unwrap().id;
  state.playlist.audiograph.lock().unwrap().remove_node(first).unwrap();
  assert!(!clip_paths(&state).contains(&clip));

  // nothing on the playlist plays it, but undo would
  assert!(in_use(&state, Path::new(&clip)));
  assert!(delete_recording_in(&state, &clip, Library::Clips).is_err());
  assert!(in_use(&state, &dir.join("song").join("clips")));
  assert!(Path::new(&clip).is_file());
  // once that can't be undone any more, it isn't
  state.history.lock().unwrap().clear();
  assert!(!in_use(&state, Path::new(&clip)));
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_a_new_clip_of_a_renamed_file_shares_its_sound() {
  let (state, dir, _, clip) = project_with_a_clip_file("clips-cache");
  let renamed = rename_recording_in(&state, &clip, "mix", Library::Clips).unwrap();
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let old = audiograph.nodes.iter().find(|node| node.sample_path() == renamed.path).unwrap().id;
  let new = audiograph.construct_and_add_node(renamed.path.clone(), std::time::Duration::from_secs(2), 2).unwrap();
  let (old, new) = (audiograph.get_node_by_id(old).unwrap().clone(), audiograph.get_node_by_id(new).unwrap().clone());
  assert_eq!(old.sample_path(), renamed.path);
  assert!(old.shares_sound_with(&new));
  drop(audiograph);

  // and another file put where it was is read as what it is
  std::fs::copy("assets/assets_66-bd-01.wav", &clip).unwrap();
  let other = state.playlist.audiograph.lock().unwrap()
    .construct_and_add_node(clip.clone(), std::time::Duration::from_secs(4), 3).unwrap();
  let other = state.playlist.audiograph.lock().unwrap().get_node_by_id(other).unwrap().clone();
  assert!(!other.shares_sound_with(&old));
  assert_ne!(other.sample_duration(), old.sample_duration());
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_clips_of_a_project_that_was_never_saved() {
  let state = daw::InnerState::default();
  assert_eq!(existing_dir(&state, Library::Clips), None);
  let error = rename_recording_in(&state, "/nowhere/a.wav", "b", Library::Clips).unwrap_err();
  assert_eq!(error, "the project has no clips yet");
  assert_eq!(dir_to_change(&state, Library::Clips, true).unwrap_err(), "the project has no clips yet");
  assert_eq!(delete_recording_in(&state, "/nowhere/a.wav", Library::Clips).unwrap_err(), "the project has no clips yet");
}

#[test]
fn test_recordings_are_the_default_library() {
  assert_eq!(Library::default(), Library::Recordings);
  let (state, dir, _, take) = saved_project("library-default");
  let renamed = rename_recording_in(&state, &take, "lead", Library::default()).unwrap();
  assert!(renamed.path.ends_with("recordings/lead.wav"), "{}", renamed.path);
  let error = delete_recording_folder_in(&state, "", Library::Recordings).unwrap_err();
  assert_eq!(error, "the recordings folder itself can't be deleted");
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_moved_path() {
  let from = Path::new("/song/recordings/Vocals");
  let to = Path::new("/song/recordings/Old/Vocals");
  // a take in the folder, or the folder itself
  assert_eq!(moved_path("/song/recordings/Vocals/a.wav", from, to), Some("/song/recordings/Old/Vocals/a.wav".into()));
  assert_eq!(moved_path("/song/recordings/Vocals/x/b.wav", from, to), Some("/song/recordings/Old/Vocals/x/b.wav".into()));
  // a take that moved on its own
  let take = Path::new("/song/recordings/a.wav");
  assert_eq!(moved_path("/song/recordings/a.wav", take, Path::new("/song/recordings/B/a.wav")), Some("/song/recordings/B/a.wav".into()));
  // not in it, even with a name that starts the same
  assert_eq!(moved_path("/song/recordings/Vocals 2/a.wav", from, to), None);
  assert_eq!(moved_path("/samples/kick.wav", from, to), None);
}
