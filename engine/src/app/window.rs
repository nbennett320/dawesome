use std;
use std::error::{Error};
use serde::{Serialize};
use tauri::{
  self,
  menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
  window::Color,
  AppHandle,
  Emitter,
  Manager,
  Theme,
  WebviewWindow,
  WebviewWindowBuilder,
};
use crate::preferences::{AppearanceSetting, AppPreferences, Preferences};
use crate::recent::{self, RecentProjects};
use crate::theme::{self, Appearance, ResolvedTheme, Rgba};

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
  create_main_window(app.handle())?;
  Ok(())
}

// the script that hands the frontend its settings before any page script
// runs, so the first paint is already themed
pub fn init_script(prefs: &Preferences, themes: &[ResolvedTheme]) -> String {
  let data = serde_json::json!({ "preferences": prefs, "themes": themes });
  format!("window.__DAWESOME_APPEARANCE__ = {};", data)
}

// the theme key behind everything: the page backdrop (src/global.scss) and the
// native window around it, which shows before the first paint
pub const BACKDROP: &str = "panel";

// the window's background for an appearance: the backdrop of the theme the
// preferences name for it, or of the default theme if that isn't found
pub fn backdrop_for(prefs: &Preferences, themes: &[ResolvedTheme], appearance: Appearance) -> Rgba {
  let (id, default) = match appearance {
    Appearance::Light => (&prefs.light_theme, theme::DEFAULT_LIGHT),
    Appearance::Dark => (&prefs.dark_theme, theme::DEFAULT_DARK),
  };
  let backdrop = |id: &str| {
    themes.iter()
      .find(|t| t.id == id && t.appearance == appearance)
      .and_then(|t| t.colors.get(BACKDROP))
      .and_then(|c| Rgba::parse(c))
  };
  // last resort, so a broken theme set still gives a plain window
  backdrop(id).or_else(|| backdrop(default)).unwrap_or(match appearance {
    Appearance::Light => Rgba { r: 255, g: 255, b: 255, a: 255 },
    Appearance::Dark => Rgba { r: 24, g: 24, b: 24, a: 255 },
  })
}

// none follows the system
pub fn pinned_theme(setting: AppearanceSetting) -> Option<Theme> {
  match setting {
    AppearanceSetting::System => None,
    AppearanceSetting::Light => Some(Theme::Light),
    AppearanceSetting::Dark => Some(Theme::Dark),
  }
}

pub fn to_color(rgba: Rgba) -> Color {
  Color(rgba.r, rgba.g, rgba.b, rgba.a)
}

// a color from the frontend: #rgb, #rrggbb or #rrggbbaa
pub fn parse_color(text: &str) -> Result<Color, String> {
  Rgba::parse(text).map(to_color).ok_or(format!("{:?} is not a color", text))
}

// built here rather than from tauri.conf.json alone so it can start hidden,
// with the user's appearance, and be shown once painted in the right color
fn create_main_window(app: &AppHandle) -> Result<(), Box<dyn Error>> {
  let config = app.config().app.windows.iter()
    .find(|w| w.label == "main")
    .ok_or("no main window in tauri.conf.json")?
    .clone();
  let state = app.state::<AppPreferences>();
  let prefs = state.current();
  let script = init_script(&prefs, &state.themes);

  let window = WebviewWindowBuilder::from_config(app, &config)?
    .visible(false)
    .theme(pinned_theme(prefs.appearance))
    .initialization_script(script)
    .build()?;

  // whatever goes wrong with the color, the window must not stay hidden
  if let Err(e) = paint(&window, &prefs, &state.themes) {
    println!("could not set the window background: {}", e);
  }
  window.show()?;
  Ok(())
}

fn paint(window: &WebviewWindow, prefs: &Preferences, themes: &[ResolvedTheme]) -> tauri::Result<()> {
  let appearance = match window.theme()? {
    Theme::Dark => Appearance::Dark,
    _ => Appearance::Light,
  };
  window.set_background_color(Some(to_color(backdrop_for(prefs, themes, appearance))))
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
    &item("view-preferences", "Preferences…", Some("CmdOrCtrl+,"))?,
    &PredefinedMenuItem::separator(app)?,
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
    &PredefinedMenuItem::separator(app)?,
    // opens the Mixdown tab, as the Window menu opens tabs
    &item("view-mixdown", "Mixdown…", None)?,
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
    &item("view-sequencer", "Drum Sequencer", Some("CmdOrCtrl+Shift+E"))?,
    &item("view-note-sequencer", "Note Sequencer", None)?,
    &item("view-piano-roll", "Piano Roll", None)?,
    &item("view-effects-rack", "Effects Rack", None)?,
  ])?;

  // define preference menu: opens tabs, like the Window menu
  let preference_submenu = Submenu::with_items(app, "Preferences", true, &[
    &item("view-appearance", "Appearance…", None)?,
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

#[cfg(test)]
mod tests {
  use super::*;

  fn prefs() -> Preferences {
    Preferences::default()
  }

  fn backdrop_of(id: &str) -> Rgba {
    let themes = theme::stock();
    let t = themes.iter().find(|t| t.id == id).unwrap();
    Rgba::parse(&t.colors[BACKDROP]).unwrap()
  }

  #[test]
  fn test_init_script_shape() {
    let themes = theme::stock();
    let script = init_script(&prefs(), &themes);
    let prefix = "window.__DAWESOME_APPEARANCE__ = ";
    assert!(script.starts_with(prefix));
    let json = script[prefix.len()..].strip_suffix(';').unwrap();
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(v["preferences"]["lightTheme"], theme::DEFAULT_LIGHT);
    assert_eq!(v["preferences"]["darkTheme"], theme::DEFAULT_DARK);
    assert_eq!(v["preferences"]["appearance"], "system");
    let list = v["themes"].as_array().unwrap();
    assert_eq!(list.len(), 4);
    for t in list {
      assert_eq!(t["colors"].as_object().unwrap().len(), 119);
    }
  }

  #[test]
  fn test_backdrop_for_picks_by_appearance() {
    let themes = theme::stock();
    let p = Preferences {
      light_theme: "charcoal-light".to_string(),
      dark_theme: "slate-dark".to_string(),
      ..prefs()
    };
    assert_eq!(backdrop_for(&p, &themes, Appearance::Light), backdrop_of("charcoal-light"));
    assert_eq!(backdrop_for(&p, &themes, Appearance::Dark), backdrop_of("slate-dark"));
  }

  #[test]
  fn test_backdrop_for_falls_back_to_defaults() {
    let themes = theme::stock();
    let p = Preferences {
      light_theme: "nope".to_string(),
      // exists, but is the wrong appearance
      dark_theme: "slate-light".to_string(),
      ..prefs()
    };
    assert_eq!(backdrop_for(&p, &themes, Appearance::Light), backdrop_of(theme::DEFAULT_LIGHT));
    assert_eq!(backdrop_for(&p, &themes, Appearance::Dark), backdrop_of(theme::DEFAULT_DARK));
    // no themes at all is still a color
    backdrop_for(&p, &[], Appearance::Dark);
  }

  #[test]
  fn test_parse_color() {
    assert_eq!(parse_color("#fff").unwrap().0, 255);
    let c = parse_color("#10203040").unwrap();
    assert_eq!((c.0, c.1, c.2, c.3), (0x10, 0x20, 0x30, 0x40));
    assert!(parse_color("red").is_err());
    assert!(parse_color("#12345").is_err());
    assert!(parse_color("").is_err());
  }

  #[test]
  fn test_pinned_theme() {
    assert_eq!(pinned_theme(AppearanceSetting::System), None);
    assert_eq!(pinned_theme(AppearanceSetting::Dark), Some(Theme::Dark));
  }
}
