// recently opened or saved projects, shown under File › Open Recent
//
// the list is saved in the user's config folder so it survives restarts,
// and the submenu is rebuilt whenever it changes
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::menu::{MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};

// the most projects listed
pub const MAX_RECENT: usize = 8;

// menu item ids
pub const CLEAR_ID: &str = "recent-clear";
const SLOT_PREFIX: &str = "recent-";

pub fn slot_id(i: usize) -> String {
  format!("{}{}", SLOT_PREFIX, i)
}

// which row a menu item id is, if it's one of the recent project rows
pub fn slot_index(id: &str) -> Option<usize> {
  id.strip_prefix(SLOT_PREFIX)?.parse().ok()
}

// where the list is saved, e.g. ~/Library/Application Support/dawesome
fn default_file() -> Option<PathBuf> {
  dirs::config_dir().map(|dir| dir.join("dawesome").join("recent-projects.json"))
}

// the list of recent project paths, newest first
#[derive(Debug, Default, PartialEq)]
pub struct RecentList {
  pub paths: Vec<String>,
}

impl RecentList {
  // read a saved list. a missing or unreadable file is an empty list
  pub fn load(file: &Path) -> Self {
    let paths: Vec<String> = fs::read_to_string(file)
      .ok()
      .and_then(|json| serde_json::from_str(&json).ok())
      .unwrap_or_default();

    let mut list = RecentList::default();
    // oldest first, so the newest ends up on top
    for path in paths.iter().rev() {
      list.add(path);
    }
    list
  }

  pub fn save(&self, file: &Path) -> Result<(), String> {
    if let Some(dir) = file.parent() {
      fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&self.paths).map_err(|e| e.to_string())?;
    fs::write(file, json).map_err(|e| e.to_string())
  }

  // put a project at the top, keeping at most MAX_RECENT
  pub fn add(&mut self, path: &str) {
    self.remove(path);
    self.paths.insert(0, path.to_string());
    self.paths.truncate(MAX_RECENT);
  }

  pub fn remove(&mut self, path: &str) {
    self.paths.retain(|p| p != path);
  }

  pub fn clear(&mut self) {
    self.paths.clear();
  }

  // what each row of the menu shows: the project's name, with its folder
  // added when two projects share a name
  pub fn titles(&self) -> Vec<String> {
    let name = |path: &str| -> String {
      let path = Path::new(path);
      path.file_stem().unwrap_or(path.as_os_str()).to_string_lossy().into_owned()
    };
    let folder = |path: &str| -> String {
      Path::new(path)
        .parent()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default()
    };

    self.paths.iter().map(|path| {
      let title = name(path);
      let shared = self.paths.iter().filter(|other| name(other) == title).count() > 1;

      if shared && !folder(path).is_empty() {
        format!("{} — {}", title, folder(path))
      } else {
        title
      }
    }).collect()
  }
}

// the recent list, the file it's saved in, and the submenu that shows it
pub struct RecentProjects {
  file: Option<PathBuf>,
  pub list: Mutex<RecentList>,
  // set when the menu is built
  submenu: Mutex<Option<Submenu<Wry>>>,
}

impl RecentProjects {
  // read the saved list from the user's config folder
  pub fn load() -> Self {
    let file = default_file();
    let list = file.as_deref().map(RecentList::load).unwrap_or_default();

    RecentProjects {
      file,
      list: Mutex::new(list),
      submenu: Mutex::new(None),
    }
  }

  // the path a recent project row opens
  pub fn path_at(&self, i: usize) -> Option<String> {
    self.list.lock().unwrap().paths.get(i).cloned()
  }

  // change the list, save it, and show it in the menu
  pub fn update(&self, change: impl FnOnce(&mut RecentList)) -> Result<(), String> {
    let mut list = self.list.lock().unwrap();
    change(&mut list);

    if let Some(file) = &self.file {
      if let Err(e) = list.save(file) {
        println!("could not save recent projects to {}: {}", file.display(), e);
      }
    }

    match self.submenu.lock().unwrap().as_ref() {
      Some(submenu) => fill(submenu, &list).map_err(|e| e.to_string()),
      None => Ok(()),
    }
  }

  // the Open Recent submenu, with a row for each project in the list
  pub fn submenu(&self, app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let submenu = Submenu::new(app, "Open Recent", true)?;
    fill(&submenu, &self.list.lock().unwrap())?;
    *self.submenu.lock().unwrap() = Some(submenu.clone());
    Ok(submenu)
  }
}

// replace the submenu's rows with the list's
fn fill(submenu: &Submenu<Wry>, list: &RecentList) -> tauri::Result<()> {
  let app = submenu.app_handle();
  while submenu.remove_at(0)?.is_some() {}

  let titles = list.titles();
  if titles.is_empty() {
    submenu.append(&MenuItem::new(app, "No Recent Projects", false, None::<&str>)?)?;
  }
  for (i, title) in titles.iter().enumerate() {
    submenu.append(&MenuItem::with_id(app, slot_id(i), title, true, None::<&str>)?)?;
  }

  submenu.append(&PredefinedMenuItem::separator(app)?)?;
  submenu.append(&MenuItem::with_id(app, CLEAR_ID, "Clear Recent", !titles.is_empty(), None::<&str>)?)
}

// note a project that was just opened or saved
#[tauri::command]
pub fn add_recent_project(
  recent: tauri::State<'_, RecentProjects>,
  path: String,
) -> Result<(), String> {
  recent.update(|list| list.add(&path))
}

// forget a project, e.g. one that couldn't be opened
#[tauri::command]
pub fn remove_recent_project(
  recent: tauri::State<'_, RecentProjects>,
  path: String,
) -> Result<(), String> {
  recent.update(|list| list.remove(&path))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_newest_first_without_repeats() {
    let mut list = RecentList::default();
    list.add("/a.dawesome");
    list.add("/b.dawesome");
    list.add("/a.dawesome");
    assert_eq!(list.paths, vec!["/a.dawesome", "/b.dawesome"]);

    for i in 0..20 {
      list.add(&format!("/{}.dawesome", i));
    }
    assert_eq!(list.paths.len(), MAX_RECENT);
    assert_eq!(list.paths[0], "/19.dawesome");

    list.remove("/19.dawesome");
    assert_eq!(list.paths[0], "/18.dawesome");
    list.clear();
    assert!(list.paths.is_empty());
  }

  #[test]
  fn test_titles_add_the_folder_when_names_repeat() {
    let mut list = RecentList::default();
    list.add("/music/old/Song.dawesome");
    list.add("/music/new/Song.dawesome");
    list.add("/music/Beat.dawesome");

    assert_eq!(list.titles(), vec!["Beat", "Song — new", "Song — old"]);
  }

  #[test]
  fn test_saved_and_loaded() {
    let dir = std::env::temp_dir().join(format!("dawesome-recent-{}", std::process::id()));
    let file = dir.join("nested").join("recent-projects.json");

    let mut list = RecentList::default();
    list.add("/a.dawesome");
    list.add("/b.dawesome");
    list.save(&file).unwrap();
    assert_eq!(RecentList::load(&file), list);

    // a broken or missing file is an empty list, not an error
    fs::write(&file, "not json").unwrap();
    assert_eq!(RecentList::load(&file), RecentList::default());
    assert_eq!(RecentList::load(&dir.join("missing.json")), RecentList::default());

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_slot_ids() {
    assert_eq!(slot_index(&slot_id(3)), Some(3));
    assert_eq!(slot_index(CLEAR_ID), None);
    assert_eq!(slot_index("save"), None);
  }
}
