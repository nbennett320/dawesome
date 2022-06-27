use std::vec::{Vec};
use std::fs::{File};
use std::io::{BufReader};
use rodio::{Decoder, Source};
use svg::node::element::{
  SVG,
  Path,
};
use svg::node::element;

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
) -> WaveformData {
  // get frames and interpolate points
  let xs: Vec<i32> = (0..samples.len())
    .into_iter()
    .map(|x| x as i32)
    .collect();
  let ys: Vec<i32> = samples
    .iter()
    .map(|y| *y as i32)
    .collect();
  // let interp = util::math::local_extremes(xs, ys);
  let interp = Some((xs as Vec<i32>, ys as Vec<i32>));
  let (x_interps, y_interps) = interp.unwrap();
  println!("interp lens: {:?}, {:?}", x_interps.len(), y_interps.len());

  // calculate bounding box
  let (min_x, min_y) = (0, -y_interps.iter().max().unwrap());
  let (max_x, max_y) = (
    x_interps.len() as i32 / channels as i32, 
    2 * (*y_interps.iter().max().unwrap() as i32));
  let svg = SVG::new()
    .set("viewBox", (min_x, min_y, max_x, max_y));

  // initialize data path
  let mut data = element::path::Data::new().move_to((0, 0));
  
  // fill path
  for idx in 1..y_interps.len() {
    let p1 = (idx, y_interps[idx]);
    data = data.line_to(p1)
  }

  // close the path
  data = data.close();

  // fetch calculated path value
  let path = Path::new()
    .set("stroke", "black")
    .set("stroke-width", "0.05%")
    .set("fill", "black")
    .set("d", data);

  let pathd = path.get_inner().get_attributes().get("d").unwrap();
  let viewbox = svg.get_inner().get_attributes().get("viewBox").unwrap();

  WaveformData::from(pathd.to_string(), viewbox.to_string())
}

// get a path of a node's normalized audio waveform, given
// the path to a sound sample
pub fn calc_waveform_from_file_path(sample_path: &str) -> WaveformData {
  let file = BufReader::new(File::open(sample_path).unwrap());
  let source = Decoder::new(file).unwrap();
  let channels = source.channels();
  let mut samples = Vec::<i16>::new();

  for sample in source {
    samples.push(sample);
  }

  calc_waveform_from_samples(samples, channels)
}
