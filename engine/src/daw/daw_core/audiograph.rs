use crate::{
  daw,
  util,
};
use std::sync::{
  Arc, 
  Mutex,
};
use std::marker::{PhantomData};
use std::time::{Duration};
use std::vec::{Vec};
use std::fs::{File};
use std::io::{BufReader};
use rodio::{
  Decoder, 
  OutputStreamHandle,
};
use rodio::source::{Source};
use rodio::dynamic_mixer::{
  DynamicMixer,
  DynamicMixerController,
};

// TODO(linux): the Linux node below played through PulseAudio directly, and
// had fallen behind (it no longer compiled). it's kept as a reminder to
// get Linux builds working again, likely by dropping the cfg gates below,
// since cpal plays through ALSA on Linux
// #[cfg(target_os = "linux")]
// use psimple;
// #[cfg(target_os = "linux")]
// use pulse;
//
// #[cfg(target_os = "linux")]
// #[derive(Clone)]
// pub struct AudioNode {
//   pub id: u64,
//   sample_path: String,
//   start_time: Option<u64>,
//   start_offset: u64,
//   track_number: u32,
//   raw_source: sync::Arc<sync::Mutex<Vec<i16>>>,
//   sink: sync::Arc<sync::Mutex<Sink>>,
//   handle: sync::Arc<sync::Mutex<Option<thread::JoinHandle<()>>>>,
//   running: bool,
// }
//
// // audio node implementation for Linux
// #[cfg(target_os = "linux")]
// impl AudioNode {  
//   pub fn new(
//     id: u64,
//     sample_path: String,
//     start_offset: u64,
//     track_number: u32,
//   ) -> Self {
//     let file_buf = BufReader::new(File::open(&sample_path).unwrap());
//     let source = Decoder::new(file_buf).unwrap();
//
//     let spec = pulse::sample::Spec {
//       format: pulse::sample::Format::S16NE,
//       channels: 2,
//       rate: 44_100,
//     };
//
//
//     let sink = psimple::Simple::new(
//       None,
//       "dawesome",
//       pulse::stream::Direction::Playback,
//       None,
//       "dawesome output",
//       &spec,
//       None,
//       None,
//     ).unwrap();
//
//     let raw_source: rodio::source::SamplesConverter<_, i16> = source.convert_samples();
//     let raw_source: std::vec::Vec<i16> = raw_source.collect();
//
//     AudioNode {
//       id,
//       sample_path,
//       start_time: None,
//       start_offset,
//       track_number,
//       raw_source,
//       sink: sync::Arc::new(sync::Mutex::from(sink)),
//       handle: sync::Arc::new(sync::Mutex::from(None)),
//       running: false,
//     }
//   }
//
//   // play the audio node
//   pub fn play(self) {
//     let handle = thread::spawn(move || {
//       let sink = self.sink.lock().unwrap();
//       let raw_source = self.raw_source.lock().unwrap();
//
//       unsafe {
//         let raw_slice = raw_source.align_to::<u8>().1;
//         sink.write(raw_slice).unwrap();
//         sink.drain().unwrap();
//       }
//     });
//
//     *self.handle.lock().unwrap() = Some(handle);
//   }
//
//   pub fn stop(self) {
//
//   }
//
//   // calculate and set a node's (real) start time in the playlist
//   pub fn set_start_time(&mut self, start_time: u64) {
//     self.start_time = Some(start_time + self.start_offset);
//   }
//
//   // reset node start time 
//   pub fn clear_start_time(&mut self) {
//     self.start_time = None;
//   }
//
//   // attach a thread handle to a node
//   pub fn set_handle(self, handle: thread::JoinHandle<()>) {
//     *self.handle.lock().unwrap() = Some(handle);
//   }
//
//   // get a path of a node's normalized audio waveform
//   pub fn get_waveform(&self) -> (String, String) {
//     let file = BufReader::new(File::open(&self.sample_path).unwrap());
//     let source = Decoder::new(file).unwrap();
//     let mut samples = std::vec::Vec::<i16>::new();
//
//     println!("raw len: {:?}", source.size_hint());
//     for sample in source {
//       samples.push(sample);
//     }
//
//     // get frames and interpolate points
//     let xs: Vec<i32> = (0..samples.len()).into_iter().map(|x| x as i32).collect();
//     let ys: Vec<i32> = samples.iter().map(|y| *y as i32).collect();
//     // let interp = util::math::local_extremes(xs, ys);
//     let interp = Some((xs as Vec<i32>, ys as Vec<i32>));
//     let (x_interps, y_interps) = interp.unwrap();
//     println!("interp lens: {:?}, {:?}", x_interps.len(), y_interps.len());
//
//     // calculate bounding box
//     let (min_x, min_y) = (0, -y_interps.iter().max().unwrap());
//     let (max_x, max_y) = (x_interps.len(), 2 * (*y_interps.iter().max().unwrap() as i32));
//     let svg = element::SVG::new()
//       .set("viewBox", (min_x, min_y, max_x, max_y));
//
//     // initialize data path
//     let mut data = element::path::Data::new().move_to((0, 0));
//
//     // fill path
//     for idx in 1..y_interps.len() {
//       let p1 = (idx, y_interps[idx]);
//       data = data.line_to(p1)
//     }
//
//     // close the path
//     data = data.close();
//
//     // fetch calculated path value
//     let path = element::Path::new()
//       .set("stroke", "black")
//       .set("stroke-width", "0.05%")
//       .set("fill", "black")
//       .set("d", data);
//
//     let pathd = path.get_inner().get_attributes().get("d").unwrap();
//     let viewbox = svg.get_inner().get_attributes().get("viewBox").unwrap();
//
//     (pathd.to_string(), viewbox.to_string())
//   }
// }

#[derive(Clone)]
#[cfg(not(target_os = "linux"))]
pub struct AudioNode {
  pub id: u64,
  pub start_offset: Duration,
  pub track_number: u32,
  sample_path: String,
  channels: u16,
  buffer: daw::sound_buffer::SampleBuffer,
  // the sample's length, before the clip's settings change its speed
  duration: Duration,
  sample_rate: u32,
  waveform: Vec<f32>,
  // how the clip is played back
  settings: daw::ClipSettings,
}

// a sample's length and the waveform drawn for it
// - samples: interleaved samples for all channels
fn duration_and_waveform(
  sample_path: &str,
  samples: Vec<i16>,
  channels: u16,
  sample_rate: u32,
) -> Result<(Duration, Vec<f32>), String> {
  if samples.is_empty() || channels == 0 {
    return Err(format!("sample '{}' contains no audio", sample_path));
  }
  let dur_s = (samples.len() as f32 / sample_rate as f32) as f32 / channels as f32;
  let duration = Duration::from_secs_f32(dur_s);
  let waveform = daw::calc_waveform_from_samples(samples, channels)
    .ok_or_else(|| format!("sample '{}' is too short to draw a waveform", sample_path))?;

  Ok((duration, waveform))
}

// decode a sample's length and waveform without creating a node,
// so the frontend can draw a clip before it's dropped into the playlist
pub fn preview_sample(
  sample_path: &str,
  sample_rate: u32,
) -> Result<(Duration, Vec<f32>), String> {
  let file_buf = BufReader::new(File::open(sample_path)
    .map_err(|e| format!("could not open sample '{}': {}", sample_path, e))?);
  let source = Decoder::new(file_buf)
    .map_err(|e| format!("could not decode sample '{}': {}", sample_path, e))?;
  let channels = source.channels();
  let samples: Vec<i16> = source.collect();

  duration_and_waveform(sample_path, samples, channels, sample_rate)
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
  ) -> Result<Self, String> {
    let err = |what: &str, e: &dyn std::fmt::Display| {
      format!("{} '{}': {}", what, sample_path, e)
    };

    let sound_buf = daw::SampleBuffer::load(&sample_path)
      .map_err(|e| err("could not read sample", &e))?;

    let file_buf = BufReader::new(
      File::open(&sample_path).map_err(|e| err("could not open sample", &e))?);
    let source = Decoder::new(file_buf)
      .map_err(|e| err("could not decode sample", &e))?;
    let channels = source.channels();
    let samples: Vec<i16> = source.collect();
    let (duration, waveform) =
      duration_and_waveform(&sample_path, samples, channels, sample_rate)?;
    println!("length of sample in ms: {:?}", duration.as_millis());
    println!("channels: {:?}", channels);
    println!("track number: {}", track_number);

    Ok(AudioNode {
      id,
      sample_path,
      start_offset,
      track_number,
      channels,
      buffer: sound_buf,
      duration,
      sample_rate,
      waveform,
      settings: daw::ClipSettings::default(),
    })
  }

  // how long the clip plays for, once its settings change its speed
  pub fn duration(&self) -> Duration {
    self.settings.played_duration(self.duration)
  }

  // the sample's length, before any speed change
  pub fn sample_duration(&self) -> Duration {
    self.duration
  }

  pub fn settings(&self) -> daw::ClipSettings {
    self.settings
  }

  // the path of the sample file the clip plays
  pub fn sample_path(&self) -> &str {
    &self.sample_path
  }

  // change how the clip is played back, returns the settings kept in range
  pub fn set_settings(&mut self, settings: daw::ClipSettings) -> daw::ClipSettings {
    self.settings = settings.clamped(self.duration);
    self.settings
  }

  // the clip's audio, played with its settings (always stereo)
  pub fn source(&self) -> impl Source<Item = f32> + Send + 'static {
    self.settings.apply(self.buffer.decoder().convert_samples::<f32>(), self.duration)
  }
  
  pub fn get_waveform(&mut self) -> &mut Vec<f32> {
    &mut self.waveform
  }
}

impl std::fmt::Display for AudioNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "AudioNode {{
  id: {},
  start_offset (ms): {}ms,
  track_number: {},
  sample_path: {},
  channels: {},
  duration (ms): {}ms,
}}", 
      self.id, 
      self.start_offset.as_millis(), 
      self.track_number, 
      self.sample_path, 
      self.channels, 
      self.duration().as_millis())
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

// a pattern placed in the playlist. it stays linked to its pattern, so it
// plays whatever the pattern holds now, and is as long as the pattern is
#[derive(Clone, Debug, PartialEq)]
pub struct PatternNode {
  pub id: u64,
  pub pattern_id: u64,
  pub start_offset: Duration,
  pub track_number: u32,
}

// when to play a sound at `at`, `len` long, in the slice of time from
// `window_start`, `dur` long
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scheduled {
  // it starts in the slice, this far in
  Delay(Duration),
  // it started before the slice and is still sounding: play the rest, from
  // this far in. only on a resume (a run's first slice)
  Skip(Duration),
}

pub fn schedule_in_window(
  at: Duration,
  len: Duration,
  window_start: Duration,
  dur: Duration,
  resume: bool,
) -> Option<Scheduled> {
  if at >= window_start && at < window_start + dur {
    Some(Scheduled::Delay(at - window_start))
  } else if resume && at < window_start && at + len > window_start {
    Some(Scheduled::Skip(window_start - at))
  } else {
    None
  }
}

pub struct AudioGraph<'a> {
  pub nodes: Vec<AudioNode>,
  // patterns placed in the playlist. they share the ids of `nodes`, so an
  // id names one clip of either kind
  pub pattern_nodes: Vec<PatternNode>,
  pub running: bool,
  pub current_offset: Option<Duration>,
  sample_rate: u32,
  tempo: f32,
  max_beats: u64,
  next_id: u64,
  // every clip plays through its track's strip (shared with the playlist
  // state, so the mixer's controls don't need the graph's lock)
  pub mixer: Arc<daw::Mixer>,
  // the patterns that pattern clips play (shared with the playlist state).
  // always locked after the graph, never before it
  pub sequencer: Arc<Mutex<daw::Sequencer>>,
  _phantom: PhantomData<&'a str>,
}

impl AudioGraph<'static> {
  pub fn new(
    sample_rate: u32, 
    tempo: f32,
    max_beats: u64,
  ) -> Self {
    AudioGraph {
      nodes: Vec::<AudioNode>::new(),
      pattern_nodes: Vec::new(),
      running: false,
      current_offset: Some(Duration::ZERO),
      sample_rate,
      tempo,
      max_beats,
      next_id: 0,
      mixer: Arc::new(daw::Mixer::default()),
      sequencer: Arc::new(Mutex::new(daw::Sequencer::default())),
      _phantom: PhantomData,
    }
  }

  // start a run of the graph (when it has clips to play)
  pub fn init(&mut self) {
    if !self.has_clips() { return; }

    self.running = true;
  }

  // schedule the next `dur` of the graph on this run's master bus
  // - stream: the run's output, which the metronome plays on directly so the
  //   mixer doesn't change it
  // - bus: the master bus (see daw::run_playlist), which every clip plays
  //   through
  // - resume: the run's first slice, so clips that started before it and are
  //   still sounding are played from where the playhead is in them
  pub fn run_for(
    &self,
    stream: &OutputStreamHandle,
    bus: &DynamicMixerController<f32>,
    dur: Duration,
    metronome_enabled: bool,
    resume: bool,
  ) {
    if !self.has_clips() {
      println!("Tried to run audio graph. No nodes in graph!");
      return;
    }

    if !self.running { return; }

    if metronome_enabled {
      if let Err(e) = stream.play_raw(daw::METRONOME_TICK_SOURCE.convert_samples()) {
        println!("could not play metronome: {}", e);
      }
    }

    let (_controller, mixer) = self.slice_mixer(dur, resume);
    bus.add(mixer);
  }

  // nodes that start within [window_start, window_start + dur)
  fn nodes_in_window(
    &self,
    window_start: Duration,
    dur: Duration,
  ) -> impl Iterator<Item = &AudioNode> {
    let window_end = window_start + dur;

    self.nodes.iter().filter(move |node|
      node.start_offset >= window_start && node.start_offset < window_end)
  }

  // nodes that started before `at` and are still playing then
  fn nodes_sounding_at(&self, at: Duration) -> impl Iterator<Item = &AudioNode> {
    self.nodes.iter().filter(move |node|
      node.start_offset < at && node.start_offset + node.duration() > at)
  }

  // the clips that start in the next `dur`, each through its track's strip
  // (the metronome isn't included, see run_for)
  pub fn buffer_slice(&self, dur: Duration) -> Option<(Arc<DynamicMixerController<f32>>, DynamicMixer<f32>)> {
    Some(self.slice_mixer(dur, false))
  }

  // buffer_slice, also playing the rest of every clip already sounding at
  // the slice's start when `resume` is set
  fn slice_mixer(&self, dur: Duration, resume: bool) -> (Arc<DynamicMixerController<f32>>, DynamicMixer<f32>) {
    let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(2, self.sample_rate);
    let silence = rodio::source::Zero::<f32>::new(2, self.sample_rate).take_duration(dur);

    controller.add(silence);
    
    let window_start = self.current_offset.unwrap_or(Duration::ZERO);
    println!("current offset: {}", window_start.as_millis());

    if resume {
      for node in self.nodes_sounding_at(window_start) {
        let played = window_start - node.start_offset;
        println!("resumed node: {} ({}ms in)", node.sample_path, played.as_millis());
        let source = self.mixer.track_strip(node.track_number, node.source().skip_duration(played));
        controller.add(source);
      }
    }

    // schedule every node that starts within this time slice
    for node in self.nodes_in_window(window_start, dur) {
      let delay_offset = node.start_offset - window_start;
      println!("added node: {} (delay offset: {}ms)", node.sample_path, delay_offset.as_millis());
      let source = self.mixer.track_strip(node.track_number, node.source()).delay(delay_offset);
      controller.add(source);
    }

    // and every pattern clip's hits, through its track's strip
    if !self.pattern_nodes.is_empty() {
      let sequencer = self.sequencer.lock().unwrap();
      let beat = self.beat_interval();
      for node in &self.pattern_nodes {
        for (row, offset) in sequencer.hits(node.pattern_id, beat) {
          let at = node.start_offset + offset;
          match schedule_in_window(at, row.duration(), window_start, dur, resume) {
            Some(Scheduled::Delay(d)) =>
              controller.add(self.mixer.track_strip(node.track_number, row.source()).delay(d)),
            Some(Scheduled::Skip(d)) =>
              controller.add(self.mixer.track_strip(node.track_number, row.source().skip_duration(d))),
            None => {}
          }
        }
      }
    }

    (controller, mixer)
  }

  // schedule the next `dur` of a pattern played on its own (in the Sequencer
  // tab) on the run's master bus. the graph's current offset is where the
  // run is in the pattern. like run_for, but the hits go straight to the bus
  // (there's no playlist track), and only the pattern plays
  pub fn run_pattern_for(
    &self,
    stream: &OutputStreamHandle,
    bus: &DynamicMixerController<f32>,
    pattern_id: u64,
    dur: Duration,
    metronome_enabled: bool,
    resume: bool,
  ) {
    if metronome_enabled {
      if let Err(e) = stream.play_raw(daw::METRONOME_TICK_SOURCE.convert_samples()) {
        println!("could not play metronome: {}", e);
      }
    }

    let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(2, self.sample_rate);
    controller.add(rodio::source::Zero::<f32>::new(2, self.sample_rate).take_duration(dur));

    let window_start = self.current_offset.unwrap_or(Duration::ZERO);
    let sequencer = self.sequencer.lock().unwrap();
    for (row, at) in sequencer.hits(pattern_id, self.beat_interval()) {
      match schedule_in_window(at, row.duration(), window_start, dur, resume) {
        Some(Scheduled::Delay(d)) => controller.add(row.source().delay(d)),
        Some(Scheduled::Skip(d)) => controller.add(row.source().skip_duration(d)),
        None => {}
      }
    }

    bus.add(mixer);
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

  pub fn sample_rate(&self) -> u32 {
    self.sample_rate
  }

  // length of one beat at the current tempo (60 / bpm seconds)
  pub fn beat_interval(&self) -> Duration {
    daw::timing::beat_interval_from_tempo(self.tempo)
  }

  pub fn interval_of_subdivision<T>(
    &self, 
    note: T
  ) -> Duration 
  where T: daw::timing::MusicalTiming {
    daw::timing::subdivision_interval_from_tempo(self.tempo, note)
  }

  // add nodes to the audio graph and sort the graph by
  // offset start time
  pub fn add_node(&mut self, node: AudioNode) {
    // keep the counter ahead of any externally assigned id
    self.next_id = self.next_id.max(node.id + 1);
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
  ) -> Result<u64, String> {
    let id = self.next_id;
    let node = AudioNode::new(
      id,
      sample_path,
      start_offset,
      track_number,
      self.sample_rate,
    )?;
    println!("start offset is: {}ms", start_offset.as_millis());

    self.add_node(node);

    Ok(id)
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
  ) -> Result<u64, String> {
    let id = self.next_id;
    let snapped_offset = self
      .find_nearest_offset_subdivision(start_offset, subdivision);
    
    println!("added to snapped offset: {}ms", snapped_offset.as_millis());

    let node = AudioNode::new(
      id,
      sample_path,
      snapped_offset,
      track_number,
      self.sample_rate,
    )?;

    self.add_node(node);

    Ok(id)
  }

  // move node in graph by node id
  pub fn move_node(
    &mut self,
    id: u64,
    start_offset: Duration,
    track_number: u32
  ) -> Result<(), String> {
    self.set_placement(id, start_offset, track_number)?;
    self.nodes.sort_by(|a, b| a.start_offset.cmp(&b.start_offset));

    Ok(())
  }

  // move node in graph by id, and snap to nearest subdivision
  pub fn move_node_with_snap<T: daw::timing::MusicalTiming>(
    &mut self,
    id: u64,
    start_offset: Duration,
    track_number: u32,
    subdivision: T
  ) -> Result<(), String> {
    let snapped_offset = self
      .find_nearest_offset_subdivision(start_offset, subdivision);
    
    println!("moved to snapped offset: {}ms", snapped_offset.as_millis());

    self.move_node(id, snapped_offset, track_number)
  }

  // move several nodes together, keeping their spacing: the anchor (the
  // clip that was grabbed) goes to `anchor_offset` on `anchor_track`, snapped
  // when `subdivision` is given, and every other node moves by as much. the
  // group stops at the playlist's start and at `max_offset`, and between
  // the first track and the new-track area, as a whole. returns where each
  // node ended up, as (id, start offset, track)
  pub fn move_nodes<T: daw::timing::MusicalTiming>(
    &mut self,
    ids: &[u64],
    anchor_id: u64,
    anchor_offset: Duration,
    anchor_track: u32,
    max_offset: Duration,
    subdivision: Option<T>,
  ) -> Result<Vec<(u64, Duration, u32)>, String> {
    let placed = ids.iter()
      .map(|&id| self.placement(id)
        .map(|(start, track)| (id, start, track))
        .ok_or_else(|| format!("no node with id {}", id)))
      .collect::<Result<Vec<_>, String>>()?;

    let target = match subdivision {
      Some(s) => self.find_nearest_offset_subdivision(anchor_offset, s),
      None => anchor_offset,
    };

    let moved = group_move(&placed, anchor_id, target, anchor_track, max_offset, self.track_count())?;
    for &(id, start_offset, track_number) in &moved {
      self.set_placement(id, start_offset, track_number).expect("checked above");
    }
    self.nodes.sort_by(|a, b| a.start_offset.cmp(&b.start_offset));

    Ok(moved)
  }

  // remove node from graph with provided id
  // returns an error if id does not exist
  // remove every node, e.g. before loading a project. ids keep counting up,
  // so an id from before never names a new node
  pub fn clear(&mut self) {
    self.nodes.clear();
    self.pattern_nodes.clear();
    self.current_offset = Some(Duration::ZERO);
  }

  // take a clip of either kind out of the playlist
  pub fn remove_node(&mut self, id: u64) -> Result<(), String> {
    if let Some(idx) = self.nodes.iter().position(|a| a.id == id) {
      self.nodes.remove(idx);
      return Ok(());
    }
    let idx = self.pattern_nodes.iter().position(|n| n.id == id)
      .ok_or_else(|| format!("no node with id {}", id))?;
    self.pattern_nodes.remove(idx);
    Ok(())
  }

  // place a pattern in the playlist, snapped when `subdivision` is given.
  // returns the clip's id
  pub fn add_pattern_node<T: daw::timing::MusicalTiming>(
    &mut self,
    pattern_id: u64,
    start_offset: Duration,
    track_number: u32,
    subdivision: Option<T>,
  ) -> Result<u64, String> {
    if self.sequencer.lock().unwrap().pattern(pattern_id).is_none() {
      return Err(format!("no pattern with id {}", pattern_id));
    }
    let start_offset = match subdivision {
      Some(s) => self.find_nearest_offset_subdivision(start_offset, s),
      None => start_offset,
    };
    let id = self.next_id;
    self.add_pattern_node_with_id(PatternNode { id, pattern_id, start_offset, track_number });
    Ok(id)
  }

  // add a pattern clip with a known id (from a project)
  pub fn add_pattern_node_with_id(&mut self, node: PatternNode) {
    self.next_id = self.next_id.max(node.id + 1);
    self.pattern_nodes.push(node);
  }

  pub fn get_pattern_node_by_id(&self, id: u64) -> Option<&PatternNode> {
    self.pattern_nodes.iter().find(|n| n.id == id)
  }

  // take out every clip of a pattern (the pattern is being deleted).
  // returns their ids
  pub fn remove_pattern_nodes_of(&mut self, pattern_id: u64) -> Vec<u64> {
    let ids = self.pattern_nodes.iter()
      .filter(|n| n.pattern_id == pattern_id)
      .map(|n| n.id)
      .collect();
    self.pattern_nodes.retain(|n| n.pattern_id != pattern_id);
    ids
  }

  // split a pattern clip into a clip per sample it plays: each plays a
  // pattern holding just that sample's steps (see Sequencer::split_pattern),
  // on a track of its own (see explode_tracks), so each sample gets its own
  // mixer strip. unnamed tracks are named after their samples. returns the
  // new clips' ids, in row order
  pub fn explode_pattern_node(&mut self, id: u64) -> Result<Vec<u64>, String> {
    let clip = self.get_pattern_node_by_id(id).cloned()
      .ok_or_else(|| format!("no pattern clip with id {}", id))?;

    let (split, placed_patterns) = {
      let mut sequencer = self.sequencer.lock().unwrap();
      let split = sequencer.split_pattern(clip.pattern_id)?;
      // other pattern clips count as their sample's when they play just one
      let placed: Vec<(u32, Option<String>)> = self.pattern_nodes.iter()
        .filter(|n| n.id != id)
        .map(|n| (n.track_number, sequencer.single_sample(n.pattern_id).map(str::to_string)))
        .collect();
      (split, placed)
    };

    let paths: Vec<&str> = split.iter().map(|(path, _)| path.as_str()).collect();
    let placed: Vec<(u32, Option<&str>)> = self.nodes.iter()
      .map(|n| (n.track_number, Some(n.sample_path())))
      .chain(placed_patterns.iter().map(|(t, p)| (*t, p.as_deref())))
      .collect();
    let tracks = explode_tracks(&paths, clip.track_number, &placed, daw::defaults::NUM_OF_TRACKS);

    self.remove_node(id)?;
    let mut ids = Vec::new();
    for ((path, pattern_id), &track) in split.iter().zip(&tracks) {
      let new_id = self.next_id;
      self.add_pattern_node_with_id(PatternNode {
        id: new_id,
        pattern_id: *pattern_id,
        start_offset: clip.start_offset,
        track_number: track,
      });
      ids.push(new_id);
      if self.mixer.name(track).is_none() {
        self.mixer.set_name(track, &daw::sample_name(path));
      }
    }

    Ok(ids)
  }

  // whether the playlist has any clips, of either kind
  pub fn has_clips(&self) -> bool {
    !self.nodes.is_empty() || !self.pattern_nodes.is_empty()
  }

  // where a clip of either kind is: (start offset, track)
  pub fn placement(&self, id: u64) -> Option<(Duration, u32)> {
    self.get_node_by_id(id).map(|n| (n.start_offset, n.track_number))
      .or_else(|| self.get_pattern_node_by_id(id).map(|n| (n.start_offset, n.track_number)))
  }

  // move a clip of either kind (the caller keeps `nodes` sorted)
  fn set_placement(&mut self, id: u64, start_offset: Duration, track_number: u32) -> Result<(), String> {
    if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
      node.start_offset = start_offset;
      node.track_number = track_number;
      return Ok(());
    }
    let node = self.pattern_nodes.iter_mut().find(|n| n.id == id)
      .ok_or_else(|| format!("no node with id {}", id))?;
    node.start_offset = start_offset;
    node.track_number = track_number;
    Ok(())
  }

  // end the graph's run
  pub fn pause(&mut self) {
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

  // where the last clip finishes playing (zero with no clips)
  pub fn end(&self) -> Duration {
    let clips = self.nodes
      .iter()
      .map(|node| node.start_offset + node.duration())
      .max()
      .unwrap_or(Duration::ZERO);
    if self.pattern_nodes.is_empty() { return clips; }

    // a pattern clip ends with its pattern (its last hits ring past it)
    let sequencer = self.sequencer.lock().unwrap();
    let beat = self.beat_interval();
    self.pattern_nodes
      .iter()
      .filter_map(|node| sequencer.pattern_len(node.pattern_id, beat).map(|len| node.start_offset + len))
      .fold(clips, Duration::max)
  }

  pub fn duration_max(&self) -> Duration {
    self.beat_interval() * self.max_beats as u32
  }

  // count number of nodes
  pub fn len(&self) -> usize {
    let res = self.nodes.len();
    res
  }

  pub fn count_nodes(&self) -> usize {
    let res = self.nodes.len();
    res
  }
  
  pub fn get_mut_node_by_id(&mut self, id: u64) -> Option<&mut AudioNode> {
    self.nodes.iter_mut().find(|e| { e.id == id })
  }

  pub fn get_node_by_id(&self, id: u64) -> Option<&AudioNode> {
    self.nodes.iter().find(|e| { e.id == id })
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

    let nearest_offset_micros = util::math::round_to_nearest_multiple(
      offset.as_micros(),
      interval.as_micros());

    Duration::from_micros(nearest_offset_micros.try_into().unwrap())
  }
  
  // return the unique audio tracks numbers in the graph
  pub fn track_numbers(&self) -> Vec<u32> {
    let mut counted: Vec<u32> = Vec::new();

    let tracks = self.nodes.iter().map(|n| n.track_number)
      .chain(self.pattern_nodes.iter().map(|n| n.track_number));
    for track_number in tracks {
      if !counted.contains(&track_number) {
        counted.push(track_number);
      }
    }
    
    counted
  }

  // how many tracks the playlist shows: the default number, or enough to
  // reach the highest track a clip is on. (counting only the tracks that
  // have clips would hide a clip on track 4 when just two tracks are used)
  pub fn track_count(&self) -> u32 {
    let needed = self.nodes.iter().map(|node| node.track_number)
      .chain(self.pattern_nodes.iter().map(|node| node.track_number))
      .map(|track_number| track_number + 1)
      .max()
      .unwrap_or(0);

    needed.max(daw::defaults::NUM_OF_TRACKS)
  }

  // get a vector of AudioNodes with the specified track number
  pub fn get_track_by_track_number(&self, track_number: u32) -> Vec<&AudioNode> {
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
      let track = self.get_track_by_track_number(track_num);

      tracks.push(track)
    }

    tracks
  }
}

// which track each sample of an exploded pattern goes on, so every sample
// gets a track (and mixer strip) of its own
// - paths: the pattern's samples, in row order
// - clip_track: the track the pattern clip was on
// - placed: every other clip in the playlist, as (track, sample path), with
//   None for pattern clips
// - min_tracks: how many tracks the playlist always shows
//
// a sample goes on the track already holding only clips of it (its track
// from an earlier explode), if there is one. otherwise the first sample
// stays on the clip's track, unless that has other samples on it, and the
// rest go on the first empty track, or a new one at the bottom
pub fn explode_tracks(
  paths: &[&str],
  clip_track: u32,
  placed: &[(u32, Option<&str>)],
  min_tracks: u32,
) -> Vec<u32> {
  let mut placed: Vec<(u32, Option<&str>)> = placed.to_vec();
  let mut tracks = Vec::new();

  for (i, &path) in paths.iter().enumerate() {
    let has_other_sample = |placed: &[(u32, Option<&str>)], track: u32| placed.iter()
      .any(|&(t, p)| t == track && p.map_or(false, |p| p != path));
    let dedicated = placed.iter()
      .filter(|&&(_, p)| p == Some(path))
      .map(|&(t, _)| t)
      .filter(|&t| !has_other_sample(&placed, t))
      .min();

    let track = dedicated
      .or_else(|| if i == 0 && !has_other_sample(&placed, clip_track) { Some(clip_track) } else { None })
      .unwrap_or_else(|| {
        let count = placed.iter().map(|&(t, _)| t + 1).max().unwrap_or(0).max(min_tracks);
        (0..count).find(|&t| t != clip_track && !placed.iter().any(|&(pt, _)| pt == t)).unwrap_or(count)
      });

    placed.push((track, Some(path)));
    tracks.push(track);
  }

  tracks
}

// where a group of clips lands when one of them, the anchor, is dropped at
// `target` on `target_track`. every clip moves by the anchor's change in
// time and track, so the group keeps its shape. the change is cut short so
// no clip starts before 0 or after `max_offset`, or goes above track 0 or
// below `max_track` (the new-track area). takes and returns
// (id, start offset, track)
pub fn group_move(
  placed: &[(u64, Duration, u32)],
  anchor_id: u64,
  target: Duration,
  target_track: u32,
  max_offset: Duration,
  max_track: u32,
) -> Result<Vec<(u64, Duration, u32)>, String> {
  let &(_, anchor_start, anchor_track) = placed.iter()
    .find(|(id, _, _)| *id == anchor_id)
    .ok_or_else(|| format!("the grabbed clip {} isn't in the group", anchor_id))?;

  // in microseconds, signed, so the change can go either way
  let micros = |d: Duration| d.as_micros() as i128;
  let earliest = placed.iter().map(|p| micros(p.1)).min().unwrap();
  let latest = placed.iter().map(|p| micros(p.1)).max().unwrap();
  let shift = (micros(target) - micros(anchor_start))
    .clamp(-earliest, (micros(max_offset) - latest).max(-earliest));

  let top = placed.iter().map(|p| p.2 as i64).min().unwrap();
  let bottom = placed.iter().map(|p| p.2 as i64).max().unwrap();
  let tracks = (target_track as i64 - anchor_track as i64)
    .clamp(-top, (max_track as i64 - bottom).max(-top));

  Ok(placed.iter()
    .map(|&(id, start, track)| (
      id,
      Duration::from_micros((micros(start) + shift) as u64),
      (track as i64 + tracks) as u32,
    ))
    .collect())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::daw::{self};
  use std::thread;
  use std::time::Instant;
  use rodio::OutputStream;

  fn ms(n: u64) -> Duration { Duration::from_millis(n) }

  #[test]
  fn test_group_move_keeps_spacing() {
    let placed = [(1, ms(1_000), 0), (2, ms(1_250), 1), (3, ms(3_000), 1)];

    // clip 2 grabbed and moved 500ms later, one track down
    let moved = group_move(&placed, 2, ms(1_750), 2, ms(8_000), 4).unwrap();
    assert_eq!(moved, vec![(1, ms(1_500), 1), (2, ms(1_750), 2), (3, ms(3_500), 2)]);

    // and earlier, one track up
    let moved = group_move(&placed, 1, ms(400), 0, ms(8_000), 4).unwrap();
    assert_eq!(moved, vec![(1, ms(400), 0), (2, ms(650), 1), (3, ms(2_400), 1)]);
  }

  #[test]
  fn test_group_move_stops_at_the_playlist_edges() {
    let placed = [(1, ms(1_000), 0), (2, ms(3_000), 1)];

    // dragged past the start: the first clip stops at 0
    let moved = group_move(&placed, 2, ms(500), 1, ms(8_000), 4).unwrap();
    assert_eq!(moved, vec![(1, ms(0), 0), (2, ms(2_000), 1)]);

    // dragged past the end: the last clip stops at the end
    let moved = group_move(&placed, 1, ms(7_500), 0, ms(8_000), 4).unwrap();
    assert_eq!(moved, vec![(1, ms(6_000), 0), (2, ms(8_000), 1)]);
  }

  #[test]
  fn test_group_move_stops_at_the_first_track_and_the_new_track_area() {
    let placed = [(1, ms(0), 1), (2, ms(500), 3)];

    // up past track 0: the top clip stops on track 0
    let moved = group_move(&placed, 2, ms(500), 0, ms(8_000), 4).unwrap();
    assert_eq!(moved, vec![(1, ms(0), 0), (2, ms(500), 2)]);

    // down past the new-track area (track 4): the bottom clip stops there
    let moved = group_move(&placed, 1, ms(0), 6, ms(8_000), 4).unwrap();
    assert_eq!(moved, vec![(1, ms(0), 2), (2, ms(500), 4)]);
  }

  #[test]
  fn test_group_move_needs_the_anchor() {
    assert!(group_move(&[(1, ms(0), 0)], 9, ms(0), 0, ms(8_000), 4).is_err());
  }

  #[test]
  fn test_move_nodes_snaps_only_the_anchor() {
    // 120bpm: a beat is 500ms
    let mut graph = graph_with_ticks(&[1_000, 1_130]);
    let ids: Vec<u64> = graph.nodes.iter().map(|n| n.id).collect();
    let (a, b) = (ids[0], ids[1]);

    // a dropped at 1_620ms snaps to 1_500ms; b keeps its 130ms gap, off the grid
    let moved = graph.move_nodes(&ids, a, ms(1_620), 0, ms(8_000), Some(daw::timing::Note::QuarterNote)).unwrap();
    assert_eq!(moved, vec![(a, ms(1_500), 0), (b, ms(1_630), 1)]);
    assert_eq!(graph.get_node_by_id(b).unwrap().start_offset, ms(1_630));

    // without snap the anchor lands exactly where it was dropped
    graph.move_nodes(&ids, a, ms(1_620), 1, ms(8_000), None::<daw::timing::Note>).unwrap();
    assert_eq!(graph.get_node_by_id(a).unwrap().start_offset, ms(1_620));
    assert_eq!(graph.get_node_by_id(b).unwrap().track_number, 2);

    // an unknown id moves nothing
    assert!(graph.move_nodes(&[a, 9_999], a, ms(0), 0, ms(8_000), None::<daw::timing::Note>).is_err());
    assert_eq!(graph.get_node_by_id(a).unwrap().start_offset, ms(1_620));
  }

  #[test]
  fn test_node_initialization() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(0);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate).unwrap();

    assert_eq!(node.start_offset, Duration::from_millis(0));
    assert_eq!(node.duration().as_millis(), daw::METRONOME_TICK_SOURCE.convert_samples().total_duration().unwrap().as_millis());
    assert_eq!(node.channels, daw::METRONOME_TICK_SOURCE.convert_samples().channels());
    assert_eq!(node.sample_path, daw::METRONOME_TICK_PATH);
    assert_eq!(node.sample_rate, 44_100);
    assert_eq!(node.track_number, track_number);
  }

  #[test]
  fn test_audiograph_initialization() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(400);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate).unwrap();
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

  #[test]
  fn test_audiograph_run() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(650);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate).unwrap();

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);

    audiograph.add_node(node);
    
    // open the stream before timing, it's slow to open
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let runtime = Duration::from_secs(1);
    let now = Instant::now();

    let (bus, _bus_out) = rodio::dynamic_mixer::mixer::<f32>(2, sample_rate);
    audiograph.run_for(&stream_handle, &bus, runtime, true, false);
    thread::sleep(runtime);

    let now_elapsed = now.elapsed();
    let later = Instant::now();

    assert_ne!(now_elapsed.as_millis(), later.elapsed().as_millis());

    let diff = now_elapsed - runtime;
    let result = diff.as_millis() < 10;

    assert_eq!(result, true);
  }

  #[test]
  fn test_audiograph_buffer_slice_with_silence() {
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

    // a second of stereo silence. rodio rounds the time per sample to whole
    // nanoseconds, so it runs a few samples over
    let expected = 44_100 * 2;
    assert!(
      zero_samples.len() >= expected && zero_samples.len() < expected + 10,
      "expected about {} samples of silence, got {}", expected, zero_samples.len());
    assert_eq!(nonzero_samples.len(), 0);
  }

  #[test]
  fn test_audiograph_buffer_slice_with_node() {
    let id = 1;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(650);
    let track_number = 0;
    let sample_rate = 44_100;

    let node = AudioNode::new(id, sample_path, start_offset, track_number, sample_rate).unwrap();

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

    // (this was * 4 while the slice also held a metronome tick, which is the
    // same sample. the metronome now plays outside the mixer, see run_for)
    assert_eq!(node_samples.len() * 2, nonzero_samples.len());
  }

  #[test]
  fn test_audiograph_beat_interval() {
    let mut audiograph = AudioGraph::new(44_100, 120f32, 8);
    assert_eq!(audiograph.beat_interval().as_millis(), 500);

    audiograph.set_tempo(140f32);
    assert_eq!(audiograph.beat_interval().as_millis(), 428);

    audiograph.set_tempo(60f32);
    assert_eq!(audiograph.beat_interval().as_millis(), 1_000);
    assert_eq!(audiograph.duration_max().as_millis(), 8_000);

    assert_eq!(audiograph.beat_interval(), daw::timing::beat_interval_from_tempo(audiograph.tempo()));
  }

  #[test]
  fn test_bad_sample_returns_error() {
    let mut audiograph = AudioGraph::new(44_100, 120f32, 8);

    // missing file
    let res = audiograph.construct_and_add_node(
      "assets/does-not-exist.wav".to_string(), Duration::ZERO, 0);
    assert!(res.unwrap_err().contains("does-not-exist.wav"));

    // todo: also test a file that exists but is not audio. currently
    // rodio 0.15 falls back to minimp3 -> slice-deque, which hits UB
    // (aborts in debug) on unrecognised input, so it can't be tested yet

    // failed loads add nothing, don't use up ids,
    // and leave the graph usable
    assert_eq!(audiograph.len(), 0);
    let id = audiograph.construct_and_add_node(
      daw::METRONOME_TICK_PATH.to_string(), Duration::ZERO, 0).unwrap();
    assert_eq!(id, 0);
  }

  #[test]
  fn test_preview_sample_matches_a_node() {
    let path = daw::METRONOME_TICK_PATH;
    let (dur, waveform) = preview_sample(path, 44_100).unwrap();
    let node = AudioNode::new(0, path.to_string(), Duration::ZERO, 0, 44_100).unwrap();

    assert_eq!(dur, node.sample_duration());
    assert_eq!(&waveform, &node.waveform);
    assert!(preview_sample("does/not/exist.wav", 44_100).is_err());
  }

  #[test]
  fn test_track_count_reaches_the_highest_track() {
    let path = daw::METRONOME_TICK_PATH.to_string();
    let mut audiograph = AudioGraph::new(44_100, 120., 16);
    let default = daw::defaults::NUM_OF_TRACKS;

    assert_eq!(audiograph.track_count(), default);

    // two tracks used, but one is past the default number of tracks
    audiograph.construct_and_add_node(path.clone(), Duration::ZERO, 0).unwrap();
    audiograph.construct_and_add_node(path.clone(), Duration::ZERO, default + 1).unwrap();
    assert_eq!(audiograph.track_count(), default + 2);

    audiograph.clear();
    assert_eq!(audiograph.track_count(), default);
  }

  #[test]
  fn test_node_settings_change_its_length() {
    let path = daw::METRONOME_TICK_PATH;
    let mut node = AudioNode::new(0, path.to_string(), Duration::ZERO, 0, 44_100).unwrap();
    let sample_dur = node.sample_duration();
    assert_eq!(node.duration(), sample_dur);

    // an octave up plays twice as fast, so half as long
    let kept = node.set_settings(daw::ClipSettings {
      pitch: 12.,
      attack_ms: 1_000_000.,
      ..Default::default()
    });
    assert_eq!(node.duration(), sample_dur.div_f32(2.));
    assert_eq!(node.sample_duration(), sample_dur);
    // the fade was kept within the clip
    assert!((kept.attack_ms - node.duration().as_secs_f32() * 1000.).abs() < 0.01);

    // where the graph ends follows the clip's played length
    let mut audiograph = AudioGraph::new(44_100, 120., 16);
    audiograph.add_node(node);
    assert_eq!(audiograph.duration(), sample_dur.div_f32(2.));
  }

  #[test]
  fn test_waveform_rejects_empty_and_tiny_samples() {
    assert!(daw::calc_waveform_from_samples(vec![], 2).is_none());
    assert!(daw::calc_waveform_from_samples(vec![1, 2, 3], 0).is_none());
    // a few samples (< 0.25ms) used to divide by zero
    assert!(daw::calc_waveform_from_samples(vec![100, -100, 50, -50], 1).is_some());
  }

  #[test]
  fn test_audiograph_node_ids_not_reused() {
    let mut audiograph = AudioGraph::new(44_100, 120f32, 8);
    let path = || daw::METRONOME_TICK_PATH.to_string();

    let kick = audiograph.construct_and_add_node(path(), Duration::from_millis(0), 0).unwrap();
    let snare = audiograph.construct_and_add_node(path(), Duration::from_millis(500), 1).unwrap();
    let hat = audiograph.construct_and_add_node(path(), Duration::from_millis(250), 2).unwrap();
    assert_eq!((kick, snare, hat), (0, 1, 2));

    // removing a node must not free its id for reuse
    audiograph.remove_node(kick).unwrap();
    let clap = audiograph.construct_and_add_node(path(), Duration::from_millis(750), 3).unwrap();
    assert_eq!(clap, 3);

    // every remaining id resolves to exactly one node
    for id in [snare, hat, clap] {
      assert_eq!(audiograph.nodes.iter().filter(|n| n.id == id).count(), 1);
    }
    assert_eq!(audiograph.get_node_by_id(hat).unwrap().track_number, 2);
    assert_eq!(audiograph.get_node_by_id(clap).unwrap().track_number, 3);

    // externally assigned ids push the counter forward
    let node = AudioNode::new(10, path(), Duration::ZERO, 0, 44_100).unwrap();
    audiograph.add_node(node);
    let next = audiograph.construct_and_add_node(path(), Duration::ZERO, 0).unwrap();
    assert_eq!(next, 11);
  }

  // build a graph with one metronome tick per offset (ms), each on its own track
  fn graph_with_ticks(offsets_ms: &[u64]) -> AudioGraph<'static> {
    let mut audiograph = AudioGraph::new(44_100, 120f32, 8);
    for (track, ms) in offsets_ms.iter().enumerate() {
      audiograph.construct_and_add_node(
        daw::METRONOME_TICK_PATH.to_string(),
        Duration::from_millis(*ms),
        track as u32).unwrap();
    }

    audiograph
  }

  #[test]
  fn test_nodes_in_window() {
    let audiograph = graph_with_ticks(&[0, 0, 200, 500, 900]);
    let beat = audiograph.beat_interval(); // 500ms at 120bpm
    let starts = |from_ms: u64| -> Vec<u128> {
      audiograph
        .nodes_in_window(Duration::from_millis(from_ms), beat)
        .map(|n| n.start_offset.as_millis())
        .collect()
    };

    // all nodes in the window, including simultaneous ones
    assert_eq!(starts(0), vec![0, 0, 200]);
    // a node exactly at the window end belongs to the next window only
    assert_eq!(starts(500), vec![500, 900]);
    assert_eq!(starts(1_000), Vec::<u128>::new());
  }

  #[test]
  fn test_end_is_where_the_last_clip_finishes() {
    assert_eq!(AudioGraph::new(44_100, 120f32, 8).end(), Duration::ZERO);

    let audiograph = graph_with_ticks(&[900, 0, 200]);
    let tick = audiograph.nodes[0].duration();
    assert_eq!(audiograph.end(), Duration::from_millis(900) + tick);
  }

  #[test]
  fn test_nodes_sounding_at() {
    let audiograph = graph_with_ticks(&[0, 200, 500]);
    let tick = audiograph.nodes[0].duration();
    let sounding = |at: Duration| -> Vec<u128> {
      audiograph
        .nodes_sounding_at(at)
        .map(|n| n.start_offset.as_millis())
        .collect()
    };

    // a node starting right at the playhead isn't resumed, it's scheduled
    // the usual way
    assert!(!sounding(Duration::from_millis(200)).contains(&200));
    // partway through the first tick
    assert_eq!(sounding(Duration::from_millis(1)), vec![0]);
    // after every tick has ended
    assert_eq!(sounding(Duration::from_millis(500) + tick), Vec::<u128>::new());
  }

  #[test]
  fn test_resumed_slice_plays_the_rest_of_a_sounding_clip() {
    let energy = |at_ms: u64, resume: bool| -> f64 {
      let mut audiograph = graph_with_ticks(&[0]);
      audiograph.set_current_offset(Some(Duration::from_millis(at_ms)));
      let (_controller, mixer) = audiograph.slice_mixer(audiograph.beat_interval(), resume);
      mixer.map(|x| x.abs() as f64).sum()
    };

    let tick = graph_with_ticks(&[0]).nodes[0].duration().as_millis() as u64;
    assert!(tick > 2, "the tick sample is too short to resume inside");
    let mid = tick / 2;

    // not resuming: the clip started before the slice, so it's silent
    assert_eq!(energy(mid, false), 0.);
    // resuming plays its second half, which is less than the whole tick
    let rest = energy(mid, true);
    assert!(rest > 0.);
    assert!(rest < energy(0, false));
  }

  #[test]
  fn test_audiograph_buffer_slice_plays_every_node_in_window() {
    // total signal energy of the first beat, relative to a single tick
    let energy = |offsets_ms: &[u64]| -> f64 {
      let audiograph = graph_with_ticks(offsets_ms);
      let (_controller, mixer) = audiograph
        .buffer_slice(audiograph.beat_interval())
        .unwrap();
      mixer.map(|x| x.abs() as f64).sum()
    };
    let one = energy(&[0]);
    let ratio = |offsets_ms: &[u64]| (energy(offsets_ms) / one * 100.).round() / 100.;

    // previously each of these mixed only the first node (1.0)
    assert_eq!(ratio(&[0, 0, 0]), 3.);
    assert_eq!(ratio(&[0, 200, 400]), 3.);
    // node at 900ms is in the next beat
    assert_eq!(ratio(&[0, 200, 400, 900]), 3.);
  }

  #[test]
  fn test_audiograph_buffer_set_functions() {
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

  #[test]
  fn test_audiograph_node_functions() {
    let id = 100;
    let sample_path = daw::METRONOME_TICK_PATH.to_string();
    let start_offset = Duration::from_millis(350);
    let sample_rate = 44_100;

    let node = AudioNode::new(
      id,
      sample_path,
      start_offset,
      0,
      sample_rate).unwrap();

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);
    let buf = node.buffer.clone();

    audiograph.add_node(node);
    let id2 = audiograph.construct_and_add_node(
      "assets/assets_66-bd-01.wav".to_string(),
      Duration::from_millis(100),
      1).unwrap();
    let id3 = audiograph.construct_and_add_node_with_snap(
      "assets/assets_66-sd-01.wav".to_string(),
      Duration::from_millis(600),
      2,
      daw::timing::SixteenthNote::new()).unwrap();
    
    assert_eq!(audiograph.len(), 3);
    // clips on tracks 0-2 fit in the default number of tracks
    assert_eq!(audiograph.track_count(), daw::defaults::NUM_OF_TRACKS);
    assert_eq!(audiograph.get_node_by_id(id2).unwrap().start_offset.as_millis(), 100);

    // 600ms snapped to a 125ms grid (sixteenth notes at 120bpm) = 625ms
    assert_eq!(audiograph.get_node_by_id(id3).unwrap().start_offset.as_millis(), 625);
    
    let target = audiograph.get_node_by_id(id).unwrap();

    assert_eq!(target.id, id);
    assert_eq!(target.track_number, 0);
    assert_eq!(target.start_offset.as_millis(), start_offset.as_millis());

    audiograph.move_node(
      id, 
      Duration::from_millis(0),
      2).unwrap();
    let target = &audiograph.nodes[0];

    assert_eq!(target.id, id);
    assert_eq!(target.track_number, 2);
    assert_eq!(target.start_offset.as_millis(), 0);

    audiograph.move_node_with_snap(
      id, 
      Duration::from_millis(12),
      1,
    daw::timing::EighthNote::new()).unwrap();
    let target = &audiograph.nodes[0];

    assert_eq!(target.id, id);
    assert_eq!(target.track_number, 1);
    assert_eq!(target.start_offset.as_millis(), 0);

    // unknown ids return an error instead of panicking
    assert!(audiograph.move_node(9_999, Duration::ZERO, 0).is_err());
    assert!(audiograph.move_node_with_snap(
      9_999, Duration::ZERO, 0, daw::timing::EighthNote::new()).is_err());
    assert!(audiograph.remove_node(9_999).is_err());
    assert_eq!(audiograph.len(), 3);
  }

  const KICK: &str = "assets/assets_66-bd-01.wav";

  // a graph with one pattern (one bar, kick on beats 1 and 3), and its ids
  fn graph_with_pattern() -> (AudioGraph<'static>, u64, u64) {
    let graph = AudioGraph::new(44_100, 120., 16);
    let (row, pattern) = {
      let mut seq = graph.sequencer.lock().unwrap();
      let row = seq.add_row(KICK, 44_100).unwrap();
      let pattern = seq.add_pattern(1, 4).unwrap();
      seq.set_step(pattern, row, 0, true).unwrap();
      seq.set_step(pattern, row, 8, true).unwrap();
      (row, pattern)
    };
    (graph, row, pattern)
  }

  #[test]
  fn test_schedule_in_window() {
    let (w, d) = (ms(1_000), ms(500));
    // starts in the slice
    assert_eq!(schedule_in_window(ms(1_000), ms(100), w, d, false), Some(Scheduled::Delay(ms(0))));
    assert_eq!(schedule_in_window(ms(1_250), ms(100), w, d, false), Some(Scheduled::Delay(ms(250))));
    // the slice's end belongs to the next one
    assert_eq!(schedule_in_window(ms(1_500), ms(100), w, d, false), None);
    // already sounding: only played on a resume
    assert_eq!(schedule_in_window(ms(900), ms(300), w, d, false), None);
    assert_eq!(schedule_in_window(ms(900), ms(300), w, d, true), Some(Scheduled::Skip(ms(100))));
    // finished before the slice
    assert_eq!(schedule_in_window(ms(500), ms(300), w, d, true), None);
  }

  #[test]
  fn test_pattern_nodes_share_ids_and_snap() {
    let (mut graph, _, pattern) = graph_with_pattern();
    let clip = graph.construct_and_add_node(KICK.to_string(), ms(0), 0).unwrap();
    let placed = graph.add_pattern_node(pattern, ms(1_100), 2, Some(daw::timing::QuarterNote::new())).unwrap();

    assert_ne!(placed, clip);
    assert_eq!(graph.placement(placed), Some((ms(1_000), 2)));
    assert!(graph.has_clips());
    // a pattern that doesn't exist can't be placed
    assert!(graph.add_pattern_node::<daw::timing::QuarterNote>(99, ms(0), 0, None).is_err());
  }

  #[test]
  fn test_pattern_nodes_move_and_remove_like_clips() {
    let (mut graph, _, pattern) = graph_with_pattern();
    let clip = graph.construct_and_add_node(KICK.to_string(), ms(0), 0).unwrap();
    let placed = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(500), 1, None).unwrap();

    graph.move_node(placed, ms(2_000), 3).unwrap();
    assert_eq!(graph.placement(placed), Some((ms(2_000), 3)));

    // moved in a group with a sample clip
    let moved = graph.move_nodes::<daw::timing::QuarterNote>(
      &[clip, placed], clip, ms(500), 1, ms(8_000), None).unwrap();
    assert_eq!(moved, vec![(clip, ms(500), 1), (placed, ms(2_500), 4)]);

    graph.remove_node(placed).unwrap();
    assert!(graph.get_pattern_node_by_id(placed).is_none());
    assert!(graph.remove_node(placed).is_err());
  }

  #[test]
  fn test_pattern_nodes_count_toward_tracks_and_the_end() {
    let (mut graph, _, pattern) = graph_with_pattern();
    assert_eq!(graph.end(), ms(0));
    graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(3_000), 6, None).unwrap();

    assert_eq!(graph.track_count(), 7);
    // a one bar pattern is 2s at 120bpm
    assert_eq!(graph.end(), ms(5_000));
    // a longer pattern makes its clips longer
    graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();
    assert_eq!(graph.end(), ms(7_000));
  }

  #[test]
  fn test_deleting_a_pattern_takes_its_clips() {
    let (mut graph, _, pattern) = graph_with_pattern();
    let other = graph.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
    let a = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
    let b = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(2_000), 0, None).unwrap();
    let keep = graph.add_pattern_node::<daw::timing::QuarterNote>(other, ms(0), 1, None).unwrap();

    assert_eq!(graph.remove_pattern_nodes_of(pattern), vec![a, b]);
    assert_eq!(graph.pattern_nodes.len(), 1);
    assert_eq!(graph.pattern_nodes[0].id, keep);
  }

  #[test]
  fn test_clear_takes_pattern_nodes() {
    let (mut graph, _, pattern) = graph_with_pattern();
    graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
    graph.clear();
    assert!(!graph.has_clips());
  }

  #[test]
  fn test_explode_tracks_on_an_empty_playlist() {
    // the first sample stays on the clip's track, the rest take empty ones
    assert_eq!(explode_tracks(&["kick", "snare", "hat"], 1, &[], 4), vec![1, 0, 2]);
    // past the tracks shown: new ones at the bottom
    assert_eq!(explode_tracks(&["a", "b", "c", "d", "e"], 0, &[], 4), vec![0, 1, 2, 3, 4]);
  }

  #[test]
  fn test_explode_tracks_reuses_a_samples_track() {
    // an earlier explode put kicks on 0 and snares on 5; another copy of the
    // pattern sits on 0
    let placed = [(0, Some("kick")), (5, Some("snare")), (0, None), (2, Some("bass"))];
    assert_eq!(explode_tracks(&["kick", "snare", "hat"], 0, &placed, 4), vec![0, 5, 1]);
  }

  #[test]
  fn test_explode_tracks_skips_shared_tracks() {
    // the clip's track holds another sample, so the first sample moves off it,
    // and a track with the sample and something else isn't its own
    let placed = [(0, Some("bass")), (1, Some("kick")), (1, Some("bass"))];
    assert_eq!(explode_tracks(&["kick"], 0, &placed, 4), vec![2]);
  }

  #[test]
  fn test_explode_a_pattern_clip() {
    let (mut graph, kick, pattern) = graph_with_pattern();
    const SNARE: &str = "assets/assets_66-sd-01.wav";
    {
      let mut seq = graph.sequencer.lock().unwrap();
      let snare = seq.add_row(SNARE, 44_100).unwrap();
      seq.set_step(pattern, snare, 4, true).unwrap();
    }
    graph.mixer.set_name(0, "Drums");
    let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(2_000), 0, None).unwrap();

    let ids = graph.explode_pattern_node(clip).unwrap();
    assert!(graph.get_pattern_node_by_id(clip).is_none());
    // no sample clips: a pattern clip per sample
    assert!(graph.nodes.is_empty());
    assert_eq!(ids.len(), 2);

    let made: Vec<PatternNode> = ids.iter().map(|&id| graph.get_pattern_node_by_id(id).unwrap().clone()).collect();
    // where the clip was, the kick on its track, the snare on its own
    assert_eq!(made.iter().map(|n| (n.start_offset, n.track_number)).collect::<Vec<_>>(),
      vec![(ms(2_000), 0), (ms(2_000), 1)]);
    {
      let seq = graph.sequencer.lock().unwrap();
      assert_eq!(seq.single_sample(made[0].pattern_id), Some(KICK));
      assert_eq!(seq.single_sample(made[1].pattern_id), Some(SNARE));
      assert!(seq.pattern(made[0].pattern_id).unwrap().lit(kick, 8));
      // the source pattern is still there
      assert_eq!(seq.patterns().len(), 3);
    }

    // a named track keeps its name, an unnamed one takes its sample's
    assert_eq!(graph.mixer.name(0).as_deref(), Some("Drums"));
    assert_eq!(graph.mixer.name(1).as_deref(), Some("assets_66-sd-01"));

    // a second copy, exploded, reuses the patterns and the tracks
    let copy = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(4_000), 3, None).unwrap();
    let again = graph.explode_pattern_node(copy).unwrap();
    let placed: Vec<(u64, u32)> = again.iter()
      .map(|&id| graph.get_pattern_node_by_id(id).unwrap())
      .map(|n| (n.pattern_id, n.track_number))
      .collect();
    assert_eq!(placed, vec![(made[0].pattern_id, 0), (made[1].pattern_id, 1)]);
    assert_eq!(graph.sequencer.lock().unwrap().patterns().len(), 3);

    // sample clips can't be exploded
    let sample = graph.construct_and_add_node(KICK.to_string(), ms(0), 0).unwrap();
    assert!(graph.explode_pattern_node(sample).is_err());
  }

  #[test]
  fn test_an_empty_pattern_clip_stays() {
    let mut graph = AudioGraph::new(44_100, 120., 16);
    let pattern = graph.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
    let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
    assert!(graph.explode_pattern_node(clip).is_err());
    assert!(graph.get_pattern_node_by_id(clip).is_some());
  }
}
