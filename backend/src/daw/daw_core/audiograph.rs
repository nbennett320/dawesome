use crate::daw::SoundBuffer;
use crate::{
  daw,
  util,
};
use std::{thread};
use std::sync::{
  Arc, 
  Mutex,
};
use std::marker::{PhantomData};
use std::time::{
  Duration,
  Instant,
};
use std::vec::{Vec};
use std::fs::{File};
use std::io::{BufReader};
use futures;
use rodio::{
  Decoder, 
  OutputStream, 
  Sink
};
use rodio::source::{
  SamplesConverter, 
  Buffered, 
  Source,
  TakeDuration,
  SkipDuration,
};
use rodio::dynamic_mixer::{
  DynamicMixer,
  DynamicMixerController,
};

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
  pub start_offset: Duration,
  pub track_number: u32,
  sample_path: String,
  start_time: Option<Instant>,
  samples: Arc<Mutex<Buffered<Decoder<BufReader<File>>>>>,
  channels: u16,
  sink: Arc<Mutex<Sink>>,
  handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
  buffer: daw::sound_buffer::SoundBuffer,
  // buffer: Buffered<
  //   TakeDuration<TakeDuration<SkipDuration<Buffered<BufReader<File>>>>>
  // >,
  running: Arc<Mutex<bool>>,
  duration: Duration,
  sample_rate: u32,
  waveform: Vec<f32>,
}

// audio node implementation for Mac/Windows
#[cfg(not(target_os = "linux"))]
impl AudioNode {  
  pub fn new(
    id: u64,
    sample_path: String,
    start_offset: Duration,
    track_number: u32,
    sample_rate: u32,
  ) -> Self {
    let sound_buf = SoundBuffer::load(&sample_path).unwrap();

    let file_buf = BufReader::new(File::open(&sample_path).unwrap());
    let source = Decoder::new(file_buf).unwrap();
    let sample_buf = source.buffered();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let channels = sample_buf.channels();

    // todo: find a better way of calculating sample length
    // without reading the file buffer twice
    let file_buf_len = BufReader::new(File::open(&sample_path).unwrap());
    let source_len = Decoder::new(file_buf_len).unwrap();
    let mut samples = Vec::<i16>::new();
    for sample in source_len {
      samples.push(sample);
    }

    let dur_ms = (samples.len() as f32 / sample_rate as f32) * 1_000 as f32 / channels as f32;
    let duration = Duration::from_millis(dur_ms.round() as u64);
    println!("length of sample in ms: {:?}", dur_ms);
    println!("channels: {:?}", channels);
    println!("track number: {}", track_number);
    let waveform = daw::calc_waveform_from_samples(samples, channels);


    AudioNode {
      id,
      sample_path,
      start_time: None,
      start_offset,
      track_number,
      samples: Arc::new(Mutex::from(sample_buf)),
      channels,
      sink: Arc::new(Mutex::from(sink)),
      handle: Arc::new(Mutex::from(None)),
      buffer: sound_buf,
      // buffer: Buffered<
      //   TakeDuration<TakeDuration<SkipDuration<Buffered<BufReader<File>>>>
      // >,
      running: Arc::new(Mutex::from(false)),
      duration,
      sample_rate,
      waveform,
    }
  }

  pub fn buffer_sink(&mut self) {
    let sink = self.sink.lock().unwrap();
    let samples = self.samples.lock().unwrap().to_owned();
    sink.pause();
    sink.append(samples);
    println!("buffered samples for: {}", self.sample_path);
  }

  // play the audio node
  pub async fn play(self) {
    println!("playing sample ({})", self.sample_path);
    thread::spawn(move || {
      if *self.running.lock().unwrap() {
        println!("node is running ({})", self.sample_path);
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        stream_handle.play_raw(self.buffer.decoder().convert_samples()).unwrap();
        thread::sleep(self.duration);

        *self.running.lock().unwrap() = false;
      }
    });
  }

  pub fn stop(self) {

  }

  pub fn duration(&self) -> Duration {
    self.duration
  }
  
  // pub fn get_samples(self) -> Arc<Mutex<SamplesConverter<Decoder<BufReader<File>>, f32>>> {
  //   self.samples
  // }

  // calculate and set a node's (real) start time in the playlist
  pub fn set_start_time(&mut self, start_time: Instant) {
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

  pub fn toggle_running(&mut self) {
    let val = !*self.running.lock().unwrap();
    *self.running.lock().unwrap() = val;
  }

  pub fn get_waveform(&mut self) -> &mut Vec<f32> {
    &mut self.waveform
  }
}

// implemement node destructor
impl Drop for AudioNode {
  fn drop(&mut self) {
    let sink = &*self.sink.lock().unwrap();
    drop(sink);
  }
}

pub struct AudioGraph<'a> {
  pub nodes: Vec<AudioNode>,
  pub running: bool,
  pub started_time: Option<Instant>,
  pub current_offset: Option<Duration>,
  // stream_handle: Arc<Mutex<OutputStream>>,
  controller: Arc<DynamicMixerController<f32>>,
  mixer: DynamicMixer<f32>,
  sample_rate: u32,
  tempo: f32,
  max_beats: u64,
  _phantom: PhantomData<&'a str>,
}

impl AudioGraph<'static> {
  pub fn new(
    sample_rate: u32, 
    tempo: f32,
    max_beats: u64,
  ) -> Self {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let (controller, mixer) = rodio::dynamic_mixer::mixer(2, sample_rate);

    AudioGraph {
      nodes: Vec::<AudioNode>::new(),
      running: false,
      started_time: None,
      current_offset: Some(Duration::ZERO),
      // stream_handle: Arc::new(Mutex::from(stream_handle)),
      controller,
      mixer,
      sample_rate,
      tempo,
      max_beats,
      _phantom: PhantomData,
    }
  }

  // initialize the audiograph with a 
  // start time (from the playlist)
  pub fn init(&mut self, start_time: Instant) {
    if self.nodes.len() == 0 { return; }

    for node in self.nodes.as_mut_slice() {
      node.set_start_time(start_time);
    }

    self.started_time = Some(start_time);
    self.running = true;
  }

  // pub fn play_node(&self, node: &AudioNode) {
  //   self.controller.add(node.samples.lock().unwrap());
  // }

  // run graph and schedule nodes to be played
  // n milliseconds in advance
  pub fn run_for(&self, dur: Duration) {
    if self.nodes.len() == 0 {
      println!("Tried to run audio graph. No nodes in graph!");
      return;
    }
    
    // get starting index of nodes within this time slice
    let idx_start_opt = self.nodes.iter()
      .position(|node| 
          node.start_offset >= self.current_offset.unwrap() &&
          node.start_offset < (self.current_offset.unwrap() + dur));

    if idx_start_opt == None {
      println!("No start index!");
      return;
    }

    let idx_start = idx_start_opt.unwrap();

    // get last index of nodes within this time slice
    let idx_end_opt = self.nodes.iter()
      .rposition(|node| node.start_offset > (self.current_offset.unwrap() + dur));
    
    let idx_end = match idx_end_opt {
      Some(x) => x,
      None => idx_start + 1,
    };
    
    println!("start, end: {}, {}", idx_start, idx_end);

    let self_arc = Arc::new(Mutex::new(self));
    // schedule samples within this timeslice to play
    let slice = self_arc.lock().unwrap().nodes[idx_start..idx_end].to_vec();
    println!("slice len: {}, self.nodes len: {}",slice.len(), self.nodes.len());

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    
    let source: Sink

    // run  on each node in time slice
    for mut node in slice {
      let start_offset = Arc::new(Mutex::new(node.start_offset));
      let current_offset = Arc::new(Mutex::new(self_arc.lock().unwrap().current_offset));
      let running = Arc::new(Mutex::new(self_arc.lock().unwrap().running));

      self.controller.add(node.buffer.decoder().convert_samples());

    }
    thread::sleep(dur);

    // for mut node in slice {
    //   println!("node.start_offset: {}ms", node.start_offset.as_millis());
    //   let start_offset = Arc::new(Mutex::new(node.start_offset));
    //   let current_offset = Arc::new(Mutex::new(self_arc.lock().unwrap().current_offset));
    //   let running = Arc::new(Mutex::new(self_arc.lock().unwrap().running));

    //   // run time slice
    //   thread::spawn(move || {
    //     // calculate time until sample is played
    //     let dur = *start_offset.lock().unwrap() - current_offset.lock().unwrap().unwrap();

    //     // prepare the sink to be played
    //     // node.buffer_sink();
        
    //     // sleep this thread until it's time to play the sample, then play it
    //     thread::sleep(dur);
    //     if *running.lock().unwrap() {
    //       node.toggle_running();
          
    //       futures::executor::block_on(node.play());
    //       // node.toggle_running();
    //     }
    //   });
    // }
  }

  pub fn set_tempo(&mut self, tempo: f32) {
    self.tempo = tempo;
  }

  pub fn tempo(&self) -> f32 {
    self.tempo
  }

  pub fn set_max_beats(&mut self, max_beats: u64) {
    self.max_beats = max_beats;
  }

  pub fn max_beats(&self) -> u64 {
    self.max_beats
  }

  pub fn beat_interval(&self) -> Duration {
    let beats_per_sec = self.tempo / 60. / 4.;
    let dur = Duration::from_secs_f32(beats_per_sec);

    dur
  }

  pub fn interval_of_subdivision<T>(
    &self, 
    note: T
  ) -> Duration 
  where T: daw::timing::MusicalTiming {
    let (_, subdivision) = note.ratio();

    self.beat_interval() / subdivision
  }

  // add nodes to the audio graph and sort the graph by
  // offset start time
  pub fn add_node(&mut self, node: AudioNode) {
    self.nodes.push(node);
    self.nodes.sort_by(|a, b| a.start_offset.cmp(&b.start_offset));
  }

  // construct an audio node from a sample path and start offset,
  // and add it to the audio graph
  // returns the id of the constructed node
  pub fn construct_and_add_node(
    &mut self,
    sample_path: String,
    start_offset: Duration,
    track_number: u32,
  ) -> u64 {
    let id = self.nodes.len().clone().try_into().unwrap();
    let node = AudioNode::new(
      id,
      sample_path,
      start_offset,
      track_number,
      self.sample_rate,
    );

    self.add_node(node);

    id
  }

  // construct an audio node from a sample path and start offset,
  // and add it to the audio graph
  // returns the id of the constructed node
  pub fn construct_and_add_node_with_snap<T: daw::timing::MusicalTiming>(
    &mut self,
    sample_path: String,
    start_offset: Duration,
    track_number: u32,
    subdivision: T
  ) -> u64 {
    let id = self.nodes.len().clone().try_into().unwrap();

    let snapped_offset = self
      .find_nearest_offset_subdivision(start_offset, subdivision);
    
    println!("added to snapped offset: {}ms", snapped_offset.as_millis());

    let node = AudioNode::new(
      id,
      sample_path,
      snapped_offset,
      track_number,
      self.sample_rate,
    );

    self.add_node(node);

    id
  }

  // move node in graph by node id
  pub fn move_node(
    &mut self,
    id: u64,
    start_offset: Duration,
    track_number: u32
  ) {
    let mut node = self.get_mut_node(id);
    node.start_offset = start_offset;
    node.track_number = track_number;

    self.nodes.sort_by(|a, b| a.start_offset.cmp(&b.start_offset));
  }

  // move node in graph by id, and snap to nearest subdivision
  pub fn move_node_with_snap<T: daw::timing::MusicalTiming>(
    &mut self,
    id: u64,
    start_offset: Duration,
    track_number: u32,
    subdivision: T
  ) {
    let snapped_offset = self
      .find_nearest_offset_subdivision(start_offset, subdivision);
    
    println!("moved to snapped offset: {}ms", snapped_offset.as_millis());

    self.move_node(id, snapped_offset, track_number);
  }

  // remove node from graph with provided id
  // panics if id does not exist
  pub fn remove_node(&mut self, id: u64) -> AudioNode {
    let idx = self.nodes.iter().position(|a| a.id == id);
    let node = self.nodes.remove(idx.unwrap());

    node
  }

  // stop playback of all nodes and clear start times
  pub fn pause(&mut self) {
    for node in self.nodes.as_mut_slice() {
      if *node.running.lock().unwrap() {
        *node.running.lock().unwrap() = false;
      }

      node.clear_start_time();
    }

    self.current_offset = Some(Duration::ZERO);
    self.running = false;
  }

  // set offset of audiograph 
  pub fn set_current_offset(&mut self, offset: Option<Duration>) {
    self.current_offset = offset;
  }

  // get the length from the start 
  // of the playlist to the last node in the graph, 
  // not including the length of node (but should in the future)
  // (does not include padding added by the playlist)
  pub fn duration(self) -> Duration {
    if self.nodes.len() == 0 { return Duration::from_millis(0); }
    self.nodes.last().unwrap().start_offset
  }

  pub fn duration_max(&self) -> Duration {
    self.beat_interval() * self.max_beats as u32
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
      let new_offset = (node.start_offset.as_millis() as f32 * ratio).round() as u64;
      let dur = Duration::from_millis(new_offset);
      node.set_start_time(self.started_time.unwrap() + dur);
    }
  }

  pub fn get_mut_node(&mut self, id: u64) -> &mut AudioNode {
    let idx = self.nodes.iter().position(|a| a.id == id);
    &mut self.nodes[idx.unwrap()]
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

  pub fn get_nth_beat_offset(&self, n: u64) -> Duration {
    let interval = self.beat_interval();
    
    if n <= 1 {
      return Duration::ZERO;
    }
    
    let n_offset = n - 1;
    interval * n_offset as u32
  }

  pub fn get_beat_offsets(&self) -> Vec<Duration> {
    let mut offsets = Vec::<Duration>::new();

    for i in 0..self.max_beats {
      offsets.push(self.get_nth_beat_offset(i));
    }

    offsets
  }

  pub fn find_nearest_offset_subdivision<T: daw::timing::MusicalTiming>(
    &self, 
    offset: Duration, 
    subdivision: T
  ) -> Duration {
    let interval = daw::timing::subdivision_interval_from_tempo(
      self.tempo,
      subdivision);

    let nearest_offset_millis = util::math::round_to_nearest_unsigned_multiple(
      offset.as_millis(),
      interval.as_millis());

    Duration::from_millis(nearest_offset_millis.try_into().unwrap())
  }
  
  // return the unique audio tracks numbers in the graph
  pub fn track_numbers(&self) -> Vec<u32> {
    let mut counted: Vec<u32> = Vec::new();

    for node in &self.nodes {
      if !counted.contains(&node.track_number) {
        counted.push(node.track_number);
      }
    }
    
    counted
  }

  // get a vector of AudioNodes with the specified track number
  pub fn track(&self, track_number: u32) -> Vec<&AudioNode> {
    let mut track_nodes: Vec<&AudioNode> = Vec::new();

    for node in &self.nodes {
      if node.track_number == track_number {
        track_nodes.push(node);
      }
    }

    track_nodes
  }

  // get a vector of all tracks in the audiograph
  pub fn tracks(&self) -> Vec<Vec<&AudioNode>> {
    let mut tracks: Vec<Vec<&AudioNode>> = Vec::new();

    for track_num in self.track_numbers() {
      let track = self.track(track_num);

      tracks.push(track)
    }

    tracks
  }
}
