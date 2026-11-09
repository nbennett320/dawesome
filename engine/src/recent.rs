// recently opened or saved projects, shown under File › Open Recent
//
// the list is saved in the user's config folder so it survives restarts.
// Tauri 1.0-rc.6 can rename and disable menu items but not add or remove
// them, so the menu gets one row per recent project when the app starts.
// projects opened or saved later move to the top of those rows straight away,
// and the menu grows to fit them at the next launch
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
  tauri::api::path::config_dir().map(|dir| dir.join("dawesome").join("recent-projects.json"))
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

// the recent list, the file it's saved in, and how many rows the menu has
pub struct RecentProjects {
  file: Option<PathBuf>,
  // fixed when the menu is built
  slots: usize,
  pub list: Mutex<RecentList>,
}

impl RecentProjects {
  // read the saved list from the user's config folder
  pub fn load() -> Self {
    let file = default_file();
    let list = file.as_deref().map(RecentList::load).unwrap_or_default();

    RecentProjects {
      file,
      // at least one row, to say there are no recent projects
      slots: list.paths.len().max(1),
      list: Mutex::new(list),
    }
  }

  // the path a recent project row opens
  pub fn path_at(&self, i: usize) -> Option<String> {
    self.list.lock().unwrap().paths.get(i).cloned()
  }

  // change the list, save it, and show it in the menu
  pub fn update<R: tauri::Runtime>(
    &self,
    window: &tauri::Window<R>,
    change: impl FnOnce(&mut RecentList),
  ) -> Result<(), String> {
    let mut list = self.list.lock().unwrap();
    change(&mut list);

    if let Some(file) = &self.file {
      if let Err(e) = list.save(file) {
        println!("could not save recent projects to {}: {}", file.display(), e);
      }
    }

    self.show(window, &list)
  }

  fn show<R: tauri::Runtime>(&self, window: &tauri::Window<R>, list: &RecentList) -> Result<(), String> {
    let menu = window.menu_handle();
    let titles = list.titles();

    for i in 0..self.slots {
      let item = menu.get_item(&slot_id(i));
      let (title, enabled) = match titles.get(i) {
        Some(title) => (title.as_str(), true),
        None if i == 0 => ("No Recent Projects", false),
        None => ("", false),
      };
      item.set_title(title).map_err(|e| e.to_string())?;
      item.set_enabled(enabled).map_err(|e| e.to_string())?;
    }

    menu.get_item(CLEAR_ID).set_enabled(!list.paths.is_empty()).map_err(|e| e.to_string())
  }

  // the Open Recent submenu, with a row for each project in the list
  pub fn submenu(&self) -> tauri::Submenu {
    let list = self.list.lock().unwrap();
    let titles = list.titles();
    let mut menu = tauri::Menu::new();

    for i in 0..self.slots {
      menu = menu.add_item(match titles.get(i) {
        Some(title) => tauri::CustomMenuItem::new(slot_id(i), title),
        None => tauri::CustomMenuItem::new(slot_id(i), "No Recent Projects").disabled(),
      });
    }

    let clear = tauri::CustomMenuItem::new(CLEAR_ID.to_string(), "Clear Recent");
    menu = menu
      .add_native_item(tauri::MenuItem::Separator)
      .add_item(if list.paths.is_empty() { clear.disabled() } else { clear });

    tauri::Submenu::new("Open Recent", menu)
  }
}

// note a project that was just opened or saved
#[tauri::command]
pub fn add_recent_project(
  window: tauri::Window,
  recent: tauri::State<'_, RecentProjects>,
  path: String,
) -> Result<(), String> {
  recent.update(&window, |list| list.add(&path))
}

// forget a project, e.g. one that couldn't be opened
#[tauri::command]
pub fn remove_recent_project(
  window: tauri::Window,
  recent: tauri::State<'_, RecentProjects>,
  path: String,
) -> Result<(), String> {
  recent.update(&window, |list| list.remove(&path))
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
