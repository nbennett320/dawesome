use std;
use std::error::{Error};
use serde::{Serialize};
use tauri::{
  self,
  menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
  AppHandle,
  Emitter,
  Manager,
};
use crate::recent::{self, RecentProjects};

#[derive(Clone, Serialize)]
struct EventPayload {
  data: String,
}

// initialize the application
pub fn setup(
  app: &mut tauri::App<>
) -> Result<(), Box<dyn Error>> {
  let menu = build_menu(app.handle())?;
  app.set_menu(menu)?;
  app.on_menu_event(on_menu_event);
  Ok(())
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
  match event.id().as_ref() {
    "quit" => {
      std::process::exit(0);
    }
    // the frontend asks where to open or save, then calls the engine's
    // project commands
    id @ ("new" | "open" | "save" | "save-as") => {
      app.emit("menu-project", EventPayload {
        data: id.into()
      }).unwrap();
    }
    recent::CLEAR_ID => {
      let recent = app.state::<RecentProjects>();
      if let Err(e) = recent.update(|list| list.clear()) {
        println!("could not clear recent projects: {}", e);
      }
    }
    // a recent project's row: the frontend opens it, like File › Open
    id if recent::slot_index(id).is_some() => {
      let recent = app.state::<RecentProjects>();
      if let Some(path) = recent::slot_index(id).and_then(|i| recent.path_at(i)) {
        app.emit("menu-open-recent", EventPayload {
          data: path
        }).unwrap();
      }
    }
    // Edit menu: the frontend does it, to the project or (in a text field)
    // to the text, e.g. "edit-undo" → "undo"
    id if id.starts_with("edit-") => {
      app.emit("menu-edit", EventPayload {
        data: id.trim_start_matches("edit-").into()
      }).unwrap();
    }
    // Window menu: the frontend opens the tab, e.g. "view-mixer" → "mixer"
    id if id.starts_with("view-") => {
      app.emit("menu-open-view", EventPayload {
        data: id.trim_start_matches("view-").into()
      }).unwrap();
    }
    _ => {}
  }
}

// build the containing window menus
pub fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
  let item = |id: &str, text: &str, accelerator: Option<&str>| {
    MenuItem::with_id(app, id, text, true, accelerator)
  };

  // define main menu
  let main_submenu = Submenu::with_items(app, "Dawesome", true, &[
    &item("quit", "Quit", None)?,
  ])?;

  // define file menu
  let recent = app.state::<RecentProjects>();
  let file_submenu = Submenu::with_items(app, "File", true, &[
    &item("new", "New Project", Some("CmdOrCtrl+N"))?,
    &item("open", "Open…", Some("CmdOrCtrl+O"))?,
    &recent.submenu(app)?,
    &PredefinedMenuItem::separator(app)?,
    &item("save", "Save", Some("CmdOrCtrl+S"))?,
    &item("save-as", "Save As…", Some("CmdOrCtrl+Shift+S"))?,
  ])?;

  // define edit menu. undo, redo, repeat, select all and join are done by
  // the frontend, which knows whether the keyboard is in a text field (see
  // src/events/window.ts). cut, copy and paste are the system's own, so they
  // work on text as they do anywhere; the frontend hears of them too, and
  // does them to clips when the playlist has the keyboard
  let edit_submenu = Submenu::with_items(app, "Edit", true, &[
    &item("edit-undo", "Undo", Some("CmdOrCtrl+Z"))?,
    &item("edit-redo", "Redo", Some("CmdOrCtrl+Shift+Z"))?,
    &PredefinedMenuItem::separator(app)?,
    &PredefinedMenuItem::cut(app, None)?,
    &PredefinedMenuItem::copy(app, None)?,
    &PredefinedMenuItem::paste(app, None)?,
    &item("edit-repeat", "Repeat", Some("CmdOrCtrl+R"))?,
    &item("edit-select-all", "Select All", Some("CmdOrCtrl+A"))?,
    &PredefinedMenuItem::separator(app)?,
    // (shift+J, handled in the window: as a menu shortcut it would take
    // capital Js out of text fields)
    &item("edit-join", "Join", None)?,
  ])?;

  // define window menu: opens (or selects) a tab. the frontend knows the
  // views by these names (see src/events/window.ts)
  let window_submenu = Submenu::with_items(app, "Window", true, &[
    &item("view-playlist", "Playlist", None)?,
    &item("view-sample", "Sample", None)?,
    &item("view-mixer", "Mixer", Some("CmdOrCtrl+Shift+M"))?,
    &item("view-transport", "Transport", None)?,
    &item("view-sequencer", "Sequencer", Some("CmdOrCtrl+Shift+E"))?,
  ])?;

  // define preference menu: opens tabs, like the Window menu
  let preference_submenu = Submenu::with_items(app, "Preferences", true, &[
    &item("view-audio", "Audio Settings…", None)?,
  ])?;

  Menu::with_items(app, &[
    &main_submenu,
    &file_submenu,
    &edit_submenu,
    &window_submenu,
    &preference_submenu,
  ])
}
