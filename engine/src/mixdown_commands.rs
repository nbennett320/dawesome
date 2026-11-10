// the Mixdown tab's commands: what to call the file and where to put it,
// starting a mixdown (it renders on a thread of its own), how it's going,
// cancelling it, and showing the file in the Finder
//
// one mixdown at a time. it carries on with the tab closed; the tab asks
// how it's going (as the record button and meters do) rather than being told
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::daw;
use crate::mixdown::{self, Progress, CANCELLED};
use crate::mixdown_file::{Format, MixdownFile};

// the folder in a saved project's folder that mixdowns go in by default
const MIXDOWNS_FOLDER: &str = "mixdowns";
const UNTITLED: &str = "Untitled";
// what's said when a mixdown is started while one is going
pub const BUSY: &str = "A mixdown is already going";

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Status {
  Idle,
  // progress: 0 to 1
  Rendering { progress: f32 },
  Done { path: String },
  Failed { message: String },
  Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
  Idle,
  Rendering,
  Done(PathBuf),
  Failed(String),
  Cancelled,
}

// the mixdown going (or the last one), shared with the thread rendering it
#[derive(Default)]
pub struct Mixdowns {
  phase: Mutex<Option<Phase>>,
  progress: Progress,
  cancel: AtomicBool,
}

impl Mixdowns {
  pub fn status(&self) -> Status {
    match self.phase.lock().unwrap().clone().unwrap_or(Phase::Idle) {
      Phase::Idle => Status::Idle,
      Phase::Rendering => Status::Rendering { progress: self.progress.fraction() },
      Phase::Done(path) => Status::Done { path: path.to_string_lossy().into_owned() },
      Phase::Failed(message) => Status::Failed { message },
      Phase::Cancelled => Status::Cancelled,
    }
  }

  fn finished(&self, result: Result<PathBuf, String>) {
    *self.phase.lock().unwrap() = Some(match result {
      Ok(path) => Phase::Done(path),
      Err(e) if e == CANCELLED => Phase::Cancelled,
      Err(e) => Phase::Failed(e),
    });
  }
}

pub type MixdownState<'a> = tauri::State<'a, Arc<Mixdowns>>;

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MixdownDefaults {
  pub name: String,
  pub dir: String,
}

// a saved project's name, and a folder next to its file. a project that's
// never been saved: "Untitled", in the Music folder
fn defaults_for(project: Option<&Path>) -> MixdownDefaults {
  let name = project.and_then(crate::project_files::project_name).unwrap_or_else(|| UNTITLED.into());
  let dir = match project {
    Some(path) => path.parent().unwrap_or_else(|| Path::new("")).join(MIXDOWNS_FOLDER),
    None => dirs::audio_dir().or_else(dirs::home_dir).unwrap_or_default(),
  };
  MixdownDefaults { name, dir: dir.to_string_lossy().into_owned() }
}

#[tauri::command]
pub fn mixdown_defaults(state: daw::TState<'_>) -> MixdownDefaults {
  defaults_for(state.project.lock().unwrap().path.as_deref())
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MixdownOptions {
  pub name: String,
  pub dir: String,
  pub format: Format,
}

// start a mixdown of the song as it is now. anything that stops it before
// it starts (nothing to mix, no name, a folder that can't be written to) is
// the error; anything after, the status says
#[tauri::command]
pub fn start_mixdown(state: daw::TState<'_>, mixdowns: MixdownState<'_>, options: MixdownOptions) -> Result<(), String> {
  start(&state, mixdowns.inner(), options, daw::open_engine_rate().unwrap_or(44_100)).map(|_| ())
}

// - rate: the engine's
fn start(
  state: &daw::InnerState,
  mixdowns: &Arc<Mixdowns>,
  options: MixdownOptions,
  rate: u32,
) -> Result<std::thread::JoinHandle<()>, String> {
  {
    let mut phase = mixdowns.phase.lock().unwrap();
    if *phase == Some(Phase::Rendering) { return Err(BUSY.into()); }
    *phase = Some(Phase::Rendering);
  }
  mixdowns.cancel.store(false, Ordering::Relaxed);

  let rate = options.format.render_rate(rate);
  let started = mixdown::snapshot(state)
    .and_then(|snapshot| Ok((snapshot, MixdownFile::create(Path::new(&options.dir), &options.name, options.format, rate)?)));
  let (snapshot, mut file) = match started {
    Ok(started) => started,
    Err(e) => {
      *mixdowns.phase.lock().unwrap() = None;
      return Err(e);
    }
  };

  let mixdowns = mixdowns.clone();
  Ok(std::thread::spawn(move || {
    let rendered = mixdown::render(snapshot, rate, &mut |chunk| file.write(chunk), &mixdowns.progress, &mixdowns.cancel);
    mixdowns.finished(rendered.and_then(|_| file.finish()));
  }))
}

#[tauri::command]
pub fn mixdown_status(mixdowns: MixdownState<'_>) -> Status {
  mixdowns.status()
}

// stop the mixdown going, if there is one. nothing of it is kept
#[tauri::command]
pub fn cancel_mixdown(mixdowns: MixdownState<'_>) {
  mixdowns.cancel.store(true, Ordering::Relaxed);
}

// show a file in the Finder (or the file manager), selected
#[tauri::command]
pub fn reveal_file(path: String) -> Result<(), String> {
  let path = PathBuf::from(path);
  if !path.exists() { return Err(format!("{} isn't there any more", path.display())); }

  #[cfg(target_os = "macos")]
  let command = std::process::Command::new("open").arg("-R").arg(&path).spawn();
  #[cfg(target_os = "windows")]
  let command = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
  #[cfg(not(any(target_os = "macos", target_os = "windows")))]
  let command = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(&path)).spawn();

  command.map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::event_dispatch::add_clip;
  use crate::mixdown_file::Bits;

  const KICK: &str = "assets/assets_66-bd-01.wav";
  const LONG: &str = "assets/slide_guitar.wav";

  fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dawesome-mixdowns-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
  }

  fn options(dir: &Path, name: &str) -> MixdownOptions {
    MixdownOptions { name: name.into(), dir: dir.to_string_lossy().into_owned(), format: Format::Wav { bits: Bits::Int16 } }
  }

  #[test]
  fn test_a_mixdown_goes_from_rendering_to_done() {
    let dir = folder("done");
    let state = daw::InnerState::default();
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    let mixdowns = Arc::new(Mixdowns::default());
    assert_eq!(mixdowns.status(), Status::Idle);

    let thread = start(&state, &mixdowns, options(&dir, "song"), 48_000).unwrap();
    assert!(matches!(mixdowns.status(), Status::Rendering { .. } | Status::Done { .. }));
    thread.join().unwrap();
    let path = dir.join("song.wav");
    assert_eq!(mixdowns.status(), Status::Done { path: path.to_string_lossy().into_owned() });
    let spec = hound::WavReader::open(&path).unwrap().spec();
    assert_eq!((spec.sample_rate, spec.bits_per_sample), (48_000, 16));

    // again: the next name, and an MP3 at a rate it takes
    let mut mp3 = options(&dir, "song");
    mp3.format = Format::Mp3 { kbps: 192 };
    start(&state, &mixdowns, options(&dir, "song"), 48_000).unwrap().join().unwrap();
    start(&state, &mixdowns, mp3, 96_000).unwrap().join().unwrap();
    assert_eq!(mixdowns.status(), Status::Done { path: dir.join("song.mp3").to_string_lossy().into_owned() });
    assert!(dir.join("song 2.wav").is_file());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_one_at_a_time() {
    let dir = folder("busy");
    let state = daw::InnerState::default();
    add_clip(&state, LONG.into(), 0, Some(0.)).unwrap();
    let mixdowns = Arc::new(Mixdowns::default());
    // (as if one were going)
    *mixdowns.phase.lock().unwrap() = Some(Phase::Rendering);
    assert_eq!(start(&state, &mixdowns, options(&dir, "song"), 48_000).err().unwrap(), BUSY);
    assert!(!dir.exists());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_cancelled_mixdown_leaves_nothing() {
    let dir = folder("cancel");
    let state = daw::InnerState::default();
    add_clip(&state, LONG.into(), 0, Some(0.)).unwrap();
    let mixdowns = Arc::new(Mixdowns::default());
    let thread = start(&state, &mixdowns, options(&dir, "song"), 48_000).unwrap();
    mixdowns.cancel.store(true, Ordering::Relaxed);
    thread.join().unwrap();
    // it may have finished first, on a fast machine: then it's done, else cancelled
    match mixdowns.status() {
      Status::Cancelled => assert!(std::fs::read_dir(&dir).unwrap().next().is_none()),
      Status::Done { .. } => {}
      other => panic!("{:?}", other),
    }
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_refused_before_it_starts_leaves_it_idle() {
    let dir = folder("refused");
    let state = daw::InnerState::default();
    let mixdowns = Arc::new(Mixdowns::default());
    assert_eq!(start(&state, &mixdowns, options(&dir, "song"), 48_000).err().unwrap(), mixdown::NOTHING_TO_MIX);
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    assert_eq!(start(&state, &mixdowns, options(&dir, "  "), 48_000).err().unwrap(), crate::mixdown_file::NEEDS_NAME);
    assert_eq!(mixdowns.status(), Status::Idle);
    assert!(!dir.exists());
  }

  #[test]
  fn test_defaults() {
    let saved = defaults_for(Some(Path::new("/music/My Song/My Song.dawesome")));
    assert_eq!(saved, MixdownDefaults { name: "My Song".into(), dir: "/music/My Song/mixdowns".into() });
    let unsaved = defaults_for(None);
    assert_eq!(unsaved.name, "Untitled");
    assert!(!unsaved.dir.is_empty());
  }
}
