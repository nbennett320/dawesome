// the Audio Settings tab's commands (see daw/daw_core/audio_settings.rs)
//
// they run off the main thread: listing devices and reopening the output can
// take a moment
use crate::daw;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

// the input, while it's open (for the meter, or recording)
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputStatus {
  device: String,
  channels: u16,
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
      latency_ms: input.latency().as_secs_f64() * 1000.,
    }),
    notice: open_error.or_else(daw::audio_notice),
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
    if let Some(take) = daw::reset_input() {
      println!("the audio settings changed while recording: kept {}", take.path.display());
    }
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

// a take that's been recorded
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TakeInfo {
  path: String,
  seconds: f64,
  channels: u16,
  // samples lost because the disk couldn't keep up
  dropped: u64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
  recording: bool,
  // the take just finished, when recording stopped
  take: Option<TakeInfo>,
}

// start recording a take into the project's recordings folder (or, before
// the project's saved, a folder of its own), or stop and keep it
#[tauri::command(async)]
pub fn t_toggle_recording(state: daw::TState<'_>) -> Result<RecordingStatus, String> {
  if let Some(take) = daw::stop_recording()? {
    println!("recorded {} ({:.2}s, {} samples dropped)", take.path.display(), take.duration().as_secs_f64(), take.dropped);
    return Ok(RecordingStatus {
      recording: false,
      take: Some(TakeInfo {
        path: take.path.to_string_lossy().into_owned(),
        seconds: take.duration().as_secs_f64(),
        channels: take.channels,
        dropped: take.dropped,
      }),
    });
  }

  let path = {
    let mut location = state.project.lock().unwrap();
    let dir = location.recordings_dir()?;
    crate::project_files::next_take_path(&dir, &location.name(), &crate::project_files::today())
  };
  daw::start_recording(&path)?;
  println!("recording to {}", path.display());
  Ok(RecordingStatus { recording: true, take: None })
}
