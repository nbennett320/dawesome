use super::*;
use crate::daw::test_support::{KICK, SNARE};
use crate::daw::actions::clip::add_clips;
use std::sync::Arc;

const MP3: &str = "assets/Db_maj7_short.mp3";

// an empty folder of its own for a test
fn test_dir(name: &str) -> PathBuf {
  let dir = std::env::temp_dir().join(format!("dawesome-drop-{}-{}", name, std::process::id()));
  let _ = fs::remove_dir_all(&dir);
  fs::create_dir_all(&dir).unwrap();
  fs::canonicalize(dir).unwrap()
}

// a state whose project is saved in `dir`/song
fn saved_in(dir: &Path) -> Arc<daw::Session> {
  let state = Arc::new(daw::Session::default());
  state.project.lock().unwrap().reset(Some(dir.join("song").join("song.dawesome")));
  state
}

fn put(from: &str, to: &Path) -> String {
  fs::create_dir_all(to.parent().unwrap()).unwrap();
  fs::copy(from, to).unwrap();
  to.to_string_lossy().into_owned()
}

fn write(to: &Path, bytes: &[u8]) -> String {
  fs::create_dir_all(to.parent().unwrap()).unwrap();
  fs::write(to, bytes).unwrap();
  to.to_string_lossy().into_owned()
}

fn s(path: &Path) -> String {
  path.to_string_lossy().into_owned()
}

#[test]
fn test_scan_keeps_what_plays_and_says_what_doesnt() {
  let dir = test_dir("scan");
  let finder = dir.join("finder");
  let kick = put(KICK, &finder.join("kick.wav"));
  let notes = write(&finder.join("notes.txt"), b"hello");
  let broken = write(&finder.join("broken.wav"), b"not a wav");
  // a folder: files in order, numbers as numbers, junk left out quietly
  let pack = finder.join("Pack");
  put(KICK, &pack.join("Kick 10.wav"));
  put(KICK, &pack.join("Kick 2.wav"));
  put(MP3, &pack.join("Chords").join("pad.mp3"));
  write(&pack.join("readme.pdf"), b"read me");
  write(&pack.join(".DS_Store"), b"");
  let bad = write(&pack.join("bad.aif"), b"nope");

  let scan = scan(&[kick.clone(), notes.clone(), s(&pack), broken.clone(), s(&finder.join("gone.wav"))]);
  let files: Vec<(&str, &str)> = scan.files.iter()
    .map(|f| (Path::new(&f.path).file_name().unwrap().to_str().unwrap(), f.folder.as_str()))
    .collect();
  assert_eq!(files, vec![
    ("kick.wav", ""),
    ("pad.mp3", "Pack/Chords"),
    ("Kick 2.wav", "Pack"),
    ("Kick 10.wav", "Pack"),
  ]);
  assert_eq!(scan.skipped, vec![
    Skipped { path: notes, reason: NOT_AUDIO.into() },
    Skipped { path: bad, reason: UNREADABLE.into() },
    Skipped { path: broken, reason: UNREADABLE.into() },
    Skipped { path: s(&finder.join("gone.wav")), reason: NOT_FOUND.into() },
  ]);
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_import_copies_once_and_never_writes_over() {
  let dir = test_dir("import");
  let state = saved_in(&dir);
  let clips = dir.join("song").join("clips");
  let finder = dir.join("finder");
  let kick = put(KICK, &finder.join("kick.wav"));
  // the same bytes under another name, and another file with kick's name
  let same = put(KICK, &finder.join("other").join("same kick.wav"));
  let snare_called_kick = put(SNARE, &finder.join("elsewhere").join("kick.wav"));

  let first = import(&state, &[DroppedFile { path: kick.clone(), folder: "".into() }], "").unwrap();
  assert_eq!(first.files, vec![ImportedFile { source: kick.clone(), path: s(&clips.join("kick.wav")), reused: false }]);
  assert_eq!(fs::read(clips.join("kick.wav")).unwrap(), fs::read(KICK).unwrap());

  let files = [
    DroppedFile { path: kick.clone(), folder: "".into() },
    DroppedFile { path: same.clone(), folder: "".into() },
    DroppedFile { path: snare_called_kick.clone(), folder: "".into() },
    // twice in one drop: copied once
    DroppedFile { path: snare_called_kick.clone(), folder: "".into() },
  ];
  let again = import(&state, &files, "").unwrap();
  let paths: Vec<(&str, bool)> = again.files.iter().map(|f| (f.path.as_str(), f.reused)).collect();
  let kick_copy = s(&clips.join("kick.wav"));
  let snare_copy = s(&clips.join("kick 1.wav"));
  assert_eq!(paths, vec![
    (kick_copy.as_str(), true),
    (kick_copy.as_str(), true),
    (snare_copy.as_str(), false),
    (snare_copy.as_str(), true),
  ]);
  assert_eq!(fs::read(&snare_copy).unwrap(), fs::read(SNARE).unwrap());
  assert!(again.skipped.is_empty());
  // nothing half copied left about
  let names: Vec<String> = fs::read_dir(&clips).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
  assert_eq!(names.len(), 2, "{:?}", names);
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_import_keeps_folders_and_merges_into_ones_there() {
  let dir = test_dir("folders");
  let state = saved_in(&dir);
  let clips = dir.join("song").join("clips");
  fs::create_dir_all(clips.join("Drums").join("Pack")).unwrap();
  let pack = dir.join("finder").join("Pack");
  put(KICK, &pack.join("kick.wav"));
  put(SNARE, &pack.join("Snares").join("snare.wav"));

  let scanned = scan(&[s(&pack)]);
  let imported = import(&state, &scanned.files, "Drums").unwrap();
  let paths: Vec<&str> = imported.files.iter().map(|f| f.path.as_str()).collect();
  assert_eq!(paths, vec![
    s(&clips.join("Drums/Pack/kick.wav")).as_str(),
    s(&clips.join("Drums/Pack/Snares/snare.wav")).as_str(),
  ]);
  assert!(imported.files.iter().all(|f| Path::new(&f.path).is_file() && !f.reused));
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_import_uses_the_projects_own_files_where_they_are() {
  let dir = test_dir("own");
  let state = saved_in(&dir);
  let take = put(KICK, &dir.join("song").join("recordings").join("song_11182026_0.wav"));
  let clip = put(SNARE, &dir.join("song").join("clips").join("Old").join("snare.wav"));

  let files = [DroppedFile { path: take.clone(), folder: "".into() }, DroppedFile { path: clip.clone(), folder: "".into() }];
  let imported = import(&state, &files, "").unwrap();
  assert_eq!(imported.files, vec![
    ImportedFile { source: take.clone(), path: take, reused: true },
    ImportedFile { source: clip.clone(), path: clip, reused: true },
  ]);
  assert_eq!(fs::read_dir(dir.join("song").join("clips")).unwrap().count(), 1);
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_import_skips_what_it_cant_copy_and_brings_the_rest() {
  let dir = test_dir("skips");
  let state = saved_in(&dir);
  let kick = put(KICK, &dir.join("finder").join("kick.wav"));
  let gone = s(&dir.join("finder").join("gone.wav"));

  let files = [DroppedFile { path: gone.clone(), folder: "".into() }, DroppedFile { path: kick, folder: "".into() }];
  let imported = import(&state, &files, "").unwrap();
  assert_eq!(imported.skipped, vec![Skipped { path: gone, reason: NOT_FOUND.into() }]);
  assert_eq!(imported.files.len(), 1);
  // a folder that would be outside the clips
  for into in ["..", "../x", "/etc", "a/../../b", ".hidden"] {
    assert!(import(&state, &files, into).is_err(), "{}", into);
  }
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_an_unsaved_project_imports_into_its_own_clips() {
  let dir = test_dir("unsaved");
  let state = daw::Session::default();
  let session = dir.join("unsaved-clips");
  state.project.lock().unwrap().clips_session = Some(session.clone());
  let kick = put(KICK, &dir.join("finder").join("kick.wav"));

  let imported = import(&state, &[DroppedFile { path: kick, folder: "".into() }], "").unwrap();
  assert_eq!(imported.files[0].path, s(&session.join("kick.wav")));
  assert!(session.join("kick.wav").is_file());
  assert!(crate::daw::project::has_unsaved_changes(&state));
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_placing_files_is_one_step_one_per_track() {
  let state = Arc::new(daw::Session::default());
  let paths = vec![KICK.to_string(), SNARE.to_string(), KICK.to_string()];
  let placed = add_clips(&state, &paths, 2, Some(4.)).unwrap();
  assert_eq!(placed.len(), 3);
  {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let tracks: Vec<u32> = placed.iter().map(|p| audiograph.get_node_by_id(p.id).unwrap().track_number).collect();
    assert_eq!(tracks, vec![2, 3, 4]);
  }
  assert!(placed.iter().all(|p| p.start_beat == placed[0].start_beat));
  assert_eq!(state.history.lock().unwrap().len(), 1);

  crate::daw::history::undo(&state).unwrap();
  assert!(placed.iter().all(|p| state.playlist.audiograph.lock().unwrap().get_node_by_id(p.id).is_none()));
}

#[test]
fn test_placing_files_adds_none_if_one_cant_be_read() {
  let dir = test_dir("place-broken");
  let broken = write(&dir.join("broken.wav"), b"nope");
  let state = daw::Session::default();
  let error = add_clips(&state, &[KICK.to_string(), broken.clone()], 0, Some(0.)).unwrap_err();
  assert!(error.contains("broken.wav"), "{}", error);
  assert!(state.playlist.audiograph.lock().unwrap().nodes.is_empty());
  assert_eq!(state.history.lock().unwrap().len(), 0);
  assert!(add_clips(&state, &[], 0, Some(0.)).unwrap().is_empty());
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_a_copy_plays_the_sound_its_ghost_decoded() {
  let dir = test_dir("ghost");
  let state = saved_in(&dir);
  let pad = put(MP3, &dir.join("finder").join("pad.mp3"));
  // drawn as it was dragged in: decoded then
  let (length, _) = crate::daw::audiograph::preview_sample(&pad).unwrap();

  let imported = import(&state, &[DroppedFile { path: pad.clone(), folder: String::new() }], "").unwrap();
  let copy = &imported.files[0].path;
  assert!(!imported.files[0].reused);
  let placed = add_clips(&state, &[copy.clone()], 0, Some(0.)).unwrap();
  let graph = state.playlist.audiograph.lock().unwrap();
  let node = graph.get_node_by_id(placed[0].id).unwrap();
  assert_eq!(node.sample_path(), copy);
  assert_eq!(node.duration(), length);
  // not decoded again for the copy: the sound the preview decoded (and holds)
  let original = crate::daw::audiograph::AudioNode::new(0, pad, std::time::Duration::ZERO, 0).unwrap();
  assert!(node.shares_sound_with(&original));
}
