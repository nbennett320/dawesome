// the Audio Settings tab's commands (see actions/audio.rs)
//
// they run off the main thread: listing devices and reopening the output can
// take a moment
use crate::daw::actions::audio::{
  delete_recording_folder_in, delete_recording_in, dir_to_change, existing_dir, move_recording_folder_in,
  move_recording_in, record_setup, rename_recording_folder_in, rename_recording_in, status, AudioStatus, Library,
  MovedRecording, RecordSetup,
};
use crate::daw::playback;
use crate::engine::{self, audio_settings, input};
use crate::commands::TState;
use crate::engine::drivers;

#[tauri::command]
pub fn get_audio_drivers() -> Result<Vec<String>, String> {
  Ok(drivers::get_sound_host_names())
}

#[tauri::command(async)]
pub fn get_audio_settings() -> AudioStatus {
  status()
}

// use these devices and buffer size from now on. playback pauses first (it
// keeps its place). settings that don't work are refused, leaving the old
// ones playing
#[tauri::command(async)]
pub fn set_audio_settings(state: TState<'_>, settings: audio_settings::AudioSettings) -> Result<AudioStatus, String> {
  if state.playlist.playing() {
    playback::pause_playlist(&state);
  }

  let before = engine::audio_settings();
  // the effects come off the output before it's replaced, and go onto
  // whichever one is open after (the old one, if the new settings failed),
  // with their plugins prepared for its rate
  let reopens = engine::engine_if_open().is_some()
    && (before.output_device != settings.output_device || before.buffer_frames != settings.buffer_frames);
  let effects = &state.playlist.effects;
  if reopens { effects.detach(); }
  let applied = engine::apply_audio_settings(settings.clone());
  effects.attach();
  applied?;

  // the input records at the output's rate, from the chosen device
  if before.input_device != settings.input_device
    || before.output_device != settings.output_device
    || before.buffer_frames != settings.buffer_frames
  {
    // (playback, and so any recording, has already stopped)
    input::reset_input();
  }
  Ok(status())
}

// start or stop metering the input, for as long as the Audio tab shows it
#[tauri::command(async)]
pub fn listen_to_input(on: bool) -> Result<(), String> {
  input::listen_to_input(on)
}

// each input channel's loudest level since last asked, from 0 to 1 (empty
// while nothing's listening)
#[tauri::command]
pub fn get_input_levels() -> Vec<f32> {
  input::input_if_open().map(|input| input.take_peaks()).unwrap_or_default()
}

#[tauri::command(async)]
pub fn get_record_setup(state: TState<'_>) -> RecordSetup {
  record_setup(&state)
}

// arm a track to record these input channels, or (with none) disarm it
#[tauri::command(async)]
pub fn set_track_armed(
  state: TState<'_>,
  track_number: u32,
  input: Option<crate::daw::recording::InputChoice>,
) -> RecordSetup {
  state.recorder.lock().unwrap().arm(track_number, input);
  crate::daw::recording::hold_input_for_armed(&state);
  record_setup(&state)
}

// record every armed track, playing from the playhead
#[tauri::command(async)]
pub fn start_recording(
  state: TState<'_>,
  prefs: tauri::State<'_, crate::preferences::AppPreferences>,
) -> Result<crate::daw::recording::Recorded, String> {
  crate::daw::recording::start(state.inner(), prefs.current().loop_recording)?;
  Ok(crate::daw::recording::collect(&state))
}

// stop recording (and playback, which keeps its place), with the clips the
// takes became
#[tauri::command(async)]
pub fn stop_recording(state: TState<'_>) -> crate::daw::recording::Recorded {
  if crate::daw::recording::is_recording(&state) {
    playback::pause_playlist(&state);
  }
  crate::daw::recording::collect(&state)
}

// whether it's still recording, and the clips recorded since last asked
// (recording also ends when playback stops, or at a loop's end)
#[tauri::command]
pub fn collect_recording(state: TState<'_>) -> crate::daw::recording::Recorded {
  crate::daw::recording::collect(&state)
}

// the project's takes (newest first) or clips, and folders, for the sidebar's
// browsers
#[tauri::command(async)]
pub fn list_recordings(state: TState<'_>, library: Option<Library>) -> crate::daw::project_files::RecordingsListing {
  let dir = existing_dir(&state, library.unwrap_or_default());
  dir.map(|dir| crate::daw::project_files::list_recordings(&dir)).unwrap_or_default()
}

// rename a take (it stays in its folder), keeping the clips and sequencer
// rows that play it
#[tauri::command]
pub fn rename_recording(state: TState<'_>, path: String, name: String, library: Option<Library>) -> Result<MovedRecording, String> {
  rename_recording_in(&state, &path, &name, library.unwrap_or_default())
}

// move a take into a folder (relative to the recordings folder, "" for the
// top), keeping the clips and sequencer rows that play it
#[tauri::command]
pub fn move_recording(state: TState<'_>, path: String, to_folder: String, library: Option<Library>) -> Result<MovedRecording, String> {
  move_recording_in(&state, &path, &to_folder, library.unwrap_or_default())
}

// move one folder into another (both relative to the recordings folder),
// with everything in it
#[tauri::command]
pub fn move_recording_folder(state: TState<'_>, folder: String, to_folder: String, library: Option<Library>) -> Result<MovedRecording, String> {
  move_recording_folder_in(&state, &folder, &to_folder, library.unwrap_or_default())
}

// a new, empty folder inside `parent` (relative to the recordings folder,
// "" for the top). returns its path, relative
#[tauri::command]
pub fn create_recording_folder(state: TState<'_>, parent: String, name: String, library: Option<Library>) -> Result<String, String> {
  let library = library.unwrap_or_default();
  let dir = dir_to_change(&state, library, true)?;
  library.worded(crate::daw::project_files::create_folder(&dir, &parent, &name))
}

// rename a folder, keeping the clips and sequencer rows that play what's in it
#[tauri::command]
pub fn rename_recording_folder(state: TState<'_>, folder: String, name: String, library: Option<Library>) -> Result<MovedRecording, String> {
  rename_recording_folder_in(&state, &folder, &name, library.unwrap_or_default())
}

// move a take or clip the project doesn't use to the trash
#[tauri::command]
pub fn delete_recording(state: TState<'_>, path: String, library: Option<Library>) -> Result<(), String> {
  delete_recording_in(&state, &path, library.unwrap_or_default())
}

// move a folder, and everything in it, to the trash. refused while the
// project uses any take in it
#[tauri::command]
pub fn delete_recording_folder(state: TState<'_>, folder: String, library: Option<Library>) -> Result<(), String> {
  delete_recording_folder_in(&state, &folder, library.unwrap_or_default())
}
