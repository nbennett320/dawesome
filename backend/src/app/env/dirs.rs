use std::vec;
use std::fs;

pub fn get_sidebar_samples() -> vec::Vec<String> {
  let paths = fs::read_dir("./assets/").unwrap();
  let mut samples: vec::Vec<String> = vec::Vec::new();

  for path in paths {
    let str = String::from(path.unwrap().path().to_str().unwrap());
    samples.push(str);
  }

  samples
}
