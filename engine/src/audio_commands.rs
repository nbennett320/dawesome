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
