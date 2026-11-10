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
  daw::apply_audio_settings(settings.clone())?;

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
pub fn t_start_recording(state: daw::TState<'_>) -> Result<crate::recording::Recorded, String> {
  crate::recording::start(state.inner())?;
  Ok(crate::recording::collect(&state))
}

// stop recording (and playback, which keeps its place), with the clips the
// takes became
#[tauri::command(async)]
pub fn t_stop_recording(state: daw::TState<'_>) -> crate::recording::Recorded {
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

// the project's takes (newest first) and folders, for the sidebar's
// recordings browser. an unsaved project that hasn't recorded anything has
// none
#[tauri::command(async)]
pub fn t_list_recordings(state: daw::TState<'_>) -> crate::project_files::RecordingsListing {
  let dir = state.project.lock().unwrap().existing_recordings_dir();
  dir.map(|dir| crate::project_files::list_recordings(&dir)).unwrap_or_default()
}

// the recordings folder, for changing what's in it, as long as nothing's
// being recorded into it
// - make: make the folder for an unsaved project that hasn't recorded yet
fn recordings_dir_to_change(state: &daw::InnerState, make: bool) -> Result<PathBuf, String> {
  if crate::recording::is_busy(state) {
    return Err("wait for the recording to finish".into());
  }
  let mut location = state.project.lock().unwrap();
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
  let sequencer = if !moved.is_empty() && seq.move_row_samples(&moved) { Some(seq.state()) } else { None };
  drop(seq);
  drop(audiograph);
  // clips and rows that undoing would put back follow the file too
  state.history.lock().unwrap().repoint(&|path| moved_path(path, from, to));

  (moved_clips, sequencer)
}

// whether any clip or sequencer row plays `path` (a take, or anything in a
// folder)
fn in_use(state: &daw::InnerState, path: &Path) -> bool {
  state.playlist.audiograph.lock().unwrap().nodes.iter().any(|node| Path::new(node.sample_path()).starts_with(path))
    || state.playlist.sequencer.lock().unwrap().rows().iter().any(|row| Path::new(&row.path).starts_with(path))
}

// rename a take (it stays in its folder), keeping the clips and sequencer
// rows that play it
#[tauri::command]
pub fn t_rename_recording(state: daw::TState<'_>, path: String, name: String) -> Result<MovedRecording, String> {
  rename_recording(&state, &path, &name)
}

fn rename_recording(state: &daw::InnerState, path: &str, name: &str) -> Result<MovedRecording, String> {
  let dir = recordings_dir_to_change(state, false)?;
  let from = PathBuf::from(path);
  let to = crate::project_files::rename_take(&from, &dir, name)?;
  let path = to.to_string_lossy().into_owned();
  Ok(moved(state, &from, &to, path))
}

// move a take into a folder (relative to the recordings folder, "" for the
// top), keeping the clips and sequencer rows that play it
#[tauri::command]
pub fn t_move_recording(state: daw::TState<'_>, path: String, to_folder: String) -> Result<MovedRecording, String> {
  move_recording(&state, &path, &to_folder)
}

fn move_recording(state: &daw::InnerState, path: &str, to_folder: &str) -> Result<MovedRecording, String> {
  let dir = recordings_dir_to_change(state, false)?;
  let from = PathBuf::from(path);
  if !from.is_file() {
    return Err("that isn't one of the project's recordings".into());
  }
  let to = crate::project_files::move_into(&dir, &from, to_folder)?;
  let path = to.to_string_lossy().into_owned();
  Ok(moved(state, &from, &to, path))
}

// move one folder into another (both relative to the recordings folder),
// with everything in it
#[tauri::command]
pub fn t_move_recording_folder(state: daw::TState<'_>, folder: String, to_folder: String) -> Result<MovedRecording, String> {
  move_recording_folder(&state, &folder, &to_folder)
}

fn move_recording_folder(state: &daw::InnerState, folder: &str, to_folder: &str) -> Result<MovedRecording, String> {
  let dir = recordings_dir_to_change(state, false)?;
  if folder.is_empty() {
    return Err("the recordings folder itself can't be moved".into());
  }
  let from = crate::project_files::folder_in(&dir, folder)?;
  let to = crate::project_files::move_into(&dir, &from, to_folder)?;
  let path = crate::project_files::relative(&dir, &to);
  Ok(moved(state, &from, &to, path))
}

// a new, empty folder inside `parent` (relative to the recordings folder,
// "" for the top). returns its path, relative
#[tauri::command]
pub fn t_create_recording_folder(state: daw::TState<'_>, parent: String, name: String) -> Result<String, String> {
  let dir = recordings_dir_to_change(&state, true)?;
  crate::project_files::create_folder(&dir, &parent, &name)
}

// rename a folder, keeping the clips and sequencer rows that play what's in it
#[tauri::command]
pub fn t_rename_recording_folder(state: daw::TState<'_>, folder: String, name: String) -> Result<MovedRecording, String> {
  rename_recording_folder(&state, &folder, &name)
}

fn rename_recording_folder(state: &daw::InnerState, folder: &str, name: &str) -> Result<MovedRecording, String> {
  let dir = recordings_dir_to_change(state, false)?;
  let (from, to) = crate::project_files::rename_folder(&dir, folder, name)?;
  let path = crate::project_files::relative(&dir, &to);
  Ok(moved(state, &from, &to, path))
}

// move a take the project doesn't use to the trash
#[tauri::command]
pub fn t_delete_recording(state: daw::TState<'_>, path: String) -> Result<(), String> {
  let dir = recordings_dir_to_change(&state, false)?;
  let file = Path::new(&path);
  if !file.is_file() || !crate::project_files::is_inside(file, &dir) {
    return Err("that isn't one of the project's recordings".into());
  }
  if in_use(&state, file) {
    return Err("the project still uses this recording".into());
  }
  trash::delete(file).map_err(|e| format!("could not move the recording to the trash: {}", e))
}

// move a folder, and everything in it, to the trash. refused while the
// project uses any take in it
#[tauri::command]
pub fn t_delete_recording_folder(state: daw::TState<'_>, folder: String) -> Result<(), String> {
  let dir = recordings_dir_to_change(&state, false)?;
  if folder.is_empty() {
    return Err("the recordings folder itself can't be deleted".into());
  }
  let path = crate::project_files::folder_in(&dir, &folder)?;
  if in_use(&state, &path) {
    return Err("the project still uses recordings in this folder".into());
  }
  trash::delete(&path).map_err(|e| format!("could not move the folder to the trash: {}", e))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
      .join(format!("dawesome-audio-commands-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
  }

  // a saved project with a take on the playlist and in a sequencer row
  fn saved_project(name: &str) -> (daw::InnerState, PathBuf, PathBuf, String) {
    let dir = test_dir(name);
    let state = daw::InnerState::default();
    let file = dir.join("song").join("song.dawesome");
    let recordings = dir.join("song").join("recordings");
    std::fs::create_dir_all(&recordings).unwrap();
    let take = recordings.join("song_11092026_0.wav").to_string_lossy().into_owned();
    std::fs::copy(daw::METRONOME_TICK_PATH, &take).unwrap();
    std::fs::create_dir_all(recordings.join("Vocals")).unwrap();

    state.playlist.audiograph.lock().unwrap()
      .construct_and_add_node(take.clone(), std::time::Duration::ZERO, 0).unwrap();
    state.playlist.sequencer.lock().unwrap().add_row(&take).unwrap();
    crate::project::save_project_to(&state, &file, false).unwrap();
    (state, dir, file, take)
  }

  // the project as it opens next time, and the samples it couldn't find
  fn reopen(file: &Path) -> (crate::project::LoadedProject, daw::InnerState) {
    let state = daw::InnerState::default();
    let loaded = crate::project::load_from(&state, file).unwrap();
    (loaded, state)
  }

  #[test]
  fn test_a_moved_take_is_found_when_the_project_is_opened_again() {
    let (state, dir, file, take) = saved_project("reopen-moved");
    let moved = move_recording(&state, &take, "Vocals").unwrap();

    let (loaded, _) = reopen(&file);
    assert!(loaded.missing.is_empty(), "{:?}", loaded.missing);
    assert_eq!(loaded.clips[0].path, moved.path);
    assert_eq!(loaded.sequencer.rows[0].path, moved.path);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_renamed_takes_and_folders_are_found_when_the_project_is_opened_again() {
    let (state, dir, file, take) = saved_project("reopen-renamed");
    let renamed = rename_recording(&state, &take, "lead").unwrap();
    move_recording(&state, &renamed.path, "Vocals").unwrap();
    rename_recording_folder(&state, "Vocals", "Lead vocals").unwrap();
    std::fs::create_dir_all(dir.join("song").join("recordings").join("Old")).unwrap();
    let last = move_recording_folder(&state, "Lead vocals", "Old").unwrap();
    assert_eq!(last.path, "Old/Lead vocals");

    let (loaded, _) = reopen(&file);
    let expected = dir.join("song").join("recordings").join("Old").join("Lead vocals").join("lead.wav");
    assert!(loaded.missing.is_empty(), "{:?}", loaded.missing);
    assert_eq!(loaded.clips[0].path, expected.to_string_lossy());
    assert_eq!(loaded.sequencer.rows[0].path, expected.to_string_lossy());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_moving_a_take_saves_nothing_else() {
    let (state, dir, file, take) = saved_project("reopen-unsaved");
    // a change made after saving, and not saved
    state.playlist.audiograph.lock().unwrap()
      .construct_and_add_node(daw::METRONOME_TICK_PATH.to_string(), std::time::Duration::ZERO, 1).unwrap();
    move_recording(&state, &take, "Vocals").unwrap();

    let (loaded, _) = reopen(&file);
    assert_eq!(loaded.clips.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_moved_path() {
    let from = Path::new("/song/recordings/Vocals");
    let to = Path::new("/song/recordings/Old/Vocals");
    // a take in the folder, or the folder itself
    assert_eq!(moved_path("/song/recordings/Vocals/a.wav", from, to), Some("/song/recordings/Old/Vocals/a.wav".into()));
    assert_eq!(moved_path("/song/recordings/Vocals/x/b.wav", from, to), Some("/song/recordings/Old/Vocals/x/b.wav".into()));
    // a take that moved on its own
    let take = Path::new("/song/recordings/a.wav");
    assert_eq!(moved_path("/song/recordings/a.wav", take, Path::new("/song/recordings/B/a.wav")), Some("/song/recordings/B/a.wav".into()));
    // not in it, even with a name that starts the same
    assert_eq!(moved_path("/song/recordings/Vocals 2/a.wav", from, to), None);
    assert_eq!(moved_path("/samples/kick.wav", from, to), None);
  }
}
