// native windows and views for plugin editors: a floating window per
// popped-out plugin, and the one plugin docked inside the main window. the
// layer the effect window commands call. it takes raw window pointers and a
// Plugin, and knows nothing of Tauri or the engine
//
// everything here is for the main thread, like Plugin's own methods. an
// EditorWindow or a Dock may be kept next to its Plugin (they're Send, like
// it), but every call, and dropping them, must happen on the main thread.
// called anywhere else they do nothing
//
// floating windows (open_window):
// - `parent_window` is the main NSWindow*, only used to centre a new window
//   over it (it may be null)
// - `position` is where the window's top-left corner was, as position()
//   gave it: macOS screen points with y going up. keep it and hand it back;
//   it's moved back on screen if that screen is gone
// - `with_plugin` lets the window reach its plugin when the user does
//   something to it: it's called with a function to run on the plugin, and
//   should run it on the plugin that was opened (or not at all if it's
//   gone). it's only called from the window's own events (the user resizing
//   or closing it), never from inside a call into this module, so it may
//   lock whatever holds the plugins
// - when the user closes the window, the editor is closed (through
//   with_plugin), the window is hidden, then `on_close` runs. on_close
//   should forget the EditorWindow (keeping its position() first) by calling
//   close() or dropping it; it may do so right there
// - close() closes the editor (unless the user already did) and the window
//
// docking (dock): see Dock below for which view to pass. the page's numbers
// are CSS pixels, which are points at zoom 1, and that's assumed: a zoomed
// webview would need them scaled first. Retina screens need nothing, since
// AppKit works in points
use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
#[allow(unused_imports)]
pub use mac::{dock, open_window, Dock, EditorWindow};
#[cfg(not(target_os = "macos"))]
#[allow(unused_imports)]
pub use other::{dock, open_window, Dock, EditorWindow};

// a rectangle on the page: CSS pixels from the webview's top-left, y down
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

// runs a function on the plugin a window was opened for (see the top)
pub type WithPlugin = Box<dyn Fn(&mut dyn FnMut(&mut crate::plugin::Plugin))>;

// other systems don't have plugin windows yet
#[cfg(not(target_os = "macos"))]
mod other {
  use super::{Rect, WithPlugin};
  use crate::plugin::Plugin;
  use std::ffi::c_void;

  const UNSUPPORTED: &str = "Plugin windows aren't supported on this system yet";

  pub struct EditorWindow;
  pub struct Dock;

  pub fn open_window(
    _plugin: &mut Plugin,
    _title: &str,
    _parent_window: *mut c_void,
    _position: Option<(f64, f64)>,
    _with_plugin: WithPlugin,
    _on_close: Box<dyn Fn()>,
  ) -> Result<EditorWindow, String> {
    Err(UNSUPPORTED.into())
  }

  impl EditorWindow {
    pub fn focus(&self) {}
    pub fn position(&self) -> (f64, f64) {
      (0., 0.)
    }
    pub fn ns_window(&self) -> *mut c_void {
      std::ptr::null_mut()
    }
    pub fn close(self, _plugin: &mut Plugin) {}
  }

  pub fn dock(_plugin: &mut Plugin, _host_view: *mut c_void, _clip: Rect, _x: f64, _y: f64) -> Result<Dock, String> {
    Err(UNSUPPORTED.into())
  }

  impl Dock {
    pub fn place(&self, _clip: Rect, _x: f64, _y: f64) {}
    pub fn size(&self) -> (u32, u32) {
      (0, 0)
    }
    pub fn undock(self, _plugin: &mut Plugin) {}
  }
}
