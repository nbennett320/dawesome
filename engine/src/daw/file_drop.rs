// files dragged in from outside the app (from Finder): what in them can be
// played, and copies of them in the project's clips folder
//
// a copy is the file as it was, byte for byte. a file the clips folder
// already holds (the same bytes, under any name) isn't copied again, and one
// that's already the project's (in its clips or recordings) is used where it
// is. nothing is ever written over. commands/file_drop.rs is the frontend's
// side of these
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::daw;
use crate::daw::actions::audio::{dir_to_change, Library};
use crate::daw::{audiograph, bounce};
use crate::daw::project_files::{free_path, is_audio, is_inside, natural_cmp, relative, walk};

// a file that can be played, from what was dropped
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DroppedFile {
  pub path: String,
  // the folder it goes in, inside the clips folder, with / between folders:
  // "" for a file dropped by itself, "Pack/Kicks" for one in Kicks in a
  // dropped folder Pack
  pub folder: String,
}

// something dropped that won't be brought in, and why, in words for the user
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
  pub path: String,
  pub reason: String,
}

// what was dropped, as files that can be played and things that can't
#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DropScan {
  pub files: Vec<DroppedFile>,
  pub skipped: Vec<Skipped>,
}

pub const NOT_AUDIO: &str = "not audio";
pub const UNREADABLE: &str = "could not be read";
pub const NOT_FOUND: &str = "not found";

// whether the engine can play a file: its decoder opens it (which reads
// only the start of it, so it's quick)
fn playable(path: &Path) -> bool {
  fs::File::open(path).ok()
    .map_or(false, |file| rodio::Decoder::new(std::io::BufReader::new(file)).is_ok())
}

fn skipped(path: &Path, reason: &str) -> Skipped {
  Skipped { path: path.to_string_lossy().into_owned(), reason: reason.to_string() }
}

// what was dropped, in the order it was dropped. a folder's files (its
// folders' too) come in order of where they are in it, numbers as numbers,
// and keep where they were in it under a folder of its name. files in a
// folder that aren't audio are left out without a word (a sample pack's
// readme, say); a file dropped by itself that isn't, or audio that can't be
// read, is listed as skipped
pub fn scan(paths: &[String]) -> DropScan {
  let mut scan = DropScan::default();
  for dropped in paths.iter().map(Path::new) {
    if dropped.is_dir() {
      let name = dropped.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
      let mut found: Vec<(String, PathBuf)> = Vec::new();
      walk(dropped, &mut |path, is_dir| {
        if !is_dir && is_audio(path) { found.push((relative(dropped, path), path.to_path_buf())); }
      });
      found.sort_by(|(a, _), (b, _)| natural_cmp(&a.to_lowercase(), &b.to_lowercase()));
      for (_, path) in found {
        if !playable(&path) {
          scan.skipped.push(skipped(&path, UNREADABLE));
          continue;
        }
        let inside = path.parent().map(|parent| relative(dropped, parent)).unwrap_or_default();
        let folder = if inside.is_empty() { name.clone() } else { format!("{}/{}", name, inside) };
        scan.files.push(DroppedFile { path: path.to_string_lossy().into_owned(), folder });
      }
    } else if !dropped.is_file() {
      scan.skipped.push(skipped(dropped, NOT_FOUND));
    } else if !is_audio(dropped) {
      scan.skipped.push(skipped(dropped, NOT_AUDIO));
    } else if !playable(dropped) {
      scan.skipped.push(skipped(dropped, UNREADABLE));
    } else {
      scan.files.push(DroppedFile { path: dropped.to_string_lossy().into_owned(), folder: String::new() });
    }
  }
  scan
}

// a file brought in: where it came from, and the file that's to be played
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedFile {
  pub source: String,
  pub path: String,
  // not copied: the project already had it (the same bytes in its clips, or
  // the file itself in its clips or recordings)
  pub reused: bool,
}

#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
  // in the order they were given
  pub files: Vec<ImportedFile>,
  pub skipped: Vec<Skipped>,
}

// a folder inside the clips folder, from its parts with / between them: only
// names (no "..", no root), so nothing lands outside it
fn folder_inside(clips: &Path, folder: &str) -> Result<PathBuf, String> {
  let refused = || format!("'{}' isn't a folder in the clips", folder);
  if folder.starts_with('/') || folder.contains('\\') { return Err(refused()); }
  let mut dir = clips.to_path_buf();
  for part in folder.split('/').filter(|part| !part.is_empty()) {
    match Path::new(part).components().collect::<Vec<_>>().as_slice() {
      [Component::Normal(name)] if !part.starts_with('.') => dir.push(name),
      _ => return Err(refused()),
    }
  }
  Ok(dir)
}

// whether two files hold the same bytes
fn same_bytes(a: &Path, b: &Path) -> bool {
  let (Ok(a), Ok(b)) = (fs::File::open(a), fs::File::open(b)) else { return false };
  let (mut a, mut b) = (std::io::BufReader::new(a), std::io::BufReader::new(b));
  let (mut x, mut y) = ([0u8; 64 * 1024], [0u8; 64 * 1024]);
  loop {
    let (Ok(n), Ok(m)) = (read_full(&mut a, &mut x), read_full(&mut b, &mut y)) else { return false };
    if n != m || x[..n] != y[..m] { return false; }
    if n == 0 { return true; }
  }
}

// as much of `buf` as the file has left
fn read_full(file: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
  let mut filled = 0;
  while filled < buf.len() {
    match file.read(&mut buf[filled..])? {
      0 => break,
      n => filled += n,
    }
  }
  Ok(filled)
}

// the clips folder's files by size, to find one with the same bytes as a
// file being brought in without reading every file
struct Known(HashMap<u64, Vec<PathBuf>>);

impl Known {
  fn of(clips: &Path) -> Self {
    let mut known: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    walk(clips, &mut |path, is_dir| {
      if is_dir { return; }
      if let Ok(meta) = fs::metadata(path) { known.entry(meta.len()).or_default().push(path.to_path_buf()); }
    });
    Known(known)
  }

  fn find(&self, path: &Path, len: u64) -> Option<&PathBuf> {
    self.0.get(&len)?.iter().find(|known| same_bytes(known, path))
  }

  fn add(&mut self, path: PathBuf, len: u64) {
    self.0.entry(len).or_default().push(path);
  }
}

// copy a file to `target`, by way of a hidden file beside it, so a file
// that's half copied never has the name
fn copy_to(source: &Path, target: &Path) -> Result<(), String> {
  let name = target.file_name().unwrap_or_default().to_string_lossy();
  let part = target.with_file_name(format!(".{}.part", name));
  let copied = fs::copy(source, &part).and_then(|_| fs::rename(&part, target));
  copied.map_err(|e| {
    let _ = fs::remove_file(&part);
    format!("could not be copied ({})", e)
  })
}

// bring files into the project's clips folder (an unsaved project's own
// until it's saved), each into `into` (a folder in the clips, "" for the
// top) and then its own folder. a file that can't be brought in is skipped,
// and the rest still are
pub fn import(state: &daw::Session, files: &[DroppedFile], into: &str) -> Result<Imported, String> {
  let mut imported = Imported::default();
  if files.is_empty() { return Ok(imported); }

  let clips = dir_to_change(state, Library::Clips, true)?;
  let recordings = state.project.lock().unwrap().existing_recordings_dir();
  let into = folder_inside(&clips, into)?;
  // (one writer at a time: two could choose the same name)
  let _writing = bounce::writing();
  let mut known = Known::of(&clips);

  for file in files {
    let source = Path::new(&file.path);
    let take = |path: &Path, reused: bool| ImportedFile {
      source: file.path.clone(), path: path.to_string_lossy().into_owned(), reused,
    };

    let already_ours = is_inside(source, &clips) || recordings.as_deref().map_or(false, |dir| is_inside(source, dir));
    if already_ours {
      imported.files.push(take(source, true));
      continue;
    }
    let len = match fs::metadata(source) {
      Ok(meta) if meta.is_file() => meta.len(),
      _ => {
        imported.skipped.push(skipped(source, NOT_FOUND));
        continue;
      }
    };
    if let Some(same) = known.find(source, len) {
      audiograph::decoded_copied(&file.path, &same.to_string_lossy());
      imported.files.push(take(&same.clone(), true));
      continue;
    }

    let made = folder_inside(&into, &file.folder).and_then(|dir| {
      fs::create_dir_all(&dir).map_err(|e| format!("could not make its folder ({})", e))?;
      let name = source.file_name().ok_or_else(|| UNREADABLE.to_string())?.to_string_lossy().into_owned();
      let target = free_path(&dir, &name);
      copy_to(source, &target)?;
      Ok(target)
    });
    match made {
      Ok(target) => {
        // (its sound, if it's been decoded already for the ghost drawn while
        // it was dragged in, is the copy's too)
        audiograph::decoded_copied(&file.path, &target.to_string_lossy());
        imported.files.push(take(&target, false));
        known.add(target, len);
      }
      Err(reason) => imported.skipped.push(skipped(source, &reason)),
    }
  }
  Ok(imported)
}

#[cfg(test)]
mod tests;
