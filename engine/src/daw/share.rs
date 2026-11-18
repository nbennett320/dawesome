// shareable projects (.sharedawesome): a project's folder zipped with every
// sample it uses, so it opens on someone else's computer as it does here
//
//   song.sharedawesome (a zip)
//     song/
//       song.dawesome
//       clips/  recordings/  mixdowns/
//       samples/
//         Drums/kicks/kick.wav    (from the File Paths folder "Drums")
//         other/snare.wav         (from a folder that isn't in File Paths)
//
// the project file in it is the project as it's open now (unsaved changes
// too), with its samples from outside its folder pointing at their copies
// in samples/. the project on disk isn't touched. the samples Dawesome
// comes with aren't copied: they're saved by name (see library.rs), and
// every Dawesome has them
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::daw::{self, library, native, project, project_files};
use crate::daw::file_drop::same_bytes;
use crate::daw::project::Project;

pub const SHARE_EXTENSION: &str = "sharedawesome";
// where samples from a folder that isn't in File Paths go
const OTHER_SAMPLES: &str = "other";
// the folders of a project, for one whose file isn't in a folder of its own
// (see folder_files)
const PROJECT_FOLDERS: [&str; 4] = [
  project_files::RECORDINGS_FOLDER,
  project_files::CLIPS_FOLDER,
  crate::daw::actions::mixdown::MIXDOWNS_FOLDER,
  project_files::SAMPLES_FOLDER,
];

// what exporting made
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Exported {
  pub path: String,
  // samples the project uses that couldn't be found, so aren't in it
  pub missing: Vec<String>,
  // the plugins the project uses (Dawesome's own effects aside), which
  // whoever opens it needs too
  pub plugins: Vec<String>,
}

// how far along an export is, in bytes
pub type Progress<'a> = &'a mut dyn FnMut(u64, u64);

// zip the open project, as it is now, into `dest`
// - sample_paths: the folders in File Paths, which name the folders samples
//   from them go in
// - layout: the window's panes and tabs, as a save writes them
pub fn export(
  state: &daw::Session,
  dest: &Path,
  sample_paths: &[String],
  layout: Option<serde_json::Value>,
  progress: Progress,
) -> Result<Exported, String> {
  let project_path = state.project.lock().unwrap().path.clone()
    .ok_or_else(|| "save the project before sharing it".to_string())?;
  let dir = project::project_dir(&project_path)?;
  let mut captured = project::capture(state, Some(&dir));
  captured.layout = layout;
  pack(captured, &project_path, dest, sample_paths, progress)
}

// a file going into the zip: from where, to where (under the top folder)
struct Entry {
  from: PathBuf,
  to: String,
}

// zip `project`, saved as `project_path` (with its sample paths stored for
// its folder), into `dest`
pub(crate) fn pack(
  mut project: Project,
  project_path: &Path,
  dest: &Path,
  sample_paths: &[String],
  progress: Progress,
) -> Result<Exported, String> {
  let dir = project::project_dir(project_path)?;
  let name = project_files::project_name(project_path)
    .unwrap_or_else(|| project_files::UNTITLED.into());

  let (copies, missing) = bring_in_samples(&mut project, &dir, sample_paths);
  let mut entries = folder_files(&dir, project_path, dest);
  entries.extend(copies);
  let plugins = plugins_of(&project);

  let json = serde_json::to_string_pretty(&project)
    .map_err(|e| format!("could not write the project: {}", e))?;
  write_zip(dest, &name, &json, &entries, progress)?;

  Ok(Exported { path: dest.to_string_lossy().into_owned(), missing, plugins })
}

// the folders in File Paths, each with the name its samples' folder in
// samples/ gets: its own, numbered when two share one
fn buckets(sample_paths: &[String]) -> Vec<(PathBuf, String)> {
  let mut taken: BTreeSet<String> = BTreeSet::from([OTHER_SAMPLES.to_string()]);
  sample_paths.iter().map(|path| {
    let root = fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    let base = root.file_name().map_or_else(|| "samples".into(), |n| n.to_string_lossy().into_owned());
    let name = (0..).map(|n| if n == 0 { base.clone() } else { format!("{} {}", base, n) })
      .find(|name| !taken.contains(&name.to_lowercase()))
      .unwrap();
    taken.insert(name.to_lowercase());
    (root, name)
  }).collect()
}

// `rel` (a path under the project's folder, with forward slashes) with a
// number before its extension: "samples/other/kick 1.wav"
fn numbered(rel: &str, n: u32) -> String {
  let (folder, file) = rel.rsplit_once('/').map_or(("", rel), |(f, n)| (f, n));
  let path = Path::new(file);
  let stem = path.file_stem().unwrap_or_default().to_string_lossy();
  let numbered = match path.extension() {
    Some(ext) => format!("{} {}.{}", stem, n, ext.to_string_lossy()),
    None => format!("{} {}", stem, n),
  };
  if folder.is_empty() { numbered } else { format!("{}/{}", folder, numbered) }
}

// the samples the project uses from outside its folder (saved as absolute
// paths), each given a place in samples/ and pointed at there. returns the
// files to copy in, and the samples that couldn't be found (left as they
// were)
fn bring_in_samples(project: &mut Project, dir: &Path, sample_paths: &[String]) -> (Vec<Entry>, Vec<String>) {
  let buckets = buckets(sample_paths);
  // where each sample went, by its saved path
  let mut placed: HashMap<String, String> = HashMap::new();
  // what's been given each place in samples/ (lowercase: macOS doesn't
  // tell kick.wav from Kick.wav)
  let mut taken: HashMap<String, PathBuf> = HashMap::new();
  let mut copies = Vec::new();
  let mut missing = BTreeSet::new();

  let mut place = |stored: &mut String| {
    if stored.starts_with(library::BUNDLED_PREFIX) || Path::new(stored.as_str()).is_relative() {
      return;
    }
    if let Some(rel) = placed.get(stored.as_str()) {
      *stored = rel.clone();
      return;
    }
    let from = PathBuf::from(stored.as_str());
    if !from.is_file() {
      missing.insert(stored.clone());
      return;
    }
    let canonical = fs::canonicalize(&from).unwrap_or_else(|_| from.clone());
    let rest = |root: &Path| canonical.strip_prefix(root).ok()
      .map(|rest| rest.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
    let wanted = buckets.iter()
      .find_map(|(root, name)| rest(root).map(|rest| format!("{}/{}/{}", project_files::SAMPLES_FOLDER, name, rest)))
      .unwrap_or_else(|| format!("{}/{}/{}", project_files::SAMPLES_FOLDER, OTHER_SAMPLES,
        canonical.file_name().unwrap_or_default().to_string_lossy()));

    // the first free place, or one that already has this very file: from a
    // share before (the project's own samples/ is zipped with it)
    let mut copy = true;
    let rel = (0..).map(|n| if n == 0 { wanted.clone() } else { numbered(&wanted, n) })
      .find(|rel| {
        if taken.contains_key(&rel.to_lowercase()) { return false; }
        let there = dir.join(rel);
        if !there.exists() { return true; }
        if there.is_file() && same_bytes(&there, &canonical) { copy = false; return true; }
        false
      })
      .unwrap();
    taken.insert(rel.to_lowercase(), canonical.clone());
    if copy {
      copies.push(Entry { from: canonical, to: rel.clone() });
    }
    placed.insert(stored.clone(), rel.clone());
    *stored = rel;
  };
  project.clips.iter_mut().for_each(|clip| place(&mut clip.path));
  project.sequencer.sample_paths_mut().for_each(|path| place(path));

  (copies, missing.into_iter().collect())
}

// the project folder's files, as they go in the zip. a project in a folder
// of its own (named after it, as Save As makes them) is zipped whole. one
// whose file is loose in some other folder takes only its own folders, not
// everything else in there. left out either way: hidden files, the project
// files there (the one written is the project as it's open), older
// versions' copies of them, and shareable files (`dest` too)
fn folder_files(dir: &Path, project_path: &Path, dest: &Path) -> Vec<Entry> {
  let own_folder = dir.file_name().is_some() && dir.file_name() == project_path.file_stem();
  let dest = fs::canonicalize(dest.parent().unwrap_or(Path::new(".")))
    .ok()
    .zip(dest.file_name())
    .map(|(parent, name)| parent.join(name));

  let mut entries = Vec::new();
  project_files::walk(dir, &mut |path, is_dir| {
    let Ok(rel) = path.strip_prefix(dir) else { return };
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let top_level_file = parts.len() == 1 && !is_dir;
    let in_own_folders = PROJECT_FOLDERS.iter().any(|folder| parts[0] == *folder);
    if !own_folder && !in_own_folders { return; }
    let file_name = parts.last().map(|s| s.to_lowercase()).unwrap_or_default();
    if top_level_file && file_name.ends_with(&format!(".{}", project_files::PROJECT_EXTENSION)) { return; }
    if file_name.ends_with(&format!(".{}", SHARE_EXTENSION)) || file_name.ends_with(&format!(".{}.tmp", SHARE_EXTENSION)) { return; }
    if dest.as_deref() == Some(path) { return; }
    if is_dir { return; }
    entries.push(Entry { from: path.to_path_buf(), to: parts.join("/") });
  });
  entries
}

// the plugins a project's effects and synth rows use, by name
fn plugins_of(project: &Project) -> Vec<String> {
  let named = |name: &str, vendor: &str| if vendor.is_empty() { name.to_string() } else { format!("{} ({})", name, vendor) };
  let effects = project.effects.iter()
    .flat_map(|rack| rack.slots.iter())
    .filter(|slot| !native::is_native(&slot.plugin_id))
    .map(|slot| named(&slot.name, &slot.vendor));
  let mut sequencer = project.sequencer.clone();
  let synths: Vec<String> = sequencer.synths_mut()
    .filter(|synth| !native::is_native(&synth.plugin_id))
    .map(|synth| named(&synth.name, &synth.vendor))
    .collect();
  effects.chain(synths).collect::<BTreeSet<_>>().into_iter().collect()
}

// how a file is stored: audio as it is (it barely shrinks, and storing is
// quick), the rest compressed
fn options_for(path: &Path, size: u64) -> SimpleFileOptions {
  let method = if project_files::is_audio(path) { CompressionMethod::Stored } else { CompressionMethod::Deflated };
  SimpleFileOptions::default()
    .compression_method(method)
    .large_file(size >= u32::MAX as u64)
}

// write the zip next to `dest` and move it into place once it's whole, so a
// failed export leaves nothing half written
fn write_zip(dest: &Path, name: &str, json: &str, entries: &[Entry], progress: Progress) -> Result<(), String> {
  let size_of = |entry: &Entry| fs::metadata(&entry.from).map(|m| m.len()).unwrap_or(0);
  let total = json.len() as u64 + entries.iter().map(size_of).sum::<u64>();
  let mut done = 0;
  progress(done, total);

  let mut tmp = dest.as_os_str().to_owned();
  tmp.push(".tmp");
  let tmp = PathBuf::from(tmp);

  let mut write = || -> Result<(), String> {
    let file = fs::File::create(&tmp).map_err(|e| format!("could not write '{}': {}", tmp.display(), e))?;
    let mut zip = ZipWriter::new(std::io::BufWriter::new(file));
    let failed = |e: zip::result::ZipError| format!("could not write '{}': {}", dest.display(), e);

    zip.start_file(format!("{}/{}.{}", name, name, project_files::PROJECT_EXTENSION),
      SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)).map_err(failed)?;
    zip.write_all(json.as_bytes()).map_err(|e| failed(e.into()))?;
    done += json.len() as u64;
    progress(done, total);

    let mut buffer = vec![0u8; 1 << 20];
    for entry in entries {
      let mut from = fs::File::open(&entry.from)
        .map_err(|e| format!("could not read '{}': {}", entry.from.display(), e))?;
      zip.start_file(format!("{}/{}", name, entry.to), options_for(&entry.from, size_of(entry))).map_err(failed)?;
      loop {
        let n = from.read(&mut buffer)
          .map_err(|e| format!("could not read '{}': {}", entry.from.display(), e))?;
        if n == 0 { break; }
        zip.write_all(&buffer[..n]).map_err(|e| failed(e.into()))?;
        done += n as u64;
        progress(done, total);
      }
    }
    let mut out = zip.finish().map_err(failed)?;
    out.flush().map_err(|e| failed(e.into()))?;
    Ok(())
  };

  write()
    .and_then(|_| fs::rename(&tmp, dest).map_err(|e| format!("could not write '{}': {}", dest.display(), e)))
    .map_err(|e| {
      let _ = fs::remove_file(&tmp);
      e
    })
}

#[cfg(test)]
mod tests;
