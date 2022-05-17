use std::thread;
use std::sync;
use std::time;
use std::marker;
use std::vec::Vec;
use std::fs::File;
use std::io::BufReader;
use futures;
use rodio::{Decoder, OutputStream, Sink};
use rodio::queue::{SourcesQueueOutput};
use svg::node::element;
use crate::daw;
use crate::util;

#[cfg(target_os = "linux")]
use psimple;
#[cfg(target_os = "linux")]
use pulse;

#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct AudioNode {
  pub id: u64,
  sample_path: String,
  start_time: Option<u64>,
  start_offset: u64,
  track_number: u32,
  raw_source: sync::Arc<sync::Mutex<Vec<i16>>>,
  sink: sync::Arc<sync::Mutex<Sink>>,
  handle: sync::Arc<sync::Mutex<Option<thread::JoinHandle<()>>>>,
  running: bool,
}

// audio node implementation for Linux
#[cfg(target_os = "linux")]
impl AudioNode {  
  pub fn new(
    id: u64,
    sample_path: String,
    start_offset: u64,
    track_number: u32,
  ) -> Self {
    let file_buf = BufReader::new(File::open(&sample_path).unwrap());
    let source = Decoder::new(file_buf).unwrap();

    let spec = pulse::sample::Spec {
      format: pulse::sample::Format::S16NE,
      channels: 2,
      rate: 44_100,
    };


    let sink = psimple::Simple::new(
      None,
      "dawesome",
      pulse::stream::Direction::Playback,
      None,
      "dawesome output",
      &spec,
      None,
      None,
    ).unwrap();

    let raw_source: rodio::source::SamplesConverter<_, i16> = source.convert_samples();
    let raw_source: std::vec::Vec<i16> = raw_source.collect();

    AudioNode {
      id,
      sample_path,
      start_time: None,
      start_offset,
      track_number,
      raw_source,
      sink: sync::Arc::new(sync::Mutex::from(sink)),
      handle: sync::Arc::new(sync::Mutex::from(None)),
      running: false,
    }
  }

  // play the audio node
  pub fn play(self) {
    let handle = thread::spawn(move || {
      let sink = self.sink.lock().unwrap();
      let raw_source = self.raw_source.lock().unwrap();

      unsafe {
        let raw_slice = raw_source.align_to::<u8>().1;
        sink.write(raw_slice).unwrap();
        sink.drain().unwrap();
      }
    });

    *self.handle.lock().unwrap() = Some(handle);
  }

  pub fn stop(self) {

  }

  // calculate and set a node's (real) start time in the playlist
  pub fn set_start_time(&mut self, start_time: u64) {
    self.start_time = Some(start_time + self.start_offset);
  }

  // reset node start time 
  pub fn clear_start_time(&mut self) {
    self.start_time = None;
  }

  // attach a thread handle to a node
  pub fn set_handle(self, handle: thread::JoinHandle<()>) {
    *self.handle.lock().unwrap() = Some(handle);
  }

  // get a path of a node's normalized audio waveform
  pub fn get_waveform(&self) -> (String, String) {
    let file = BufReader::new(File::open(&self.sample_path).unwrap());
    let source = Decoder::new(file).unwrap();
    let mut samples = std::vec::Vec::<i16>::new();

    println!("raw len: {:?}", source.size_hint());
    for sample in source {
      samples.push(sample);
    }

    // get frames and interpolate points
    let xs: Vec<i32> = (0..samples.len()).into_iter().map(|x| x as i32).collect();
    let ys: Vec<i32> = samples.iter().map(|y| *y as i32).collect();
    // let interp = util::math::local_extremes(xs, ys);
    let interp = Some((xs as Vec<i32>, ys as Vec<i32>));
    let (x_interps, y_interps) = interp.unwrap();
    println!("interp lens: {:?}, {:?}", x_interps.len(), y_interps.len());

    // calculate bounding box
    let (min_x, min_y) = (0, -y_interps.iter().max().unwrap());
    let (max_x, max_y) = (x_interps.len(), 2 * (*y_interps.iter().max().unwrap() as i32));
    let svg = element::SVG::new()
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
    let path = element::Path::new()
      .set("stroke", "black")
      .set("stroke-width", "0.05%")
      .set("fill", "black")
      .set("d", data);

    let pathd = path.get_inner().get_attributes().get("d").unwrap();
    let viewbox = svg.get_inner().get_attributes().get("viewBox").unwrap();

    (pathd.to_string(), viewbox.to_string())
  }
}

#[derive(Clone)]
#[cfg(not(target_os = "linux"))]
pub struct AudioNode {
  pub id: u64,
  sample_path: String,
  start_time: Option<u64>,
  start_offset: u64,
  track_number: u32,
  sink: sync::Arc<sync::Mutex<Sink>>,
  handle: sync::Arc<sync::Mutex<Option<thread::JoinHandle<()>>>>,
  running: bool,
}

// audio node implementation for Mac/Windows
#[cfg(not(target_os = "linux"))]
impl AudioNode {  
  pub fn new(
    id: u64,
    sample_path: String,
    start_offset: u64,
    track_number: u32,
  ) -> Self {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let file_buf = BufReader::new(File::open(&sample_path).unwrap());
    let source = Decoder::new(file_buf).unwrap();

    sink.append(source);
    sink.pause();

    AudioNode {
      id,
      sample_path,
      start_time: None,
      start_offset,
      track_number,
      sink: sync::Arc::new(sync::Mutex::from(sink)),
      handle: sync::Arc::new(sync::Mutex::from(None)),
      running: false,
    }
  }

  // play the audio node
  pub async fn play(self) {
    println!("going to play");
    thread::spawn(move || {
      let sink = self.sink.lock().unwrap();
      println!("playing for {}", sink.empty());
      
      sink.play();
      sink.sleep_until_end();
      println!("should have played");
    });

    // *self.handle.lock().unwrap() = Some(handle);
  }

  pub fn stop(self) {

  }

  // calculate and set a node's (real) start time in the playlist
  pub fn set_start_time(&mut self, start_time: u64) {
    self.start_time = Some(start_time + self.start_offset);
  }

  // reset node start time 
  pub fn clear_start_time(&mut self) {
    self.start_time = None;
  }

  // attach a thread handle to a node
  pub fn set_handle(self, handle: thread::JoinHandle<()>) {
    *self.handle.lock().unwrap() = Some(handle);
  }

  // get a path of a node's normalized audio waveform
  pub fn get_waveform(&self) -> (String, String) {
    let file = BufReader::new(File::open(&self.sample_path).unwrap());
    let source = Decoder::new(file).unwrap();
    let mut samples = std::vec::Vec::<i16>::new();

    println!("raw len: {:?}", source.size_hint());
    for sample in source {
      samples.push(sample);
    }

    // get frames and interpolate points
    let xs: Vec<i32> = (0..samples.len()).into_iter().map(|x| x as i32).collect();
    let ys: Vec<i32> = samples.iter().map(|y| *y as i32).collect();
    // let interp = util::math::local_extremes(xs, ys);
    let interp = Some((xs as Vec<i32>, ys as Vec<i32>));
    let (x_interps, y_interps) = interp.unwrap();
    println!("interp lens: {:?}, {:?}", x_interps.len(), y_interps.len());

    // calculate bounding box
    let (min_x, min_y) = (0, -y_interps.iter().max().unwrap());
    let (max_x, max_y) = (x_interps.len(), 2 * (*y_interps.iter().max().unwrap() as i32));
    let svg = element::SVG::new()
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
    let path = element::Path::new()
      .set("stroke", "black")
      .set("stroke-width", "0.05%")
      .set("fill", "black")
      .set("d", data);

    let pathd = path.get_inner().get_attributes().get("d").unwrap();
    let viewbox = svg.get_inner().get_attributes().get("viewBox").unwrap();

    (pathd.to_string(), viewbox.to_string())
  }
}

pub struct AudioGraph<'a> {
  pub nodes: std::vec::Vec<AudioNode>,
  pub running: bool,
  pub started_time: Option<u64>,
  pub current_offset: u64,
  _phantom: marker::PhantomData<&'a str>,
}

impl AudioGraph<'static> {
  pub fn new() -> Self {
    AudioGraph {
      nodes: std::vec::Vec::<AudioNode>::new(),
      running: false,
      started_time: None,
      current_offset: 0,
      _phantom: marker::PhantomData,
    }
  }

  // initialize the audiograph with a 
  // start time (from the playlist)
  pub fn init(&mut self, start_time: u64) {
    if self.nodes.len() == 0 { return; }

    for node in self.nodes.as_mut_slice() {
      node.set_start_time(start_time);
    }

    self.started_time = Some(start_time);
    self.running = true;
  }

  // run graph and schedule nodes to be played
  // n milliseconds in advance
  pub fn run_for(&self, time_ms: u64) {
    if self.nodes.len() == 0 {
      println!("Tried to run audio graph. No nodes in graph!");
      return;
    }
    
    // get starting index of nodes within this time slice
    let idx_start_opt = self.nodes.iter()
      .position(|node| 
          node.start_offset >= self.current_offset &&
          node.start_offset < (self.current_offset + time_ms));

    if idx_start_opt == None {
      println!("No start index!");
      return;
    }

    let idx_start = idx_start_opt.unwrap();

    // get last index of nodes within this time slice
    let idx_end_opt = self.nodes.iter()
      .rposition(|node| node.start_offset > (self.current_offset + time_ms));
    
    let idx_end = match idx_end_opt {
      Some(x) => x,
      None => idx_start,
    };
    
    println!("start, end: {}, {}", idx_start, idx_end);

    let self_arc = std::sync::Arc::new(std::sync::Mutex::new(self));

    // schedule samples within this timeslice to play
    let slice = self_arc.lock().unwrap().nodes[idx_start..idx_end].to_vec();
    println!("len: {}, {}",slice.len(), self.nodes.len());
    for node in slice {
      println!("node.start_offset: {}", node.start_offset);
      let sample_path = std::sync::Arc::new(std::sync::Mutex::new(Box::from(node.sample_path.as_ref())));
      let start_offset = std::sync::Arc::new(std::sync::Mutex::new(node.start_offset));
      let current_offset = std::sync::Arc::new(std::sync::Mutex::new(self_arc.lock().unwrap().current_offset));
      let running = std::sync::Arc::new(std::sync::Mutex::new(self_arc.lock().unwrap().running));

      // run time slice
      thread::spawn(move || {
        // calculate time until sample is played
        let dur = *start_offset.lock().unwrap() - *current_offset.lock().unwrap();
        
        // sleep this thread until it's time to play the sample, then play it
        thread::sleep(time::Duration::from_millis(dur));
        if *running.lock().unwrap() {
          futures::executor::block_on(daw::play_sample(&sample_path.lock().unwrap()));
        }
      });
    }
  }

  // add nodes to the audio graph and sort the graph by
  // offset start time
  pub fn add_node(&mut self, node: AudioNode) {
    self.nodes.push(node);
    self.nodes.sort_unstable_by(|a, b| a.start_offset.cmp(&b.start_offset));
  }

  // construct an audio node from a sample path and start offset,
  // and add it to the audio graph
  // returns the id of the constructed node
  pub fn construct_and_add_node(
    &mut self,
    sample_path: String,
    start_offset: u64,
    track_number: u32,
  ) -> u64 {
    let id = self.nodes.len().clone().try_into().unwrap();
    let node = AudioNode::new(
      id,
      sample_path,
      start_offset,
      track_number,
    );

    self.add_node(node);

    id
  }

  // remove node from graph with provided id
  // panics if id does not exist
  pub fn remove_node(&mut self, id: u64) {
    let idx = self.nodes.iter().position(|a| a.id == id);
    self.nodes.remove(idx.unwrap());
  }

  // stop playback of all nodes and clear start times
  pub fn pause(&mut self) {
    for node in self.nodes.as_mut_slice() {
      if node.running {
        node.running = false;
      }

      node.clear_start_time();
    }

    self.current_offset = 0;
    self.running = false;
  }

  // set offset of audiograph 
  pub fn set_current_offset(&mut self, offset: u64) {
    self.current_offset = offset;
  }

  // get the length in milliseconds from the start 
  // of the playlist to the last node in the graph, 
  // not including the length of node (but should in the future)
  // (does not include padding added by the playlist)
  pub fn len_real_in_ms(self) -> u64 {
    if self.nodes.len() == 0 { return 0; }
    self.nodes.last().unwrap().start_offset
  }

  // count number of nodes
  pub fn len(&self) -> usize {
    let res = self.nodes.len();
    res
  }
  
  // adjust all node start times to match tempo
  pub fn fit_nodes_to_tempo(
    &mut self, 
    new_tempo: f32, 
    old_tempo: f32
  ) {
    let ratio = old_tempo / new_tempo;

    println!("ratio: {}",ratio);

    for node in self.nodes.as_mut_slice() {
      let new_offset = (node.start_offset as f32 * ratio).round() as u64;
      node.set_start_time(new_offset);
    }
  }

  // get the id of all nodes in a given playlist track
  pub fn get_nodes_in_playlist_track(
    &mut self, 
    track_number: u32
  ) -> Vec<u64> {
    let mut nodes = Vec::<u64>::new();

    for node in self.nodes.as_mut_slice() {
      if node.track_number == track_number {
        let id = node.id;
        nodes.push(id);
      }
    }

    nodes
  }
}
