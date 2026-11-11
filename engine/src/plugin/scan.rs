// finding the plugins installed on this machine
//
// bundles are found in the usual VST3 folders, and each one is scanned in a
// child process (`app --scan-plugin <path>`, see scanner_main): a plugin that
// crashes or hangs while it's being read takes only that process down. what
// was found is kept in a catalog (a JSON file), so later refreshes only scan
// bundles that are new or have changed, and skip the ones that failed until
// they change
use super::vst3;
use super::PluginInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, UNIX_EPOCH};

// how long a bundle gets to be scanned
pub const SCAN_TIMEOUT: Duration = Duration::from_secs(15);

// the folders VST3 plugins are installed in, on this system
pub fn plugin_folders() -> Vec<PathBuf> {
  let mut folders = Vec::new();
  let home = dirs::home_dir();
  if cfg!(target_os = "macos") {
    folders.push(PathBuf::from("/Library/Audio/Plug-Ins/VST3"));
    if let Some(home) = home {
      folders.push(home.join("Library/Audio/Plug-Ins/VST3"));
    }
  } else if cfg!(target_os = "windows") {
    let common = std::env::var_os("COMMONPROGRAMFILES").map(PathBuf::from);
    folders.push(common.unwrap_or_else(|| PathBuf::from("C:\\Program Files\\Common Files")).join("VST3"));
  } else {
    if let Some(home) = home {
      folders.push(home.join(".vst3"));
    }
    folders.push(PathBuf::from("/usr/lib/vst3"));
    folders.push(PathBuf::from("/usr/local/lib/vst3"));
  }
  folders
}

// every bundle in the plugin folders
pub fn find_bundles() -> Vec<PathBuf> {
  let mut bundles = Vec::new();
  for folder in plugin_folders() {
    find_in(&folder, 0, &mut bundles);
  }
  bundles
}

fn is_bundle(path: &Path) -> bool {
  path.extension().map_or(false, |e| e.eq_ignore_ascii_case("vst3"))
}

// a .vst3 folder is a bundle, so it isn't looked inside. (a .vst3 file is a
// single-file plugin, on Windows)
pub fn find_in(folder: &Path, depth: usize, bundles: &mut Vec<PathBuf>) {
  // deep enough for vendor folders, and a stop for symlink loops
  if depth > 8 {
    return;
  }
  let Ok(entries) = std::fs::read_dir(folder) else { return };
  let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
  entries.sort();
  for path in entries {
    if is_bundle(&path) {
      bundles.push(path);
    } else if path.is_dir() {
      find_in(&path, depth + 1, bundles);
    }
  }
}

// when a bundle last changed, in milliseconds since 1970: the newer of the
// bundle's own date and its binary's (installers don't always touch the
// folder)
pub fn modified(bundle: &Path) -> u64 {
  let time = |p: &Path| {
    std::fs::metadata(p)
      .and_then(|m| m.modified())
      .ok()
      .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
      .map_or(0, |d| d.as_millis() as u64)
  };
  time(bundle).max(time(&vst3::bundle_binary(bundle)))
}

// the plugins in a bundle, read in this process
pub fn scan_bundle(path: &Path) -> Result<Vec<PluginInfo>, String> {
  vst3::scan_bundle(path)
}

// what a scanning child prints: one line of JSON
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ScanReply {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  plugins: Option<Vec<PluginInfo>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  error: Option<String>,
}

// the child's side: with the arguments `--scan-plugin <path>`, scan that
// bundle, print the result and give the exit code. called first thing in
// main, before anything else starts
pub fn scanner_main(args: &[String]) -> Option<i32> {
  if args.len() != 2 || args[0] != "--scan-plugin" {
    return None;
  }
  let (reply, code) = match scan_bundle(Path::new(&args[1])) {
    Ok(plugins) => (ScanReply { plugins: Some(plugins), error: None }, 0),
    Err(e) => (ScanReply { plugins: None, error: Some(e) }, 1),
  };
  use std::io::Write;
  let mut out = std::io::stdout().lock();
  let _ = writeln!(out, "{}", serde_json::to_string(&reply).unwrap_or_default());
  let _ = out.flush();
  Some(code)
}

// the parent's side: run `exe --scan-plugin <path>` and read what it says.
// a crash, a hang (past `timeout`) or output that isn't a reply is an error
// saying which
pub fn scan_in_child(exe: &Path, bundle: &Path, timeout: Duration) -> Result<Vec<PluginInfo>, String> {
  let mut child = Command::new(exe)
    .arg("--scan-plugin")
    .arg(bundle)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .map_err(|e| format!("couldn't start the scanner: {}", e))?;
  // read the output as it comes, so a chatty plugin can't fill the pipe and
  // stall the child
  let mut stdout = child.stdout.take().unwrap();
  let reader = std::thread::spawn(move || {
    let mut out = Vec::new();
    let _ = stdout.read_to_end(&mut out);
    out
  });

  let started = Instant::now();
  let status = loop {
    match child.try_wait() {
      Ok(Some(status)) => break Some(status),
      Ok(None) if started.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
      Ok(None) => {
        let _ = child.kill();
        let _ = child.wait();
        break None;
      }
      Err(e) => return Err(format!("lost the scanner: {}", e)),
    }
  };
  let out = reader.join().unwrap_or_default();
  let Some(status) = status else {
    return Err(format!("didn't finish scanning in {} seconds", timeout.as_secs()));
  };

  // plugins sometimes print things of their own, so the reply is the last
  // line that reads as one
  let out = String::from_utf8_lossy(&out);
  let reply = out.lines().rev().find_map(|line| serde_json::from_str::<ScanReply>(line.trim()).ok());
  match reply {
    Some(ScanReply { plugins: Some(plugins), .. }) if status.success() => Ok(plugins),
    Some(ScanReply { error: Some(e), .. }) => Err(e),
    _ if !status.success() => Err(match status.code() {
      Some(code) => format!("the scanner failed (exit code {})", code),
      None => "crashed while being scanned".to_string(),
    }),
    _ => Err("the scanner gave no result".to_string()),
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FailedBundle {
  pub path: String,
  pub reason: String,
}

// what was found, saved between runs
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Catalog {
  pub plugins: Vec<PluginInfo>,
  // every bundle that's been scanned (or failed), and when it had last
  // changed then
  pub bundles: BTreeMap<String, u64>,
  pub failed: Vec<FailedBundle>,
}

impl Catalog {
  // a missing or unreadable file is an empty catalog
  pub fn load(path: &Path) -> Catalog {
    std::fs::read(path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
  }

  pub fn save(&self, path: &Path) -> Result<(), String> {
    if let Some(folder) = path.parent() {
      std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
  }

  // look again: forget bundles that are gone, and scan the ones that are new
  // or have changed (every one with `everything`), each in a child process
  // started from `exe`. `progress` hears (scanned, to scan)
  pub fn refresh(&mut self, exe: &Path, everything: bool, progress: &mut dyn FnMut(usize, usize)) {
    let found = find_bundles().into_iter().map(|b| {
      let modified = modified(&b);
      (b, modified)
    });
    let found: Vec<(PathBuf, u64)> = found.collect();
    self.refresh_with(&found, everything, &mut |bundle| scan_in_child(exe, bundle, SCAN_TIMEOUT), progress);
  }

  // refresh, given the bundles there are (with when they changed) and how
  // to scan one
  pub fn refresh_with(
    &mut self,
    found: &[(PathBuf, u64)],
    everything: bool,
    scan: &mut dyn FnMut(&Path) -> Result<Vec<PluginInfo>, String>,
    progress: &mut dyn FnMut(usize, usize),
  ) {
    let key = |p: &Path| p.to_string_lossy().into_owned();
    let present: BTreeMap<String, u64> = found.iter().map(|(p, m)| (key(p), *m)).collect();
    self.plugins.retain(|p| present.contains_key(&p.path));
    self.failed.retain(|f| present.contains_key(&f.path));
    self.bundles.retain(|p, _| present.contains_key(p));

    let to_scan: Vec<&(PathBuf, u64)> =
      found.iter().filter(|(p, m)| everything || self.bundles.get(&key(p)) != Some(m)).collect();
    let total = to_scan.len();
    progress(0, total);
    for (i, (bundle, modified)) in to_scan.into_iter().enumerate() {
      let path = key(bundle);
      self.plugins.retain(|p| p.path != path);
      self.failed.retain(|f| f.path != path);
      match scan(bundle) {
        Ok(mut plugins) => {
          // the catalog's path is the one it was found at
          for plugin in &mut plugins {
            plugin.path = path.clone();
          }
          self.plugins.extend(plugins);
        }
        Err(reason) => self.failed.push(FailedBundle { path: path.clone(), reason }),
      }
      self.bundles.insert(path, *modified);
      progress(i + 1, total);
    }
    self.plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.path.cmp(&b.path)));
  }

  pub fn find(&self, id: &str) -> Option<&PluginInfo> {
    self.plugins.iter().find(|p| p.id == id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugin::PluginKind;

  fn info(name: &str, path: &str) -> PluginInfo {
    PluginInfo {
      id: format!("vst3:{:0>32}", name.len()),
      path: path.to_string(),
      name: name.to_string(),
      vendor: "Acme".into(),
      version: "1.0".into(),
      kind: PluginKind::Effect,
      category: "Other".into(),
      subcategories: vec!["Fx".into()],
    }
  }

  fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dawesome-scan-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
  }

  // a scanner that knows two good bundles and one that crashes, and counts
  // what it was asked to scan
  fn fake(scanned: &mut Vec<String>) -> impl FnMut(&Path) -> Result<Vec<PluginInfo>, String> + '_ {
    move |bundle: &Path| {
      let path = bundle.to_string_lossy().into_owned();
      scanned.push(path.clone());
      match bundle.file_name().unwrap().to_str().unwrap() {
        "Crash.vst3" => Err("crashed while being scanned".into()),
        "Two.vst3" => Ok(vec![info("Zeta", "elsewhere"), info("alpha", "elsewhere")]),
        _ => Ok(vec![info("Comp", "elsewhere")]),
      }
    }
  }

  #[test]
  fn test_refresh_scans_new_bundles_and_remembers_failures() {
    let mut catalog = Catalog::default();
    let found = vec![
      (PathBuf::from("/vst/One.vst3"), 1),
      (PathBuf::from("/vst/Two.vst3"), 1),
      (PathBuf::from("/vst/Crash.vst3"), 1),
    ];
    let mut scanned = vec![];
    let mut calls = vec![];
    catalog.refresh_with(&found, false, &mut fake(&mut scanned), &mut |done, total| calls.push((done, total)));
    assert_eq!(scanned.len(), 3);
    assert_eq!(calls, vec![(0, 3), (1, 3), (2, 3), (3, 3)]);
    let names: Vec<&str> = catalog.plugins.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "Comp", "Zeta"]);
    // plugins carry the path they were found at
    assert!(catalog.plugins.iter().all(|p| p.path.starts_with("/vst/")));
    assert_eq!(catalog.failed, vec![FailedBundle { path: "/vst/Crash.vst3".into(), reason: "crashed while being scanned".into() }]);
    assert_eq!(catalog.bundles.len(), 3);
  }

  #[test]
  fn test_refresh_skips_unchanged_and_failed_bundles() {
    let mut catalog = Catalog::default();
    let found = vec![(PathBuf::from("/vst/One.vst3"), 1), (PathBuf::from("/vst/Crash.vst3"), 1)];
    catalog.refresh_with(&found, false, &mut fake(&mut vec![]), &mut |_, _| {});

    let mut scanned = vec![];
    catalog.refresh_with(&found, false, &mut fake(&mut scanned), &mut |_, _| {});
    assert!(scanned.is_empty());
    assert_eq!(catalog.plugins.len(), 1);
    assert_eq!(catalog.failed.len(), 1);

    // a changed bundle is scanned again, failed or not
    let changed = vec![(PathBuf::from("/vst/One.vst3"), 1), (PathBuf::from("/vst/Crash.vst3"), 2)];
    catalog.refresh_with(&changed, false, &mut fake(&mut scanned), &mut |_, _| {});
    assert_eq!(scanned, vec!["/vst/Crash.vst3"]);
    assert_eq!(catalog.failed.len(), 1);
    assert_eq!(catalog.bundles["/vst/Crash.vst3"], 2);
  }

  #[test]
  fn test_refresh_everything_scans_all_again() {
    let mut catalog = Catalog::default();
    let found = vec![(PathBuf::from("/vst/One.vst3"), 1), (PathBuf::from("/vst/Crash.vst3"), 1)];
    catalog.refresh_with(&found, false, &mut fake(&mut vec![]), &mut |_, _| {});
    let mut scanned = vec![];
    catalog.refresh_with(&found, true, &mut fake(&mut scanned), &mut |_, _| {});
    assert_eq!(scanned.len(), 2);
    // no duplicates from scanning twice
    assert_eq!(catalog.plugins.len(), 1);
    assert_eq!(catalog.failed.len(), 1);
  }

  #[test]
  fn test_refresh_forgets_bundles_that_are_gone() {
    let mut catalog = Catalog::default();
    let found = vec![
      (PathBuf::from("/vst/One.vst3"), 1),
      (PathBuf::from("/vst/Two.vst3"), 1),
      (PathBuf::from("/vst/Crash.vst3"), 1),
    ];
    catalog.refresh_with(&found, false, &mut fake(&mut vec![]), &mut |_, _| {});
    let mut scanned = vec![];
    catalog.refresh_with(&found[..1], false, &mut fake(&mut scanned), &mut |_, _| {});
    assert!(scanned.is_empty());
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.failed.is_empty());
    assert_eq!(catalog.bundles.keys().collect::<Vec<_>>(), vec!["/vst/One.vst3"]);
  }

  #[test]
  fn test_catalog_saves_and_loads() {
    let dir = temp("catalog");
    let file = dir.join("deeper/plugins.json");
    let mut catalog = Catalog::default();
    catalog.plugins.push(info("Comp", "/vst/One.vst3"));
    catalog.bundles.insert("/vst/One.vst3".into(), 42);
    catalog.failed.push(FailedBundle { path: "/vst/Bad.vst3".into(), reason: "hung".into() });
    catalog.save(&file).unwrap();
    assert_eq!(Catalog::load(&file), catalog);
    assert_eq!(catalog.find(&catalog.plugins[0].id.clone()).unwrap().name, "Comp");
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_missing_or_broken_catalog_is_empty() {
    let dir = temp("broken");
    assert_eq!(Catalog::load(&dir.join("nothing.json")), Catalog::default());
    std::fs::write(dir.join("broken.json"), b"{ not json").unwrap();
    assert_eq!(Catalog::load(&dir.join("broken.json")), Catalog::default());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_bundles_are_found_without_looking_inside_them() {
    let dir = temp("find");
    std::fs::create_dir_all(dir.join("Vendor/Deep/Reverb.vst3/Contents/Inner.vst3")).unwrap();
    std::fs::create_dir_all(dir.join("Comp.VST3/Contents")).unwrap();
    std::fs::write(dir.join("Single.vst3"), b"").unwrap();
    std::fs::write(dir.join("readme.txt"), b"").unwrap();
    let mut found = vec![];
    find_in(&dir, 0, &mut found);
    let names: Vec<String> = found.iter().map(|p| p.strip_prefix(&dir).unwrap().to_string_lossy().into_owned()).collect();
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(names, vec!["Comp.VST3".to_string(), "Single.vst3".to_string(), format!("Vendor{0}Deep{0}Reverb.vst3", sep)]);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_scanner_main_only_answers_its_own_arguments() {
    assert_eq!(scanner_main(&[]), None);
    assert_eq!(scanner_main(&["--debug".into()]), None);
    assert_eq!(scanner_main(&["--scan-plugin".into(), "/nowhere/Missing.vst3".into()]), Some(1));
  }

  #[test]
  fn test_scan_reply_reads_back() {
    let reply = ScanReply { plugins: Some(vec![info("Comp", "/vst/One.vst3")]), error: None };
    let line = serde_json::to_string(&reply).unwrap();
    assert!(!line.contains('\n'));
    assert_eq!(serde_json::from_str::<ScanReply>(&line).unwrap(), reply);
    assert_eq!(
      serde_json::from_str::<ScanReply>(r#"{"error":"nope"}"#).unwrap(),
      ScanReply { plugins: None, error: Some("nope".into()) }
    );
  }

  #[test]
  fn test_plugin_folders_are_absolute() {
    let folders = plugin_folders();
    assert!(!folders.is_empty());
    assert!(folders.iter().all(|f| f.is_absolute() || cfg!(target_os = "windows")));
  }
}
