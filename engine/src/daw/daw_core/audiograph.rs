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
use num_traits::Float;
use rodio::{
  Decoder, 
  OutputStream, 
  Sink,
  Sample,
};
use rodio::source::{
  SamplesConverter, 
  Buffered, 
  Source,
  TakeDuration,
  SkipDuration,
  Zero,
};
use rodio::dynamic_mixer::{
  DynamicMixer,
  DynamicMixerController,
};
use rodio::buffer::{SamplesBuffer};

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
  buffer: daw::sound_buffer::SampleBuffer,
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
  ) -> Self where Self: Sized {
    let sound_buf = daw::SampleBuffer::load(&sample_path).unwrap();

    let file_buf = BufReader::new(File::open(&sample_path).unwrap());
    let source = Decoder::new(file_buf).unwrap();
    let sample_buf = source.buffered();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let channels = sample_buf.channels();
    let samples: Vec<i16> = sample_buf.clone().collect();
    let dur_s = (samples.len() as f32 / sample_rate as f32) as f32 / channels as f32;
    let duration = Duration::from_secs_f32(dur_s);
    println!("length of sample in ms: {:?}", duration.as_millis());
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
    let samples = self.buffer.decoder().convert_samples::<f32>();
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
        let samples = self.buffer.decoder().convert_samples::<f32>();
        // stream_handle.play_raw(self.buffer.decoder().convert_samples()).unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
    sink.pause();
    sink.append(samples);

        if !sink.empty() {
          sink.play();
          sink.sleep_until_end();
        }

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

// impl Iterator for AudioNode {
//   type Item = dyn Sample;

//   fn next(&mut self) -> Option<Self::Item> {
//     self.samples.lock().unwrap().next()
//   }
// }

// impl Source for AudioNode {
//   fn channels(&self) -> u16 {
//     self.channels
//   }

//   fn current_frame_len(&self) -> Option<usize> {
//     if self.duration.is_zero() {
//       return Some(0)
//     }

//     let res = self.duration.as_secs_f64() * self.sample_rate as f64;

//     Some(res.round().into())
//   }

//   fn sample_rate(&self) -> u32 {
//     self.sample_rate
//   }

//   fn total_duration(&self) -> Option<Duration> {
//     Some(self.duration)
//   }
// }

// implemement node destructor
impl Drop for AudioNode {
  fn drop(&mut self) {
    let sink = &*self.sink.lock().unwrap();
    drop(sink);
  }
}

pub struct Track {
  pub name: String,
  pub idx: usize,
  pub nodes: Vec<AudioNode>,
}

impl Track {
  pub fn new(name: String, idx: usize) -> Self {
    Track {
      name,
      idx,
      nodes: Vec::<AudioNode>::new(),
    }
  }

  pub fn sort(&mut self) {
    self.nodes.sort_by(|a, b| a.start_offset.cmp(&b.start_offset));
  }

  pub fn buffer_from(&self, start_offset: Duration) {

  }
}

pub struct AudioGraph<'a> {
  pub nodes: Vec<AudioNode>,
  pub running: bool,
  pub started_time: Option<Instant>,
  pub current_offset: Option<Duration>,
  controller: Arc<DynamicMixerController<f32>>,
  mixer: Arc<Mutex<DynamicMixer<f32>>>,
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
    let (controller, mixer) = rodio::dynamic_mixer::mixer(2, sample_rate);

    AudioGraph {
      nodes: Vec::<AudioNode>::new(),
      running: false,
      started_time: None,
      current_offset: Some(Duration::ZERO),
      controller,
      mixer: Arc::new(Mutex::from(mixer)),
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

    let (_controller, mixer) = self.buffer_slice(dur).unwrap();

    let running = self.running;
    thread::spawn(move || {
      if running {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.pause();
        sink.append(mixer);
        println!("System time (PB): {:?}", std::time::Instant::now());
        sink.play();
        sink.sleep_until_end();
        
        // sink.detach();
        // stream_handle.play_raw(mixer).unwrap();
        // thread::sleep(dur);
      }
    });
  }

  pub fn buffer_slice(&self, dur: Duration) -> Option<(Arc<DynamicMixerController<f32>>, DynamicMixer<f32>)> {
    let silence = rodio::source::Zero::<f32>::new(2, self.sample_rate).take_duration(dur);
    let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(2, self.sample_rate);

    controller.add(silence);
    
    // todo: add conditional for metronome state
    // controller.add(daw::METRONOME_TICK_SOURCE.convert_samples());
    
    // get starting index of nodes within this time slice
    let idx_start_opt = self.nodes.iter()
      .position(|node| 
        node.start_offset >= self.current_offset.unwrap() &&
        node.start_offset < (self.current_offset.unwrap() + dur));

    if idx_start_opt == None {
      println!("No start index! Could not buffer.");
      return Some((controller, mixer));
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


    println!("items (len: {}) in nodes[{}..{}]:", self.nodes.len(), idx_start, idx_end);
    for node in &slice {
      println!("path: {}", node.sample_path);
    }

    for node in slice {
      println!("node.start_offset: {}ms", node.start_offset.as_millis());
      let start_offset = Arc::new(Mutex::new(node.start_offset));
      let current_offset = Arc::new(Mutex::new(self_arc.lock().unwrap().current_offset));

      let delay_offset = *start_offset.lock().unwrap() - current_offset.lock().unwrap().unwrap();
      println!("delay offset: {}ms", delay_offset.as_millis());
      let source = node.buffer.decoder().delay(delay_offset).convert_samples();
      controller.add(source);
      println!("added node: {}", node.sample_path);
    }

    Some((controller, mixer))
  }

  pub fn buffer_for(&self, dur: Duration) -> Option<DynamicMixer<f32>> {
    let frames_per_slice = (dur.as_secs_f64() * self.sample_rate as f64).round() as i32;

    if self.nodes.len() == 0 {
      println!("No nodes in graph! Filling with {} frames of silence.", frames_per_slice);

      let (_stream, stream_handle) = OutputStream::try_default().unwrap();
      
      let silence = rodio::source::Zero::<f32>::new(2, self.sample_rate).take_duration(dur);
      let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(2, self.sample_rate);
      controller.add(silence);
      return Some(mixer);
    }

    // get starting index of nodes within this time slice
    let idx_start_opt = self.nodes.iter()
      .position(|node| 
          node.start_offset >= self.current_offset.unwrap() &&
          node.start_offset < (self.current_offset.unwrap() + dur));

    if idx_start_opt == None {
      println!("No start index!");
      return None;
    }

    let idx_start = idx_start_opt.unwrap();

    // get last index of nodes within this time slice
    let idx_end_opt = self.nodes.iter()
      .rposition(|node| node.start_offset > (self.current_offset.unwrap() + dur));
    
    let idx_end = match idx_end_opt {
      Some(x) => x,
      None => idx_start + 1,
    };
    
    println!("buffer start, end: {}, {}", idx_start, idx_end);

    let self_arc = Arc::new(Mutex::new(self));
    // schedule samples within this timeslice to play
    let slice = self_arc.lock().unwrap().nodes[idx_start..idx_end].to_vec();
    println!("slice len: {}, self.nodes len: {}",slice.len(), self.nodes.len());

    // let root = rodio::source::Zero::<f32>::new(2, 44_100).take_duration(dur);
    let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(2, self.sample_rate);

    println!("items (len: {}) in nodes[{}..{}]:", self.nodes.len(), idx_start, idx_end);
    for node in slice {
      println!("node.start_offset: {}ms", node.start_offset.as_millis());
      let start_offset = Arc::new(Mutex::new(node.start_offset));
      let current_offset = Arc::new(Mutex::new(self_arc.lock().unwrap().current_offset));

      let delay_offset = *start_offset.lock().unwrap() - current_offset.lock().unwrap().unwrap();
      println!("delay offset: {}ms", delay_offset.as_millis());
      let source = node.buffer.decoder().delay(delay_offset).convert_samples();
      controller.add(source);
      println!("added node: {}", node.sample_path);
    }

    Some(mixer)
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
    let node = self.get_mut_node(id).unwrap();
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
    let last: &AudioNode = self.nodes.last().unwrap();

    last.start_offset + last.duration()
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

  pub fn get_mut_node(&mut self, id: u64) -> Option<&mut AudioNode> {
    self.nodes.iter_mut().find(|e| { e.id == id })
  }

  // get the id of all nodes in a given playlist track
  pub fn get_node_ids_in_playlist_track(
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::daw;
  use futures_test::{self};

  #[test]
  fn test_node_initialization() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(0);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate);

    assert_eq!(node.start_offset, Duration::from_millis(0));
    assert_eq!(node.duration().as_millis(), daw::METRONOME_TICK_SOURCE.convert_samples().total_duration().unwrap().as_millis());
    assert_eq!(node.channels, daw::METRONOME_TICK_SOURCE.convert_samples().channels());
    assert_eq!(node.sample_path, daw::METRONOME_TICK_PATH);
    assert_eq!(node.sample_rate, 44_100);
    assert_eq!(node.track_number, track_number);
  }

  #[futures_test::test]
  async fn test_node_play() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(0);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate);

    let now = Instant::now();

    node.play().await;

    thread::sleep(Duration::from_millis(100));

    let later = Instant::now();

    assert_ne!(now.elapsed().as_millis(), later.elapsed().as_millis());
  }

  #[test]
  fn test_audiograph_initialization() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(400);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate);
    let node_dur = node.duration().clone();

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);

    assert_eq!(audiograph.len(), 0);

    audiograph.add_node(node);
    
    assert_eq!(audiograph.len(), 1);
    assert_eq!(audiograph.nodes[0].id, 1);
    assert_eq!(audiograph.duration(), start_offset + node_dur);
  }

  #[futures_test::test]
  async fn test_audiograph_run() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(650);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate);

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);

    audiograph.add_node(node);
    
    let runtime = Duration::from_secs(1);
    let now = Instant::now();

    audiograph.run_for(runtime);
    thread::sleep(runtime);

    let now_elapsed = now.elapsed();
    let later = Instant::now();

    assert_ne!(now_elapsed.as_millis(), later.elapsed().as_millis());

    let diff = now_elapsed - runtime;
    let result = diff.as_nanos() >= 0;

    assert_eq!(result, true);
  }

  #[futures_test::test]
  async fn test_audiograph_buffer_slice_with_silence() {
    let sample_rate = 44_100;
    let tempo = 120f32;
    let max_beats: u64 = 8;

    let audiograph = AudioGraph::new(sample_rate, tempo, max_beats);
    let runtime = Duration::from_secs(1);

    let (_controller, mixer) = audiograph.buffer_slice(runtime).unwrap();

    let mut zero_samples = Vec::<f32>::new();
    let mut nonzero_samples = Vec::<f32>::new();
    mixer.for_each(|x| {
      if x == 0f32 {
        zero_samples.push(x);
      } else {
        nonzero_samples.push(x);
      }
    });

    assert_eq!(zero_samples.len(), 44_100 / 2);
    assert_eq!(nonzero_samples.len(), 0);
  }

  #[futures_test::test]
  async fn test_audiograph_buffer_slice_with_node() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(650);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate);

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);
    let buf = node.buffer.clone();

    audiograph.add_node(node);
    
    let runtime = Duration::from_secs(1);

    let (_controller, mixer) = audiograph.buffer_slice(runtime).unwrap();

    let mut nonzero_samples = Vec::<f32>::new();
    mixer.for_each(|x| {
      if x != 0f32 {
        nonzero_samples.push(x);
      }
    });

    let mut node_samples = Vec::<f32>::new();
    buf.convert_samples().for_each(|x| {
      if x != 0f32 {
        node_samples.push(x);
      }
    });

    assert_eq!(node_samples.len() * 2, nonzero_samples.len());
  }

  #[futures_test::test]
  async fn test_audiograph_buffer_set_functions() {
    let sample_rate = 44_100;
    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);

    assert_eq!(audiograph.current_offset.unwrap().as_millis(), 0);
    audiograph.set_current_offset(Some(Duration::from_secs(2)));
    assert_eq!(audiograph.current_offset.unwrap().as_millis(), 2_000);

    assert_eq!(audiograph.max_beats(), max_beats);
    audiograph.set_max_beats(500);
    assert_eq!(audiograph.max_beats(), 500);

    assert_eq!(audiograph.tempo(), tempo);
    audiograph.set_tempo(75f32);
    assert_eq!(audiograph.tempo(), 75f32);
  }

  #[futures_test::test]
  async fn test_audiograph_node_functions() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(350);
    let sample_rate = 44_100;

    let node = AudioNode::new(
      id,
      sample_path,
      start_offset,
      0,
      sample_rate);

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);
    let buf = node.buffer.clone();

    audiograph.add_node(node);
    audiograph.construct_and_add_node(
      "assets/assets_66-bd-01.wav".to_string(),
      Duration::from_millis(100),
      1);
    audiograph.construct_and_add_node_with_snap(
      "assets/assets_66-sd-01.wav".to_string(),
      Duration::from_millis(600),
      1,
      daw::timing::QuarterNote::new());
    
    assert_eq!(audiograph.len(), 3);
    
    let target_id = audiograph.get_mut_node(1).unwrap().id;
    audiograph.move_node(
      target_id, 
      Duration::from_millis(0),
      2);
    let target = &audiograph.nodes[0];

    assert_eq!(target.id, target_id);
    assert_eq!(target.track_number, 2);
    assert_eq!(target.start_offset.as_millis(), 0);

    let runtime = Duration::from_secs(1);

    let (_controller, mixer) = audiograph.buffer_slice(runtime).unwrap();

    let mut nonzero_samples = Vec::<f32>::new();
    mixer.for_each(|x| {
      if x != 0f32 {
        nonzero_samples.push(x);
      }
    });

    let mut node_samples = Vec::<f32>::new();
    buf.convert_samples().for_each(|x| {
      if x != 0f32 {
        node_samples.push(x);
      }
    });
  }
}
