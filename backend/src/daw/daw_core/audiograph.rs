use std::thread;
use std::time;
use std::marker;
use futures;
use crate::daw;

#[derive(Clone)]
pub struct AudioNode {
  id: u64,
  sample_path: String,
  start_time: Option<u64>,
  start_offset: u64
}

impl AudioNode {
  pub fn new(
    id: u64,
    sample_path: String,
    start_offset: u64
  ) -> Self {
    AudioNode {
      id,
      sample_path,
      start_time: None,
      start_offset
    }
  }

  // calculate and set a node's (real) start time in the playlist
  pub fn set_start_time(&mut self, start_time: u64) {
    self.start_time = Some(start_time + self.start_offset);
  }

  pub fn clear_start_time(&mut self) {
    self.start_time = None;
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
      .position(|node| node.start_offset >= self.current_offset);

    if idx_start_opt == None {
      return;
    }

    let idx_start = idx_start_opt.unwrap();

    // get last index of nodes within this time slice
    let idx_end = self.nodes.iter().rev()
      .position(|node| (node.start_offset + time_ms) > self.current_offset).unwrap() + 1;

    let self_arc = std::sync::Arc::new(std::sync::Mutex::new(self));

    // schedule samples within this timeslice to play
    let slice = self_arc.lock().unwrap().nodes[idx_start..idx_end].to_vec();
    println!("len: {}, {}",slice.len(), self.nodes.len());
    for node in slice {
      println!("node.start_offset: {}", node.start_offset);
      let sample_path = std::sync::Arc::new(std::sync::Mutex::new(node.sample_path));
      let start_offset = std::sync::Arc::new(std::sync::Mutex::new(node.start_offset));
      let current_offset = std::sync::Arc::new(std::sync::Mutex::new(self_arc.lock().unwrap().current_offset));
      // let current_offset = current_offset.load(std::sync::atomic::Ordering::SeqCst).clone();

      thread::spawn(move || {
        // calculate time until sample is played
        let dur = *start_offset.lock().unwrap() - *current_offset.lock().unwrap();
        
        // sleep this thread until it's time to play the sample, then play it
        thread::sleep(time::Duration::from_millis(dur));
        futures::executor::block_on(daw::play_sample(&*sample_path.lock().unwrap()));
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
    start_offset: u64
  ) -> u64 {
    let id = self.nodes.len().clone().try_into().unwrap();
    let node = AudioNode::new(
      id,
      sample_path,
      start_offset
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

  // clear all start times
  pub fn pause(&mut self) {
    for node in self.nodes.as_mut_slice() {
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
}
