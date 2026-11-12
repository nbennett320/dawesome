use super::*;

fn test_dir(name: &str) -> PathBuf {
  let dir = std::env::temp_dir()
    .join(format!("dawesome-files-{}-{}", name, std::process::id()));
  let _ = fs::remove_dir_all(&dir);
  fs::create_dir_all(&dir).unwrap();
  dir
}

fn touch(path: &Path) -> String {
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, b"take").unwrap();
  path.to_string_lossy().into_owned()
}

#[test]
fn test_project_file_for() {
  let p = |s: &str| project_file_for(Path::new(s));

  assert_eq!(p("/music/song.dawesome"), PathBuf::from("/music/song/song.dawesome"));
  // the dialog may leave off the extension
  assert_eq!(p("/music/song"), PathBuf::from("/music/song/song.dawesome"));
  // already in its own folder: not nested again
  assert_eq!(p("/music/song/song.dawesome"), PathBuf::from("/music/song/song.dawesome"));
  // inside another project's folder: a folder of its own
  assert_eq!(p("/music/song/other.dawesome"), PathBuf::from("/music/song/other/other.dawesome"));
}

#[test]
fn test_recordings_and_names() {
  let path = Path::new("/music/song/song.dawesome");
  assert_eq!(recordings_dir_for(path), PathBuf::from("/music/song/recordings"));
  assert_eq!(project_name(path).unwrap(), "song");

  // a bare project file's recordings go next to it
  assert_eq!(recordings_dir_for(Path::new("/music/old.dawesome")), PathBuf::from("/music/recordings"));
}

#[test]
fn test_take_names() {
  assert_eq!(take_name("my_project", "11092026", 3), "my_project_11092026_3.wav");
  assert_eq!(
    parse_take_name("my_project_11092026_3.wav"),
    Some(("my_project".to_string(), "11092026".to_string(), 3)));
  assert_eq!(parse_take_name("untitled_01022027_12.wav").unwrap().2, 12);

  // not takes
  assert_eq!(parse_take_name("kick.wav"), None);
  assert_eq!(parse_take_name("loop_120_3.wav"), None);
  assert_eq!(parse_take_name("_11092026_3.wav"), None);
  assert_eq!(parse_take_name("song_11092026_3.mp3"), None);
}

#[test]
fn test_session_folder_names_sort_by_date() {
  assert_eq!(session_folder_name("11092026", 0xabc123), "20261109-abc123");
  assert_eq!(session_folder_name("11092026", 0xff), "20261109-0000ff");
}

#[test]
fn test_takes_recorded_together_get_their_own_numbers() {
  let dir = test_dir("next-takes");
  touch(&dir.join("song_11092026_1.wav"));

  assert_eq!(next_take_paths(&dir, "song", "11092026", 3), vec![
    dir.join("song_11092026_0.wav"),
    dir.join("song_11092026_2.wav"),
    dir.join("song_11092026_3.wav"),
  ]);

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_next_take_skips_taken_numbers() {
  let dir = test_dir("next-take");
  assert_eq!(next_take_paths(&dir, "song", "11092026", 1)[0], dir.join("song_11092026_0.wav"));

  touch(&dir.join("song_11092026_0.wav"));
  touch(&dir.join("song_11092026_1.wav"));
  // another day's takes don't count
  touch(&dir.join("song_11082026_2.wav"));
  assert_eq!(next_take_paths(&dir, "song", "11092026", 1)[0], dir.join("song_11092026_2.wav"));

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_unsaved_takes_move_into_the_project() {
  let dir = test_dir("move");
  let session = dir.join("unsaved");
  let recordings = dir.join("song").join("recordings");
  let a = touch(&session.join("untitled_11092026_0.wav"));
  let b = touch(&session.join("untitled_11102026_0.wav"));
  // not a take's name
  let c = touch(&session.join("odd name.wav"));
  // in folders, one of them empty
  let d = touch(&session.join("Vocals").join("Verse").join("untitled_11092026_1.wav"));
  fs::create_dir_all(session.join("Ideas")).unwrap();
  // a take already in the project, in another folder: numbers don't repeat
  touch(&recordings.join("Keep").join("song_11092026_0.wav"));

  let (moved, errors) = relocate_takes(&session, &recordings, "song", false);

  assert!(errors.is_empty(), "{:?}", errors);
  let to = |from: &str| moved.iter().find(|r| r.from == from).map(|r| PathBuf::from(&r.to));
  assert_eq!(moved.len(), 4);
  assert_eq!(to(&b), Some(recordings.join("song_11102026_0.wav")));
  assert_eq!(to(&c), Some(recordings.join("odd name.wav")));
  // each name once, across every folder
  let a_to = to(&a).unwrap();
  let d_to = to(&d).unwrap();
  assert_eq!(a_to.parent().unwrap(), recordings);
  assert_eq!(d_to.parent().unwrap(), recordings.join("Vocals").join("Verse"));
  let mut numbers = vec![a_to.file_name().unwrap().to_owned(), d_to.file_name().unwrap().to_owned()];
  numbers.sort();
  assert_eq!(numbers, vec!["song_11092026_1.wav", "song_11092026_2.wav"]);
  assert!(recordings.join("Ideas").is_dir());

  // moved, not copied, and the folders left behind are gone
  assert!(!Path::new(&a).exists() && !Path::new(&d).exists());
  assert!(a_to.is_file() && d_to.is_file());
  assert!(!session.join("Vocals").exists() && !session.join("Ideas").exists());

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_save_as_copies_takes() {
  let dir = test_dir("copy");
  let old = dir.join("song").join("recordings");
  let new = dir.join("remix").join("recordings");
  let take = touch(&old.join("song_11092026_0.wav"));
  let kept = touch(&old.join("Vocals").join("lead.wav"));

  let (copied, errors) = relocate_takes(&old, &new, "remix", true);

  assert!(errors.is_empty());
  let to = |from: &str| copied.iter().find(|r| r.from == from).map(|r| r.to.clone());
  assert_eq!(to(&take), Some(new.join("remix_11092026_0.wav").to_string_lossy().into_owned()));
  assert_eq!(to(&kept), Some(new.join("Vocals").join("lead.wav").to_string_lossy().into_owned()));
  // the old project keeps its takes and folders
  assert!(Path::new(&take).is_file() && Path::new(&kept).is_file());

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_nothing_to_relocate() {
  let dir = test_dir("relocate-nothing");
  let (moved, errors) = relocate_takes(&dir.join("missing"), &dir.join("to"), "song", false);
  assert!(moved.is_empty() && errors.is_empty());
  assert!(!dir.join("to").exists());
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_an_unsaved_project_records_into_a_folder_of_its_own() {
  let mut location = ProjectLocation::default();
  assert_eq!(location.name(), UNTITLED);
  let session = location.recordings_dir().unwrap();
  // the same folder until the project's saved, and not made yet
  assert_eq!(location.recordings_dir().unwrap(), session);
  assert!(session.parent().unwrap().ends_with("dawesome/unsaved"));

  // once it has a file, takes go next to it
  location.reset(Some(PathBuf::from("/music/song/song.dawesome")));
  assert_eq!(location.name(), "song");
  assert_eq!(location.recordings_dir().unwrap(), PathBuf::from("/music/song/recordings"));
  assert!(location.session.is_none());
}

#[test]
fn test_empty_folders_are_tidied_away() {
  let dir = test_dir("tidy");
  let empty = dir.join("empty");
  let full = dir.join("full");
  fs::create_dir_all(&empty).unwrap();
  touch(&full.join("take.wav"));

  remove_if_empty(&empty);
  remove_if_empty(&full);
  assert!(!empty.exists());
  assert!(full.exists());

  let _ = fs::remove_dir_all(&dir);
}

fn write_wav(path: &Path, rate: u32, frames: u32) {
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  let spec = hound::WavSpec { channels: 2, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
  let mut writer = hound::WavWriter::create(path, spec).unwrap();
  for _ in 0..frames * 2 { writer.write_sample(0i16).unwrap(); }
  writer.finalize().unwrap();
}

#[test]
fn test_list_recordings() {
  let dir = test_dir("list");
  let recordings = dir.join("recordings");
  write_wav(&recordings.join("song_11092026_9.wav"), 48_000, 24_000);
  write_wav(&recordings.join("song_11092026_10.wav"), 44_100, 88_200);
  // not a take, or not readable as one
  touch(&recordings.join("notes.txt"));
  touch(&recordings.join("BROKEN.WAV"));
  fs::create_dir_all(recordings.join("old.wav")).unwrap();
  // folders, one empty, and a take inside one
  write_wav(&recordings.join("Vocals").join("Verse 2").join("lead.wav"), 48_000, 48_000);
  fs::create_dir_all(recordings.join("Empty")).unwrap();
  touch(&recordings.join(".hidden").join("secret.wav"));

  let listing = list_recordings(&recordings);
  let takes = &listing.takes;
  let names: Vec<&str> = takes.iter().map(|t| t.name.as_str()).collect();
  assert_eq!(names.len(), 4);
  assert!(names.contains(&"BROKEN"));
  assert_eq!(listing.folders, vec!["Empty", "old.wav", "Vocals", "Vocals/Verse 2"]);

  let take = |name: &str| takes.iter().find(|t| t.name == name).unwrap();
  assert_eq!(take("song_11092026_9").duration_secs, Some(0.5));
  assert_eq!(take("song_11092026_10").duration_secs, Some(2.0));
  assert_eq!(take("BROKEN").duration_secs, None);
  assert!(take("BROKEN").modified_ms.is_some());
  assert_eq!(take("song_11092026_9").folder, "");
  assert_eq!(take("lead").folder, "Vocals/Verse 2");

  // newest first
  assert!(takes.windows(2).all(|w| w[0].modified_ms >= w[1].modified_ms));

  // nothing recorded yet
  assert_eq!(list_recordings(&dir.join("missing")), RecordingsListing::default());
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_take_numbers_count_every_folder() {
  let dir = test_dir("numbers");
  let recordings = dir.join("recordings");
  touch(&recordings.join("song_11092026_0.wav"));
  touch(&recordings.join("Keepers").join("Song_11092026_1.wav"));

  let next = next_take_paths(&recordings, "song", "11092026", 2);
  assert_eq!(next, vec![recordings.join("song_11092026_2.wav"), recordings.join("song_11092026_3.wav")]);
  assert_eq!(next_take_paths(&recordings, "song", "11092026", 1)[0], recordings.join("song_11092026_2.wav"));
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_folder_in() {
  let dir = test_dir("folder-in");
  let recordings = dir.join("recordings");
  fs::create_dir_all(recordings.join("a").join("b")).unwrap();
  touch(&recordings.join("take.wav"));

  assert_eq!(folder_in(&recordings, "").unwrap(), recordings);
  assert_eq!(folder_in(&recordings, "a/b").unwrap(), recordings.join("a").join("b"));
  assert!(folder_in(&recordings, "missing").is_err());
  assert!(folder_in(&recordings, "take.wav").is_err());
  assert!(folder_in(&recordings, "..").is_err());
  assert!(folder_in(&recordings, "a/../..").is_err());
  // a leading slash is still inside the recordings folder
  assert!(folder_in(&recordings, "/a").unwrap().starts_with(&recordings));
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_folders() {
  let dir = test_dir("folders");
  let recordings = dir.join("recordings");

  // the recordings folder is made for the first folder
  assert_eq!(create_folder(&recordings, "", " Vocals ").unwrap(), "Vocals");
  assert_eq!(create_folder(&recordings, "Vocals", "Verse").unwrap(), "Vocals/Verse");
  assert!(create_folder(&recordings, "Vocals", "Verse").is_err());
  assert!(create_folder(&recordings, "missing", "x").is_err());
  assert!(create_folder(&recordings, "", "..").is_err());
  assert!(create_folder(&recordings, "", "a/b").is_err());

  // renaming keeps it where it is
  let (from, to) = rename_folder(&recordings, "Vocals/Verse", "Chorus").unwrap();
  assert_eq!(from, recordings.join("Vocals").join("Verse"));
  assert_eq!(to, recordings.join("Vocals").join("Chorus"));
  assert!(to.is_dir() && !from.exists());
  assert!(rename_folder(&recordings, "", "x").is_err());
  create_folder(&recordings, "", "Drums").unwrap();
  assert!(rename_folder(&recordings, "Drums", "Vocals").is_err());
  // only a change of case
  assert!(rename_folder(&recordings, "Drums", "drums").is_ok());
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_move_into() {
  let dir = test_dir("move-into");
  let recordings = dir.join("recordings");
  let take = PathBuf::from(touch(&recordings.join("take.wav")));
  fs::create_dir_all(recordings.join("A").join("B")).unwrap();
  touch(&recordings.join("A").join("other.wav"));

  // a take into a folder, and back out
  let moved = move_into(&recordings, &take, "A/B").unwrap();
  assert_eq!(moved, recordings.join("A").join("B").join("take.wav"));
  assert!(moved.is_file() && !take.exists());
  let back = move_into(&recordings, &moved, "").unwrap();
  assert_eq!(back, take);
  // already there
  assert_eq!(move_into(&recordings, &take, "").unwrap(), take);

  // a folder, with what's in it
  let folder = move_into(&recordings, &recordings.join("A").join("B"), "").unwrap();
  assert_eq!(folder, recordings.join("B"));
  assert!(recordings.join("B").is_dir());

  // not inside itself, or its own folders
  assert!(move_into(&recordings, &recordings.join("A"), "A").is_err());
  fs::create_dir_all(recordings.join("A").join("C")).unwrap();
  assert!(move_into(&recordings, &recordings.join("A"), "A/C").is_err());

  // a name that's taken
  touch(&recordings.join("other.wav"));
  assert!(move_into(&recordings, &recordings.join("A").join("other.wav"), "").is_err());

  // nothing outside, and not the recordings folder itself
  let outside = PathBuf::from(touch(&dir.join("outside.wav")));
  assert!(move_into(&recordings, &outside, "A").is_err());
  assert!(move_into(&recordings, &recordings, "A").is_err());
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_rename_take_stays_in_its_folder() {
  let dir = test_dir("rename-in-folder");
  let recordings = dir.join("recordings");
  let take = PathBuf::from(touch(&recordings.join("Vocals").join("take.wav")));
  assert_eq!(rename_take(&take, &recordings, "lead").unwrap(), recordings.join("Vocals").join("lead.wav"));
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_natural_cmp() {
  use std::cmp::Ordering::*;
  assert_eq!(natural_cmp("take_9", "take_10"), Less);
  assert_eq!(natural_cmp("take_10", "take_10"), Equal);
  assert_eq!(natural_cmp("a_2_1", "a_10_0"), Less);
}

#[test]
fn test_existing_recordings_dir_does_not_make_a_session() {
  let mut location = ProjectLocation::default();
  assert_eq!(location.existing_recordings_dir(), None);
  assert!(location.session.is_none());

  let session = location.recordings_dir().unwrap();
  assert_eq!(location.existing_recordings_dir(), Some(session));

  location.reset(Some(PathBuf::from("/music/song/song.dawesome")));
  assert_eq!(location.existing_recordings_dir(), Some(PathBuf::from("/music/song/recordings")));
}


#[test]
fn test_take_file_name() {
  assert_eq!(take_file_name("  vocal idea "), Ok("vocal idea.wav".to_string()));
  assert_eq!(take_file_name("riff.WAV"), Ok("riff.wav".to_string()));
  assert_eq!(take_file_name("v1.2"), Ok("v1.2.wav".to_string()));
  assert!(take_file_name("   ").is_err());
  assert!(take_file_name(".wav").is_err());
  assert!(take_file_name(".hidden").is_err());
  assert!(take_file_name("../escape").is_err());
  assert!(take_file_name("a/b").is_err());
  assert!(take_file_name("a\\b").is_err());
  assert!(take_file_name(&"x".repeat(121)).is_err());
}

#[test]
fn test_rename_take() {
  let dir = test_dir("rename");
  let recordings = dir.join("recordings");
  let take = PathBuf::from(touch(&recordings.join("song_11092026_0.wav")));
  let other = PathBuf::from(touch(&recordings.join("other.wav")));
  let outside = PathBuf::from(touch(&dir.join("elsewhere.wav")));

  let renamed = rename_take(&take, &recordings, "Vocal idea").unwrap();
  assert_eq!(renamed, recordings.join("Vocal idea.wav"));
  assert!(renamed.is_file() && !take.exists());

  // the same name again does nothing
  assert_eq!(rename_take(&renamed, &recordings, "Vocal idea.wav").unwrap(), renamed);
  // only a change of case
  let cased = rename_take(&renamed, &recordings, "vocal idea").unwrap();
  assert!(cased.is_file());

  // taken, outside the folder, or missing
  assert!(rename_take(&cased, &recordings, "other").is_err());
  assert!(other.is_file());
  assert!(rename_take(&outside, &recordings, "moved").is_err());
  assert!(rename_take(&recordings.join("gone.wav"), &recordings, "x").is_err());
  let _ = fs::remove_dir_all(&dir);
}
