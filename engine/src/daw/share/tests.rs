use super::*;
use std::collections::BTreeMap;
use crate::daw::playback;
use crate::daw::project::save_project_to;

fn saved_json_for_tests(path: &Path) -> Project {
  Project::from_json(&fs::read_to_string(path).unwrap()).unwrap()
}

// an empty folder for one test's files
fn test_dir(name: &str) -> PathBuf {
  let dir = std::env::temp_dir().join(format!("dawesome-share-{}-{}", name, std::process::id()));
  let _ = fs::remove_dir_all(&dir);
  fs::create_dir_all(&dir).unwrap();
  fs::canonicalize(dir).unwrap()
}

// a short sample (any will do) at `path`, made different from others by
// `tag` when given
fn sample_at(path: &Path, tag: Option<&[u8]>) -> String {
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  let mut bytes = fs::read(playback::METRONOME_TICK_PATH).unwrap();
  if let Some(tag) = tag { bytes.extend_from_slice(tag); }
  fs::write(path, bytes).unwrap();
  path.to_string_lossy().into_owned()
}

fn add_clip(state: &daw::Session, path: &str, track: u32) {
  state.playlist.audiograph.lock().unwrap()
    .construct_and_add_node(path.to_string(), std::time::Duration::ZERO, track).unwrap();
}

fn bundled_sample() -> String {
  library::bundled_root().unwrap().join("TR-808").join("CB.WAV").to_string_lossy().into_owned()
}

// a zip's files, by name, with what's in them
fn zipped(path: &Path) -> BTreeMap<String, Vec<u8>> {
  let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
  (0..archive.len()).map(|i| {
    let mut file = archive.by_index(i).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    (file.name().to_string(), bytes)
  }).collect()
}

fn names(files: &BTreeMap<String, Vec<u8>>) -> Vec<&str> {
  files.keys().map(String::as_str).collect()
}

fn no_progress() -> impl FnMut(u64, u64) { |_, _| {} }

#[test]
fn test_a_project_is_zipped_with_the_samples_it_uses() {
  let dir = test_dir("whole");
  let library = dir.join("library");
  let drums = sample_at(&library.join("Drums").join("kicks").join("kick.wav"), Some(b"kick"));
  let loose = sample_at(&dir.join("desktop").join("snare.wav"), Some(b"snare"));
  let row = sample_at(&library.join("Drums").join("hat.wav"), Some(b"hat"));

  let state = daw::Session::default();
  add_clip(&state, &drums, 0);
  add_clip(&state, &loose, 1);
  add_clip(&state, &bundled_sample(), 2);
  // the same sample twice is copied once
  add_clip(&state, &drums, 3);
  state.playlist.sequencer.lock().unwrap().add_row(&row).unwrap();
  let saved = save_project_to(&state, &dir.join("song"), true, None).unwrap();
  let folder = dir.join("song");
  assert_eq!(saved.path, folder.join("song.dawesome").to_string_lossy());

  // what's in the folder besides
  let take = sample_at(&folder.join("recordings").join("take.wav"), Some(b"take"));
  add_clip(&state, &take, 4);
  fs::create_dir_all(folder.join("mixdowns")).unwrap();
  fs::write(folder.join("mixdowns").join("song.wav"), b"mix").unwrap();
  fs::write(folder.join("notes.txt"), b"notes").unwrap();
  fs::write(folder.join(".DS_Store"), b"").unwrap();
  fs::write(folder.join("song.v3.dawesome"), b"{}").unwrap();
  fs::write(folder.join("old.sharedawesome"), b"").unwrap();

  let dest = dir.join("song.sharedawesome");
  let user = vec![library.join("Drums").to_string_lossy().into_owned()];
  let mut progress = Vec::new();
  let exported = export(&state, &dest, &user, Some(serde_json::json!({ "pane": 1 })), &mut |done, total| progress.push((done, total))).unwrap();
  assert_eq!(exported.missing, Vec::<String>::new());
  assert_eq!(exported.plugins, Vec::<String>::new());

  let files = zipped(&dest);
  assert_eq!(names(&files), vec![
    "song/mixdowns/song.wav",
    "song/notes.txt",
    "song/recordings/take.wav",
    "song/samples/Drums/hat.wav",
    "song/samples/Drums/kicks/kick.wav",
    "song/samples/other/snare.wav",
    "song/song.dawesome",
  ]);
  assert_eq!(files["song/samples/Drums/kicks/kick.wav"], fs::read(&drums).unwrap());

  // the project as it's open (the take not saved yet), its samples in the zip
  let project = Project::from_json(std::str::from_utf8(&files["song/song.dawesome"]).unwrap()).unwrap();
  let paths: Vec<&str> = project.clips.iter().map(|clip| clip.path.as_str()).collect();
  assert_eq!(paths, vec![
    "samples/Drums/kicks/kick.wav",
    "samples/other/snare.wav",
    "dawesome:TR-808/CB.WAV",
    "samples/Drums/kicks/kick.wav",
    "recordings/take.wav",
  ]);
  assert_eq!(project.sequencer.rows[0].instrument.sample_path(), Some("samples/Drums/hat.wav"));
  assert_eq!(project.layout, Some(serde_json::json!({ "pane": 1 })));

  // the project on disk isn't touched
  let on_disk = saved_json_for_tests(&folder.join("song.dawesome"));
  assert_eq!(on_disk.clips.len(), 4);
  assert!(on_disk.clips.iter().any(|clip| clip.path == drums));
  assert!(!folder.join("samples").exists());

  // progress goes up to the whole
  let (last_done, total) = *progress.last().unwrap();
  assert_eq!(last_done, total);
  assert!(progress.windows(2).all(|pair| pair[0].0 <= pair[1].0));
  assert!(!dir.join("song.sharedawesome.tmp").exists());

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_a_shared_project_shared_again_keeps_its_samples() {
  let dir = test_dir("again");
  let folder = dir.join("song");
  // a project that came as a share, its samples in its own folder
  let kept = sample_at(&folder.join("samples").join("other").join("kick.wav"), Some(b"kick"));
  // and two more samples called kick.wav: the same file, and another
  let same = sample_at(&dir.join("a").join("kick.wav"), Some(b"kick"));
  let other = sample_at(&dir.join("b").join("kick.wav"), Some(b"another kick"));

  let state = daw::Session::default();
  add_clip(&state, &kept, 0);
  add_clip(&state, &same, 1);
  add_clip(&state, &other, 2);
  save_project_to(&state, &folder.join("song.dawesome"), false, None).unwrap();

  let dest = dir.join("song.sharedawesome");
  export(&state, &dest, &[], None, &mut no_progress()).unwrap();
  let files = zipped(&dest);
  assert_eq!(names(&files), vec![
    "song/samples/other/kick 1.wav",
    "song/samples/other/kick.wav",
    "song/song.dawesome",
  ]);
  assert_eq!(files["song/samples/other/kick 1.wav"], fs::read(&other).unwrap());
  let project = Project::from_json(std::str::from_utf8(&files["song/song.dawesome"]).unwrap()).unwrap();
  let paths: Vec<&str> = project.clips.iter().map(|clip| clip.path.as_str()).collect();
  assert_eq!(paths, vec!["samples/other/kick.wav", "samples/other/kick.wav", "samples/other/kick 1.wav"]);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_a_project_file_loose_in_a_folder_takes_only_its_own_folders() {
  let dir = test_dir("loose");
  // an old project saved straight into a big folder
  let take = sample_at(&dir.join("recordings").join("take.wav"), Some(b"take"));
  fs::write(dir.join("taxes.pdf"), b"taxes").unwrap();
  fs::create_dir_all(dir.join("Photos")).unwrap();
  fs::write(dir.join("Photos").join("cat.jpg"), b"cat").unwrap();
  fs::write(dir.join("other song.dawesome"), b"{}").unwrap();

  let state = daw::Session::default();
  add_clip(&state, &take, 0);
  save_project_to(&state, &dir.join("song.dawesome"), false, None).unwrap();

  let dest = dir.join("song.sharedawesome");
  export(&state, &dest, &[], None, &mut no_progress()).unwrap();
  assert_eq!(names(&zipped(&dest)), vec!["song/recordings/take.wav", "song/song.dawesome"]);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_missing_samples_are_named_and_left_out() {
  let dir = test_dir("missing");
  let gone = sample_at(&dir.join("elsewhere").join("gone.wav"), None);
  let state = daw::Session::default();
  add_clip(&state, &gone, 0);
  save_project_to(&state, &dir.join("song"), true, None).unwrap();
  fs::remove_file(&gone).unwrap();

  let dest = dir.join("song.sharedawesome");
  let exported = export(&state, &dest, &[], None, &mut no_progress()).unwrap();
  assert_eq!(exported.missing, vec![gone.clone()]);
  let files = zipped(&dest);
  assert_eq!(names(&files), vec!["song/song.dawesome"]);
  let project = Project::from_json(std::str::from_utf8(&files["song/song.dawesome"]).unwrap()).unwrap();
  assert_eq!(project.clips[0].path, gone);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_an_unsaved_project_is_saved_first() {
  let dir = test_dir("unsaved");
  let error = export(&daw::Session::default(), &dir.join("x.sharedawesome"), &[], None, &mut no_progress()).unwrap_err();
  assert!(error.contains("save the project"), "{}", error);
  assert!(!dir.join("x.sharedawesome").exists());
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_folders_with_the_same_name_are_told_apart() {
  let named: Vec<String> = buckets(&["/a/Drums".into(), "/b/drums".into(), "/c/other".into(), "/d/Drums".into()])
    .into_iter().map(|(_, name)| name).collect();
  assert_eq!(named, vec!["Drums", "drums 1", "other 1", "Drums 2"]);
  assert_eq!(numbered("samples/other/kick.wav", 2), "samples/other/kick 2.wav");
  assert_eq!(numbered("samples/other/README", 1), "samples/other/README 1");
}

#[test]
fn test_the_plugins_a_project_needs_are_listed() {
  let project: Project = Project::from_json(r#"{
    "version": 4,
    "effects": [
      { "track": 0, "slots": [
        { "pluginId": "vst3:abc", "name": "Valhalla", "vendor": "Valhalla DSP" },
        { "pluginId": "dawesome:eq", "name": "EQ" }
      ] },
      { "track": null, "slots": [{ "pluginId": "vst3:abc", "name": "Valhalla", "vendor": "Valhalla DSP" }] }
    ]
  }"#).unwrap();
  assert_eq!(plugins_of(&project), vec!["Valhalla (Valhalla DSP)".to_string()]);
}
