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

// the file names already used anywhere in a recordings folder (its folders
// too), in lower case, so a take moved between folders can't land on a name
// that's taken
pub fn names_in_tree(dir: &Path) -> std::collections::HashSet<String> {
  let mut names = std::collections::HashSet::new();
  walk(dir, &mut |path, is_dir| {
    if !is_dir {
      if let Some(name) = path.file_name() {
        names.insert(name.to_string_lossy().to_lowercase());
      }
    }
  });
  names
}

// the next `count` free take names in a folder, numbered one after another
// (for takes recorded together)
pub fn next_take_paths(dir: &Path, project: &str, date: &str, count: usize) -> Vec<PathBuf> {
  let taken = names_in_tree(dir);
  (0..)
    .map(|n| take_name(project, date, n))
    .filter(|name| !taken.contains(&name.to_lowercase()))
    .map(|name| dir.join(name))
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

// bring everything in the recordings folder `from` into `to`, keeping its
// folders (empty ones too), used or not. takes named like takes are renamed
// after `project` (keeping the day they were recorded, numbered so no name
// repeats anywhere in `to`), other files keep their names. they're moved, or
// copied when `copy` is set (so the project they came from keeps them).
// files that can't be brought over are left where they are, and reported in
// the errors
pub fn relocate_takes(from: &Path, to: &Path, project: &str, copy: bool) -> (Vec<Relocated>, Vec<String>) {
  let mut relocated: Vec<Relocated> = Vec::new();
  let mut errors = Vec::new();
  if !from.is_dir() { return (relocated, errors); }

  // everything first: moving files while walking the folder could miss some
  let mut folders = Vec::new();
  let mut files = Vec::new();
  walk(from, &mut |path, is_dir| {
    if is_dir { folders.push(path.to_path_buf()) } else { files.push(path.to_path_buf()) }
  });
  if files.is_empty() && folders.is_empty() { return (relocated, errors); }

  if let Err(e) = fs::create_dir_all(to) {
    errors.push(format!("could not make the folder '{}': {}", to.display(), e));
    return (relocated, errors);
  }
  for folder in &folders {
    let made = to.join(folder.strip_prefix(from).unwrap_or(folder));
    if let Err(e) = fs::create_dir_all(&made) {
      errors.push(format!("could not make the folder '{}': {}", made.display(), e));
    }
  }

  let mut taken = names_in_tree(to);
  for source in files {
    let within = source.parent().and_then(|parent| parent.strip_prefix(from).ok()).unwrap_or_else(|| Path::new(""));
    let dest = to.join(within);
    if !dest.is_dir() { continue; } // its folder couldn't be made, which is reported

    let file_name = source.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let target = match parse_take_name(&file_name) {
      Some((_, date, _)) => (0..)
        .map(|n| take_name(project, &date, n))
        .find(|name| !taken.contains(&name.to_lowercase()) && !dest.join(name).exists())
        .map(|name| dest.join(name))
        .expect("a free take number"),
      None => free_path(&dest, &file_name),
    };

    let done = if copy {
      fs::copy(&source, &target).map(|_| ())
    } else {
      // a different disk can't be renamed onto: copy, then remove
      fs::rename(&source, &target).or_else(|_| {
        fs::copy(&source, &target).and_then(|_| fs::remove_file(&source))
      })
    };

    match done {
      Ok(()) => {
        if let Some(name) = target.file_name() { taken.insert(name.to_string_lossy().to_lowercase()); }
        relocated.push(Relocated {
          from: source.to_string_lossy().into_owned(),
          to: target.to_string_lossy().into_owned(),
        });
      }
      Err(e) => errors.push(format!("could not bring '{}' into the project: {}", file_name, e)),
    }
  }

  // moved: the folders left behind are empty now (unless something couldn't
  // be moved). the deepest go first, so their parents empty too
  if !copy {
    folders.sort_by_key(|folder| std::cmp::Reverse(folder.components().count()));
    for folder in &folders { remove_if_empty(folder); }
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
  // the folder it's in, relative to the recordings folder ("" at the top)
  pub folder: String,
  // how long it plays, or None when it can't be read (e.g. still recording,
  // before its header is written)
  pub duration_secs: Option<f64>,
  // when it was last written, in ms since the unix epoch
  pub modified_ms: Option<u64>,
}

// what's in a recordings folder
#[derive(serde::Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordingsListing {
  // newest first
  pub takes: Vec<RecordingFile>,
  // every folder in it, relative to it, so empty ones show too. sorted by
  // name, numbers as numbers
  pub folders: Vec<String>,
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

// folders inside folders can go this deep before the rest is left out
const MAX_DEPTH: usize = 32;

// call `visit` with everything inside `dir`, folders before what's in them.
// hidden files are skipped, and links aren't followed (so a link to a folder
// above can't loop forever)
fn walk(dir: &Path, visit: &mut dyn FnMut(&Path, bool)) {
  fn go(dir: &Path, depth: usize, visit: &mut dyn FnMut(&Path, bool)) {
    if depth > MAX_DEPTH { return; }
    let entries = match fs::read_dir(dir) {
      Ok(entries) => entries,
      Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
      if entry.file_name().to_string_lossy().starts_with('.') { continue; }
      let kind = match entry.file_type() {
        Ok(kind) => kind,
        Err(_) => continue,
      };
      let path = entry.path();
      if kind.is_dir() {
        visit(&path, true);
        go(&path, depth + 1, visit);
      } else if kind.is_file() {
        visit(&path, false);
      }
    }
  }
  go(dir, 0, visit);
}

// a path inside the recordings folder, relative to it, with / between folders
pub fn relative(dir: &Path, path: &Path) -> String {
  path.strip_prefix(dir).ok()
    .map(|rel| rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
    .unwrap_or_default()
}

// the takes and folders in a recordings folder. a folder that isn't there
// (nothing recorded yet) has none
pub fn list_recordings(dir: &Path) -> RecordingsListing {
  let mut listing = RecordingsListing::default();
  walk(dir, &mut |path, is_dir| {
    if is_dir {
      listing.folders.push(relative(dir, path));
      return;
    }
    if !is_wav(path) { return; }

    let modified_ms = fs::metadata(path).and_then(|m| m.modified()).ok()
      .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
      .map(|d| d.as_millis() as u64);
    listing.takes.push(RecordingFile {
      name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
      folder: path.parent().map(|parent| relative(dir, parent)).unwrap_or_default(),
      duration_secs: wav_seconds(path),
      modified_ms,
      path: path.to_string_lossy().into_owned(),
    });
  });

  // newest first. takes written together (or on a file system that rounds
  // times) fall back to their names, highest number first
  listing.takes.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then_with(|| natural_cmp(&b.name, &a.name)));
  listing.folders.sort_by(|a, b| natural_cmp(&a.to_lowercase(), &b.to_lowercase()));
  listing
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

// a name typed for a take or a folder, trimmed, refusing names that would
// put it somewhere else, or hide it
fn check_name<'a>(typed: &'a str, what: &str) -> Result<&'a str, String> {
  let name = typed.trim();
  if name.is_empty() {
    return Err(format!("a {} needs a name", what));
  }
  if name.starts_with('.') {
    return Err(format!("a {}'s name can't start with a dot", what));
  }
  if let Some(c) = name.chars().find(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()) {
    return Err(format!("a {}'s name can't have '{}' in it", what, c));
  }
  if name.chars().count() > 120 {
    return Err("that name is too long".into());
  }
  Ok(name)
}

// the file name a take is renamed to, from what was typed: the .wav is
// added (or kept, if it was typed)
pub fn take_file_name(typed: &str) -> Result<String, String> {
  let mut name = typed.trim();
  if name.len() >= 4 && name.is_char_boundary(name.len() - 4) && name[name.len() - 4..].eq_ignore_ascii_case(".wav") {
    name = name[..name.len() - 4].trim_end();
  }
  Ok(format!("{}.wav", check_name(name, "recording")?))
}

// a folder's name, from what was typed
pub fn folder_name(typed: &str) -> Result<String, String> {
  check_name(typed, "folder").map(str::to_string)
}

// a folder in a recordings folder, from its path relative to it ("" is the
// recordings folder itself). it has to be there already, and the path can't
// lead out of it
pub fn folder_in(dir: &Path, rel: &str) -> Result<PathBuf, String> {
  let mut path = dir.to_path_buf();
  for part in rel.split('/').filter(|part| !part.is_empty()) {
    if part == "." || part == ".." || part.contains('\\') || Path::new(part).is_absolute() {
      return Err(format!("'{}' isn't a folder in the recordings", rel));
    }
    path.push(part);
  }
  if !path.is_dir() || !(path == dir || is_inside(&path, dir)) {
    return Err(format!("there's no folder '{}' in the recordings", rel));
  }
  Ok(path)
}

// whether `to` names the same file as `from` (a change of case, on a file
// system that ignores it)
fn same_file(from: &Path, to: &Path) -> bool {
  match (fs::canonicalize(from), fs::canonicalize(to)) {
    (Ok(a), Ok(b)) => a == b,
    _ => false,
  }
}

// rename `from` to `to`, unless something else is called that already
fn rename_to(from: &Path, to: &Path, what: &str) -> Result<(), String> {
  if to == from { return Ok(()); }
  if to.exists() && !same_file(from, to) {
    return Err(format!("there's already a {} called '{}' there", what, to.file_name().unwrap_or_default().to_string_lossy()));
  }
  fs::rename(from, to).map_err(|e| format!("could not move the {}: {}", what, e))
}

// a new, empty folder inside `parent` (relative to the recordings folder,
// which is made if it isn't there yet). returns where it is, relative
pub fn create_folder(dir: &Path, parent: &str, typed: &str) -> Result<String, String> {
  let name = folder_name(typed)?;
  if parent.is_empty() {
    fs::create_dir_all(dir).map_err(|e| format!("could not make the recordings folder: {}", e))?;
  }
  let target = folder_in(dir, parent)?.join(&name);
  if target.exists() {
    return Err(format!("there's already something called '{}' there", name));
  }
  fs::create_dir(&target).map_err(|e| format!("could not make the folder: {}", e))?;
  Ok(relative(dir, &target))
}

// rename a folder, returning where it was and where it is now
pub fn rename_folder(dir: &Path, rel: &str, typed: &str) -> Result<(PathBuf, PathBuf), String> {
  if rel.is_empty() { return Err("the recordings folder itself can't be renamed".into()); }
  let from = folder_in(dir, rel)?;
  let to = from.parent().unwrap_or(dir).join(folder_name(typed)?);
  rename_to(&from, &to, "folder")?;
  Ok((from, to))
}

// move a take, or a folder (with everything in it), into another folder,
// returning where it is now. a folder can't go inside itself
pub fn move_into(dir: &Path, path: &Path, to_folder: &str) -> Result<PathBuf, String> {
  if !path.exists() || !is_inside(path, dir) || same_dir(path, dir) {
    return Err("that isn't in the project's recordings".into());
  }
  let dest = folder_in(dir, to_folder)?;
  if path.is_dir() && is_inside(&dest, path) {
    return Err("a folder can't go inside itself".into());
  }

  let to = dest.join(path.file_name().unwrap_or_default());
  let what = if path.is_dir() { "folder" } else { "recording" };
  if same_file(path, &to) { return Ok(path.to_path_buf()); }
  if to.exists() {
    return Err(format!("there's already a {} called '{}' there", what, to.file_name().unwrap_or_default().to_string_lossy()));
  }
  fs::rename(path, &to).map_err(|e| format!("could not move the {}: {}", what, e))?;
  Ok(to)
}

// rename a take in a recordings folder (it stays in the folder it's in),
// returning where it is now. the new name can't be taken (a change of case
// alone is fine, on file systems that ignore case)
pub fn rename_take(path: &Path, dir: &Path, typed: &str) -> Result<PathBuf, String> {
  if !path.is_file() || !is_inside(path, dir) {
    return Err("that isn't one of the project's recordings".into());
  }

  let target = path.parent().unwrap_or(dir).join(take_file_name(typed)?);
  rename_to(path, &target, "recording")?;
  Ok(target)
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

}
