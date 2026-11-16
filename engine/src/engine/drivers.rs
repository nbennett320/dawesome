use rodio::cpal;

#[cfg(target_os = "linux")]
use pulse;
#[cfg(target_os = "linux")]
use psimple;

pub fn get_sound_host_names() -> Vec<String> {
  cpal::available_hosts().iter().map(|e| { String::from(e.name()) }).collect()
}
