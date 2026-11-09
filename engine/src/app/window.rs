use std;
use std::error::{Error};
use serde::{Serialize};
use tauri::{
  self, 
  Manager
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
  let window = app.get_window("main").unwrap();
  let window_cp = window.clone();

  window.on_menu_event(move |event| {
    match event.menu_item_id() {
      "quit" => {
        std::process::exit(0);
      }
      // the frontend asks where to open or save, then calls the engine's
      // project commands
      "new" | "open" | "save" | "save-as" => {
        window_cp.app_handle().emit_all("menu-project", EventPayload {
          data: event.menu_item_id().into()
        }).unwrap();
      }
      recent::CLEAR_ID => {
        let recent = window_cp.state::<RecentProjects>();
        if let Err(e) = recent.update(&window_cp, |list| list.clear()) {
          println!("could not clear recent projects: {}", e);
        }
      }
      // a recent project's row: the frontend opens it, like File › Open
      id if recent::slot_index(id).is_some() => {
        let recent = window_cp.state::<RecentProjects>();
        if let Some(path) = recent::slot_index(id).and_then(|i| recent.path_at(i)) {
          window_cp.app_handle().emit_all("menu-open-recent", EventPayload {
            data: path
          }).unwrap();
        }
      }
      // Window menu: the frontend opens the tab, e.g. "view-mixer" → "mixer"
      id if id.starts_with("view-") => {
        window_cp.app_handle().emit_all("menu-open-view", EventPayload {
          data: id.trim_start_matches("view-").into()
        }).unwrap();
      }
      "device" => {
        println!("emit device preference");
        window_cp.app_handle().emit_all("menu-device-preference", EventPayload {
          data: "menu-device-preference".into()
        }).unwrap();
      }
      _ => {}
    }
  });
  Ok(())
}

// build the containing window menus
pub fn build_menu(recent: &RecentProjects) -> tauri::Menu {
  // define main menu
  let quit_menu = tauri::CustomMenuItem::new("quit".to_string(), "Quit");
  let main_submenu = tauri::Submenu::new("Dawesome".to_string(), tauri::Menu::new()
    .add_item(quit_menu));

  // define file menu
  let new_menu = tauri::CustomMenuItem::new("new".to_string(), "New Project")
    .accelerator("CmdOrCtrl+N");
  let open_menu = tauri::CustomMenuItem::new("open".to_string(), "Open…")
    .accelerator("CmdOrCtrl+O");
  let save_menu = tauri::CustomMenuItem::new("save".to_string(), "Save")
    .accelerator("CmdOrCtrl+S");
  let save_as_menu = tauri::CustomMenuItem::new("save-as".to_string(), "Save As…")
    .accelerator("CmdOrCtrl+Shift+S");
  let file_submenu = tauri::Submenu::new("File", tauri::Menu::new()
    .add_item(new_menu)
    .add_item(open_menu)
    .add_submenu(recent.submenu())
    .add_native_item(tauri::MenuItem::Separator)
    .add_item(save_menu)
    .add_item(save_as_menu));
  
  // define window menu: opens (or selects) a tab. the frontend knows the
  // views by these names (see src/events/window.ts)
  let window_submenu = tauri::Submenu::new("Window", tauri::Menu::new()
    .add_item(tauri::CustomMenuItem::new("view-playlist".to_string(), "Playlist"))
    .add_item(tauri::CustomMenuItem::new("view-sample".to_string(), "Sample"))
    .add_item(tauri::CustomMenuItem::new("view-mixer".to_string(), "Mixer")
      .accelerator("CmdOrCtrl+Shift+M"))
    .add_item(tauri::CustomMenuItem::new("view-transport".to_string(), "Transport")));

  // define preference menu
  let device_menu = tauri::CustomMenuItem::new("device".to_string(), "Devices");
  let preference_submenu = tauri::Submenu::new("Preferences", tauri::Menu::new()
    .add_item(device_menu));
  
  tauri::Menu::new()
    .add_submenu(main_submenu)
    .add_submenu(file_submenu)
    .add_submenu(window_submenu)
    .add_submenu(preference_submenu)
}
