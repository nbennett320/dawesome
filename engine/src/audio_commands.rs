// the Audio Settings tab's commands (see daw/daw_core/audio_settings.rs)
//
// they run off the main thread: listing devices and reopening the output can
// take a moment
use crate::daw;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

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

  daw::apply_audio_settings(settings)?;
  Ok(status())
}
