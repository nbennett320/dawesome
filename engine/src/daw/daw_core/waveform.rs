use std::vec::{Vec};
use std::fs::{File};
use std::io::{BufReader};
use num_traits::ToPrimitive;
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

// calculate a path of a node's normalized audio waveform
// returns None if the samples are empty or too short to draw
pub fn calc_waveform_from_samples(
  samples: Vec<i16>, 
  channels: u16,
) -> Option<Vec<f32>> {
  if samples.is_empty() || channels == 0 {
    return None;
  }

  let dur_ms = samples.len() as f32 / 44_100. * 1_000. / channels as f32;
  // at least one point, so very short samples don't divide by zero
  let lod = (dur_ms.round() as u32 * 2).max(1);
  println!("dur_ms: {}, lod: {}", dur_ms, lod);

  // get frames and interpolate points
  let xs: Vec<i32> = (0..samples.len())
    .into_iter()
    .map(|x| x as i32)
    .collect();
  let ys: Vec<i32> = samples
    .iter()
    .map(|y| *y as i32)
    .collect();
  let y_gauss = util::math::gaussian_1d(&ys, 1., false)?;
  let y_smoothed = util::math::sample_to_n_elements(&y_gauss, lod as u32)?;
  let len = y_smoothed.len();
  let y_norms = util::math::f_normalize::<f32>(y_smoothed);

  println!("ysmoothed len: {}", len);
  println!("dur_ms: {}", dur_ms);

  let xsf: Vec<f32> = xs
    .iter()
    .map(|x| x.to_f32().unwrap())
    .collect();

  util::math::interleave(xsf.get(0..(lod as usize))?.to_vec(), y_norms)
}

// get a path of a node's normalized audio waveform, given
// the path to a sound sample
pub fn calc_waveform_from_file_path(sample_path: &str) -> Option<Vec<f32>> {
  let file = BufReader::new(File::open(sample_path).ok()?);
  let source = Decoder::new(file).ok()?;
  let channels = source.channels();
  let mut samples = Vec::<i16>::new();

  for sample in source {
    samples.push(sample);
  }

  calc_waveform_from_samples(samples, channels)
}
