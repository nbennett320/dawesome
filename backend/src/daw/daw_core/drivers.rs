use rodio::cpal::{self, traits::HostTrait};
use rodio::{self, DeviceTrait};

#[cfg(target_os = "linux")]
use pulse;
#[cfg(target_os = "linux")]
use psimple;

pub fn print_device_drivers() -> Result<(), String> {
  println!("Supported hosts:\n  {:?}", cpal::ALL_HOSTS);
  let available_hosts = cpal::available_hosts();
  println!("Available hosts:\n  {:?}", available_hosts);

  for host_id in available_hosts {
    println!("{}", host_id.name());
    let host = cpal::host_from_id(host_id).unwrap();

    let default_in = host.default_input_device().map(|e| e.name().unwrap());
    let default_out = host.default_output_device().map(|e| e.name().unwrap());
    println!("  Default Input Device:\n    {:?}", default_in);
    println!("  Default Output Device:\n    {:?}", default_out);

    let devices = host.devices().unwrap();
    println!("  Devices: ");
    for (device_index, device) in devices.enumerate() {
      println!("  {}. \"{}\"", device_index + 1, device.name().unwrap());

      // Input configs
      if let Ok(conf) = device.default_input_config() {
        println!("    Default input stream config:\n      {:?}", conf);
      }
      let input_configs = match device.supported_input_configs() {
        Ok(f) => f.collect(),
        Err(e) => {
          println!("    Error getting supported input configs: {:?}", e);
          Vec::new()
        }
      };
      if !input_configs.is_empty() {
        println!("    All supported input stream configs:");
        for (config_index, config) in input_configs.into_iter().enumerate() {
          println!(
            "      {}.{}. {:?}",
            device_index + 1,
            config_index + 1,
            config
          );
        }
      }

      // Output configs
      if let Ok(conf) = device.default_output_config() {
        println!("    Default output stream config:\n      {:?}", conf);
      }
      let output_configs = match device.supported_output_configs() {
        Ok(f) => f.collect(),
        Err(e) => {
          println!("    Error getting supported output configs: {:?}", e);
          Vec::new()
        }
      };
      if !output_configs.is_empty() {
        println!("    All supported output stream configs:");
        for (config_index, config) in output_configs.into_iter().enumerate() {
          println!(
            "      {}.{}. {:?}",
            device_index + 1,
            config_index + 1,
            config
          );
        }
      }
    }
  }

  Ok(())
}

pub fn get_sound_host_names() -> Vec<String> {
  cpal::available_hosts().iter().map(|e| { String::from(e.name()) }).collect()
}
