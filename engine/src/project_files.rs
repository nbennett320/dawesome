// where a project's files live
//
// a project is a folder named after it, holding the project file and its
// recordings:
//
//   my_project/
//     recordings/
//       my_project_11092026_0.wav
//     my_project.dawesome
//
// projects saved before this (a bare .dawesome file) still load and save
// where they are, with their recordings in a recordings folder next to the
// file. takes recorded before a project is first saved go in a folder of
// their own in the app's data folder, and move into the project's
// recordings folder when it's saved
use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};

pub const PROJECT_EXTENSION: &str = "dawesome";
pub const RECORDINGS_FOLDER: &str = "recordings";
// what takes are called before the project has a name
pub const UNTITLED: &str = "untitled";

// where the project is, and where its takes go before it's saved
#[derive(Debug, Default)]
pub struct ProjectLocation {
  // the project file, once it's been saved or opened
  pub path: Option<PathBuf>,
  // this unsaved project's folder of takes, once something's been recorded
  pub session: Option<PathBuf>,
}

impl ProjectLocation {
  // the project's name, as its takes are named
  pub fn name(&self) -> String {
    self.path.as_deref().and_then(project_name).unwrap_or_else(|| UNTITLED.to_string())
  }

  // where takes are recorded to: the project's recordings folder, or (for a
  // project that's never been saved) a folder of its own in the app's data.
  // the folder isn't made until something's recorded
  pub fn recordings_dir(&mut self) -> Result<PathBuf, String> {
    if let Some(path) = &self.path {
      return Ok(recordings_dir_for(path));
    }
    if let Some(session) = &self.session {
      return Ok(session.clone());
    }

    let root = unsaved_root().ok_or("there's no app data folder to record into")?;
    let session = root.join(session_folder_name(&today(), random_hex()));
    self.session = Some(session.clone());
    Ok(session)
  }

  // the recordings folder as it is now, without making an unsaved project
  // a folder of its own: None until an unsaved project records something
  pub fn existing_recordings_dir(&self) -> Option<PathBuf> {
    match (&self.path, &self.session) {
      (Some(path), _) => Some(recordings_dir_for(path)),
      (None, Some(session)) => Some(session.clone()),
      (None, None) => None,
    }
  }

  // a new or opened project: takes no longer go in the old project's
  // places. an unsaved project's folder of takes is tidied away if nothing
  // was recorded into it
  pub fn reset(&mut self, path: Option<PathBuf>) {
    if let Some(session) = self.session.take() {
      remove_if_empty(&session);
    }
    self.path = path;
  }
}

// the file a project is saved to when it's saved somewhere new: in a folder
// named after it, unless the chosen file is already in one
pub fn project_file_for(chosen: &Path) -> PathBuf {
  let stem = match chosen.file_stem() {
    Some(stem) => stem.to_owned(),
    None => return chosen.to_path_buf(),
  };
  let mut file = PathBuf::from(&stem);
  file.set_extension(PROJECT_EXTENSION);

  let parent = chosen.parent().unwrap_or_else(|| Path::new(""));
  if parent.file_name() == Some(stem.as_os_str()) {
    parent.join(file)
  } else {
    parent.join(&stem).join(file)
  }
}

// a project's name, from its file
pub fn project_name(project_path: &Path) -> Option<String> {
  project_path.file_stem().map(|stem| stem.to_string_lossy().into_owned())
}

// a project's recordings folder, next to its file
pub fn recordings_dir_for(project_path: &Path) -> PathBuf {
  project_path.parent().unwrap_or_else(|| Path::new("")).join(RECORDINGS_FOLDER)
}

// where unsaved projects' takes go, e.g.
// ~/Library/Application Support/dawesome/unsaved
fn unsaved_root() -> Option<PathBuf> {
  tauri::api::path::data_dir().map(|dir| dir.join("dawesome").join("unsaved"))
}

// a take's date, month first as in its name: 11092026
pub fn today() -> String {
  chrono::Local::now().format("%m%d%Y").to_string()
}

fn random_hex() -> u32 {
  let mut hasher = RandomState::new().build_hasher();
  hasher.write_u128(std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_nanos())
    .unwrap_or_default());
  hasher.finish() as u32
}

// an unsaved project's folder of takes: the date, year first so they sort,
// and a random part to tell apart ones started the same day
// - date: as in take names (month, day, year)
pub fn session_folder_name(date: &str, random: u32) -> String {
  let year_first = if date.len() == 8 { format!("{}{}", &date[4..], &date[..4]) } else { date.to_string() };
  format!("{}-{:06x}", year_first, random & 0xff_ffff)
}

// a take's file name: the project, the day it was recorded, and its number
// that day
pub fn take_name(project: &str, date: &str, number: u32) -> String {
  format!("{}_{}_{}.wav", project, date, number)
}

// a take's project, date and number, if the file is named like one
pub fn parse_take_name(file_name: &str) -> Option<(String, String, u32)> {
  let stem = file_name.strip_suffix(".wav")?;
  let mut parts = stem.rsplitn(3, '_');
  let number = parts.next()?.parse().ok()?;
  let date = parts.next()?;
  let project = parts.next()?;

  if date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) || project.is_empty() {
    return None;
  }
  Some((project.to_string(), date.to_string(), number))
}

// the next free take name in a folder, for a project on a date
pub fn next_take_path(dir: &Path, project: &str, date: &str) -> PathBuf {
  (0..)
    .map(|n| dir.join(take_name(project, date, n)))
    .find(|path| !path.exists())
    .expect("a free take number")
}

// the next `count` free take names in a folder, numbered one after another
// (for takes recorded together)
pub fn next_take_paths(dir: &Path, project: &str, date: &str, count: usize) -> Vec<PathBuf> {
  (0..)
    .map(|n| dir.join(take_name(project, date, n)))
    .filter(|path| !path.exists())
    .take(count)
    .collect()
}

// a free path in `dir` for a file called `file_name`, numbering it when
// that's taken
fn free_path(dir: &Path, file_name: &str) -> PathBuf {
  let path = dir.join(file_name);
  if !path.exists() { return path; }

  let name = Path::new(file_name);
  let stem = name.file_stem().unwrap_or_default().to_string_lossy();
  let extension = name.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
  (1..)
    .map(|n| dir.join(format!("{} {}{}", stem, n, extension)))
    .find(|path| !path.exists())
    .expect("a free file name")
}

// whether `path` is inside `dir` (both resolved, so links and relative
// paths compare properly)
pub fn is_inside(path: &Path, dir: &Path) -> bool {
  match (fs::canonicalize(path), fs::canonicalize(dir)) {
    (Ok(path), Ok(dir)) => path.starts_with(dir),
    _ => false,
  }
}

pub fn same_dir(a: &Path, b: &Path) -> bool {
  match (fs::canonicalize(a), fs::canonicalize(b)) {
    (Ok(a), Ok(b)) => a == b,
    _ => a == b,
  }
}

// a take moved (or copied) into a project's recordings folder
#[derive(Debug, Clone, PartialEq)]
pub struct Relocated {
  pub from: String,
  pub to: String,
}

// bring the takes among `paths` that are in `from` into `to`, renamed after
// `project` (keeping the day they were recorded). takes are moved, or copied
// when `copy` is set (so the project they came from keeps them). each path
// is brought over once, however often it's listed. ones that can't be are
// left where they are, and reported in the errors
pub fn relocate_takes(
  paths: &[String],
  from: &Path,
  to: &Path,
  project: &str,
  copy: bool,
) -> (Vec<Relocated>, Vec<String>) {
  let mut relocated: Vec<Relocated> = Vec::new();
  let mut errors = Vec::new();

  for path in paths {
    if relocated.iter().any(|r| &r.from == path) { continue; }
    let source = Path::new(path);
    if !source.is_file() || !is_inside(source, from) { continue; }

    if let Err(e) = fs::create_dir_all(to) {
      errors.push(format!("could not make the folder '{}': {}", to.display(), e));
      break;
    }

    let file_name = source.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let target = match parse_take_name(&file_name) {
      Some((_, date, _)) => next_take_path(to, project, &date),
      None => free_path(to, &file_name),
    };

    let done = if copy {
      fs::copy(source, &target).map(|_| ())
    } else {
      // a different disk can't be renamed onto: copy, then remove
      fs::rename(source, &target).or_else(|_| {
        fs::copy(source, &target).and_then(|_| fs::remove_file(source))
      })
    };

    match done {
      Ok(()) => relocated.push(Relocated { from: path.clone(), to: target.to_string_lossy().into_owned() }),
      Err(e) => errors.push(format!("could not bring '{}' into the project: {}", file_name, e)),
    }
  }

  (relocated, errors)
}

// remove a folder if there's nothing in it
pub fn remove_if_empty(dir: &Path) {
  let empty = fs::read_dir(dir).map(|mut entries| entries.next().is_none()).unwrap_or(false);
  if empty {
    let _ = fs::remove_dir(dir);
  }
}

// a take in the recordings folder, for the sidebar's recordings browser
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingFile {
  pub path: String,
  // the file name, without .wav
  pub name: String,
  // how long it plays, or None when it can't be read (e.g. still recording,
  // before its header is written)
  pub duration_secs: Option<f64>,
  // when it was last written, in ms since the unix epoch
  pub modified_ms: Option<u64>,
}

// how long a wav file plays, from its header
fn wav_seconds(path: &Path) -> Option<f64> {
  let reader = hound::WavReader::open(path).ok()?;
  let rate = reader.spec().sample_rate;
  if rate == 0 { return None; }
  Some(reader.duration() as f64 / rate as f64)
}

fn is_wav(path: &Path) -> bool {
  path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("wav"))
}

// the takes in a recordings folder, newest first. a folder that isn't there
// (nothing recorded yet) has none
pub fn list_recordings(dir: &Path) -> Vec<RecordingFile> {
  let entries = match fs::read_dir(dir) {
    Ok(entries) => entries,
    Err(_) => return Vec::new(),
  };

  let mut takes: Vec<RecordingFile> = entries
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.is_file() && is_wav(path))
    .map(|path| {
      let modified_ms = fs::metadata(&path).and_then(|m| m.modified()).ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
      RecordingFile {
        name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        duration_secs: wav_seconds(&path),
        modified_ms,
        path: path.to_string_lossy().into_owned(),
      }
    })
    .collect();

  // newest first. takes written together (or on a file system that rounds
  // times) fall back to their names, highest number first
  takes.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then_with(|| natural_cmp(&b.name, &a.name)));
  takes
}

// compares names with the numbers in them as numbers, so take_10 comes after
// take_9
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
  fn parts(s: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut digits = String::new();
    for c in s.chars() {
      if c.is_ascii_digit() {
        digits.push(c);
      } else {
        if !digits.is_empty() {
          out.push((std::mem::take(&mut text), digits.parse().unwrap_or(u64::MAX)));
          digits.clear();
        }
        text.push(c);
      }
    }
    out.push((text, if digits.is_empty() { 0 } else { digits.parse().unwrap_or(u64::MAX) }));
    out
  }
  parts(a).cmp(&parts(b))
}

#[cfg(test)]
mod tests {
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
    assert_eq!(next_take_path(&dir, "song", "11092026"), dir.join("song_11092026_0.wav"));

    touch(&dir.join("song_11092026_0.wav"));
    touch(&dir.join("song_11092026_1.wav"));
    // another day's takes don't count
    touch(&dir.join("song_11082026_2.wav"));
    assert_eq!(next_take_path(&dir, "song", "11092026"), dir.join("song_11092026_2.wav"));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_unsaved_takes_move_into_the_project() {
    let dir = test_dir("move");
    let session = dir.join("unsaved");
    let recordings = dir.join("song").join("recordings");
    let a = touch(&session.join("untitled_11092026_0.wav"));
    let b = touch(&session.join("untitled_11102026_0.wav"));
    // not a take's name, and a sample from elsewhere
    let c = touch(&session.join("odd name.wav"));
    let elsewhere = touch(&dir.join("samples").join("kick.wav"));
    // a take already in the project
    touch(&recordings.join("song_11092026_0.wav"));

    let paths = vec![a.clone(), b.clone(), c.clone(), a.clone(), elsewhere.clone()];
    let (moved, errors) = relocate_takes(&paths, &session, &recordings, "song", false);

    assert!(errors.is_empty(), "{:?}", errors);
    let to = |from: &str| moved.iter().find(|r| r.from == from).map(|r| PathBuf::from(&r.to));
    assert_eq!(to(&a), Some(recordings.join("song_11092026_1.wav")));
    assert_eq!(to(&b), Some(recordings.join("song_11102026_0.wav")));
    assert_eq!(to(&c), Some(recordings.join("odd name.wav")));
    assert_eq!(to(&elsewhere), None);
    assert_eq!(moved.len(), 3);

    // moved, not copied
    assert!(!Path::new(&a).exists());
    assert!(recordings.join("song_11092026_1.wav").is_file());
    assert!(Path::new(&elsewhere).exists());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_save_as_copies_takes() {
    let dir = test_dir("copy");
    let old = dir.join("song").join("recordings");
    let new = dir.join("remix").join("recordings");
    let take = touch(&old.join("song_11092026_0.wav"));

    let (copied, errors) = relocate_takes(&[take.clone()], &old, &new, "remix", true);

    assert!(errors.is_empty());
    assert_eq!(copied[0].to, new.join("remix_11092026_0.wav").to_string_lossy());
    // the old project keeps its take
    assert!(Path::new(&take).is_file());

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

    let takes = list_recordings(&recordings);
    let names: Vec<&str> = takes.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"BROKEN"));

    let take = |name: &str| takes.iter().find(|t| t.name == name).unwrap();
    assert_eq!(take("song_11092026_9").duration_secs, Some(0.5));
    assert_eq!(take("song_11092026_10").duration_secs, Some(2.0));
    assert_eq!(take("BROKEN").duration_secs, None);
    assert!(take("BROKEN").modified_ms.is_some());

    // newest first
    assert!(takes.windows(2).all(|w| w[0].modified_ms >= w[1].modified_ms));

    // nothing recorded yet
    assert!(list_recordings(&dir.join("missing")).is_empty());
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

}
