use std::vec::{Vec};
use std::fs::{File};
use std::io::{BufReader};
use rodio::{Decoder, Source};

use crate::util;

#[derive(Clone, PartialEq, Eq)]
pub struct WaveformData {
  pub pathd: Option<String>,
  pub viewbox: Option<String>,
}

impl WaveformData {
  pub fn new() -> Self {
    WaveformData { 
      pathd: None,
      viewbox: None
    }
  }

  pub fn from(
    pathd: String, 
    viewbox: String,
  ) -> Self {
    WaveformData {
      pathd: Some(pathd),
      viewbox: Some(viewbox),
    }
  }
}

// a sample's waveform: its level at evenly spaced points, from 0 (the lowest)
// to 1 (the highest), about 2 points to each millisecond of sound
// - samples: interleaved samples for all channels
// - sample_rate: the sample file's own rate
// returns None if the samples are empty or too short to draw
pub fn calc_waveform_from_samples(
  samples: &[i16],
  channels: u16,
  sample_rate: u32,
) -> Option<Vec<f32>> {
  if samples.is_empty() || channels == 0 || sample_rate == 0 {
    return None;
  }

  let dur_ms = samples.len() as f32 / sample_rate as f32 * 1_000. / channels as f32;
  // at least one point, so very short samples don't divide by zero
  let lod = (dur_ms.round() as u32 * 2).max(1);

  let ys: Vec<i32> = samples
    .iter()
    .map(|y| *y as i32)
    .collect();
  let y_gauss = util::math::gaussian_1d(&ys, 1., false)?;
  let y_smoothed = util::math::sample_to_n_elements(&y_gauss, lod)?;

  Some(util::math::f_normalize::<f32>(y_smoothed))
}

// a waveform's levels as sent to the frontend: each level (0..1) as a
// little-endian u16 (0..65535), in base64. a fraction of the size of the
// levels as JSON numbers (see decodeLevels in src/util/waveform.ts)
pub fn encode_levels(levels: &[f32]) -> String {
  let bytes: Vec<u8> = levels
    .iter()
    .flat_map(|level| ((level.clamp(0., 1.) * 65_535.).round() as u16).to_le_bytes())
    .collect();

  base64::encode(bytes)
}

// get a path of a node's normalized audio waveform, given
// the path to a sound sample
pub fn calc_waveform_from_file_path(sample_path: &str) -> Option<Vec<f32>> {
  let file = BufReader::new(File::open(sample_path).ok()?);
  let source = Decoder::new(file).ok()?;
  let channels = source.channels();
  let sample_rate = source.sample_rate();
  let mut samples = Vec::<i16>::new();

  for sample in source {
    samples.push(sample);
  }

  calc_waveform_from_samples(&samples, channels, sample_rate)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_encode_levels() {
    // 0, 65535, 32768 as little-endian u16s. src/util/waveform.spec.ts
    // decodes the same string
    assert_eq!(encode_levels(&[0., 1., 0.5]), "AAD//wCA");
    // kept within 0..1
    assert_eq!(encode_levels(&[-1., 2.]), encode_levels(&[0., 1.]));
    assert_eq!(encode_levels(&[]), "");
  }

  #[test]
  fn test_points_follow_the_sample_rate() {
    // a second of stereo sound gets 2000 points, whatever its rate
    let noise = |n: usize| (0..n).map(|i| ((i * 7919) % 2001) as i16 - 1000).collect::<Vec<_>>();
    assert_eq!(calc_waveform_from_samples(&noise(44_100 * 2), 2, 44_100).unwrap().len(), 2_000);
    assert_eq!(calc_waveform_from_samples(&noise(48_000 * 2), 2, 48_000).unwrap().len(), 2_000);
  }
}
