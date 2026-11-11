// window.rs on macOS: AppKit panels and views, through objc2
use super::{Rect, WithPlugin};
use crate::plugin::Plugin;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
use objc2_app_kit::{
  NSAutoresizingMaskOptions, NSBackingStoreType, NSPanel, NSScreen, NSView, NSWindow, NSWindowCollectionBehavior,
  NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::Cell;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

define_class!(
  // safe: NSView has no subclassing requirements, and this adds no Drop
  #[unsafe(super(NSView))]
  #[thread_kind = MainThreadOnly]
  #[name = "DawesomePluginEditorView"]
  #[ivars = EditorViewIvars]
  struct EditorView;

  impl EditorView {
    // y goes down, like the page's, and the plugin's view sits at the top
    // left whatever size it takes
    #[unsafe(method(isFlipped))]
    fn is_flipped(&self) -> bool {
      true
    }

    // a plugin's view belongs at the top left. some (JUCE's) place it
    // themselves while resizing, from this view's height before it's
    // changed, which leaves them off by the difference
    #[unsafe(method(setFrameSize:))]
    fn set_frame_size(&self, size: NSSize) {
      // safe: NSView's own setFrameSize:, with its own signature
      let _: () = unsafe { msg_send![super(self), setFrameSize: size] };
      if self.ivars().holds_plugin {
        for view in self.subviews().iter() {
          view.setFrameOrigin(NSPoint::ZERO);
        }
      }
    }

    // clicks on this view itself, where no plugin is, go to whatever is
    // under it (the page)
    #[unsafe(method(hitTest:))]
    fn hit_test(&self, point: NSPoint) -> *mut NSView {
      // safe: NSView's own hitTest:, with its own signature. the view it
      // gives isn't ours to release, and is handed straight back
      let hit: *mut NSView = unsafe { msg_send![super(self), hitTest: point] };
      if std::ptr::eq(hit, &**self) {
        std::ptr::null_mut()
      } else {
        hit
      }
    }
  }
);

struct EditorViewIvars {
  // the plugin's view is in this one (not in a view inside it)
  holds_plugin: bool,
}

impl EditorView {
  fn new(mtm: MainThreadMarker, frame: NSRect, holds_plugin: bool) -> Retained<EditorView> {
    let this = EditorView::alloc(mtm).set_ivars(EditorViewIvars { holds_plugin });
    // safe: NSView's designated initializer
    let view: Retained<EditorView> = unsafe { msg_send![super(this), initWithFrame: frame] };
    // the plugin sizes its own view: growing this one mustn't change it
    view.setAutoresizesSubviews(false);
    view
  }
}

struct DelegateIvars {
  panel: Weak<NSPanel>,
  with_plugin: WithPlugin,
  on_close: Box<dyn Fn()>,
  // set while the window is resized for the plugin, or the plugin is asked
  // for a size, so neither answers the other
  busy: Cell<bool>,
  // the user closed the window
  closed: Cell<bool>,
  // the editor is open (with_plugin may not have reached the plugin)
  editor_open: Cell<bool>,
}

define_class!(
  // safe: NSObject has no subclassing requirements, and this adds no Drop
  #[unsafe(super(NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "DawesomePluginWindowDelegate"]
  #[ivars = DelegateIvars]
  struct WindowDelegate;

  // safe: no requirements
  unsafe impl NSObjectProtocol for WindowDelegate {}

  // safe: the methods have the protocol's signatures
  unsafe impl NSWindowDelegate for WindowDelegate {
    // the close button: close the editor, hide the window and tell the
    // caller, who closes it for good
    #[unsafe(method(windowShouldClose:))]
    fn window_should_close(&self, _sender: &NSWindow) -> bool {
      self.user_closed();
      false
    }

    // the user is dragging the window's edge: ask the plugin to take the
    // size, and keep the window to what it took
    #[unsafe(method(windowWillResize:toSize:))]
    fn window_will_resize(&self, sender: &NSWindow, size: NSSize) -> NSSize {
      let ivars = self.ivars();
      if ivars.busy.get() || ivars.closed.get() {
        return size;
      }
      let wanted = sender.contentRectForFrameRect(NSRect::new(NSPoint::ZERO, size)).size;
      let mut took = None;
      ivars.busy.set(true);
      (ivars.with_plugin)(&mut |plugin| took = Some(plugin.resize_editor(points(wanted.width), points(wanted.height))));
      ivars.busy.set(false);
      match took {
        Some((w, h)) if w > 0 && h > 0 => sender.frameRectForContentRect(content(w, h)).size,
        _ => sender.frame().size,
      }
    }
  }
);

impl WindowDelegate {
  fn new(mtm: MainThreadMarker, panel: &Retained<NSPanel>, with_plugin: WithPlugin, on_close: Box<dyn Fn()>) -> Retained<Self> {
    let this = WindowDelegate::alloc(mtm).set_ivars(DelegateIvars {
      panel: Weak::from_retained(panel),
      with_plugin,
      on_close,
      busy: Cell::new(false),
      closed: Cell::new(false),
      editor_open: Cell::new(false),
    });
    // safe: NSObject's initializer
    unsafe { msg_send![super(this), init] }
  }

  fn user_closed(&self) {
    // on_close may drop the EditorWindow, and this with it
    let _alive = self.retain();
    let ivars = self.ivars();
    if ivars.closed.replace(true) {
      return;
    }
    let mut done = false;
    (ivars.with_plugin)(&mut |plugin| {
      forget_resizes(plugin);
      plugin.close_editor();
      done = true;
    });
    if done {
      ivars.editor_open.set(false);
    }
    if let Some(panel) = ivars.panel.load() {
      panel.orderOut(None);
    }
    (ivars.on_close)();
  }

  // make the window's content (w, h), keeping its top-left corner where it
  // is (the plugin asked for the size, or it just opened)
  fn fit(&self, w: u32, h: u32) {
    let ivars = self.ivars();
    let Some(panel) = ivars.panel.load() else { return };
    if ivars.busy.get() || w == 0 || h == 0 {
      return;
    }
    let old = panel.frame();
    let mut frame = panel.frameRectForContentRect(content(w, h));
    frame.origin = NSPoint::new(old.origin.x, old.origin.y + old.size.height - frame.size.height);
    ivars.busy.set(true);
    panel.setFrame_display(frame, true);
    ivars.busy.set(false);
  }
}

fn content(w: u32, h: u32) -> NSRect {
  NSRect::new(NSPoint::ZERO, NSSize::new(w as f64, h as f64))
}

fn points(v: f64) -> u32 {
  v.round().max(1.) as u32
}

// a weak reference to a main-thread object, for the resize callback a
// plugin keeps (which must be Send)
struct MainWeak<T: Message>(Weak<T>);

// safe: it's only loaded on the main thread (load checks), and dropping a
// weak reference is thread-safe
unsafe impl<T: Message> Send for MainWeak<T> {}

impl<T: Message> MainWeak<T> {
  fn load(&self) -> Option<Retained<T>> {
    MainThreadMarker::new()?;
    self.0.load()
  }
}

fn forget_resizes(plugin: &mut Plugin) {
  plugin.on_editor_resize(Box::new(|_, _| {}));
}

fn view_ptr(view: &NSView) -> *mut c_void {
  view as *const NSView as *mut c_void
}

// a floating window with a plugin's editor in it (see window.rs)
pub struct EditorWindow {
  panel: Retained<NSPanel>,
  delegate: Retained<WindowDelegate>,
}

// safe to move with its Plugin: every method checks it's on the main
// thread, and dropping it elsewhere doesn't free anything (see Drop)
unsafe impl Send for EditorWindow {}

pub fn open_window(
  plugin: &mut Plugin,
  title: &str,
  parent_window: *mut c_void,
  position: Option<(f64, f64)>,
  with_plugin: WithPlugin,
  on_close: Box<dyn Fn()>,
) -> Result<EditorWindow, String> {
  let mtm = MainThreadMarker::new().ok_or("plugin windows open on the main thread")?;
  let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
  let start = content(400, 300);
  let panel =
    NSPanel::initWithContentRect_styleMask_backing_defer(NSPanel::alloc(mtm), start, style, NSBackingStoreType::Buffered, false);
  // safe: the panel is held by the EditorWindow
  unsafe { panel.setReleasedWhenClosed(false) };
  // above the app's windows, hidden while another app is in front, and
  // never the main window (a panel can't be), so the page keeps its
  // shortcuts once it's clicked again
  panel.setFloatingPanel(true);
  panel.setHidesOnDeactivate(true);
  panel.setCollectionBehavior(NSWindowCollectionBehavior::FullScreenAuxiliary);
  panel.setTitle(&NSString::from_str(title));
  let holder = EditorView::new(mtm, start, true);
  panel.setContentView(Some(&holder));
  let delegate = WindowDelegate::new(mtm, &panel, with_plugin, on_close);
  panel.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
  let window = EditorWindow { panel, delegate };

  let weak = MainWeak(Weak::from_retained(&window.delegate));
  plugin.on_editor_resize(Box::new(move |w, h| {
    if let Some(delegate) = weak.load() {
      delegate.fit(w, h);
    }
  }));
  let (w, h) = match plugin.open_editor(view_ptr(&holder)) {
    Ok(size) => size,
    Err(e) => {
      forget_resizes(plugin);
      return Err(e);
    }
  };
  window.delegate.ivars().editor_open.set(true);
  if plugin.editor_can_resize() {
    window.panel.setStyleMask(style | NSWindowStyleMask::Resizable);
  }
  window.delegate.fit(w, h);

  let frame = window.panel.frame();
  let top_left = match position {
    Some((x, y)) => NSPoint::new(x, y),
    None if !parent_window.is_null() => {
      // safe: the caller passes its live NSWindow
      let parent = unsafe { &*(parent_window as *const NSWindow) }.frame();
      NSPoint::new(
        parent.origin.x + (parent.size.width - frame.size.width) / 2.,
        parent.origin.y + (parent.size.height + frame.size.height) / 2.,
      )
    }
    None => {
      window.panel.center();
      let centred = window.panel.frame();
      NSPoint::new(centred.origin.x, centred.origin.y + centred.size.height)
    }
  };
  window.panel.setFrameTopLeftPoint(on_screen(mtm, top_left, frame.size));
  window.panel.orderFront(None);
  Ok(window)
}

// where a window of `size` with its top-left corner at `top_left` should go
// to be on a screen: the one with most of it, or the main one
fn on_screen(mtm: MainThreadMarker, top_left: NSPoint, size: NSSize) -> NSPoint {
  let frame = NSRect::new(NSPoint::new(top_left.x, top_left.y - size.height), size);
  let overlap = |area: NSRect| {
    let w = (frame.origin.x + frame.size.width).min(area.origin.x + area.size.width) - frame.origin.x.max(area.origin.x);
    let h = (frame.origin.y + frame.size.height).min(area.origin.y + area.size.height) - frame.origin.y.max(area.origin.y);
    w.max(0.) * h.max(0.)
  };
  let areas: Vec<NSRect> = NSScreen::screens(mtm).iter().map(|s| s.visibleFrame()).collect();
  let best = areas.iter().copied().max_by(|a, b| overlap(*a).total_cmp(&overlap(*b)));
  let area = match best {
    Some(a) if overlap(a) > 0. => a,
    _ => match NSScreen::mainScreen(mtm) {
      Some(screen) => screen.visibleFrame(),
      None => return top_left,
    },
  };
  // the title bar on screen first, then as much of the rest as fits
  let right = area.origin.x + area.size.width;
  let top = area.origin.y + area.size.height;
  let x = top_left.x.min(right - size.width).max(area.origin.x);
  let y = top_left.y.max(area.origin.y + size.height).min(top);
  NSPoint::new(x, y)
}

impl EditorWindow {
  // bring it to the front (without taking the keyboard from the main window)
  pub fn focus(&self) {
    if MainThreadMarker::new().is_some() && !self.delegate.ivars().closed.get() {
      self.panel.orderFront(None);
    }
  }

  // its top-left corner, to hand back to open_window
  pub fn position(&self) -> (f64, f64) {
    if MainThreadMarker::new().is_none() {
      return (0., 0.);
    }
    let frame = self.panel.frame();
    (frame.origin.x, frame.origin.y + frame.size.height)
  }

  // the NSPanel*, for checks and screenshots
  pub fn ns_window(&self) -> *mut c_void {
    Retained::as_ptr(&self.panel) as *mut c_void
  }

  // close the editor (unless the user's closing the window already did),
  // then the window
  pub fn close(self, plugin: &mut Plugin) {
    if MainThreadMarker::new().is_some() && self.delegate.ivars().editor_open.replace(false) {
      forget_resizes(plugin);
      plugin.close_editor();
    }
  }
}

impl Drop for EditorWindow {
  fn drop(&mut self) {
    if MainThreadMarker::new().is_none() {
      // AppKit objects can't be freed here: keep them alive for good
      std::mem::forget(self.panel.clone());
      std::mem::forget(self.delegate.clone());
      return;
    }
    // closing it mustn't reach the delegate (on_close has run, or is moot)
    self.panel.setDelegate(None);
    self.panel.close();
  }
}

// a plugin's editor inside the main window, over the page (decision D11).
// the plugin's view sits at (x, y) on the page at its own size, and the
// parts of it outside `clip` are cut off. clicks and keys inside the plugin
// go to it, everything else to the page. any number may be docked at once
//
// the clipping view above the webview covers only the part of the plugin
// that's inside `clip`, never more: a clipping view draws black wherever
// it's bigger than the plugin in it (seen around PanCake 2, 2026-11-11), and
// the page must show there
//
// `host_view` is what Tauri's ns_view() gives: the window's content view
// (the WKWebView is found inside it, or the content view is taken to be
// where the page is) or the WKWebView itself (the clip view goes beside it,
// in its superview). page coordinates are measured from that view's top-left
pub struct Dock {
  views: DockViews,
  // where the page last put it: the clip and the plugin's top-left
  placed: Arc<Mutex<(Rect, f64, f64)>>,
}

#[derive(Clone)]
struct DockViews {
  clip_view: Retained<EditorView>,
  holder: Retained<EditorView>,
  // the view the page's coordinates are measured from
  page: Retained<NSView>,
}

// the views, weakly, for the resize callback a plugin keeps
struct WeakDockViews {
  clip_view: MainWeak<EditorView>,
  holder: MainWeak<EditorView>,
  page: MainWeak<NSView>,
}

impl WeakDockViews {
  fn of(views: &DockViews) -> Self {
    WeakDockViews {
      clip_view: MainWeak(Weak::from_retained(&views.clip_view)),
      holder: MainWeak(Weak::from_retained(&views.holder)),
      page: MainWeak(Weak::from_retained(&views.page)),
    }
  }

  fn load(&self) -> Option<DockViews> {
    Some(DockViews { clip_view: self.clip_view.load()?, holder: self.holder.load()?, page: self.page.load()? })
  }
}

// safe to move with its Plugin, as EditorWindow is
unsafe impl Send for Dock {}

// `fill` is the room there is for it: a plugin that can resize takes that
// size, snapped to one it accepts (see Dock::fill)
pub fn dock(
  plugin: &mut Plugin,
  host_view: *mut c_void,
  clip: Rect,
  x: f64,
  y: f64,
  fill: Option<(u32, u32)>,
) -> Result<Dock, String> {
  let mtm = MainThreadMarker::new().ok_or("plugins dock on the main thread")?;
  // safe: the caller passes a live NSView (or null, which gives None)
  let host = unsafe { Retained::retain(host_view as *mut NSView) }.ok_or("no view to dock the plugin in")?;
  let is_content = host.window().and_then(|w| w.contentView()).map_or(false, |c| c == host);
  let (page, parent) = if is_content {
    (find_webview(&host).unwrap_or_else(|| host.clone()), host.clone())
  } else {
    // safe: reading a live view's superview
    let parent = unsafe { host.superview() }.ok_or("the webview isn't in a window")?;
    (host.clone(), parent)
  };

  let clip_view = EditorView::new(mtm, NSRect::ZERO, false);
  // views clip by default before macOS 14, and only when asked after
  if clip_view.respondsToSelector(sel!(setClipsToBounds:)) {
    clip_view.setClipsToBounds(true);
  }
  // only as big as the plugin's view, once it has one: anything around it
  // is the page's
  let holder = EditorView::new(mtm, content(1, 1), true);
  clip_view.addSubview(&holder);
  // on top of everything in it, the webview included
  parent.addSubview(&clip_view);
  // the window resizing keeps it the same distance from the top until the
  // page says where it goes now
  if !parent.isFlipped() {
    clip_view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinYMargin);
  }
  let dock = Dock { views: DockViews { clip_view, holder, page }, placed: Arc::new(Mutex::new((clip, x, y))) };
  dock.place(clip, x, y);

  // the plugin asked for a size: the clip follows it
  let (weak, placed) = (WeakDockViews::of(&dock.views), dock.placed.clone());
  plugin.on_editor_resize(Box::new(move |w, h| {
    if let Some(views) = weak.load() {
      views.holder.setFrameSize(NSSize::new(w as f64, h as f64));
      let (clip, x, y) = *placed.lock().unwrap_or_else(|e| e.into_inner());
      views.lay_out(clip, x, y);
    }
  }));
  match plugin.open_editor(view_ptr(&dock.views.holder)) {
    Ok((w, h)) => {
      dock.resized(w, h);
      if let Some((w, h)) = fill {
        dock.fill(plugin, w, h);
      }
      Ok(dock)
    }
    Err(e) => {
      forget_resizes(plugin);
      Err(e)
    }
  }
}

// the first WKWebView inside a view
fn find_webview(view: &NSView) -> Option<Retained<NSView>> {
  let class = AnyClass::get(c"WKWebView")?;
  for sub in view.subviews().iter() {
    if sub.isKindOfClass(class) {
      return Some(sub);
    }
    if let Some(found) = find_webview(&sub) {
      return Some(found);
    }
  }
  None
}

// how far down its view the page starts: a webview that reaches under the
// window's title bar lays its page out below it, so the page's top isn't
// the view's
fn page_inset(page: &NSView) -> f64 {
  let Some(window) = page.window() else { return 0. };
  // both in the window's coordinates, which go up
  let usable = window.contentLayoutRect();
  let view = page.convertRect_toView(page.bounds(), None);
  let over = (view.origin.y + view.size.height) - (usable.origin.y + usable.size.height);
  over.max(0.)
}

impl DockViews {
  // the plugin's view at (x, y) on the page, and the clip view over just the
  // part of it inside `clip`
  fn lay_out(&self, clip: Rect, x: f64, y: f64) {
    // safe: reading a live view's superview
    let Some(parent) = (unsafe { self.clip_view.superview() }) else { return };
    let size = self.holder.frame().size;
    let (left, top) = (clip.x.max(x), clip.y.max(y));
    let right = (clip.x + clip.width.max(0.)).min(x + size.width);
    let bottom = (clip.y + clip.height.max(0.)).min(y + size.height);
    let shown = NSSize::new((right - left).max(0.), (bottom - top).max(0.));
    // that part in the page view's own coordinates, which go up or down.
    // the page starts below whatever of its view is under the title bar
    let bounds = self.page.bounds();
    let down = top + page_inset(&self.page);
    let origin_y = if self.page.isFlipped() {
      bounds.origin.y + down
    } else {
      bounds.origin.y + bounds.size.height - down - shown.height
    };
    let rect = NSRect::new(NSPoint::new(bounds.origin.x + left, origin_y), shown);
    self.clip_view.setFrame(self.page.convertRect_toView(rect, Some(&parent)));
    // the clip view goes down from its top-left, as the page does
    self.holder.setFrameOrigin(NSPoint::new(x - left, y - top));
  }
}

impl Dock {
  // the page moved under it: `clip` and (x, y) as dock takes them
  pub fn place(&self, clip: Rect, x: f64, y: f64) {
    if MainThreadMarker::new().is_none() {
      return;
    }
    *self.placed.lock().unwrap_or_else(|e| e.into_inner()) = (clip, x, y);
    self.views.lay_out(clip, x, y);
  }

  // the plugin's view is (w, h) now
  fn resized(&self, w: u32, h: u32) {
    if w == 0 || h == 0 {
      return;
    }
    self.views.holder.setFrameSize(NSSize::new(w as f64, h as f64));
    let (clip, x, y) = *self.placed.lock().unwrap_or_else(|e| e.into_inner());
    self.views.lay_out(clip, x, y);
  }

  // there's (w, h) of room for it: a plugin that can resize is asked to
  // take it, snapped as the floating window's are; a fixed one stays as it
  // is. gives the size it has now
  pub fn fill(&self, plugin: &mut Plugin, w: u32, h: u32) -> (u32, u32) {
    if MainThreadMarker::new().is_none() {
      return (0, 0);
    }
    if plugin.editor_can_resize() {
      let (w, h) = plugin.resize_editor(w.max(1), h.max(1));
      self.resized(w, h);
    }
    self.size()
  }

  // the editor's size (it may have asked for another)
  pub fn size(&self) -> (u32, u32) {
    if MainThreadMarker::new().is_none() {
      return (0, 0);
    }
    let size = self.views.holder.frame().size;
    (size.width.round() as u32, size.height.round() as u32)
  }

  // close the editor and take the views away
  pub fn undock(self, plugin: &mut Plugin) {
    if MainThreadMarker::new().is_none() {
      return;
    }
    let window = self.views.clip_view.window();
    let focused = window
      .as_ref()
      .and_then(|w| w.firstResponder())
      .and_then(|r| r.downcast::<NSView>().ok())
      .map_or(false, |v| v.isDescendantOf(&self.views.clip_view));
    forget_resizes(plugin);
    plugin.close_editor();
    self.views.clip_view.removeFromSuperview();
    // the keyboard goes back to the page
    if let (true, Some(window)) = (focused, window) {
      window.makeFirstResponder(Some(&self.views.page));
    }
  }
}

impl Drop for Dock {
  fn drop(&mut self) {
    if MainThreadMarker::new().is_none() {
      // AppKit objects can't be freed here: keep them alive for good
      std::mem::forget(self.views.clone());
      return;
    }
    self.views.clip_view.removeFromSuperview();
  }
}
