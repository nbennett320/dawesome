use std::vec::{Vec};
use std::fs::{
  self, 
  DirEntry,
};

pub fn get_sample_browser_root() -> (Vec<String>, Vec<String>) {
  let entries: Vec<DirEntry> = fs::read_dir("./assets/")
    .unwrap()
    .map(|e| { e.unwrap() })
    .collect();
  let mut dirs: Vec<String> = Vec::new();
  let mut samples: Vec<String> = Vec::new();

  for entry in entries {
    if entry.path().is_dir() {
      let val = String::from(entry.path().to_str().unwrap());
      dirs.push(val);
    } else {
      let val = String::from(entry.path().file_name().unwrap().to_str().unwrap());
      samples.push(val);
    }
  }

  (samples, dirs)
}
