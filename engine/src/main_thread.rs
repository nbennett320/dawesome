// running things on the process's main thread
//
// plugins expect every call that isn't audio (loading, state, their windows,
// idle) on the main thread (see plugin/vst3/plugin.rs). tauri runs plain
// commands there already (WebKit hands the webview's requests to the main
// thread, and tauri calls a command that isn't async straight from that
// handler), but async commands (loading and saving a project, a mixdown)
// run on a pool, and they hand their plugin work over with `run`
//
// whoever waits for the main thread must not hold any of the project's
// locks while it waits: the main thread may be in a command waiting for that
// same lock
//
// without an app (tests, the debug console) there's no main thread to go
// to, so `run` runs the work where it's called
use std::sync::{mpsc, OnceLock};
use std::thread::{self, ThreadId};

static MAIN: OnceLock<ThreadId> = OnceLock::new();
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

// called once, on the main thread, when the app has started
pub fn init(app: tauri::AppHandle) {
  let _ = MAIN.set(thread::current().id());
  let _ = APP.set(app);
}

// whether this is the main thread (or there's no app, so any thread will do)
pub fn here() -> bool {
  MAIN.get().map_or(true, |main| *main == thread::current().id())
}

// run `work` on the main thread and give back what it returns, waiting for
// it. an error only when the app is closing and the main thread no longer
// takes work
pub fn run<R: Send>(work: impl FnOnce() -> R + Send) -> Result<R, String> {
  let app = match APP.get() {
    Some(app) if !here() => app,
    _ => return Ok(work()),
  };

  let (done, result) = mpsc::sync_channel(1);
  let job: Box<dyn FnOnce() + Send + '_> = Box::new(move || {
    let _ = done.send(work());
  });
  // safe: this doesn't return until the job has run or been dropped
  // unrun (either way `done` goes, which ends the wait below), so whatever
  // it borrows outlives it
  let job: Box<dyn FnOnce() + Send + 'static> = unsafe { std::mem::transmute(job) };
  app.run_on_main_thread(job).map_err(|e| format!("the app is closing ({})", e))?;
  result.recv().map_err(|_| "the app is closing".to_string())
}

// something that has to be dropped on the main thread (a plugin), from
// wherever it ends up
pub struct DropOnMain<T: Send>(Option<T>);

impl<T: Send> DropOnMain<T> {
  pub fn new(value: T) -> Self {
    DropOnMain(Some(value))
  }
}

impl<T: Send> Drop for DropOnMain<T> {
  fn drop(&mut self) {
    if let Some(value) = self.0.take() {
      // (if the app is closing, it goes with the process)
      if let Err(e) = run(move || drop(value)) {
        println!("plugins weren't closed: {}", e);
      }
    }
  }
}
