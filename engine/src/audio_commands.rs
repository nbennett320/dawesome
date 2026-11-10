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
pub fn t_list_recordings(state: daw::TState<'_>, library: Option<Library>) -> crate::project_files::RecordingsListing {
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
pub fn t_rename_recording(state: daw::TState<'_>, path: String, name: String, library: Option<Library>) -> Result<MovedRecording, String> {
  rename_recording(&state, &path, &name, library.unwrap_or_default())
}

fn rename_recording(state: &daw::InnerState, path: &str, name: &str, library: Library) -> Result<MovedRecording, String> {
  let dir = dir_to_change(state, library, false)?;
  let from = PathBuf::from(path);
  let to = library.worded(crate::project_files::rename_take(&from, &dir, name))?;
  let path = to.to_string_lossy().into_owned();
  Ok(moved(state, &from, &to, path))
}

// move a take into a folder (relative to the recordings folder, "" for the
// top), keeping the clips and sequencer rows that play it
#[tauri::command]
pub fn t_move_recording(state: daw::TState<'_>, path: String, to_folder: String, library: Option<Library>) -> Result<MovedRecording, String> {
  move_recording(&state, &path, &to_folder, library.unwrap_or_default())
}

fn move_recording(state: &daw::InnerState, path: &str, to_folder: &str, library: Library) -> Result<MovedRecording, String> {
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
pub fn t_move_recording_folder(state: daw::TState<'_>, folder: String, to_folder: String, library: Option<Library>) -> Result<MovedRecording, String> {
  move_recording_folder(&state, &folder, &to_folder, library.unwrap_or_default())
}

fn move_recording_folder(state: &daw::InnerState, folder: &str, to_folder: &str, library: Library) -> Result<MovedRecording, String> {
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
pub fn t_create_recording_folder(state: daw::TState<'_>, parent: String, name: String, library: Option<Library>) -> Result<String, String> {
  let library = library.unwrap_or_default();
  let dir = dir_to_change(&state, library, true)?;
  library.worded(crate::project_files::create_folder(&dir, &parent, &name))
}

// rename a folder, keeping the clips and sequencer rows that play what's in it
#[tauri::command]
pub fn t_rename_recording_folder(state: daw::TState<'_>, folder: String, name: String, library: Option<Library>) -> Result<MovedRecording, String> {
  rename_recording_folder(&state, &folder, &name, library.unwrap_or_default())
}

fn rename_recording_folder(state: &daw::InnerState, folder: &str, name: &str, library: Library) -> Result<MovedRecording, String> {
  let dir = dir_to_change(state, library, false)?;
  let (from, to) = library.worded(crate::project_files::rename_folder(&dir, folder, name))?;
  let path = crate::project_files::relative(&dir, &to);
  Ok(moved(state, &from, &to, path))
}

// move a take or clip the project doesn't use to the trash
#[tauri::command]
pub fn t_delete_recording(state: daw::TState<'_>, path: String, library: Option<Library>) -> Result<(), String> {
  delete_recording(&state, &path, library.unwrap_or_default())
}

fn delete_recording(state: &daw::InnerState, path: &str, library: Library) -> Result<(), String> {
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
pub fn t_delete_recording_folder(state: daw::TState<'_>, folder: String, library: Option<Library>) -> Result<(), String> {
  delete_recording_folder(&state, &folder, library.unwrap_or_default())
}

fn delete_recording_folder(state: &daw::InnerState, folder: &str, library: Library) -> Result<(), String> {
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
    let moved = move_recording(&state, &take, "Vocals", Library::Recordings).unwrap();

    let (loaded, _) = reopen(&file);
    assert!(loaded.missing.is_empty(), "{:?}", loaded.missing);
    assert_eq!(loaded.clips[0].path, moved.path);
    assert_eq!(loaded.sequencer.rows[0].path, moved.path);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_renamed_takes_and_folders_are_found_when_the_project_is_opened_again() {
    let (state, dir, file, take) = saved_project("reopen-renamed");
    let renamed = rename_recording(&state, &take, "lead", Library::Recordings).unwrap();
    move_recording(&state, &renamed.path, "Vocals", Library::Recordings).unwrap();
    rename_recording_folder(&state, "Vocals", "Lead vocals", Library::Recordings).unwrap();
    std::fs::create_dir_all(dir.join("song").join("recordings").join("Old")).unwrap();
    let last = move_recording_folder(&state, "Lead vocals", "Old", Library::Recordings).unwrap();
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
    move_recording(&state, &take, "Vocals", Library::Recordings).unwrap();

    let (loaded, _) = reopen(&file);
    assert_eq!(loaded.clips.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
  }

  // a saved project with a clip file in clips/ used by a clip on the playlist
  fn project_with_a_clip_file(name: &str) -> (daw::InnerState, PathBuf, PathBuf, String) {
    let (state, dir, file, _) = saved_project(name);
    let clips = dir.join("song").join("clips");
    std::fs::create_dir_all(&clips).unwrap();
    let clip = clips.join("song_join_0.wav").to_string_lossy().into_owned();
    std::fs::copy(daw::METRONOME_TICK_PATH, &clip).unwrap();
    state.playlist.audiograph.lock().unwrap()
      .construct_and_add_node(clip.clone(), std::time::Duration::ZERO, 1).unwrap();
    (state, dir, file, clip)
  }

  fn clip_paths(state: &daw::InnerState) -> Vec<String> {
    state.playlist.audiograph.lock().unwrap().nodes.iter().map(|n| n.sample_path().to_string()).collect()
  }

  #[test]
  fn test_the_clips_library_lists_clips_and_not_recordings() {
    let (state, dir, _, clip) = project_with_a_clip_file("clips-list");
    let clips = crate::project_files::list_recordings(&existing_dir(&state, Library::Clips).unwrap());
    assert_eq!(clips.takes.len(), 1);
    assert_eq!(clips.takes[0].path, clip);
    let recordings = crate::project_files::list_recordings(&existing_dir(&state, Library::Recordings).unwrap());
    assert!(recordings.takes.iter().all(|take| take.path != clip));
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_renamed_clip_file_is_followed_by_its_clip() {
    let (state, dir, _, clip) = project_with_a_clip_file("clips-rename");
    let renamed = rename_recording(&state, &clip, "mix", Library::Clips).unwrap();
    assert!(renamed.path.ends_with("clips/mix.wav"), "{}", renamed.path);
    assert_eq!(renamed.moved_clips.len(), 1);
    assert_eq!(renamed.moved_clips[0].path, renamed.path);
    assert!(clip_paths(&state).contains(&renamed.path));
    assert!(!clip_paths(&state).contains(&clip));
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_clip_file_moved_into_a_new_folder_is_followed_by_its_clip() {
    let (state, dir, _, clip) = project_with_a_clip_file("clips-move");
    let dir_clips = dir.join("song").join("clips");
    let made = dir_to_change(&state, Library::Clips, true).unwrap();
    assert_eq!(made, dir_clips);
    crate::project_files::create_folder(&made, "", "Joins").unwrap();
    let moved = move_recording(&state, &clip, "Joins", Library::Clips).unwrap();
    assert!(moved.path.ends_with("clips/Joins/song_join_0.wav"), "{}", moved.path);
    assert_eq!(moved.moved_clips.len(), 1);
    assert!(clip_paths(&state).contains(&moved.path));
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_clip_file_in_use_is_not_deleted() {
    let (state, dir, _, clip) = project_with_a_clip_file("clips-delete");
    let error = delete_recording(&state, &clip, Library::Clips).unwrap_err();
    assert!(error.contains("clip"), "{}", error);
    assert!(Path::new(&clip).is_file());
    let error = delete_recording_folder(&state, "", Library::Clips).unwrap_err();
    assert!(error.contains("clips"), "{}", error);
    // a take isn't a clip file
    let take = dir.join("song").join("recordings").join("song_11092026_0.wav").to_string_lossy().into_owned();
    let error = delete_recording(&state, &take, Library::Clips).unwrap_err();
    assert!(error.contains("clips"), "{}", error);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_clips_of_a_project_that_was_never_saved() {
    let state = daw::InnerState::default();
    assert_eq!(existing_dir(&state, Library::Clips), None);
    let error = rename_recording(&state, "/nowhere/a.wav", "b", Library::Clips).unwrap_err();
    assert_eq!(error, "the project has no clips yet");
    assert_eq!(dir_to_change(&state, Library::Clips, true).unwrap_err(), "the project has no clips yet");
    assert_eq!(delete_recording(&state, "/nowhere/a.wav", Library::Clips).unwrap_err(), "the project has no clips yet");
  }

  #[test]
  fn test_recordings_are_the_default_library() {
    assert_eq!(Library::default(), Library::Recordings);
    let (state, dir, _, take) = saved_project("library-default");
    let renamed = rename_recording(&state, &take, "lead", Library::default()).unwrap();
    assert!(renamed.path.ends_with("recordings/lead.wav"), "{}", renamed.path);
    let error = delete_recording_folder(&state, "", Library::Recordings).unwrap_err();
    assert_eq!(error, "the recordings folder itself can't be deleted");
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
