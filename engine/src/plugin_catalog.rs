// the plugins installed on this machine, as the Effects browser lists them
//
// one catalog for the whole app, kept in plugins.json next to the
// preferences. at launch it's read and then refreshed in the background
// (new and changed bundles only); the browser's refresh button scans every
// bundle again. one scan at a time: the browser polls `status` while it runs
// (see plugin/scan.rs for how bundles are scanned)
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::plugin::scan::{self, Catalog, FailedBundle};
use crate::plugin::PluginInfo;

// what get_plugins returns
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalog {
  pub plugins: Vec<PluginInfo>,
  // bundles that crashed, hung or couldn't be read
  pub failed: Vec<FailedBundle>,
  // the folders that were looked in
  pub searched: Vec<String>,
  pub scanning: bool,
  // bundles scanned so far and in all, while scanning
  pub done: usize,
  pub total: usize,
}

#[derive(Default)]
pub struct Plugins {
  catalog: Mutex<Catalog>,
  // where it's saved (none: it isn't)
  file: Mutex<Option<PathBuf>>,
  scanning: AtomicBool,
  done: AtomicUsize,
  total: AtomicUsize,
}

// the catalog's file, in the app's settings folder
pub fn default_file() -> Option<PathBuf> {
  dirs::config_dir().map(|dir| dir.join("dawesome").join("plugins.json"))
}

impl Plugins {
  // read the catalog saved at `file`, and save it there after each scan
  pub fn open(&self, file: &Path) {
    *self.catalog.lock().unwrap() = Catalog::load(file);
    *self.file.lock().unwrap() = Some(file.to_path_buf());
  }

  pub fn find(&self, id: &str) -> Option<PluginInfo> {
    self.catalog.lock().unwrap().find(id).cloned()
  }

  // use this catalog (tests, which don't scan)
  #[cfg(test)]
  pub fn set(&self, catalog: Catalog) {
    *self.catalog.lock().unwrap() = catalog;
  }

  pub fn status(&self) -> PluginCatalog {
    let catalog = self.catalog.lock().unwrap();
    let scanning = self.scanning.load(Ordering::SeqCst);
    PluginCatalog {
      plugins: catalog.plugins.clone(),
      failed: catalog.failed.clone(),
      searched: scan::plugin_folders().iter().map(|f| f.to_string_lossy().into_owned()).collect(),
      scanning,
      done: if scanning { self.done.load(Ordering::SeqCst) } else { 0 },
      total: if scanning { self.total.load(Ordering::SeqCst) } else { 0 },
    }
  }

  // look for new and changed plugins (every one with `everything`) in the
  // background, each bundle in a child process of this app. false if a
  // scan is already going (it carries on)
  pub fn scan(self: &Arc<Self>, everything: bool) -> bool {
    self.scan_with(move |catalog, progress| match std::env::current_exe() {
      Ok(exe) => catalog.refresh(&exe, everything, progress),
      Err(e) => println!("plugins weren't scanned: can't find the app ({})", e),
    })
  }

  // the same, refreshing the catalog with `refresh`, which tells `progress`
  // (scanned, to scan) as it goes
  pub fn scan_with(
    self: &Arc<Self>,
    refresh: impl FnOnce(&mut Catalog, &mut dyn FnMut(usize, usize)) + Send + 'static,
  ) -> bool {
    if self.scanning.swap(true, Ordering::SeqCst) { return false; }
    self.done.store(0, Ordering::SeqCst);
    self.total.store(0, Ordering::SeqCst);

    let plugins = self.clone();
    let started = std::thread::Builder::new().name("plugin scan".into()).spawn(move || {
      // the browser keeps showing the catalog it has while this one's made
      let mut catalog = plugins.catalog.lock().unwrap().clone();
      refresh(&mut catalog, &mut |done, total| {
        plugins.done.store(done, Ordering::SeqCst);
        plugins.total.store(total, Ordering::SeqCst);
      });
      if let Some(file) = plugins.file.lock().unwrap().as_ref() {
        if let Err(e) = catalog.save(file) {
          println!("the plugin list couldn't be saved: {}", e);
        }
      }
      *plugins.catalog.lock().unwrap() = catalog;
      plugins.scanning.store(false, Ordering::SeqCst);
    });
    if let Err(e) = started {
      println!("plugins weren't scanned: {}", e);
      self.scanning.store(false, Ordering::SeqCst);
      return false;
    }
    true
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugin::PluginKind;
  use std::sync::mpsc;
  use std::time::{Duration, Instant};

  fn info(name: &str) -> PluginInfo {
    PluginInfo {
      id: format!("vst3:{:0>32}", name.len()),
      path: format!("/plugins/{}.vst3", name),
      name: name.into(),
      vendor: "Acme".into(),
      version: "1.0".into(),
      kind: PluginKind::Effect,
      category: "Dynamics".into(),
      subcategories: vec!["Fx".into()],
    }
  }

  fn wait_until_done(plugins: &Plugins) {
    let started = Instant::now();
    while plugins.status().scanning {
      assert!(started.elapsed() < Duration::from_secs(10), "the scan never finished");
      std::thread::sleep(Duration::from_millis(1));
    }
  }

  #[test]
  fn test_a_scan_reports_its_progress_and_then_its_plugins() {
    let plugins = Arc::new(Plugins::default());
    let (go_on, wait) = mpsc::channel::<()>();
    let (at_half, halfway) = mpsc::channel::<()>();
    assert!(plugins.scan_with(move |catalog, progress| {
      progress(0, 2);
      progress(1, 2);
      at_half.send(()).unwrap();
      wait.recv().unwrap();
      catalog.plugins.push(info("Comp"));
      progress(2, 2);
    }));

    halfway.recv().unwrap();
    let status = plugins.status();
    assert!(status.scanning);
    assert_eq!((status.done, status.total), (1, 2));
    assert!(status.plugins.is_empty());
    // one at a time
    assert!(!plugins.scan_with(|_, _| panic!("a second scan started")));

    go_on.send(()).unwrap();
    wait_until_done(&plugins);
    let status = plugins.status();
    assert!(!status.scanning);
    assert_eq!((status.done, status.total), (0, 0));
    assert_eq!(status.plugins, vec![info("Comp")]);
    assert_eq!(plugins.find(&info("Comp").id), Some(info("Comp")));
    assert_eq!(status.searched, scan::plugin_folders().iter().map(|f| f.to_string_lossy().into_owned()).collect::<Vec<_>>());
  }

  #[test]
  fn test_a_scan_is_saved_and_read_back() {
    let dir = std::env::temp_dir().join(format!("dawesome-catalog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("plugins.json");

    let plugins = Arc::new(Plugins::default());
    plugins.open(&file);
    assert!(plugins.scan_with(|catalog, _| catalog.plugins.push(info("Verb"))));
    wait_until_done(&plugins);

    let again = Plugins::default();
    again.open(&file);
    assert_eq!(again.status().plugins, vec![info("Verb")]);
    // and a scan can start again
    assert!(plugins.scan_with(|_, _| {}));
    wait_until_done(&plugins);

    let _ = std::fs::remove_dir_all(&dir);
  }
}
