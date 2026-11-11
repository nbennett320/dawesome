// where each plugin's own interface is showing: in a floating window of its
// own (any number at once), or docked inside the Effects Rack tab (one at a
// time). the native side is plugin/window.rs; the contract with the frontend
// is context/vst-project/04-api.md
//
// an interface is only ever in one place: docking a plugin closes its
// window, and popping it out undocks it
//
// all of this runs on the main thread, where plugins and their windows
// live: the commands aren't async, and the windows' own events arrive there
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::daw;
use crate::effects::Effects;
use crate::plugin::window::{self, Dock, EditorWindow, Rect};

#[derive(Default)]
struct Showing {
  // by slot id
  windows: HashMap<u64, EditorWindow>,
  // where each slot's window was when it was last closed, so it opens there
  // again. kept until the app closes
  positions: HashMap<u64, (f64, f64)>,
  docked: Option<(u64, Dock)>,
}

lazy_static! {
  // never locked while calling into a plugin or a window, which may call
  // back in here (a window closing, a plugin being unloaded)
  static ref SHOWING: Mutex<Showing> = Mutex::new(Showing::default());
}

// how big a docked interface is, so the page can leave room for it
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorInfo {
  pub width: u32,
  pub height: u32,
  pub resizable: bool,
}

// close a slot's interface wherever it's showing, before its plugin goes
// (it was removed, undone away, or replaced by another project)
pub fn init(effects: &Arc<Effects>) {
  let racks = Arc::downgrade(effects);
  effects.on_unload(move |slot_id| {
    if let Some(effects) = racks.upgrade() {
      close_window(&effects, slot_id);
      if docked_slot() == Some(slot_id) { undock(&effects); }
    }
  });
}

fn docked_slot() -> Option<u64> {
  SHOWING.lock().unwrap().docked.as_ref().map(|(slot_id, _)| *slot_id)
}

// the main window and the view its page is in, as the native side wants them
#[cfg(target_os = "macos")]
fn native(window: &tauri::WebviewWindow) -> (*mut c_void, *mut c_void) {
  (
    window.ns_window().unwrap_or(std::ptr::null_mut()),
    window.ns_view().unwrap_or(std::ptr::null_mut()),
  )
}

#[cfg(not(target_os = "macos"))]
fn native(_window: &tauri::WebviewWindow) -> (*mut c_void, *mut c_void) {
  (std::ptr::null_mut(), std::ptr::null_mut())
}

const NOT_LOADED: &str = "That effect isn't loaded, so it has no interface to show";

fn open_window(effects: &Arc<Effects>, parent: *mut c_void, slot_id: u64) -> Result<(), String> {
  if docked_slot() == Some(slot_id) { undock(effects); }

  let position = {
    let showing = SHOWING.lock().unwrap();
    if let Some(open) = showing.windows.get(&slot_id) {
      open.focus();
      return Ok(());
    }
    showing.positions.get(&slot_id).copied()
  };

  let title = effects.slot_name(slot_id).ok_or(NOT_LOADED)?;
  // the window reaches its plugin through the racks, so it can't outlive it
  let racks = effects.clone();
  let with_plugin: window::WithPlugin = Box::new(move |work| {
    racks.with_plugin(slot_id, |plugin| work(plugin));
  });
  // the user closed it: remember where it was
  let on_close = Box::new(move || {
    let mut showing = SHOWING.lock().unwrap();
    if let Some(closed) = showing.windows.remove(&slot_id) {
      showing.positions.insert(slot_id, closed.position());
    }
  });

  let opened = effects
    .with_plugin(slot_id, |plugin| window::open_window(plugin, &title, parent, position, with_plugin, on_close))
    .ok_or(NOT_LOADED)??;
  SHOWING.lock().unwrap().windows.insert(slot_id, opened);
  Ok(())
}

fn close_window(effects: &Effects, slot_id: u64) {
  let open = {
    let mut showing = SHOWING.lock().unwrap();
    let open = showing.windows.remove(&slot_id);
    if let Some(open) = &open {
      showing.positions.insert(slot_id, open.position());
    }
    open
  };

  // (if the plugin is already gone, dropping the window closes it)
  if let Some(open) = open {
    effects.with_plugin(slot_id, |plugin| open.close(plugin));
  }
}

fn dock(effects: &Arc<Effects>, host_view: *mut c_void, slot_id: u64, clip: Rect, x: f64, y: f64) -> Result<EditorInfo, String> {
  close_window(effects, slot_id);
  undock(effects);

  let (docked, resizable) = effects
    .with_plugin(slot_id, |plugin| {
      window::dock(plugin, host_view, clip, x, y).map(|docked| (docked, plugin.editor_can_resize()))
    })
    .ok_or(NOT_LOADED)??;
  let (width, height) = docked.size();
  SHOWING.lock().unwrap().docked = Some((slot_id, docked));

  Ok(EditorInfo { width, height, resizable })
}

fn undock(effects: &Effects) {
  let docked = SHOWING.lock().unwrap().docked.take();
  if let Some((slot_id, docked)) = docked {
    effects.with_plugin(slot_id, |plugin| docked.undock(plugin));
  }
}

#[tauri::command]
pub fn open_effect_window(window: tauri::WebviewWindow, state: daw::TState<'_>, slot_id: u64) -> Result<(), String> {
  let (parent, _) = native(&window);
  open_window(&state.playlist.effects, parent, slot_id)
}

#[tauri::command]
pub fn close_effect_window(state: daw::TState<'_>, slot_id: u64) {
  close_window(&state.playlist.effects, slot_id)
}

#[tauri::command]
pub fn dock_effect(
  window: tauri::WebviewWindow,
  state: daw::TState<'_>,
  slot_id: u64,
  clip: Rect,
  x: f64,
  y: f64,
) -> Result<EditorInfo, String> {
  let (_, host_view) = native(&window);
  dock(&state.playlist.effects, host_view, slot_id, clip, x, y)
}

#[tauri::command]
pub fn move_docked_effect(clip: Rect, x: f64, y: f64) {
  if let Some((_, docked)) = SHOWING.lock().unwrap().docked.as_ref() {
    docked.place(clip, x, y);
  }
}

#[tauri::command]
pub fn undock_effect(state: daw::TState<'_>) {
  undock(&state.playlist.effects)
}
