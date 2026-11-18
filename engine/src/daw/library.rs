// where samples come from: the ones Dawesome comes with, the folders added
// in Preferences › File Paths, and the open project's own samples folder
//
// the bundled ones are bundled into the app (assets/dawesome, see resources in
// tauri.conf.json), so where they are depends on where the app is. a project
// saves one as `dawesome:TR-808/kick.wav` instead of a path into the app, and
// so still finds it when the app has moved, or on someone else's computer
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use serde::Serialize;

// what a bundled sample's saved path starts with
pub const BUNDLED_PREFIX: &str = "dawesome:";

static BUNDLED_ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

// the folder the bundled samples are in, as the app found it on starting
// (see set_bundled_root). until then (in the tests), the repo's
// assets/dawesome, which the engine runs from in a debug build
pub fn bundled_root() -> Option<&'static Path> {
  BUNDLED_ROOT.get_or_init(|| fs::canonicalize("./assets/dawesome").ok()).as_deref()
}

// where the bundled samples are: the app's resources in a release build,
// ./assets/dawesome in a debug one. called once, as the app starts
pub fn set_bundled_root(dir: &Path) {
  let _ = BUNDLED_ROOT.set(fs::canonicalize(dir).ok());
}

// a path's parts after `root`, joined with forward slashes, when it's in it
fn under(path: &Path, root: &Path) -> Option<String> {
  let rest = path.strip_prefix(root).ok()?;
  if rest.as_os_str().is_empty() { return None; }
  Some(rest.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"))
}

// where another copy of Dawesome keeps its bundled samples: in another app
// (…/Dawesome.app/Contents/Resources/samples/…), or another checkout's debug
// build (…/engine/assets/dawesome/…). what follows, with forward slashes
fn other_copys(path: &Path) -> Option<String> {
  let parts: Vec<String> = path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
  let after = |pattern: &[&str]| parts.windows(pattern.len())
    .position(|window| window.iter().zip(pattern).all(|(part, want)| part == want))
    .map(|at| parts[at + pattern.len()..].join("/"))
    .filter(|rest| !rest.is_empty());
  after(&["Contents", "Resources", "samples"])
    .filter(|_| parts.iter().any(|part| part.ends_with(".app")))
    .or_else(|| after(&["engine", "assets", "dawesome"]))
}

// the name a sample is saved by, when it's one of the bundled ones: in this
// app's, or one of another copy's that this app has too
pub fn bundled_name(path: &Path) -> Option<String> {
  let root = bundled_root()?;
  if let Some(rest) = under(path, root) {
    return Some(format!("{}{}", BUNDLED_PREFIX, rest));
  }
  let rest = other_copys(path)?;
  root.join(&rest).is_file().then(|| format!("{}{}", BUNDLED_PREFIX, rest))
}

// where a saved bundled sample's name points (None when `stored` isn't one)
pub fn bundled_path(stored: &str) -> Option<PathBuf> {
  let rest = stored.strip_prefix(BUNDLED_PREFIX)?;
  // (a name can't climb out of the samples folder)
  let rest = Path::new(rest);
  if rest.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
    return Some(PathBuf::from(stored));
  }
  Some(match bundled_root() {
    Some(root) => root.join(rest),
    None => PathBuf::from(stored),
  })
}

// a project saved before bundled samples were named has the path into the
// app it was saved from. when that's gone, the same sample in this app's
pub fn recovered(path: &Path) -> Option<PathBuf> {
  if path.is_file() { return None; }
  let found = bundled_root()?.join(other_copys(path)?);
  found.is_file().then_some(found)
}

// what a folder of samples is, in the File Paths list
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SamplePathKind {
  // the ones Dawesome comes with
  Bundled,
  // a folder added in File Paths
  User,
  // the open project's samples folder (a shared project's samples)
  Project,
}

// a folder of samples, as File Paths and the Samples browser list it
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SamplePath {
  pub name: String,
  pub path: String,
  pub kind: SamplePathKind,
  // whether it's there (an added folder can be moved or deleted)
  pub found: bool,
}

// what the bundled folder and the project's are called in the list
pub const BUNDLED_NAME: &str = "Dawesome";
pub const PROJECT_NAME: &str = "This project";

fn folder_name(path: &Path) -> String {
  path.file_name().map_or_else(|| path.to_string_lossy().into_owned(), |name| name.to_string_lossy().into_owned())
}

// every folder of samples, in order: the bundled ones (listed from
// `bundled`, which in a debug build is all of ./assets), the user's, and
// the open project's samples folder when it has one
pub fn sample_paths(bundled: Option<&Path>, user: &[String], project: Option<&Path>) -> Vec<SamplePath> {
  let mut paths = Vec::new();
  if let Some(dir) = bundled {
    let dir = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    paths.push(SamplePath {
      name: BUNDLED_NAME.into(),
      found: dir.is_dir(),
      path: dir.to_string_lossy().into_owned(),
      kind: SamplePathKind::Bundled,
    });
  }
  paths.extend(user.iter().map(|path| SamplePath {
    name: folder_name(Path::new(path)),
    path: path.clone(),
    kind: SamplePathKind::User,
    found: Path::new(path).is_dir(),
  }));
  if let Some(dir) = project.filter(|dir| dir.is_dir()) {
    paths.push(SamplePath {
      name: PROJECT_NAME.into(),
      path: fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf()).to_string_lossy().into_owned(),
      kind: SamplePathKind::Project,
      found: true,
    });
  }
  paths
}

// the user's folders with `dir` added, or why it can't be: it isn't a
// folder, or it's already listed, or inside a folder that is, or holds one
pub fn with_added(user: &[String], dir: &Path) -> Result<Vec<String>, String> {
  let dir = fs::canonicalize(dir)
    .map_err(|e| format!("could not find the folder '{}': {}", dir.display(), e))?;
  if !dir.is_dir() {
    return Err(format!("'{}' isn't a folder", dir.display()));
  }
  let listed = bundled_root().map(|root| (BUNDLED_NAME.to_string(), root.to_path_buf()))
    .into_iter()
    .chain(user.iter().map(|path| (folder_name(Path::new(path)), fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path)))));
  for (name, path) in listed {
    if dir == path {
      return Err(format!("{} is already in File Paths", name));
    }
    if dir.starts_with(&path) {
      return Err(format!("that folder is inside {}, which is already in File Paths", name));
    }
    if path.starts_with(&dir) {
      return Err(format!("that folder holds {}, which is already in File Paths. Remove it first", name));
    }
  }
  let mut user = user.to_vec();
  user.push(dir.to_string_lossy().into_owned());
  Ok(user)
}

// the user's folders without `dir`
pub fn without(user: &[String], dir: &str) -> Vec<String> {
  user.iter().filter(|path| path.as_str() != dir).cloned().collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn a_bundled_sample() -> (PathBuf, String) {
    let root = bundled_root().expect("the repo's assets/dawesome");
    let path = root.join("TR-808").join("CB.WAV");
    assert!(path.is_file(), "expected {}", path.display());
    (path, "dawesome:TR-808/CB.WAV".to_string())
  }

  #[test]
  fn test_bundled_samples_are_named() {
    let (path, name) = a_bundled_sample();
    assert_eq!(bundled_name(&path), Some(name.clone()));
    assert_eq!(bundled_path(&name), Some(path));
    // other samples aren't
    assert_eq!(bundled_name(Path::new("/Users/me/kick.wav")), None);
    assert_eq!(bundled_path("/Users/me/kick.wav"), None);
    assert_eq!(bundled_path("recordings/take.wav"), None);
  }

  #[test]
  fn test_another_apps_bundled_samples_are_this_ones() {
    let (path, name) = a_bundled_sample();
    let elsewhere = Path::new("/Applications/Gone/Dawesome.app/Contents/Resources/samples/TR-808/CB.WAV");
    assert_eq!(bundled_name(elsewhere), Some(name.clone()));
    assert_eq!(recovered(elsewhere), Some(path.clone()));
    let checkout = Path::new("/Users/someone/code/dawesome/engine/assets/dawesome/TR-808/CB.WAV");
    assert_eq!(bundled_name(checkout), Some(name));
    assert_eq!(recovered(checkout), Some(path));
    // only ones this app has too
    let gone = Path::new("/Applications/Dawesome.app/Contents/Resources/samples/TR-808/nothing.wav");
    assert_eq!((bundled_name(gone), recovered(gone)), (None, None));
    // and only in an app
    let loose = Path::new("/Users/me/Contents/Resources/samples/TR-808/CB.WAV");
    assert_eq!(bundled_name(loose), None);
  }

  // an empty folder for one test's files
  fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dawesome-library-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::canonicalize(dir).unwrap()
  }

  #[test]
  fn test_sample_paths_are_listed_in_order() {
    let dir = test_dir("list");
    let drums = dir.join("Drums");
    let project = dir.join("song").join("samples");
    fs::create_dir_all(&drums).unwrap();
    fs::create_dir_all(&project).unwrap();
    let user = vec![drums.to_string_lossy().into_owned(), dir.join("Gone").to_string_lossy().into_owned()];

    let paths = sample_paths(bundled_root(), &user, Some(&project));
    let listed: Vec<(&str, SamplePathKind, bool)> = paths.iter().map(|p| (p.name.as_str(), p.kind, p.found)).collect();
    assert_eq!(listed, vec![
      ("Dawesome", SamplePathKind::Bundled, true),
      ("Drums", SamplePathKind::User, true),
      ("Gone", SamplePathKind::User, false),
      ("This project", SamplePathKind::Project, true),
    ]);
    // a project without a samples folder has none listed
    assert_eq!(sample_paths(None, &[], Some(&dir.join("nothing"))), vec![]);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_adding_a_folder() {
    let dir = test_dir("add");
    let drums = dir.join("Drums");
    fs::create_dir_all(drums.join("Kicks")).unwrap();
    fs::write(dir.join("file.wav"), b"").unwrap();

    let user = with_added(&[], &drums).unwrap();
    assert_eq!(user, vec![drums.to_string_lossy().into_owned()]);
    // not twice, nor a folder inside it, nor one holding it
    assert!(with_added(&user, &drums).unwrap_err().contains("already"));
    assert!(with_added(&user, &drums.join("Kicks")).unwrap_err().contains("inside Drums"));
    assert!(with_added(&user, &dir).unwrap_err().contains("holds Drums"));
    // nor the bundled samples
    assert!(with_added(&[], bundled_root().unwrap()).unwrap_err().contains("Dawesome"));
    // nor a file, nor what isn't there
    assert!(with_added(&[], &dir.join("file.wav")).is_err());
    assert!(with_added(&[], &dir.join("nothing")).is_err());

    assert_eq!(without(&user, &user[0]), Vec::<String>::new());
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_name_cant_leave_the_samples_folder() {
    assert_eq!(bundled_path("dawesome:../../secret.wav"), Some(PathBuf::from("dawesome:../../secret.wav")));
  }
}
