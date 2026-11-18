// the samples Dawesome comes with, and how a project names them
//
// they're bundled into the app (assets/dawesome, see resources in
// tauri.conf.json), so where they are depends on where the app is. a project
// saves one as `dawesome:TR-808/kick.wav` instead of a path into the app, and
// so still finds it when the app has moved, or on someone else's computer
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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

  #[test]
  fn test_a_name_cant_leave_the_samples_folder() {
    assert_eq!(bundled_path("dawesome:../../secret.wav"), Some(PathBuf::from("dawesome:../../secret.wav")));
  }
}
