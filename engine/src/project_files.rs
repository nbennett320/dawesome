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
// where clips written out by Deep copy and Join go, next to the recordings
pub const CLIPS_FOLDER: &str = "clips";
// what takes are called before the project has a name
pub const UNTITLED: &str = "untitled";

// where the project is, and where its takes go before it's saved
#[derive(Debug, Default)]
pub struct ProjectLocation {
  // the project file, once it's been saved or opened
  pub path: Option<PathBuf>,
  // this unsaved project's folder of takes, once something's been recorded
  pub session: Option<PathBuf>,
  // the project as it was last started, opened or saved, to tell whether
  // it's changed since (see project::has_unsaved_changes)
  pub baseline: Option<crate::project::Project>,
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

// a project's folder of written-out clips, next to its file
pub fn clips_dir_for(project_path: &Path) -> PathBuf {
  project_path.parent().unwrap_or_else(|| Path::new("")).join(CLIPS_FOLDER)
}

// a name fit for a file, from a sample's or a pattern's: nothing that could
// be a folder or isn't allowed in a file name, and never empty
pub fn file_stem_from(name: &str) -> String {
  let clean: String = name.trim()
    .chars()
    .map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '_' } else { c })
    .collect();
  // (short enough that "_copy_12.wav" still fits in a file name)
  let clean: String = clean.trim_matches(|c: char| c == '.' || c.is_whitespace()).chars().take(120).collect();
  let clean = clean.trim_end().to_string();
  if clean.is_empty() { "clip".into() } else { clean }
}

// a name without the "_copy_3" (for `tag` "copy") a written-out clip's file
// has on its end, so a copy of a copy is numbered on from it rather than
// growing a second ending
pub fn without_number(stem: &str, tag: &str) -> String {
  // (only the tag's own letters are lowered, so a place found in `lower` is
  // the same place in `stem`, whatever letters the name has)
  let lower = stem.to_ascii_lowercase();
  let ending = format!("_{}_", tag);
  match lower.rfind(&ending) {
    Some(at) if at > 0 && lower.len() > at + ending.len()
      && lower[at + ending.len()..].chars().all(|c| c.is_ascii_digit()) => stem[..at].to_string(),
    _ => stem.to_string(),
  }
}

// the next free "<stem>_<tag>_<n>.wav" in `dir`, numbered from 1. names are
// compared without regard to case, against every file anywhere in `dir`
// and against `taken` (lower case: names given out already and not written
// yet), so no file is ever written over
pub fn numbered_path(dir: &Path, stem: &str, tag: &str, taken: &std::collections::HashSet<String>) -> PathBuf {
  let used = names_in_tree(dir);
  (1..)
    .map(|n| format!("{}_{}_{}.wav", stem, tag, n))
    .find(|name| {
      let lower = name.to_lowercase();
      !used.contains(&lower) && !taken.contains(&lower) && !dir.join(name).exists()
    })
    .map(|name| dir.join(name))
    .expect("a free number")
}

// where unsaved projects' takes go, e.g.
// ~/Library/Application Support/dawesome/unsaved
fn unsaved_root() -> Option<PathBuf> {
  dirs::data_dir().map(|dir| dir.join("dawesome").join("unsaved"))
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
mod tests;
