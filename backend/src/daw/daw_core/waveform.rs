use std::vec::{Vec};
use std::fs::{File};
use std::io::{BufReader};
use num_traits::ToPrimitive;
use rodio::{Decoder, Source};
use svg::node::element::{
  SVG,
  Path,
};
use svg::node::element;

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

// calculate a path of a node's normalized audio waveform,
// returns an svg path and viewbox
pub fn calc_waveform_from_samples(
  samples: Vec<i16>, 
  channels: u16,
) -> Vec<f32> {
  // get frames and interpolate points
  let xs: Vec<i32> = (0..samples.len())
    .into_iter()
    .map(|x| x as i32)
    .collect();
  let ys: Vec<i32> = samples
    .iter()
    .map(|y| *y as i32)
    .collect();
  let y_smoothed = util::math::gaussian_1d(&ys).unwrap();
  let y_norms = util::math::f_normalize::<f32>(y_smoothed);

  let xsf: Vec<f32> = xs
    .iter()
    .map(|x| x.to_f32().unwrap())
    .collect();

  util::math::interleave(xsf, y_norms).unwrap()
}

// get a path of a node's normalized audio waveform, given
// the path to a sound sample
pub fn calc_waveform_from_file_path(sample_path: &str) -> Vec<f32> {
  let file = BufReader::new(File::open(sample_path).unwrap());
  let source = Decoder::new(file).unwrap();
  let channels = source.channels();
  let mut samples = Vec::<i16>::new();

  for sample in source {
    samples.push(sample);
  }

  calc_waveform_from_samples(samples, channels)
}
