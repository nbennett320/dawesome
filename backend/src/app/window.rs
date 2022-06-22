use std;
use serde::{Serialize};
use tauri::{
  self, 
  Manager
};

#[derive(Clone, Serialize)]
struct EventPayload {
  data: String,
}

// initialize the application
pub fn setup(
  app: &mut tauri::App<>
) -> Result<(), Box<dyn std::error::Error>> {
  let window = app.get_window("main").unwrap();
  let window_cp = window.clone();

  window.on_menu_event(move |event| {
    match event.menu_item_id() {
      "quit" => {
        std::process::exit(0);
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
pub fn build_menu() -> tauri::Menu {
  // define main menu
  let quit_menu = tauri::CustomMenuItem::new("quit".to_string(), "Quit");
  let main_submenu = tauri::Submenu::new("Dawesome".to_string(), tauri::Menu::new()
    .add_item(quit_menu));

  // define file menu
  let save_menu = tauri::CustomMenuItem::new("save".to_string(), "Save");
  let load_menu = tauri::CustomMenuItem::new("load".to_string(), "Load");
  let file_submenu = tauri::Submenu::new("File", tauri::Menu::new()
    .add_item(save_menu)
    .add_item(load_menu));
  
  // define preference menu
  let device_menu = tauri::CustomMenuItem::new("device".to_string(), "Devices");
  let preference_submenu = tauri::Submenu::new("Preferences", tauri::Menu::new()
    .add_item(device_menu));
  
  tauri::Menu::new()
    .add_submenu(main_submenu)
    .add_submenu(file_submenu)
    .add_submenu(preference_submenu)
}
