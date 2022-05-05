use std::thread;
use std::sync;
use std::time;
use std::marker;
use futures;
use crate::daw;

#[derive(Clone, Debug)]
pub struct AudioNode {
  id: u64,
  sample_path: String,
  start_time: Option<u64>,
  start_offset: u64,
  handle: sync::Arc<sync::Mutex<Option<thread::JoinHandle<()>>>>,
  running: bool,
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
      start_offset,
      handle: sync::Arc::new(sync::Mutex::from(None)),
      running: false,
    }
  }

  // calculate and set a node's (real) start time in the playlist
  pub fn set_start_time(&mut self, start_time: u64) {
    self.start_time = Some(start_time + self.start_offset);
  }

  // reset node start time 
  pub fn clear_start_time(&mut self) {
    self.start_time = None;
  }

  pub fn set_handle(self, handle: thread::JoinHandle<()>) {
    *self.handle.lock().unwrap() = Some(handle);
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

      let handle = thread::spawn(move || {
        // calculate time until sample is played
        let dur = *start_offset.lock().unwrap() - *current_offset.lock().unwrap();
        
        // sleep this thread until it's time to play the sample, then play it
        thread::sleep(time::Duration::from_millis(dur));
        if *running.lock().unwrap() {
          futures::executor::block_on(daw::play_sample(&*sample_path.lock().unwrap()));
        }
      });

      node.set_handle(handle);
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
  pub fn fit_nodes_to_tempo(&mut self, new_tempo: f32, old_tempo: f32) {
    let ratio = old_tempo / new_tempo;

    println!("ratio: {}",ratio);

    for node in self.nodes.as_mut_slice() {
      let new_offset = (node.start_offset as f32 * ratio).round() as u64;
      node.set_start_time(new_offset);
    }
  }
}
