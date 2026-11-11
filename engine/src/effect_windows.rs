// where each plugin's own interface is showing: in a floating window of its
// own, or docked inside the main window in its own tab's pane. any number of
// each at once. the native side is plugin/window.rs; the contract with the
// frontend is context/vst-project/04-api.md
//
// an interface is only ever in one place: docking a plugin closes its
// window, and popping it out undocks it
//
// a window's last position is kept with its effect (Effects::window_position)
// and saved with the project, so it opens there again
//
// all of this runs on the main thread, where plugins and their windows
// live: the commands aren't async, and the windows' own events arrive there
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::daw;
use crate::effects::{Effects, WindowPosition};
use crate::main_thread;
use crate::plugin::window::{self, Dock, EditorWindow, Rect};

#[derive(Default)]
struct Showing {
  // by slot id
  windows: HashMap<u64, EditorWindow>,
  docked: HashMap<u64, Dock>,
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

// the room a docked interface has (CSS pixels, which are points)
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Room {
  pub width: f64,
  pub height: f64,
}

impl Room {
  fn points(self) -> (u32, u32) {
    let points = |v: f64| if v.is_finite() { v.round().max(1.) as u32 } else { 1 };
    (points(self.width), points(self.height))
  }
}

// close a slot's interface wherever it's showing, before its plugin goes
// (it was removed, undone away, or replaced by another project)
pub fn init(effects: &Arc<Effects>) {
  let racks = Arc::downgrade(effects);
  effects.on_unload(move |slot_id| {
    if let Some(effects) = racks.upgrade() {
      close_window(&effects, slot_id);
      undock(&effects, slot_id);
    }
  });
}

// keep where every open window is now, so a project saved now has it
pub fn keep_positions(effects: &Effects) {
  let kept = main_thread::run(|| {
    let showing = SHOWING.lock().unwrap();
    for (slot_id, open) in &showing.windows {
      effects.keep_window_position(*slot_id, position_of(open));
    }
  });
  if let Err(e) = kept {
    println!("the plugin windows' places weren't kept: {}", e);
  }
}

fn position_of(open: &EditorWindow) -> WindowPosition {
  let (x, y) = open.position();
  WindowPosition { x, y }
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
const NOT_DOCKED: &str = "That effect's interface isn't docked";

fn open_window(effects: &Arc<Effects>, parent: *mut c_void, slot_id: u64) -> Result<(), String> {
  undock(effects, slot_id);

  if let Some(open) = SHOWING.lock().unwrap().windows.get(&slot_id) {
    open.focus();
    return Ok(());
  }
  // (moved back on screen by the window if that screen is gone)
  let position = effects.window_position(slot_id).map(|p| (p.x, p.y));

  let title = effects.slot_name(slot_id).ok_or(NOT_LOADED)?;
  // the window reaches its plugin through the racks, so it can't outlive it
  let racks = effects.clone();
  let with_plugin: window::WithPlugin = Box::new(move |work| {
    racks.with_plugin(slot_id, |plugin| work(plugin));
  });
  // the user closed it: remember where it was
  let racks = Arc::downgrade(effects);
  let on_close = Box::new(move || {
    let closed = SHOWING.lock().unwrap().windows.remove(&slot_id);
    if let (Some(closed), Some(effects)) = (closed, racks.upgrade()) {
      effects.keep_window_position(slot_id, position_of(&closed));
    }
  });

  let opened = effects
    .with_plugin(slot_id, |plugin| window::open_window(plugin, &title, parent, position, with_plugin, on_close))
    .ok_or(NOT_LOADED)??;
  SHOWING.lock().unwrap().windows.insert(slot_id, opened);
  Ok(())
}

fn close_window(effects: &Effects, slot_id: u64) {
  let open = SHOWING.lock().unwrap().windows.remove(&slot_id);
  // (if the plugin is already gone, dropping the window closes it)
  if let Some(open) = open {
    effects.keep_window_position(slot_id, position_of(&open));
    effects.with_plugin(slot_id, |plugin| open.close(plugin));
  }
}

// show a slot's interface in the page at (x, y), clipped to `clip`, taking
// the room `fill` gives it if it can resize. one already docked is placed
// again (its editor stays open)
fn dock(
  effects: &Arc<Effects>,
  host_view: *mut c_void,
  slot_id: u64,
  clip: Rect,
  x: f64,
  y: f64,
  fill: Option<Room>,
) -> Result<EditorInfo, String> {
  close_window(effects, slot_id);
  let fill = fill.map(Room::points);

  let docked = SHOWING.lock().unwrap().docked.remove(&slot_id);
  let placed = match docked {
    Some(docked) => effects.with_plugin(slot_id, |plugin| {
      docked.place(clip, x, y);
      if let Some((w, h)) = fill {
        docked.fill(plugin, w, h);
      }
      Ok((docked, plugin.editor_can_resize()))
    }),
    None => effects.with_plugin(slot_id, |plugin| {
      window::dock(plugin, host_view, clip, x, y, fill).map(|docked| (docked, plugin.editor_can_resize()))
    }),
  };
  let (docked, resizable) = placed.ok_or(NOT_LOADED)??;
  let (width, height) = docked.size();
  SHOWING.lock().unwrap().docked.insert(slot_id, docked);
  Ok(EditorInfo { width, height, resizable })
}

// a docked slot has (w, h) of room now
fn resize_docked(effects: &Effects, slot_id: u64, room: Room) -> Result<EditorInfo, String> {
  let docked = SHOWING.lock().unwrap().docked.remove(&slot_id).ok_or(NOT_DOCKED)?;
  let (w, h) = room.points();
  let resized = effects.with_plugin(slot_id, |plugin| (docked.fill(plugin, w, h), plugin.editor_can_resize()));
  match resized {
    Some(((width, height), resizable)) => {
      SHOWING.lock().unwrap().docked.insert(slot_id, docked);
      Ok(EditorInfo { width, height, resizable })
    }
    // its plugin's gone: so is the dock (dropping it takes its views away)
    None => Err(NOT_LOADED.into()),
  }
}

fn undock(effects: &Effects, slot_id: u64) {
  let docked = SHOWING.lock().unwrap().docked.remove(&slot_id);
  if let Some(docked) = docked {
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
  fill: Option<Room>,
) -> Result<EditorInfo, String> {
  let (_, host_view) = native(&window);
  dock(&state.playlist.effects, host_view, slot_id, clip, x, y, fill)
}

#[tauri::command]
pub fn move_docked_effect(slot_id: u64, clip: Rect, x: f64, y: f64) {
  if let Some(docked) = SHOWING.lock().unwrap().docked.get(&slot_id) {
    docked.place(clip, x, y);
  }
}

#[tauri::command]
pub fn resize_docked_effect(state: daw::TState<'_>, slot_id: u64, width: f64, height: f64) -> Result<EditorInfo, String> {
  resize_docked(&state.playlist.effects, slot_id, Room { width, height })
}

#[tauri::command]
pub fn undock_effect(state: daw::TState<'_>, slot_id: u64) {
  undock(&state.playlist.effects, slot_id)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_room_is_whole_points_and_never_nothing() {
    assert_eq!(Room { width: 640.4, height: 480.6 }.points(), (640, 481));
    assert_eq!(Room { width: 0., height: -3. }.points(), (1, 1));
    assert_eq!(Room { width: f64::NAN, height: f64::INFINITY }.points(), (1, 1));
    let room: Room = serde_json::from_value(serde_json::json!({ "width": 300, "height": 200.5 })).unwrap();
    assert_eq!(room, Room { width: 300., height: 200.5 });
  }

  #[test]
  fn test_a_slot_thats_not_docked_or_showing_is_left_alone() {
    let rig = crate::effects::testing::rig();
    let effects = rig.effects();
    undock(effects, 41);
    close_window(effects, 41);
    assert_eq!(resize_docked(effects, 41, Room { width: 10., height: 10. }), Err(NOT_DOCKED.to_string()));
    // nothing open: nothing kept, nothing lost
    keep_positions(effects);
    assert_eq!(effects.window_position(41), None);
  }
}
