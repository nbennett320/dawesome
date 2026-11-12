// the Audio Settings tab's commands (see daw/daw_core/audio_settings.rs)
//
// they run off the main thread: listing devices and reopening the output can
// take a moment
use crate::daw;
use std::path::{Path, PathBuf};
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

// the input, while it's open (for the meter, or recording)
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputStatus {
  device: String,
  channels: u16,
  // the output's, unless the input can't record at it (its takes are then
  // converted as they play)
  sample_rate: u32,
  // from a frame coming into the device to the engine getting it
  latency_ms: f64,
}

// the output that's playing
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputStatus {
  device: String,
  sample_rate: u32,
  // the most frames the device has asked for at once
  buffer_frames: u64,
  // from rendering a frame to hearing it, as the device reports it
  latency_ms: f64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioStatus {
  settings: daw::AudioSettings,
  // device names, to choose from
  outputs: Vec<String>,
  inputs: Vec<String>,
  // the buffer sizes the chosen output takes
  buffer_choices: Vec<u32>,
  // none when the output can't be opened
  output: Option<OutputStatus>,
  // none unless something's listening to the input
  input: Option<InputStatus>,
  // why it can't, or anything else to know about it
  notice: Option<String>,
}

fn device_names<I: Iterator<Item = rodio::cpal::Device>>(devices: Result<I, rodio::cpal::DevicesError>) -> Vec<String> {
  devices
    .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
    .unwrap_or_default()
}

fn status() -> AudioStatus {
  let host = rodio::cpal::default_host();
  let settings = daw::audio_settings();

  let (output, open_error) = match daw::engine() {
    Ok(engine) => (Some(OutputStatus {
      device: engine.device_name().to_string(),
      sample_rate: engine.sample_rate(),
      buffer_frames: engine.buffer_frames(),
      latency_ms: engine.latency().as_secs_f64() * 1000.,
    }), None),
    Err(e) => (None, Some(format!("The audio output couldn't be opened: {}", e))),
  };

  let buffer_choices = daw::output_device(settings.output_device.as_deref())
    .ok()
    .and_then(|device| device.default_output_config().ok())
    .map(|config| daw::buffer_choices(config.buffer_size()))
    .unwrap_or_else(|| daw::BUFFER_CHOICES.to_vec());

  AudioStatus {
    settings,
    outputs: device_names(host.output_devices()),
    inputs: device_names(host.input_devices()),
    buffer_choices,
    output,
    input: daw::input_if_open().map(|input| InputStatus {
      device: input.device_name().to_string(),
      channels: input.device_channels(),
      sample_rate: input.sample_rate(),
      latency_ms: input.latency().as_secs_f64() * 1000.,
    }),
    notice: {
      // anything wrong with the output, then the input
      let notices: Vec<String> = [open_error.or_else(daw::audio_notice), daw::input_notice()]
        .into_iter().flatten().collect();
      if notices.is_empty() { None } else { Some(notices.join(". ")) }
    },
  }
}

#[tauri::command]
pub fn get_audio_drivers() -> Result<Vec<String>, String> {
  Ok(daw::drivers::get_sound_host_names())
}

#[tauri::command(async)]
pub fn get_audio_settings() -> AudioStatus {
  status()
}

// use these devices and buffer size from now on. playback pauses first (it
// keeps its place). settings that don't work are refused, leaving the old
// ones playing
#[tauri::command(async)]
pub fn set_audio_settings(state: daw::TState<'_>, settings: daw::AudioSettings) -> Result<AudioStatus, String> {
  if state.playlist.playing() {
    daw::pause_playlist(&state);
  }

  let before = daw::audio_settings();
  // the effects come off the output before it's replaced, and go onto
  // whichever one is open after (the old one, if the new settings failed),
  // with their plugins prepared for its rate
  let reopens = daw::engine_if_open().is_some()
    && (before.output_device != settings.output_device || before.buffer_frames != settings.buffer_frames);
  let effects = &state.playlist.effects;
  if reopens { effects.detach(); }
  let applied = daw::apply_audio_settings(settings.clone());
  effects.attach();
  applied?;

  // the input records at the output's rate, from the chosen device
  if before.input_device != settings.input_device
    || before.output_device != settings.output_device
    || before.buffer_frames != settings.buffer_frames
  {
    // (playback, and so any recording, has already stopped)
    daw::reset_input();
  }
  Ok(status())
}

// start or stop metering the input, for as long as the Audio tab shows it
#[tauri::command(async)]
pub fn listen_to_input(on: bool) -> Result<(), String> {
  daw::listen_to_input(on)
}

// each input channel's loudest level since last asked, from 0 to 1 (empty
// while nothing's listening)
#[tauri::command]
pub fn get_input_levels() -> Vec<f32> {
  daw::input_if_open().map(|input| input.take_peaks()).unwrap_or_default()
}

// what recording needs to know: which tracks are armed, the input's
// channels to choose from, and how recordings are lined up
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordSetup {
  armed: Vec<crate::recording::ArmedTrack>,
  // none when there's no input to record from
  input_channels: Option<u16>,
  input_error: Option<String>,
}

fn record_setup(state: &daw::InnerState) -> RecordSetup {
  let (input_channels, input_error) = match daw::input_channel_count() {
    Ok(channels) => (Some(channels), None),
    Err(e) => (None, Some(e)),
  };
  RecordSetup {
    armed: state.recorder.lock().unwrap().armed(),
    input_channels,
    input_error,
  }
}

#[tauri::command(async)]
pub fn get_record_setup(state: daw::TState<'_>) -> RecordSetup {
  record_setup(&state)
}

// arm a track to record these input channels, or (with none) disarm it
#[tauri::command(async)]
pub fn set_track_armed(
  state: daw::TState<'_>,
  track_number: u32,
  input: Option<crate::recording::InputChoice>,
) -> RecordSetup {
  state.recorder.lock().unwrap().arm(track_number, input);
  record_setup(&state)
}

// record every armed track, playing from the playhead
#[tauri::command(async)]
pub fn start_recording(state: daw::TState<'_>) -> Result<crate::recording::Recorded, String> {
  crate::recording::start(state.inner())?;
  Ok(crate::recording::collect(&state))
}

// stop recording (and playback, which keeps its place), with the clips the
// takes became
#[tauri::command(async)]
pub fn stop_recording(state: daw::TState<'_>) -> crate::recording::Recorded {
  if crate::recording::is_recording(&state) {
    daw::pause_playlist(&state);
  }
  crate::recording::collect(&state)
}

// whether it's still recording, and the clips recorded since last asked
// (recording also ends when playback stops, or at a loop's end)
#[tauri::command]
pub fn collect_recording(state: daw::TState<'_>) -> crate::recording::Recorded {
  crate::recording::collect(&state)
}

// which of the project's folders of audio a sidebar page shows
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Library { #[default] Recordings, Clips }

impl Library {
  // what the library's files are called, in the sentences shown to the user
  fn noun(self) -> &'static str {
    match self { Library::Recordings => "recording", Library::Clips => "clip" }
  }

  fn nouns(self) -> &'static str {
    match self { Library::Recordings => "recordings", Library::Clips => "clips" }
  }

  // a refusal from the folder helpers in project_files.rs, which speak of
  // recordings, in this library's words
  fn worded<T>(self, result: Result<T, String>) -> Result<T, String> {
    match self {
      Library::Recordings => result,
      Library::Clips => result.map_err(|e| e.replace("recording", "clip")),
    }
  }
}

// the library's folder, if it's there. an unsaved project that hasn't
// recorded anything has no recordings, and has never had any clips
fn existing_dir(state: &daw::InnerState, library: Library) -> Option<PathBuf> {
  let location = state.project.lock().unwrap();
  match library {
    Library::Recordings => location.existing_recordings_dir(),
    Library::Clips => location.path.as_deref()
      .map(crate::project_files::clips_dir_for)
      .filter(|dir| dir.is_dir()),
  }
}

// the project's takes (newest first) or clips, and folders, for the sidebar's
// browsers
#[tauri::command(async)]
pub fn list_recordings(state: daw::TState<'_>, library: Option<Library>) -> crate::project_files::RecordingsListing {
  let dir = existing_dir(&state, library.unwrap_or_default());
  dir.map(|dir| crate::project_files::list_recordings(&dir)).unwrap_or_default()
}

// the library's folder, for changing what's in it. recordings wait until
// nothing's being recorded into the folder
// - make: make the folder for a project that hasn't got one yet (recordings
//   can be made for an unsaved project, clips only for a saved one)
fn dir_to_change(state: &daw::InnerState, library: Library, make: bool) -> Result<PathBuf, String> {
  if library == Library::Recordings && crate::recording::is_busy(state) {
    return Err("wait for the recording to finish".into());
  }
  let mut location = state.project.lock().unwrap();
  if library == Library::Clips {
    let dir = location.path.as_deref().map(crate::project_files::clips_dir_for)
      .ok_or_else(|| "the project has no clips yet".to_string())?;
    if !dir.is_dir() {
      if !make { return Err("the project has no clips yet".into()); }
      std::fs::create_dir_all(&dir).map_err(|e| format!("could not make the clips folder: {}", e))?;
    }
    return Ok(dir);
  }
  match location.existing_recordings_dir() {
    Some(dir) => Ok(dir),
    None if make => location.recordings_dir(),
    None => Err("the project has no recordings".to_string()),
  }
}

// a take or a folder after it's moved or renamed, and the clips and
// sequencer rows that now play from its new place
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MovedRecording {
  // where it is now: a take's full path, or a folder's path relative to the
  // recordings folder (as they were given)
  pub path: String,
  pub moved_clips: Vec<crate::project::MovedClip>,
  // the sequencer, when a row plays a take that moved
  pub sequencer: Option<daw::SequencerState>,
  // the saved project file couldn't be changed to follow it, so it'll only
  // find it once the project's saved again
  pub problem: Option<String>,
}

// where a sample is after `from` (a take, or a folder of them) moved to `to`
fn moved_path(path: &str, from: &Path, to: &Path) -> Option<String> {
  let rest = Path::new(path).strip_prefix(from).ok()?;
  let moved = if rest.as_os_str().is_empty() { to.to_path_buf() } else { to.join(rest) };
  Some(moved.to_string_lossy().into_owned())
}

// what moved, after it's moved on disk: clips and sequencer rows play it
// from where it is now, and so does the saved project file
fn moved(state: &daw::InnerState, from: &Path, to: &Path, path: String) -> MovedRecording {
  let (moved_clips, sequencer) = repoint(state, from, to);
  crate::project::follow_moved_in_baseline(state, from, to);
  let project = state.project.lock().unwrap().path.clone();
  let problem = project
    .and_then(|file| crate::project::follow_moved_recording(&file, from, to).err())
    .map(|e| format!("the project file still has the old place, until it's saved: {}", e));
  if let Some(problem) = &problem { println!("{}", problem); }
  MovedRecording { path, moved_clips, sequencer, problem }
}

// clips and sequencer rows playing `from` (a take, or anything in a folder)
// play from `to` instead
fn repoint(state: &daw::InnerState, from: &Path, to: &Path) -> (Vec<crate::project::MovedClip>, Option<daw::SequencerState>) {
  let mut moved_clips = Vec::new();
  if from == to { return (moved_clips, None); }

  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  for node in audiograph.nodes.iter_mut() {
    if let Some(path) = moved_path(node.sample_path(), from, to) {
      node.set_sample_path(path.clone());
      moved_clips.push(crate::project::MovedClip { id: node.id, path });
    }
  }

  let mut seq = state.playlist.sequencer.lock().unwrap();
  let moved: Vec<(String, String)> = seq.rows().iter()
    .filter_map(|row| moved_path(&row.path, from, to).map(|path| (row.path.clone(), path)))
    .collect();
  let sequencer = if !moved.is_empty() && seq.move_row_samples(&moved) { Some(seq.state(&*state.playlist.effects)) } else { None };
  drop(seq);
  drop(audiograph);
  // clips and rows that undoing would put back follow the file too
  state.history.lock().unwrap().repoint(&|path| moved_path(path, from, to));
  // and so does the sound decoded from it, for clips made of it from now on
  daw::decoded_moved(&|path| moved_path(path, from, to));

  (moved_clips, sequencer)
}

// whether the project uses `path` (a take, or anything in a folder): a
// clip or a sequencer row plays it, or one that undo or redo could bring
// back does
fn in_use(state: &daw::InnerState, path: &Path) -> bool {
  state.playlist.audiograph.lock().unwrap().nodes.iter().any(|node| Path::new(node.sample_path()).starts_with(path))
    || state.playlist.sequencer.lock().unwrap().rows().iter().any(|row| Path::new(&row.path).starts_with(path))
    || state.history.lock().unwrap().uses(path)
}

// rename a take (it stays in its folder), keeping the clips and sequencer
// rows that play it
#[tauri::command]
pub fn rename_recording(state: daw::TState<'_>, path: String, name: String, library: Option<Library>) -> Result<MovedRecording, String> {
  rename_recording_in(&state, &path, &name, library.unwrap_or_default())
}

fn rename_recording_in(state: &daw::InnerState, path: &str, name: &str, library: Library) -> Result<MovedRecording, String> {
  let dir = dir_to_change(state, library, false)?;
  let from = PathBuf::from(path);
  let to = library.worded(crate::project_files::rename_take(&from, &dir, name))?;
  let path = to.to_string_lossy().into_owned();
  Ok(moved(state, &from, &to, path))
}

// move a take into a folder (relative to the recordings folder, "" for the
// top), keeping the clips and sequencer rows that play it
#[tauri::command]
pub fn move_recording(state: daw::TState<'_>, path: String, to_folder: String, library: Option<Library>) -> Result<MovedRecording, String> {
  move_recording_in(&state, &path, &to_folder, library.unwrap_or_default())
}

fn move_recording_in(state: &daw::InnerState, path: &str, to_folder: &str, library: Library) -> Result<MovedRecording, String> {
  let dir = dir_to_change(state, library, false)?;
  let from = PathBuf::from(path);
  if !from.is_file() {
    return Err(format!("that isn't one of the project's {}", library.nouns()));
  }
  let to = library.worded(crate::project_files::move_into(&dir, &from, to_folder))?;
  let path = to.to_string_lossy().into_owned();
  Ok(moved(state, &from, &to, path))
}

// move one folder into another (both relative to the recordings folder),
// with everything in it
#[tauri::command]
pub fn move_recording_folder(state: daw::TState<'_>, folder: String, to_folder: String, library: Option<Library>) -> Result<MovedRecording, String> {
  move_recording_folder_in(&state, &folder, &to_folder, library.unwrap_or_default())
}

fn move_recording_folder_in(state: &daw::InnerState, folder: &str, to_folder: &str, library: Library) -> Result<MovedRecording, String> {
  let dir = dir_to_change(state, library, false)?;
  if folder.is_empty() {
    return Err(format!("the {} folder itself can't be moved", library.nouns()));
  }
  let from = library.worded(crate::project_files::folder_in(&dir, folder))?;
  let to = library.worded(crate::project_files::move_into(&dir, &from, to_folder))?;
  let path = crate::project_files::relative(&dir, &to);
  Ok(moved(state, &from, &to, path))
}

// a new, empty folder inside `parent` (relative to the recordings folder,
// "" for the top). returns its path, relative
#[tauri::command]
pub fn create_recording_folder(state: daw::TState<'_>, parent: String, name: String, library: Option<Library>) -> Result<String, String> {
  let library = library.unwrap_or_default();
  let dir = dir_to_change(&state, library, true)?;
  library.worded(crate::project_files::create_folder(&dir, &parent, &name))
}

// rename a folder, keeping the clips and sequencer rows that play what's in it
#[tauri::command]
pub fn rename_recording_folder(state: daw::TState<'_>, folder: String, name: String, library: Option<Library>) -> Result<MovedRecording, String> {
  rename_recording_folder_in(&state, &folder, &name, library.unwrap_or_default())
}

fn rename_recording_folder_in(state: &daw::InnerState, folder: &str, name: &str, library: Library) -> Result<MovedRecording, String> {
  let dir = dir_to_change(state, library, false)?;
  let (from, to) = library.worded(crate::project_files::rename_folder(&dir, folder, name))?;
  let path = crate::project_files::relative(&dir, &to);
  Ok(moved(state, &from, &to, path))
}

// move a take or clip the project doesn't use to the trash
#[tauri::command]
pub fn delete_recording(state: daw::TState<'_>, path: String, library: Option<Library>) -> Result<(), String> {
  delete_recording_in(&state, &path, library.unwrap_or_default())
}

fn delete_recording_in(state: &daw::InnerState, path: &str, library: Library) -> Result<(), String> {
  let dir = dir_to_change(state, library, false)?;
  let file = Path::new(path);
  if !file.is_file() || !crate::project_files::is_inside(file, &dir) {
    return Err(format!("that isn't one of the project's {}", library.nouns()));
  }
  if in_use(state, file) {
    return Err(format!("the project still uses this {}", library.noun()));
  }
  trash::delete(file).map_err(|e| format!("could not move the {} to the trash: {}", library.noun(), e))
}

// move a folder, and everything in it, to the trash. refused while the
// project uses any take in it
#[tauri::command]
pub fn delete_recording_folder(state: daw::TState<'_>, folder: String, library: Option<Library>) -> Result<(), String> {
  delete_recording_folder_in(&state, &folder, library.unwrap_or_default())
}

fn delete_recording_folder_in(state: &daw::InnerState, folder: &str, library: Library) -> Result<(), String> {
  let dir = dir_to_change(state, library, false)?;
  if folder.is_empty() {
    return Err(format!("the {} folder itself can't be deleted", library.nouns()));
  }
  let path = library.worded(crate::project_files::folder_in(&dir, folder))?;
  if in_use(state, &path) {
    return Err(format!("the project still uses {} in this folder", library.nouns()));
  }
  trash::delete(&path).map_err(|e| format!("could not move the folder to the trash: {}", e))
}

#[cfg(test)]
mod tests;
