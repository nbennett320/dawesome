use crate::{
  daw,
  util,
};
use std::sync::{
  Arc, 
  Mutex,
};
use std::marker::{PhantomData};
use std::collections::BTreeSet;
use std::time::{Duration};
use std::vec::{Vec};
use std::fs::{File};
use std::io::{BufReader};
use rodio::{Decoder};
use rodio::source::{Source};

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
pub struct AudioNode {
  pub id: u64,
  pub start_offset: Duration,
  pub track_number: u32,
  sample_path: String,
  channels: u16,
  // the sample, decoded. shared by every copy of the clip (the halves of a
  // cut, the copies the history keeps)
  pcm: Arc<daw::Pcm>,
  // the sample's length, before the clip's settings change its speed
  duration: Duration,
  sample_rate: u32,
  // the part of the sample the clip plays, in the sample's own time. None:
  // all of it. `start_offset` is where that part starts in the playlist
  window: Option<daw::Window>,
  // the waveform, ready to send (see daw::encode_levels). shared, as the
  // clip is copied whenever it's remembered (see history.rs)
  waveform: Arc<str>,
  // how the clip is played back
  settings: daw::ClipSettings,
}

// a sample's length and the waveform drawn for it
// - samples: interleaved samples for all channels
// - sample_rate: the sample file's own rate
fn duration_and_waveform(
  sample_path: &str,
  samples: &[i16],
  channels: u16,
  sample_rate: u32,
) -> Result<(Duration, String), String> {
  if samples.is_empty() || channels == 0 {
    return Err(format!("sample '{}' contains no audio", sample_path));
  }
  let dur_s = (samples.len() as f32 / sample_rate as f32) as f32 / channels as f32;
  let duration = Duration::from_secs_f32(dur_s);
  let levels = daw::calc_waveform_from_samples(samples, channels, sample_rate)
    .ok_or_else(|| format!("sample '{}' is too short to draw a waveform", sample_path))?;

  Ok((duration, daw::encode_levels(&levels)))
}

// decode a sample's length and waveform without creating a node,
// so the frontend can draw a clip before it's dropped into the playlist
pub fn preview_sample(sample_path: &str) -> Result<(Duration, String), String> {
  let file_buf = BufReader::new(File::open(sample_path)
    .map_err(|e| format!("could not open sample '{}': {}", sample_path, e))?);
  let source = Decoder::new(file_buf)
    .map_err(|e| format!("could not decode sample '{}': {}", sample_path, e))?;
  let channels = source.channels();
  let sample_rate = source.sample_rate();
  let samples: Vec<i16> = source.collect();

  duration_and_waveform(sample_path, &samples, channels, sample_rate)
}

// a sample file, decoded: its sound, its length and its waveform
#[derive(Clone)]
struct Decoded {
  pcm: Arc<daw::Pcm>,
  duration: Duration,
  waveform: Arc<str>,
}

// what tells a file that's been written again from the one that was read:
// its size and when it was last changed
type FileStamp = (u64, Option<std::time::SystemTime>);

fn file_stamp(path: &str) -> Option<FileStamp> {
  let metadata = std::fs::metadata(path).ok()?;
  Some((metadata.len(), metadata.modified().ok()))
}

lazy_static! {
  // the samples clips are playing, by path, so every clip of a file shares
  // one decoded copy of it (a long file takes tens of megabytes). an entry
  // lasts as long as some clip, or the undo history, still holds the sample
  static ref DECODED: Mutex<std::collections::HashMap<String, (FileStamp, std::sync::Weak<daw::Pcm>, Duration, std::sync::Weak<str>)>> =
    Mutex::new(std::collections::HashMap::new());
}

// sample files moved: the sounds decoded from them are known by where they
// are now, so a new clip of a moved file shares the sound its clips already
// play, and whatever file turns up where it was is read afresh
// - to: a path's new place, if it has one
pub fn decoded_moved(to: &dyn Fn(&str) -> Option<String>) {
  let mut known = DECODED.lock().unwrap();
  let moved: Vec<(String, String)> = known.keys()
    .filter_map(|path| to(path).map(|new| (path.clone(), new)))
    .collect();
  for (from, new) in moved {
    if let Some((_, pcm, duration, waveform)) = known.remove(&from) {
      // (as the file is now: a move may have changed when it says it was changed)
      if let Some(stamp) = file_stamp(&new) {
        known.insert(new, (stamp, pcm, duration, waveform));
      }
    }
  }
}

// a sample file's sound, length and waveform: as already decoded for
// another clip, if the file hasn't changed since, or decoded now
fn decoded(sample_path: &str) -> Result<Decoded, String> {
  let stamp = file_stamp(sample_path);
  if let Some(stamp) = stamp {
    if let Some((had, pcm, duration, waveform)) = DECODED.lock().unwrap().get(sample_path) {
      if let (true, Some(pcm), Some(waveform)) = (*had == stamp, pcm.upgrade(), waveform.upgrade()) {
        return Ok(Decoded { pcm, duration: *duration, waveform });
      }
    }
  }

  // (decoded without the lock: it takes a while)
  let pcm = daw::Pcm::load(sample_path)?;
  let (duration, waveform) =
    duration_and_waveform(sample_path, pcm.samples(), pcm.channels(), pcm.rate())?;
  let decoded = Decoded { pcm: Arc::new(pcm), duration, waveform: waveform.into() };

  if let Some(stamp) = stamp {
    let mut known = DECODED.lock().unwrap();
    known.retain(|_, (_, pcm, _, _)| pcm.strong_count() > 0);
    known.insert(
      sample_path.to_string(),
      (stamp, Arc::downgrade(&decoded.pcm), decoded.duration, Arc::downgrade(&decoded.waveform)),
    );
  }
  Ok(decoded)
}

impl AudioNode {
  pub fn new(
    id: u64,
    sample_path: String,
    start_offset: Duration,
    track_number: u32,
  ) -> Result<Self, String> {
    let Decoded { pcm, duration, waveform } = decoded(&sample_path)?;
    let channels = pcm.channels();
    let sample_rate = pcm.rate();
    println!("length of sample in ms: {:?}", duration.as_millis());
    println!("channels: {:?}", channels);
    println!("track number: {}", track_number);

    Ok(AudioNode {
      id,
      sample_path,
      start_offset,
      track_number,
      channels,
      pcm,
      duration,
      sample_rate,
      window: None,
      waveform,
      settings: daw::ClipSettings::default(),
    })
  }

  // how long the clip plays for, once its settings change its speed
  pub fn duration(&self) -> Duration {
    match self.window {
      None => self.settings.played_duration(self.duration),
      Some(_) => self.played(self.window_end()).saturating_sub(self.played(self.window_start())),
    }
  }

  // a length of the sample, as long as it plays for at the clip's speed
  fn played(&self, of_sample: Duration) -> Duration {
    self.settings.played_exactly(of_sample)
  }

  // the part of the sample the clip plays (None: all of it)
  pub fn window(&self) -> Option<daw::Window> {
    self.window
  }

  // where in the sample the clip starts, and where it ends
  pub fn window_start(&self) -> Duration {
    self.window.map_or(Duration::ZERO, |window| window.start)
  }

  pub fn window_end(&self) -> Duration {
    self.window.and_then(|window| window.end).unwrap_or(self.duration)
  }

  // play only part of the sample: from `start` to `end` of it, in the
  // sample's own time, kept within the sample and at least MIN_CLIP of
  // played sound long. the clip's start in the playlist isn't moved (see
  // AudioGraph::set_window, which keeps the sound where it is). returns the
  // window kept. a window that's the whole sample is None
  pub fn set_window(&mut self, start: Duration, end: Option<Duration>) -> Option<daw::Window> {
    // the least of the sample that plays for MIN_CLIP
    let least = MIN_CLIP.mul_f32(self.settings.playback_rate()).min(self.duration);
    // (an end too near the start is moved later, where there's room)
    let start = start.min(self.duration.saturating_sub(least));
    let end = end.map(|end| end.max(start + least)).filter(|end| *end < self.duration);

    self.window = if start.is_zero() && end.is_none() { None } else { Some(daw::Window { start, end }) };
    // the fades are kept within what's left
    self.set_settings(self.settings);
    self.window
  }

  // the part of the sample to play, as it was saved in a project: kept
  // within the sample, but otherwise as it is (a clip a hair under the
  // shortest a trim allows, which a cut can leave, isn't moved)
  pub fn restore_window(&mut self, start: Duration, end: Option<Duration>) {
    let end = end.filter(|end| *end < self.duration);
    let last = end.unwrap_or(self.duration);
    if start >= last { return; }

    self.window = if start.is_zero() && end.is_none() { None } else { Some(daw::Window { start, end }) };
    self.set_settings(self.settings);
  }

  // where the sample's first frame would be heard, were all of it played:
  // the clip's start, less the part of the sample before its window. in
  // nanoseconds, as it can be before the playlist's start
  //
  // every clip cut from one clip has the same origin, to the nanosecond:
  // their starts are worked out from it (see AudioGraph::split_node)
  pub fn origin(&self) -> i128 {
    self.start_offset.as_nanos() as i128 - self.played(self.window_start()).as_nanos() as i128
  }

  // where in the playlist a point of the sample is heard
  fn heard_at(&self, of_sample: Duration) -> i128 {
    self.origin() + self.played(of_sample).as_nanos() as i128
  }

  // the point of the sample heard at a time in the playlist (the sample's
  // start, for a time before its origin)
  fn sample_at(&self, at: Duration) -> Duration {
    let since_origin = (at.as_nanos() as i128 - self.origin()).max(0);
    self.settings.sample_time(Duration::from_nanos(since_origin as u64))
  }

  // the least of the sample the clip can play from: what's heard at the
  // playlist's start, when the sample's own start would be before it
  fn earliest_start(&self) -> Duration {
    let mut start = self.sample_at(Duration::ZERO);
    // (rounding can leave it a nanosecond short)
    while self.heard_at(start) < 0 { start += Duration::from_nanos(1); }
    start
  }

  // the clip's audio at `rate`, to play in a run of the engine
  // - starts_at: when the voice starts, in nanoseconds into the run. after
  //   the clip's start, for one picked up part-way through
  // - run_zero: where the run's start is in the playlist, in nanoseconds
  //   (the time that's frame 0 of the run)
  //
  // which frames it plays is worked out from the frames of the run its
  // origin, start and end fall on, so a clip that ends where another of the
  // same sample carries on ends on the frame before the other's first
  pub fn voice(&self, rate: u32, starts_at: i128, run_zero: i128) -> daw::ClipReader {
    let origin = run_frame(self.origin() - run_zero, rate);
    let lead = (run_frame(starts_at, rate) - origin).max(0) as u64;
    let end = self.window.and_then(|window| window.end).map(|end|
      (run_frame(self.heard_at(end) - run_zero, rate) - origin).max(0) as u64);

    self.reader(rate, lead, end)
  }

  fn reader(&self, rate: u32, lead: u64, end: Option<u64>) -> daw::ClipReader {
    // where the clip starts and ends, in played time from the sample's
    // start, which its fades are measured from. a clip that plays to the
    // sample's end ends where the whole sample does, as a clip of all of
    // it has it
    let fades = (
      self.played(self.window_start()),
      match self.window.and_then(|window| window.end) {
        Some(end) => self.played(end),
        None => self.settings.played_duration(self.duration),
      },
    );
    daw::ClipReader::new(self.pcm.clone(), &self.settings, rate, fades, lead, end)
  }

  // whether two clips play the very same decoded sound (not two copies)
  pub fn shares_sound_with(&self, other: &AudioNode) -> bool {
    Arc::ptr_eq(&self.pcm, &other.pcm)
  }

  // the sample file's own rate
  pub fn sample_rate(&self) -> u32 {
    self.sample_rate
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

  // the sample file has moved (its sound is already loaded)
  pub fn set_sample_path(&mut self, path: String) {
    self.sample_path = path;
  }

  // change how the clip is played back, returns the settings kept in range
  // (the fades within the part of the sample the clip plays)
  pub fn set_settings(&mut self, settings: daw::ClipSettings) -> daw::ClipSettings {
    let plays = self.window_end().saturating_sub(self.window_start());
    self.settings = settings.clamped(plays);
    self.settings
  }

  // the clip's audio on its own, played with its settings (always stereo),
  // at the rate its pitch gives it
  pub fn source(&self) -> daw::ClipReader {
    let rate = daw::clip_reader::pitched_rate(self.sample_rate, &self.settings);
    // (at that rate a frame out is a frame of the sample)
    let frames = |of_sample: Duration| (of_sample.as_secs_f64() * self.sample_rate as f64).round() as u64;
    let end = self.window.and_then(|window| window.end).map(frames);

    self.reader(rate, frames(self.window_start()), end)
  }
  
  pub fn get_waveform(&self) -> &str {
    &self.waveform
  }

  // whether another clip plays the same sample the same way on the same
  // track (wherever it starts)
  pub fn same_clip(&self, other: &AudioNode) -> bool {
    self.sample_path == other.sample_path
      && self.track_number == other.track_number
      && self.settings == other.settings
      && self.window == other.window
  }
}

// the shortest a sample clip can be trimmed or cut to, in played time
pub const MIN_CLIP: Duration = Duration::from_millis(10);

// the frame of a run a time falls on: `at` nanoseconds into the run, which
// may be before it. rounded as the engine rounds a voice's start (see
// Engine::frames_in)
pub fn run_frame(at: i128, rate: u32) -> i64 {
  let frames = (Duration::from_nanos(at.unsigned_abs() as u64).as_secs_f64() * rate as f64).round() as i64;
  if at < 0 { -frames } else { frames }
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

// a pattern placed in the playlist. it stays linked to its pattern, so it
// plays whatever the pattern holds now, and is as long as the pattern is
// (or as the part of it that it plays). its hits go to its track's bus, and
// its notes to its track's copy of each synth (see run_notes)
#[derive(Clone, Debug, PartialEq)]
pub struct PatternNode {
  pub id: u64,
  pub pattern_id: u64,
  pub start_offset: Duration,
  pub track_number: u32,
  // the part of the pattern the clip plays. None: all of it.
  // `start_offset` is where that part starts in the playlist
  pub window: Option<PatternWindow>,
}

// the part of a pattern a clip plays, in steps from the pattern's start
// (not always whole steps: a clip can be cut between two)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatternWindow {
  pub start: f64,
  // None: to the pattern's end, wherever that is now
  pub end: Option<f64>,
}

// the shortest a pattern clip can be trimmed or cut to, in steps
pub const MIN_PATTERN_CLIP: f64 = 1.;

// how long a number of steps lasts
fn steps_time(steps: f64, beat_interval: Duration) -> Duration {
  Duration::from_secs_f64(beat_interval.as_secs_f64() * steps.max(0.) / daw::STEPS_PER_BEAT as f64)
}

impl PatternNode {
  // a clip of a whole pattern
  pub fn new(id: u64, pattern_id: u64, start_offset: Duration, track_number: u32) -> Self {
    PatternNode { id, pattern_id, start_offset, track_number, window: None }
  }

  // the steps the clip spans in a pattern `length` steps long: from, to.
  // the window is kept as it was set, so a pattern made shorter and then
  // longer again gives the clip its steps back. a window past the pattern's
  // end leaves a clip one step long, which plays nothing
  pub fn span(&self, length: u32) -> (f64, f64) {
    let length = length as f64;
    match self.window {
      None => (0., length),
      Some(window) => {
        let end = window.end.map_or(length, |end| end.min(length));
        (window.start, end.max(window.start + MIN_PATTERN_CLIP))
      }
    }
  }

  // play only part of the pattern (see AudioGraph::set_pattern_window,
  // which keeps the hits where they are in the playlist). kept within the
  // pattern, `length` steps long, and at least a step long. returns the
  // window kept. a window that's the whole pattern is None
  pub fn set_window(&mut self, start: f64, end: Option<f64>, length: u32) -> Option<PatternWindow> {
    let length = length as f64;
    // (a number that makes no sense is the pattern's start, or its end)
    let start = if start.is_finite() { start } else { 0. };
    // (an end too near the start is moved later, where there's room)
    let start = start.min(length - MIN_PATTERN_CLIP).max(0.);
    let end = end
      .filter(|end| end.is_finite())
      .map(|end| end.max(start + MIN_PATTERN_CLIP))
      .filter(|end| *end < length);

    self.window = if start == 0. && end.is_none() { None } else { Some(PatternWindow { start, end }) };
    self.window
  }

  // where the pattern's first step would be heard, were all of it played,
  // in nanoseconds (it can be before the playlist's start). the halves of a
  // cut clip have the same origin, to the nanosecond
  pub fn origin(&self, beat_interval: Duration) -> i128 {
    let before = self.window.map_or(0., |window| window.start);
    self.start_offset.as_nanos() as i128 - steps_time(before, beat_interval).as_nanos() as i128
  }

  // where in the playlist a point of the pattern is heard
  fn heard_at(&self, steps: f64, beat_interval: Duration) -> i128 {
    self.origin(beat_interval) + steps_time(steps, beat_interval).as_nanos() as i128
  }

  // how long the clip is, for a pattern `length` steps long
  pub fn duration(&self, length: u32, beat_interval: Duration) -> Duration {
    let (from, to) = self.span(length);
    steps_time(to, beat_interval).saturating_sub(steps_time(from, beat_interval))
  }

  // whether the clip plays a hit on `step` of its pattern
  pub fn plays(&self, step: u32, length: u32) -> bool {
    let (from, to) = self.span(length);
    step < length && step as f64 >= from && (step as f64) < to
  }

  // when a hit on `step` is heard
  pub fn hit_at(&self, step: u32, beat_interval: Duration) -> Duration {
    let at = self.origin(beat_interval) + daw::step_offset(step, beat_interval).as_nanos() as i128;
    Duration::from_nanos(at.max(0) as u64)
  }
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

// a sound in a slice of a run: how far into the slice it starts, the sound,
// and where it's heard
pub type SliceSound = (Duration, daw::VoiceSource, daw::Route);

// a note in a slice of a run, for a synth: how far into the slice it
// starts, and ends (which can be in a later slice: the note-off is sent with
// the note-on, so an edit made while it sounds can't leave it hanging), and
// where it's heard: on a track (that track's copy of the synth) or, with
// none, straight on the master (the master's copy: a pattern on its own)
#[derive(Clone, Debug)]
pub struct SliceNote {
  pub on: Duration,
  pub off: Duration,
  pub synth: u64,
  pub route: Option<daw::TrackRoute>,
  pub pitch: u8,
  pub velocity: f32,
}

impl SliceNote {
  // - on, off: nanoseconds into the slice
  fn new(on: i128, off: i128, synth: u64, route: Option<daw::TrackRoute>, note: &daw::PatternNote) -> SliceNote {
    let at = |ns: i128| Duration::from_nanos(ns.max(0) as u64);
    SliceNote { on: at(on), off: at(off), synth, route, pitch: note.pitch, velocity: note.velocity }
  }
}

// the slot id of each copy of a synth that plays: by (synth, track), none
// being the master's copy (see effects.rs)
pub type CopyIds = std::collections::HashMap<(u64, Option<u32>), u64>;

// send a slice's notes to the engine, as part of run `run`, the slice
// starting `at` into the run: each to its synth's copy where it's heard. a
// note for a copy that isn't there (its plugin is missing, or still being
// loaded) isn't heard
pub fn play_slice_notes(engine: &daw::Engine, run: u64, at: Duration, notes: Vec<SliceNote>, copies: &CopyIds) -> Result<(), String> {
  let mut by_copy: std::collections::BTreeMap<(Option<u32>, u64), (Option<daw::TrackRoute>, Vec<daw::TimedNote>)> = Default::default();
  for note in notes {
    let track = note.route.as_ref().map(|route| route.number);
    let Some(&copy) = copies.get(&(note.synth, track)) else { continue };
    let (_, timed) = by_copy.entry((track, copy)).or_insert_with(|| (note.route.clone(), Vec::new()));
    timed.push(daw::TimedNote::on(engine.frames_in(at + note.on), note.pitch, note.velocity));
    timed.push(daw::TimedNote::off(engine.frames_in(at + note.off), note.pitch));
  }
  for ((_, copy), (route, timed)) in by_copy {
    match route {
      Some(route) => engine.play_notes(&route, copy, Some(run), timed)?,
      None => engine.play_master_notes(copy, Some(run), timed)?,
    }
  }
  Ok(())
}

// the metronome's tick, at the start of a slice

fn metronome_tick() -> SliceSound {
  (Duration::ZERO, Box::new(daw::METRONOME_TICK_SOURCE.convert_samples()), daw::Route::Direct)
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

  // the sounds of the next `dur` of a run of the graph, each with how far
  // into the slice it starts. clips play on their track's bus, and
  // the metronome (on the beat, when `metronome` is set) straight to the
  // output, so the mixer doesn't change it
  // - resume: the run's first slice, so clips that started before it and are
  //   still sounding are played from where the playhead is in them
  pub fn run_sounds(&self, dur: Duration, metronome: bool, resume: bool) -> Vec<SliceSound> {
    self.run_sounds_at(Duration::ZERO, self.sample_rate, dur, metronome, resume)
  }

  // - at: how far into the run the slice starts
  // - rate: the engine's rate, which sample clips are read out at
  pub fn run_sounds_at(&self, at: Duration, rate: u32, dur: Duration, metronome: bool, resume: bool) -> Vec<SliceSound> {
    if !self.has_clips() {
      println!("Tried to run audio graph. No nodes in graph!");
      return Vec::new();
    }

    if !self.running { return Vec::new(); }

    let mut sounds = Vec::new();
    if metronome { sounds.push(metronome_tick()); }
    sounds.extend(self.slice_sounds(at, rate, dur, resume));
    sounds
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

  // the clips that start in the next `dur`, mixed without the track strips
  // (those run in the engine; the metronome isn't included, see run_sounds)
  #[cfg(test)]
  pub fn buffer_slice(&self, dur: Duration) -> Option<(Arc<rodio::dynamic_mixer::DynamicMixerController<f32>>, rodio::dynamic_mixer::DynamicMixer<f32>)> {
    Some(self.slice_mix(dur, false))
  }

  #[cfg(test)]
  fn slice_mix(&self, dur: Duration, resume: bool) -> (Arc<rodio::dynamic_mixer::DynamicMixerController<f32>>, rodio::dynamic_mixer::DynamicMixer<f32>) {
    let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(2, self.sample_rate);
    controller.add(rodio::source::Zero::<f32>::new(2, self.sample_rate).take_duration(dur));
    for (delay, source, _) in self.slice_sounds(Duration::ZERO, self.sample_rate, dur, resume) {
      controller.add(source.delay(delay));
    }
    (controller, mixer)
  }

  // the clips that start in the next `dur`, each routed to its track's bus,
  // also playing the rest of every clip already sounding at the slice's
  // start when `resume` is set
  // - at: how far into the run the slice starts
  // - rate: the engine's rate
  fn slice_sounds(&self, at: Duration, rate: u32, dur: Duration, resume: bool) -> Vec<SliceSound> {
    let mut sounds: Vec<SliceSound> = Vec::new();
    let mut add = |delay: Duration, source: daw::VoiceSource, track_number: u32| {
      sounds.push((delay, source, daw::Route::Track(self.mixer.track_route(track_number))));
    };

    let window_start = self.current_offset.unwrap_or(Duration::ZERO);
    println!("current offset: {}", window_start.as_millis());
    // where the run's frame 0 is in the playlist
    let run_zero = window_start.as_nanos() as i128 - at.as_nanos() as i128;

    if resume {
      for node in self.nodes_sounding_at(window_start) {
        let played = window_start - node.start_offset;
        println!("resumed node: {} ({}ms in)", node.sample_path, played.as_millis());
        let voice = node.voice(rate, at.as_nanos() as i128, run_zero);
        add(Duration::ZERO, Box::new(voice), node.track_number);
      }
    }

    // every node that starts within this time slice
    for node in self.nodes_in_window(window_start, dur) {
      let delay_offset = node.start_offset - window_start;
      println!("added node: {} (delay offset: {}ms)", node.sample_path, delay_offset.as_millis());
      let voice = node.voice(rate, (at + delay_offset).as_nanos() as i128, run_zero);
      add(delay_offset, Box::new(voice), node.track_number);
    }

    // and every pattern clip's hits, on their track's bus
    if !self.pattern_nodes.is_empty() {
      let sequencer = self.sequencer.lock().unwrap();
      let beat = self.beat_interval();
      for node in &self.pattern_nodes {
        let length = sequencer.pattern(node.pattern_id).map_or(0, |pattern| pattern.length);
        for (row, step) in sequencer.hit_steps(node.pattern_id) {
          // only the hits in the part of the pattern the clip plays
          if !node.plays(step, length) { continue; }
          let at = node.hit_at(step, beat);
          match schedule_in_window(at, row.duration(), window_start, dur, resume) {
            Some(Scheduled::Delay(d)) =>
              add(d, Box::new(row.source()), node.track_number),
            Some(Scheduled::Skip(d)) =>
              add(Duration::ZERO, Box::new(row.source().skip_duration(d)), node.track_number),
            None => {}
          }
        }
      }
    }

    sounds
  }

  // the sounds of the next `dur` of a pattern played on its own (in the
  // Sequencer tab). the graph's current offset is where the run is in the
  // pattern. like run_sounds, but the hits go straight to the master bus
  // (there's no playlist track), and only the pattern plays
  pub fn pattern_sounds(
    &self,
    pattern_id: u64,
    dur: Duration,
    metronome: bool,
    resume: bool,
  ) -> Vec<SliceSound> {
    let mut sounds = Vec::new();
    if metronome { sounds.push(metronome_tick()); }

    let window_start = self.current_offset.unwrap_or(Duration::ZERO);
    let sequencer = self.sequencer.lock().unwrap();
    for (row, at) in sequencer.hits(pattern_id, self.beat_interval()) {
      let sound: SliceSound = match schedule_in_window(at, row.duration(), window_start, dur, resume) {
        Some(Scheduled::Delay(d)) => (d, Box::new(row.source()), daw::Route::Master),
        Some(Scheduled::Skip(d)) => (Duration::ZERO, Box::new(row.source().skip_duration(d)), daw::Route::Master),
        None => continue,
      };
      sounds.push(sound);
    }

    sounds
  }

  // the notes that start in the next `dur` of a run of the graph, for the
  // synths' copies on the pattern clips' tracks: each pattern clip's notes
  // that start in the part of its pattern it plays, each ending where it
  // ends or at the clip's end, whichever comes first. a note that started before the
  // slice isn't played (an instrument can't start partway through a note),
  // so a run from the middle of one leaves it out
  //
  // - loop_end: where the playlist jumps back to its start, when it loops.
  //   a note still sounding there ends there, so it doesn't run on into the
  //   next time round (and cut short the same pitch starting again at the
  //   top). one starting at or after it isn't played
  pub fn run_notes(&self, dur: Duration, loop_end: Option<Duration>) -> Vec<SliceNote> {
    if !self.running || self.pattern_nodes.is_empty() { return Vec::new(); }

    let window_start = self.current_offset.unwrap_or(Duration::ZERO).as_nanos() as i128;
    let window_end = window_start + dur.as_nanos() as i128;
    let loop_end = loop_end.map_or(i128::MAX, |end| end.as_nanos() as i128);
    let beat = self.beat_interval();
    let sequencer = self.sequencer.lock().unwrap();
    let mut notes = Vec::new();
    for node in &self.pattern_nodes {
      let pattern = match sequencer.pattern(node.pattern_id) {
        Some(pattern) if !pattern.notes.is_empty() => pattern,
        _ => continue,
      };
      let (from, to) = node.span(pattern.length);
      let route = self.mixer.track_route(node.track_number);
      for (&synth, synth_notes) in &pattern.notes {
        for note in synth_notes.iter().filter(|note| note.start >= from && note.start < to) {
          let on = node.heard_at(note.start, beat);
          if on < window_start || on >= window_end || on >= loop_end { continue; }
          let off = node.heard_at(note.end().min(to), beat).min(loop_end);
          notes.push(SliceNote::new(on - window_start, off - window_start, synth, Some(route.clone()), note));
        }
      }
    }
    notes
  }

  // which synths each track plays: (synth, track) for every pattern clip
  // with notes of that synth that it plays some of (or would, were its
  // pattern longer). each needs a copy of the synth on that track
  pub fn synth_tracks(&self) -> BTreeSet<(u64, u32)> {
    let sequencer = self.sequencer.lock().unwrap();
    let mut pairs = BTreeSet::new();
    for node in &self.pattern_nodes {
      let Some(pattern) = sequencer.pattern(node.pattern_id) else { continue };
      for (&synth, notes) in &pattern.notes {
        if !notes.is_empty() { pairs.insert((synth, node.track_number)); }
      }
    }
    pairs
  }

  // the notes of the next `dur` of a pattern played on its own (see
  // pattern_sounds), for the synths' copies on the master
  pub fn pattern_notes(&self, pattern_id: u64, dur: Duration) -> Vec<SliceNote> {
    let window_start = self.current_offset.unwrap_or(Duration::ZERO).as_nanos() as i128;
    let window_end = window_start + dur.as_nanos() as i128;
    let beat = self.beat_interval();
    let sequencer = self.sequencer.lock().unwrap();
    let pattern = match sequencer.pattern(pattern_id) {
      Some(pattern) => pattern,
      None => return Vec::new(),
    };
    let end = pattern.length as f64;
    pattern.notes.iter()
      .flat_map(|(&synth, notes)| notes.iter().map(move |note| (synth, note)))
      .filter(|(_, note)| note.start < end)
      .filter_map(|(synth, note)| {
        let on = steps_time(note.start, beat).as_nanos() as i128;
        if on < window_start || on >= window_end { return None; }
        let off = steps_time(note.end().min(end), beat).as_nanos() as i128;
        Some(SliceNote::new(on - window_start, off - window_start, synth, None, note))
      })
      .collect()
  }

  // change the tempo. clips keep their beat: one on beat 9 is still on beat
  // 9, so it starts sooner or later in time. (how long a sample clip plays
  // for doesn't change)
  pub fn set_tempo(&mut self, tempo: f32) {
    let before = self.beat_interval();
    self.tempo = tempo;
    let after = self.beat_interval();
    if before.is_zero() || before == after { return; }

    for node in self.nodes.iter_mut() {
      node.start_offset = daw::timing::keep_beat(node.start_offset, before, after);
    }
    for node in self.pattern_nodes.iter_mut() {
      node.start_offset = daw::timing::keep_beat(node.start_offset, before, after);
    }
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
  // add a node made elsewhere (so its sample was decoded without holding the
  // graph), giving it the next id. returns the id
  pub fn insert_node(&mut self, mut node: AudioNode) -> u64 {
    let id = self.next_id;
    node.id = id;
    self.add_node(node);
    id
  }

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
    self.add_pattern_node_with_id(PatternNode::new(id, pattern_id, start_offset, track_number));
    Ok(id)
  }

  // add a copy of a pattern clip, with a new id, which is returned
  pub fn insert_pattern_node(&mut self, mut node: PatternNode) -> u64 {
    let id = self.next_id;
    node.id = id;
    self.add_pattern_node_with_id(node);
    id
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
      // each plays the same part of its pattern as the clip did of its
      self.add_pattern_node_with_id(PatternNode {
        id: new_id,
        pattern_id: *pattern_id,
        start_offset: clip.start_offset,
        track_number: track,
        window: clip.window,
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

    // a pattern clip ends with its pattern, or with the part of it that it
    // plays (its last hits ring past it)
    let sequencer = self.sequencer.lock().unwrap();
    let beat = self.beat_interval();
    self.pattern_nodes
      .iter()
      .filter_map(|node| sequencer.pattern(node.pattern_id)
        .map(|pattern| node.start_offset + node.duration(pattern.length, beat)))
      .fold(clips, Duration::max)
  }

  // play only part of a sample clip's sample: from `start` to `end` of it
  // (None: to its end), in the sample's own time. the sound stays where it
  // is in the playlist: a later start hides the clip's beginning rather
  // than moving the rest earlier. the clip can't start before the
  // playlist does
  pub fn set_window(&mut self, id: u64, start: Duration, end: Option<Duration>) -> Result<(), String> {
    let node = self.nodes.iter_mut().find(|n| n.id == id)
      .ok_or_else(|| format!("no sample clip with id {}", id))?;
    let origin = node.origin();
    let earliest = node.earliest_start();

    let start = node.set_window(start.max(earliest), end).map_or(Duration::ZERO, |window| window.start);
    // (worked out from the origin, so the origin doesn't move by a nanosecond)
    node.start_offset = Duration::from_nanos((origin + node.played(start).as_nanos() as i128).max(0) as u64);

    self.nodes.sort_by(|a, b| a.start_offset.cmp(&b.start_offset));
    Ok(())
  }

  // stretch a sample clip, like a tape: its pitch is set, and with it how
  // fast, and so how long, it plays. one of its edges stays where it is in
  // the playlist: its start, or (`keep_end`) its end, with the start moving
  // (no earlier than the playlist's). returns the settings kept
  pub fn stretch_node(&mut self, id: u64, pitch: f32, keep_end: bool) -> Result<daw::ClipSettings, String> {
    let node = self.nodes.iter_mut().find(|n| n.id == id)
      .ok_or_else(|| format!("no sample clip with id {}", id))?;
    let end = node.start_offset + node.duration();

    let settings = node.set_settings(daw::ClipSettings { pitch, ..node.settings });
    if keep_end {
      node.start_offset = end.saturating_sub(node.duration());
    }

    self.nodes.sort_by(|a, b| a.start_offset.cmp(&b.start_offset));
    Ok(settings)
  }

  // the same for a pattern clip, in steps of its pattern
  pub fn set_pattern_window(&mut self, id: u64, start: f64, end: Option<f64>) -> Result<(), String> {
    let beat = self.beat_interval();
    let node = self.pattern_nodes.iter_mut().find(|n| n.id == id)
      .ok_or_else(|| format!("no pattern clip with id {}", id))?;
    let length = self.sequencer.lock().unwrap().pattern(node.pattern_id)
      .map(|pattern| pattern.length)
      .ok_or_else(|| format!("no pattern with id {}", node.pattern_id))?;
    let origin = node.origin(beat);
    // the steps before the playlist's start can't be played
    let earliest = if origin >= 0 || beat.is_zero() { 0. } else {
      (-origin) as f64 / 1e9 / beat.as_secs_f64() * daw::STEPS_PER_BEAT as f64
    };

    let start = node.set_window(start.max(earliest), end, length).map_or(0., |window| window.start);
    node.start_offset = Duration::from_nanos((origin + steps_time(start, beat).as_nanos() as i128).max(0) as u64);
    Ok(())
  }

  // cut a clip of either kind in two at a time in the playlist. the clip
  // keeps what's before the cut, and a new clip, whose id is returned, plays
  // what's after it. the two play what the one did (see clip_reader.rs)
  //
  // a sample clip's fade in stays with the first half and its fade out goes
  // with the second, each shortened to its half if need be
  //
  // None, with nothing changed, when the cut isn't inside the clip, or
  // would leave either half shorter than a clip can be
  pub fn split_node(&mut self, id: u64, at: Duration) -> Result<Option<u64>, String> {
    let new_id = self.next_id;

    if let Some(node) = self.nodes.iter().find(|n| n.id == id) {
      let end = node.start_offset + node.duration();
      if at < node.start_offset + MIN_CLIP || at + MIN_CLIP > end { return Ok(None); }

      // the point of the sample at the cut, which both halves are given, so
      // the first ends on the very frame the second starts on
      let cut = node.sample_at(at);
      let mut right = node.clone();
      right.id = new_id;
      right.start_offset = Duration::from_nanos(node.heard_at(cut).max(0) as u64);
      right.window = Some(daw::Window { start: cut, end: node.window.and_then(|window| window.end) });
      right.set_settings(daw::ClipSettings { attack_ms: 0., ..right.settings });

      let left = self.nodes.iter_mut().find(|n| n.id == id).expect("the clip that was just found");
      left.window = Some(daw::Window { start: left.window_start(), end: Some(cut) });
      left.set_settings(daw::ClipSettings { decay_ms: 0., ..left.settings });

      self.add_node(right);
      return Ok(Some(new_id));
    }

    let beat = self.beat_interval();
    let node = self.pattern_nodes.iter().find(|n| n.id == id)
      .ok_or_else(|| format!("no node with id {}", id))?;
    let length = self.sequencer.lock().unwrap().pattern(node.pattern_id)
      .map(|pattern| pattern.length)
      .ok_or_else(|| format!("no pattern with id {}", node.pattern_id))?;
    let (from, to) = node.span(length);
    // (a step's length in seconds, not rounded to a whole nanosecond)
    let step = beat.as_secs_f64() / daw::STEPS_PER_BEAT as f64;
    if !(step > 0.) { return Ok(None); }
    let cut = from + (at.as_secs_f64() - node.start_offset.as_secs_f64()) / step;
    // (without the rounding noise, so a cut on a step is on it)
    let cut = (cut * 1e6).round() / 1e6;
    if cut < from + MIN_PATTERN_CLIP || cut + MIN_PATTERN_CLIP > to { return Ok(None); }

    let mut right = node.clone();
    right.id = new_id;
    right.start_offset = Duration::from_nanos(node.heard_at(cut, beat).max(0) as u64);
    right.window = Some(PatternWindow { start: cut, end: node.window.and_then(|window| window.end) });

    let left = self.pattern_nodes.iter_mut().find(|n| n.id == id).expect("the clip that was just found");
    left.window = Some(PatternWindow { start: from, end: Some(cut) });

    self.add_pattern_node_with_id(right);
    Ok(Some(new_id))
  }

  pub fn duration_max(&self) -> Duration {
    self.beat_interval() * self.max_beats as u32
  }

  // count number of nodes
  pub fn len(&self) -> usize {
    let res = self.nodes.len();
    res
  }

  pub fn get_mut_node_by_id(&mut self, id: u64) -> Option<&mut AudioNode> {
    self.nodes.iter_mut().find(|e| { e.id == id })
  }

  pub fn get_node_by_id(&self, id: u64) -> Option<&AudioNode> {
    self.nodes.iter().find(|e| { e.id == id })
  }

  pub fn find_nearest_offset_subdivision<T: daw::timing::MusicalTiming>(
    &self, 
    offset: Duration, 
    subdivision: T
  ) -> Duration {
    let interval = daw::timing::subdivision_interval_from_tempo(
      self.tempo,
      subdivision);

    // in nanoseconds: a beat is rarely a whole number of microseconds, and
    // what's lost in each would add up along a long playlist
    if interval.is_zero() { return offset; }
    let nearest_offset_nanos = util::math::round_to_nearest_multiple(
      offset.as_nanos(),
      interval.as_nanos());

    Duration::from_nanos(nearest_offset_nanos.try_into().unwrap())
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
  use std::time::Instant;

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

    let node = AudioNode::new(id, sample_path, start_offset, track_number).unwrap();

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

    let node = AudioNode::new(id, sample_path, start_offset, track_number).unwrap();
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

  // render a slice's sounds through an engine, as a run would play them,
  // returning the left channel
  fn render_run(sounds: Vec<SliceSound>, rate: u32, frames: usize) -> Vec<f32> {
    let (engine, mut renderer) = daw::Engine::new(rate);
    let voices = sounds
      .into_iter()
      .map(|(delay, source, route)| engine.boxed_voice(source, Some(1), engine.frames_in(delay), route))
      .collect();
    engine.start_run(1, daw::Mixer::default().master(), voices).unwrap();

    let mut out = vec![0f32; frames * 2];
    renderer.render(&mut out, Instant::now(), Duration::ZERO);
    out.chunks(2).map(|f| f[0]).collect()
  }

  fn first_sound(samples: &[f32]) -> Option<usize> {
    samples.iter().position(|&s| s != 0.)
  }

  #[test]
  fn test_audiograph_run_plays_clips_on_their_frame() {
    let rate = 44_100;
    let mut audiograph = graph_with_ticks(&[650]);
    audiograph.init();
    let second = Duration::from_secs(1);

    // where the tick's sound starts, in the sample itself
    let tick = render_run(vec![metronome_tick()], rate, 2_000);
    let into_tick = first_sound(&tick).expect("the tick is silent");

    // the clip starts exactly 650ms (28,665 frames) into the run
    let clip = render_run(audiograph.run_sounds(second, false, false), rate, rate as usize);
    assert_eq!(first_sound(&clip), Some(28_665 + into_tick));

    // with the metronome on, its tick starts the slice
    let with_tick = render_run(audiograph.run_sounds(second, true, false), rate, rate as usize);
    assert_eq!(first_sound(&with_tick), Some(into_tick));

    // the metronome doesn't play without a run of the graph
    audiograph.pause();
    assert!(audiograph.run_sounds(second, true, false).is_empty());
  }

  #[test]
  fn test_clips_keep_their_beat_when_the_tempo_changes() {
    let rate = 44_100;
    // a tick on beat 2 at 120bpm (1s in), and a pattern clip on beat 4
    let mut audiograph = graph_with_ticks(&[1_000]);
    audiograph.pattern_nodes.push(PatternNode::new(99, 0, ms(2_000), 1));
    let played = audiograph.nodes[0].duration();

    // at 60bpm, beat 2 is 2s in, and beat 4 is 4s in
    audiograph.set_tempo(60.);
    assert_eq!(audiograph.nodes[0].start_offset, ms(2_000));
    assert_eq!(audiograph.pattern_nodes[0].start_offset, ms(4_000));
    // the clip plays for as long as it did
    assert_eq!(audiograph.nodes[0].duration(), played);

    // and it's heard there: its first frame is 2s (88,200 frames) into a run
    audiograph.init();
    let tick = render_run(vec![metronome_tick()], rate, 2_000);
    let into_tick = first_sound(&tick).expect("the tick is silent");
    let run = render_run(audiograph.run_sounds(Duration::from_secs(3), false, false), rate, 3 * rate as usize);
    assert_eq!(first_sound(&run), Some(88_200 + into_tick));
  }

  #[test]
  fn test_a_clip_on_beat_8_is_heard_on_beat_8_at_another_tempo() {
    let rate = 44_100;
    // beat 8 at 120bpm is 4s in
    let mut audiograph = graph_with_ticks(&[4_000]);
    audiograph.init();
    let tick = render_run(vec![metronome_tick()], rate, 2_000);
    let into_tick = first_sound(&tick).expect("the tick is silent");

    // at 90bpm a beat is two thirds of a second, which isn't a whole number
    // of anything: beat 8 is 5.3333s in, frame 235,200
    audiograph.set_tempo(90.);
    let run = render_run(audiograph.run_sounds(Duration::from_secs(6), false, false), rate, 6 * rate as usize);
    assert_eq!(first_sound(&run), Some(235_200 + into_tick));

    // clips keep their order (the scheduler relies on it)
    let mut audiograph = graph_with_ticks(&[900, 0, 200, 4_000]);
    audiograph.set_tempo(77.);
    let starts: Vec<Duration> = audiograph.nodes.iter().map(|n| n.start_offset).collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted);
  }

  #[test]
  fn test_a_one_second_clip_is_a_second_long_at_any_tempo() {
    let path = one_second_at_48k();
    let mut audiograph = AudioGraph::new(44_100, 120., 8);
    let id = audiograph.construct_and_add_node(path, ms(1_000), 0).unwrap();
    assert_eq!(audiograph.get_node_by_id(id).unwrap().duration(), Duration::from_secs(1));

    audiograph.set_tempo(90.);
    assert_eq!(audiograph.get_node_by_id(id).unwrap().duration(), Duration::from_secs(1));
    audiograph.set_tempo(200.);
    assert_eq!(audiograph.get_node_by_id(id).unwrap().duration(), Duration::from_secs(1));
  }

  #[test]
  fn test_snapping_is_exact_far_along_the_playlist() {
    // at 90bpm a beat is 666.666...ms. beat 200 is 133.33s in, to the
    // nanosecond (not 137 microseconds early, as snapping in whole
    // microseconds made it)
    let audiograph = AudioGraph::new(44_100, 90., 8);
    let beat = audiograph.beat_interval();
    let near = beat * 200 + Duration::from_millis(40);
    let snapped = audiograph.find_nearest_offset_subdivision(near, daw::timing::Note::QuarterNote);
    assert_eq!(snapped, beat * 200);
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

    let node = AudioNode::new(id, sample_path, start_offset, track_number).unwrap();

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);
    let buf = daw::SampleBuffer::load(node.sample_path()).unwrap();

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

  // a second of stereo noise at 48kHz, written to a temporary file
  fn one_second_at_48k() -> String {
    let path = std::env::temp_dir()
      .join(format!("dawesome-48k-{}.wav", std::process::id()));
    let spec = hound::WavSpec {
      channels: 2,
      sample_rate: 48_000,
      bits_per_sample: 16,
      sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    for i in 0..48_000 * 2 {
      writer.write_sample(((i * 7919) % 2000 - 1000) as i16).unwrap();
    }
    writer.finalize().unwrap();
    path.to_string_lossy().into_owned()
  }

  #[test]
  fn test_sample_lengths_use_the_file_rate() {
    let path = one_second_at_48k();
    let second = |d: Duration| (d.as_secs_f64() - 1.).abs() < 1e-3;

    // the graph runs at 44.1kHz, which used to make this 1.088s
    let mut audiograph = AudioGraph::new(44_100, 120f32, 8);
    let id = audiograph.construct_and_add_node(path.clone(), Duration::ZERO, 0).unwrap();
    let node = audiograph.get_node_by_id(id).unwrap();
    assert!(second(node.sample_duration()), "clip: {:?}", node.sample_duration());
    assert_eq!(node.sample_rate, 48_000);

    let (dur, _) = preview_sample(&path).unwrap();
    assert!(second(dur), "preview: {:?}", dur);

    let mut seq = daw::Sequencer::default();
    let row = seq.add_row(&path).unwrap();
    let row_dur = seq.row(row).unwrap().duration();
    assert!(second(row_dur), "sequencer row: {:?}", row_dur);

    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_preview_sample_matches_a_node() {
    let path = daw::METRONOME_TICK_PATH;
    let (dur, waveform) = preview_sample(path).unwrap();
    let node = AudioNode::new(0, path.to_string(), Duration::ZERO, 0).unwrap();

    assert_eq!(dur, node.sample_duration());
    assert_eq!(waveform.as_str(), &*node.waveform);
    assert!(preview_sample("does/not/exist.wav").is_err());
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
    let mut node = AudioNode::new(0, path.to_string(), Duration::ZERO, 0).unwrap();
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
    assert!(daw::calc_waveform_from_samples(&[], 2, 44_100).is_none());
    assert!(daw::calc_waveform_from_samples(&[1, 2, 3], 0, 44_100).is_none());
    assert!(daw::calc_waveform_from_samples(&[1, 2, 3], 1, 0).is_none());
    // a few samples (< 0.25ms) used to divide by zero
    assert!(daw::calc_waveform_from_samples(&[100, -100, 50, -50], 1, 44_100).is_some());
  }

  #[test]
  fn test_silence_has_a_flat_waveform() {
    // used to panic: silence smoothed and normalised by dividing by zero
    let levels = daw::calc_waveform_from_samples(&vec![0; 48_000], 1, 48_000).unwrap();
    assert!(!levels.is_empty());
    assert!(levels.iter().all(|&level| level == 0.));
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
    let node = AudioNode::new(10, path(), Duration::ZERO, 0).unwrap();
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
      let (_controller, mixer) = audiograph.slice_mix(audiograph.beat_interval(), resume);
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
      0).unwrap();

    let tempo = 120f32;
    let max_beats: u64 = 8;

    let mut audiograph = AudioGraph::new(sample_rate, tempo, max_beats);
    let buf = daw::SampleBuffer::load(node.sample_path()).unwrap();

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
      let row = seq.add_row(KICK).unwrap();
      let pattern = seq.add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
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

  // a graph with a synth (id 7) and a one bar note pattern of its notes
  fn graph_with_notes(notes: Vec<daw::PatternNote>) -> (AudioGraph<'static>, u64) {
    let graph = AudioGraph::new(44_100, 120., 16);
    let pattern = {
      let mut seq = graph.sequencer.lock().unwrap();
      seq.add_synth(daw::Synth {
        id: 7, plugin_id: "vst3:synth".into(), path: String::new(), name: "Synth".into(),
        vendor: String::new(), volume: 1., muted: false,
      }).unwrap();
      let pattern = seq.add_pattern(1, 4, daw::PatternKind::Note).unwrap();
      seq.set_pattern_notes(pattern, 7, notes).unwrap();
      pattern
    };
    (graph, pattern)
  }

  #[test]
  fn test_a_note_still_sounding_at_the_loops_end_ends_there() {
    // a step is 125ms at 120bpm: 60 from 1.5s to 2s, 62 from 1.875s
    let note = |pitch, start, length| daw::PatternNote { pitch, start, length, velocity: 1. };
    let (mut graph, pattern) = graph_with_notes(vec![note(60, 12., 4.), note(62, 15., 1.)]);
    graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
    graph.init();
    graph.set_current_offset(Some(ms(1_500)));
    let heard = |notes: Vec<SliceNote>| -> Vec<(u8, Duration, Duration)> {
      notes.into_iter().map(|n| (n.pitch, n.on, n.off)).collect()
    };

    // not looping: as the pattern has them
    assert_eq!(heard(graph.run_notes(ms(500), None)), vec![(60, ms(0), ms(500)), (62, ms(375), ms(500))]);
    // looping at 1.75s (a loop held while recording, say): 60 ends at the
    // loop's end, where the next time round starts, and 62 is past it
    assert_eq!(heard(graph.run_notes(ms(500), Some(ms(1_750)))), vec![(60, ms(0), ms(250))]);
    assert!(graph.run_notes(ms(500), None).iter().all(|n| n.synth == 7 && n.route.as_ref().map(|r| r.number) == Some(0)));
  }

  #[test]
  fn test_the_synths_each_track_plays() {
    let note = |pitch, start, length| daw::PatternNote { pitch, start, length, velocity: 1. };
    let (mut graph, pattern) = graph_with_notes(vec![note(60, 20., 4.)]);
    let empty = graph.sequencer.lock().unwrap().add_pattern(1, 4, daw::PatternKind::Note).unwrap();
    graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 2, None).unwrap();
    graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(4_000), 5, None).unwrap();
    graph.add_pattern_node::<daw::timing::QuarterNote>(empty, ms(0), 3, None).unwrap();
    // (a note past the pattern's end still needs its copy: lengthening the
    // pattern brings it in)
    assert_eq!(graph.synth_tracks(), BTreeSet::from([(7, 2), (7, 5)]));

    // past the end, it isn't played: on its own, nor in the playlist
    graph.init();
    graph.set_current_offset(Some(ms(0)));
    assert!(graph.pattern_notes(pattern, ms(2_000)).is_empty());
    assert!(graph.run_notes(ms(8_000), None).is_empty());
    graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();
    let solo = graph.pattern_notes(pattern, ms(4_000));
    assert_eq!(solo.len(), 1);
    assert!(solo[0].route.is_none() && solo[0].synth == 7);
  }

  #[test]
  fn test_deleting_a_pattern_takes_its_clips() {
    let (mut graph, _, pattern) = graph_with_pattern();
    let other = graph.sequencer.lock().unwrap().add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
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
      let snare = seq.add_row(SNARE).unwrap();
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
    let pattern = graph.sequencer.lock().unwrap().add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
    let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(0), 0, None).unwrap();
    assert!(graph.explode_pattern_node(clip).is_err());
    assert!(graph.get_pattern_node_by_id(clip).is_some());
  }

  // ---- clip windows: trimming and cutting ----

  // noise at a file rate of its own, written to a temporary file
  fn noise_wav(name: &str, rate: u32, channels: u16, frames: u32) -> String {
    let path = std::env::temp_dir().join(format!("dawesome-{}-{}.wav", name, std::process::id()));
    let spec = hound::WavSpec { channels, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    let mut seed = 99u32;
    for frame in 0..frames {
      for channel in 0..channels as u32 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let noise = ((seed >> 16) % 6_000) as f32 - 3_000.;
        let tone = (frame as f32 * (0.02 + channel as f32 * 0.01)).sin() * 9_000.;
        writer.write_sample((tone + noise) as i16).unwrap();
      }
    }
    writer.finalize().unwrap();
    path.to_string_lossy().into_owned()
  }

  // play the graph as the scheduler does (see daw::run_playlist): from
  // `start`, a beat at a time, on an engine at `rate`. returns what's heard
  // in `total`, left and right
  fn play(graph: &mut AudioGraph<'static>, rate: u32, start: Duration, total: Duration) -> Vec<f32> {
    let (engine, mut renderer) = daw::Engine::new(rate);
    let beat = graph.beat_interval();
    graph.init();

    let mut voices = Vec::new();
    let mut at = Duration::ZERO;
    let mut first = true;
    while at < total {
      // from between beats, the part beat comes first
      let into = Duration::from_nanos((start.as_nanos() % beat.as_nanos()) as u64);
      let slice = if first && !into.is_zero() { beat - into } else { beat };
      graph.set_current_offset(Some(start + at));
      for (delay, source, route) in graph.run_sounds_at(at, rate, slice, false, first) {
        voices.push(engine.boxed_voice(source, Some(1), engine.frames_in(at + delay), route));
      }
      at += slice;
      first = false;
    }
    graph.pause();

    engine.start_run(1, graph.mixer.master(), voices).unwrap();
    let mut out = vec![0f32; engine.frames_in(total) as usize * 2];
    renderer.render(&mut out, Instant::now(), Duration::ZERO);
    out
  }

  // a graph at an awkward tempo with one clip of `path`, not on a beat or a
  // frame, at a pitch
  fn graph_with_clip(path: &str, pitch: f32) -> (AudioGraph<'static>, u64) {
    let mut graph = AudioGraph::new(44_100, 97., 256);
    let id = graph.construct_and_add_node(path.to_string(), Duration::from_nanos(333_333_333), 0).unwrap();
    let node = graph.get_mut_node_by_id(id).unwrap();
    node.set_settings(daw::ClipSettings { pitch, ..Default::default() });
    (graph, id)
  }

  fn differences(a: &[f32], b: &[f32]) -> usize {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).filter(|(a, b)| a != b).count()
  }

  #[test]
  fn test_a_cut_clip_plays_exactly_what_the_whole_clip_did() {
    let rate = 48_000;
    let total = Duration::from_secs(3);
    for file_rate in [44_100, 48_000] {
      let path = noise_wav(&format!("cut-{}", file_rate), file_rate, 2, file_rate * 3 / 2);
      for pitch in [-7., 0., 5.] {
        let (mut graph, id) = graph_with_clip(&path, pitch);
        let clip_start = graph.get_node_by_id(id).unwrap().start_offset;
        let cut = clip_start + Duration::from_nanos(432_109_876);
        // played from the top, from a beat, from between beats before the
        // clip, from inside each half, and from the cut itself
        let starts = [
          Duration::ZERO, graph.beat_interval(), ms(100), clip_start + ms(200), cut + ms(100), cut,
          cut - Duration::from_nanos(1), clip_start,
        ];
        let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
        assert!(whole[0].iter().any(|&sample| sample != 0.));

        let right = graph.split_node(id, cut).unwrap().expect("a cut inside the clip");
        assert_eq!(graph.nodes.len(), 2);
        for (&start, whole) in starts.iter().zip(&whole) {
          let halves = play(&mut graph, rate, start, total);
          assert_eq!(differences(&halves, whole), 0,
            "{}Hz file, pitch {}, played from {:?}", file_rate, pitch, start);
        }

        // and cut again, into three
        let again = graph.get_node_by_id(right).unwrap().start_offset + ms(77);
        graph.split_node(right, again).unwrap().expect("a cut inside the second half");
        graph.split_node(id, clip_start + ms(50)).unwrap().expect("a cut inside the first half");
        assert_eq!(graph.nodes.len(), 4);
        for (&start, whole) in starts.iter().zip(&whole) {
          let pieces = play(&mut graph, rate, start, total);
          assert_eq!(differences(&pieces, whole), 0,
            "in four: {}Hz file, pitch {}, played from {:?}", file_rate, pitch, start);
        }
      }
    }
  }

  #[test]
  fn test_a_cut_clip_keeps_its_fades_exactly() {
    let rate = 48_000;
    let total = Duration::from_secs(4);
    let path = noise_wav("cut-fades", 44_100, 2, 66_150);
    for pitch in [-7., 0., 5.] {
      let (mut graph, id) = graph_with_clip(&path, pitch);
      // fades that both fit in the half they stay with
      graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings {
        pitch, gain_db: -2., pan: -0.2, attack_ms: 60., decay_ms: 90.,
      });
      let node = graph.get_node_by_id(id).unwrap().clone();
      let whole = play(&mut graph, rate, Duration::ZERO, total);

      let right = graph.split_node(id, node.start_offset + node.duration() / 2).unwrap().unwrap();
      assert_eq!(graph.get_node_by_id(id).unwrap().settings().attack_ms, 60.);
      assert_eq!(graph.get_node_by_id(right).unwrap().settings().decay_ms, 90.);
      // not one sample differs, through either fade
      assert_eq!(differences(&play(&mut graph, rate, Duration::ZERO, total), &whole), 0, "pitch {}", pitch);
    }
  }

  #[test]
  fn test_a_clip_whose_sample_starts_before_the_playlist_is_cut_exactly() {
    let rate = 48_000;
    let total = Duration::from_secs(3);
    let path = noise_wav("early", 44_100, 2, 66_150);
    let (mut graph, id) = graph_with_clip(&path, -7.);
    // its first 700ms hidden, then moved to 100ms in: its sample would
    // start well before the playlist does
    graph.set_window(id, ms(700), None).unwrap();
    graph.move_node(id, ms(100), 0).unwrap();
    assert!(graph.get_node_by_id(id).unwrap().origin() < 0);
    let starts = [Duration::ZERO, ms(350), ms(600)];
    let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
    assert!(whole[0].iter().any(|&sample| sample != 0.));

    graph.split_node(id, ms(550)).unwrap().expect("a cut inside the clip");
    for (&start, whole) in starts.iter().zip(&whole) {
      assert_eq!(differences(&play(&mut graph, rate, start, total), whole), 0, "played from {:?}", start);
    }
  }

  #[test]
  fn test_clips_of_one_file_share_its_sound() {
    let path = noise_wav("shared", 44_100, 1, 4_410);
    let mut graph = AudioGraph::new(44_100, 120., 256);
    let a = graph.construct_and_add_node(path.clone(), Duration::ZERO, 0).unwrap();
    let b = graph.construct_and_add_node(path.clone(), ms(900), 1).unwrap();
    let pcm = |graph: &AudioGraph<'static>, id: u64| graph.get_node_by_id(id).unwrap().pcm.clone();
    // one decoded copy, however many clips, cut halves included
    assert!(Arc::ptr_eq(&pcm(&graph, a), &pcm(&graph, b)));
    let half = graph.split_node(a, ms(50)).unwrap().unwrap();
    assert!(Arc::ptr_eq(&pcm(&graph, a), &pcm(&graph, half)));
    assert_eq!(graph.get_node_by_id(a).unwrap().get_waveform(), graph.get_node_by_id(b).unwrap().get_waveform());

    // the file written again (longer): a clip made now plays the new sound,
    // and the clips there already keep what they had
    let longer = noise_wav("shared", 44_100, 1, 8_820);
    assert_eq!(longer, path);
    let c = graph.construct_and_add_node(path, ms(2_000), 2).unwrap();
    assert!(!Arc::ptr_eq(&pcm(&graph, a), &pcm(&graph, c)));
    assert_eq!(pcm(&graph, a).frames(), 4_410);
    assert_eq!(pcm(&graph, c).frames(), 8_820);
  }

  #[test]
  fn test_the_halves_of_a_cut() {
    let path = noise_wav("halves", 44_100, 2, 66_150);
    let (mut graph, id) = graph_with_clip(&path, 0.);
    graph.get_mut_node_by_id(id).unwrap().track_number = 3;
    let kept = graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings {
      gain_db: -3., pan: 0.25, pitch: 2., attack_ms: 100., decay_ms: 1_200.,
    });
    let before = graph.get_node_by_id(id).unwrap().clone();
    let cut = before.start_offset + ms(400);

    let right = graph.split_node(id, cut).unwrap().unwrap();
    assert_ne!(right, id);
    let left = graph.get_node_by_id(id).unwrap().clone();
    let right = graph.get_node_by_id(right).unwrap().clone();

    // the first half is the clip that was there, where it was. the second
    // starts where the first ends, to the nanosecond
    assert_eq!(left.start_offset, before.start_offset);
    assert_eq!(left.start_offset + left.duration(), right.start_offset);
    assert!(right.start_offset.abs_diff(cut) < Duration::from_nanos(5));
    assert_eq!(left.origin(), right.origin());
    let ends = |clip: &AudioNode| clip.start_offset + clip.duration();
    assert!(ends(&right).abs_diff(ends(&before)) < Duration::from_micros(1));
    // both play the sample the clip did, the same way, on its track
    for half in [&left, &right] {
      assert_eq!(half.sample_path(), before.sample_path());
      assert_eq!(half.track_number, 3);
      assert_eq!((half.settings().gain_db, half.settings().pan, half.settings().pitch), (kept.gain_db, kept.pan, kept.pitch));
    }
    // the fade in stays with the first half, the fade out goes with the
    // second, shortened to it: the second half is under 1.2s long
    assert_eq!((left.settings().attack_ms, left.settings().decay_ms), (100., 0.));
    assert_eq!(right.settings().attack_ms, 0.);
    let right_ms = right.duration().as_secs_f32() * 1000.;
    assert!(right_ms < 1_200. && (right.settings().decay_ms - right_ms).abs() < 0.01);

    // too near an edge, or outside the clip, nothing is cut
    let count = graph.nodes.len();
    for at in [
      left.start_offset, left.start_offset + ms(9), right.start_offset - ms(9),
      right.start_offset + right.duration() - ms(5), ms(10), Duration::from_secs(30),
    ] {
      assert_eq!(graph.split_node(id, at).unwrap(), None, "cut at {:?}", at);
    }
    assert_eq!(graph.nodes.len(), count);
    assert_eq!(graph.get_node_by_id(id).unwrap().window(), left.window());
    assert!(graph.split_node(9_999, cut).is_err());
  }

  #[test]
  fn test_a_trimmed_clip_plays_its_part_of_the_sample_where_it_was() {
    let rate = 48_000;
    let total = Duration::from_secs(3);
    let path = noise_wav("trim", 44_100, 2, 66_150);
    for pitch in [0., -7.] {
      let (mut graph, id) = graph_with_clip(&path, pitch);
      let before = graph.get_node_by_id(id).unwrap().clone();
      let whole = play(&mut graph, rate, Duration::ZERO, total);

      // from 0.2s to 0.9s of the sample
      graph.set_window(id, ms(200), Some(ms(900))).unwrap();
      let node = graph.get_node_by_id(id).unwrap().clone();
      assert_eq!(node.window(), Some(daw::Window { start: ms(200), end: Some(ms(900)) }));
      // the sound hasn't moved: the clip starts later by what was hidden
      assert_eq!(node.origin(), before.origin());
      let hidden = node.settings().played_exactly(ms(200));
      assert_eq!(node.start_offset, before.start_offset + hidden);
      let length = node.settings().played_exactly(ms(900)) - hidden;
      assert_eq!(node.duration(), length);

      let trimmed = play(&mut graph, rate, Duration::ZERO, total);
      let from = run_frame(node.start_offset.as_nanos() as i128, rate) as usize * 2;
      let to = run_frame((node.start_offset + node.duration()).as_nanos() as i128, rate) as usize * 2;
      assert!(to > from + 1_000);
      // silent outside the clip, and exactly what it was inside it
      assert!(trimmed[..from].iter().all(|&sample| sample == 0.));
      assert!(trimmed[to..].iter().all(|&sample| sample == 0.));
      assert_eq!(differences(&trimmed[from..to], &whole[from..to]), 0, "pitch {}", pitch);
      assert!(whole[..from].iter().any(|&sample| sample != 0.));

      // opened out again, it's the whole clip, back where it started
      graph.set_window(id, Duration::ZERO, None).unwrap();
      let node = graph.get_node_by_id(id).unwrap();
      assert_eq!(node.window(), None);
      assert_eq!(node.start_offset, before.start_offset);
      assert_eq!(differences(&play(&mut graph, rate, Duration::ZERO, total), &whole), 0);
    }
  }

  #[test]
  fn test_a_trimmed_clip_is_as_long_at_any_tempo() {
    // a second of a two second sample, from half a second in, on beat 8
    let path = noise_wav("tempo", 44_100, 1, 88_200);
    let mut graph = AudioGraph::new(44_100, 120., 256);
    let id = graph.construct_and_add_node(path, ms(4_000), 0).unwrap();
    graph.set_window(id, ms(500), Some(ms(1_500))).unwrap();
    let length = |graph: &AudioGraph<'static>| graph.get_node_by_id(id).unwrap().duration();
    assert_eq!(length(&graph), ms(1_000));
    // (its start moved with the trim: half a second, a beat, later)
    assert_eq!(graph.get_node_by_id(id).unwrap().start_offset, ms(4_500));

    for tempo in [90., 61.3, 240., 120.] {
      graph.set_tempo(tempo);
      assert_eq!(length(&graph), ms(1_000), "at {}bpm", tempo);
      assert_eq!(graph.get_node_by_id(id).unwrap().window(), Some(daw::Window { start: ms(500), end: Some(ms(1_500)) }));
      // and it's still on beat 9
      let beat = graph.get_node_by_id(id).unwrap().start_offset.as_secs_f64() / graph.beat_interval().as_secs_f64();
      assert!((beat - 9.).abs() < 1e-6, "on beat {} at {}bpm", beat, tempo);
    }

    // heard for a second at the slower tempo: the frames past its end are silent
    graph.set_tempo(90.);
    let out = play(&mut graph, 44_100, Duration::ZERO, Duration::from_secs(9));
    let starts = run_frame(graph.get_node_by_id(id).unwrap().start_offset.as_nanos() as i128, 44_100) as usize;
    assert!(out[..starts * 2].iter().all(|&sample| sample == 0.));
    assert!(out[starts * 2..(starts + 44_100) * 2].iter().any(|&sample| sample != 0.));
    assert!(out[(starts + 44_100) * 2..].iter().all(|&sample| sample == 0.));
    assert!(out[(starts + 44_099) * 2] != 0. || out[(starts + 44_098) * 2] != 0.);
  }

  #[test]
  fn test_trimming_to_a_bar_line_keeps_the_sound_either_side_of_it() {
    // four bars of sound from bar 1, at 120bpm: eight seconds
    let rate = 44_100;
    let path = noise_wav("bars", rate, 1, rate * 8);
    let mut graph = AudioGraph::new(rate, 120., 256);
    let id = graph.construct_and_add_node(path, Duration::ZERO, 0).unwrap();
    let total = Duration::from_secs(9);
    let whole = play(&mut graph, rate, Duration::ZERO, total);
    // bar 2 starts two seconds in
    let bar_2 = rate as usize * 2 * 2;

    // the right edge dragged back to bar 2: a one bar clip, whose last
    // frame is what played just before bar 2, and nothing after it
    graph.set_window(id, Duration::ZERO, Some(ms(2_000))).unwrap();
    assert_eq!(graph.get_node_by_id(id).unwrap().duration(), ms(2_000));
    let right = play(&mut graph, rate, Duration::ZERO, total);
    assert_eq!(differences(&right[..bar_2], &whole[..bar_2]), 0);
    assert!(right[bar_2 - 2] != 0. && right[bar_2 - 2] == whole[bar_2 - 2]);
    assert!(right[bar_2..].iter().all(|&sample| sample == 0.));

    // and back to the end of bar 2: a two bar clip, ending with what played
    // just before bar 3
    let bar_3 = bar_2 * 2;
    graph.set_window(id, Duration::ZERO, Some(ms(4_000))).unwrap();
    assert_eq!(graph.get_node_by_id(id).unwrap().duration(), ms(4_000));
    let two_bars = play(&mut graph, rate, Duration::ZERO, total);
    assert_eq!(differences(&two_bars[..bar_3], &whole[..bar_3]), 0);
    assert!(two_bars[bar_3 - 2] != 0. && two_bars[bar_3..].iter().all(|&sample| sample == 0.));

    // the left edge dragged from bar 1 to bar 2 instead: the clip starts on
    // bar 2, with what played at bar 2 before
    graph.set_window(id, ms(2_000), None).unwrap();
    assert_eq!(graph.get_node_by_id(id).unwrap().start_offset, ms(2_000));
    let left = play(&mut graph, rate, Duration::ZERO, total);
    assert!(left[..bar_2].iter().all(|&sample| sample == 0.));
    assert_eq!(differences(&left[bar_2..], &whole[bar_2..]), 0);
    assert!(left[bar_2] != 0. || left[bar_2 + 2] != 0.);

    // with a 100ms fade in, the clip fades in over the 100ms after its new
    // start: silent at it, half way up 50ms on, and all there by 100ms
    graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { attack_ms: 100., ..Default::default() });
    let faded = play(&mut graph, rate, Duration::ZERO, total);
    let after = |ms: usize| bar_2 + rate as usize * ms / 1_000 * 2;
    assert_eq!(faded[bar_2], 0.);
    // (averaged over a few milliseconds of noise either side)
    let level = |samples: &[f32], at: usize| -> f32 {
      samples[at - 200..at + 200].iter().map(|sample| sample.abs()).sum()
    };
    let half = level(&faded, after(50)) / level(&whole, after(50));
    assert!((half - 0.5).abs() < 0.02, "half way up the fade the level is {}", half);
    assert_eq!(differences(&faded[after(101)..], &whole[after(101)..]), 0);
    assert!(differences(&faded[bar_2..after(99)], &whole[bar_2..after(99)]) > 1_000);
  }

  #[test]
  fn test_a_clip_stretched_to_twice_its_length_plays_twice_as_many_frames() {
    // two bars of sound from bar 1, at 120bpm: four seconds
    let rate = 48_000;
    for file_rate in [44_100, 48_000] {
      let path = noise_wav(&format!("stretch-{}", file_rate), file_rate, 1, file_rate * 4);
      let mut graph = AudioGraph::new(44_100, 120., 256);
      let id = graph.construct_and_add_node(path, Duration::ZERO, 0).unwrap();
      // the frames it sounds for: up to its last frame that isn't silent
      let sounding = |graph: &mut AudioGraph<'static>| {
        let out = play(graph, rate, Duration::ZERO, Duration::from_secs(10));
        out.chunks(2).rposition(|frame| frame[0] != 0.).unwrap() + 1
      };
      let frames = sounding(&mut graph);
      assert!((frames as i64 - 4 * rate as i64).abs() <= 1, "{} frames", frames);

      // its right edge out to bar 5: twice its length, an octave down
      let kept = graph.stretch_node(id, -12., false).unwrap();
      assert_eq!(kept.pitch, -12.);
      assert_eq!(graph.get_node_by_id(id).unwrap().start_offset, Duration::ZERO);
      let stretched = sounding(&mut graph);
      assert!((stretched as i64 - 2 * frames as i64).abs() <= 1, "{} frames, from {}", stretched, frames);

      // and from its end instead: back to its length, ending where it ends
      // now, on bar 5
      graph.stretch_node(id, 0., true).unwrap();
      let node = graph.get_node_by_id(id).unwrap();
      assert_eq!(node.start_offset, ms(4_000));
      assert_eq!(node.start_offset + node.duration(), ms(8_000));
    }
  }

  #[test]
  fn test_stretching_a_trimmed_clip_squeezes_only_what_it_plays() {
    // from 0.5s to 1.5s of a two second sample, with fades, on beat 8
    let path = noise_wav("stretch-trimmed", 44_100, 1, 88_200);
    let mut graph = AudioGraph::new(44_100, 120., 256);
    let id = graph.construct_and_add_node(path, ms(4_000), 0).unwrap();
    graph.set_window(id, ms(500), Some(ms(1_500))).unwrap();
    graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { attack_ms: 400., decay_ms: 700., ..Default::default() });
    let before = graph.get_node_by_id(id).unwrap().clone();
    let window = before.window();
    let end = before.start_offset + before.duration();

    // to half its length, from its end: an octave up
    graph.stretch_node(id, 12., true).unwrap();
    let node = graph.get_node_by_id(id).unwrap().clone();
    // it plays the same part of the sample, in half the time, and ends
    // where it did
    assert_eq!(node.window(), window);
    assert_eq!(node.duration(), ms(500));
    assert_eq!(node.start_offset + node.duration(), end);
    assert_eq!(node.start_offset, before.start_offset + ms(500));
    // its fades are kept within its new length
    assert_eq!(node.settings().attack_ms, 400.);
    assert!((node.settings().decay_ms - 500.).abs() < 0.01);

    // and it's silent outside that half second: nothing hidden is heard
    let out = play(&mut graph, 44_100, Duration::ZERO, Duration::from_secs(7));
    let from = run_frame(node.start_offset.as_nanos() as i128, 44_100) as usize * 2;
    let to = run_frame(end.as_nanos() as i128, 44_100) as usize * 2;
    assert!(out[..from].iter().all(|&sample| sample == 0.));
    assert!(out[to..].iter().all(|&sample| sample == 0.));
    assert!(out[from..to].iter().any(|&sample| sample != 0.));
  }

  #[test]
  fn test_trim_limits() {
    let path = noise_wav("limits", 44_100, 1, 44_100);
    let (mut graph, id) = graph_with_clip(&path, 0.);
    let sample = graph.get_node_by_id(id).unwrap().sample_duration();
    let window = |graph: &AudioGraph<'static>| graph.get_node_by_id(id).unwrap().window();

    // an end at or past the sample's is the sample's
    graph.set_window(id, ms(100), Some(Duration::from_secs(9))).unwrap();
    assert_eq!(window(&graph), Some(daw::Window { start: ms(100), end: None }));
    graph.set_window(id, Duration::ZERO, Some(sample)).unwrap();
    assert_eq!(window(&graph), None);

    // never shorter than 10ms: the edge that would cross the other stops
    graph.set_window(id, ms(500), Some(ms(503))).unwrap();
    assert_eq!(window(&graph), Some(daw::Window { start: ms(500), end: Some(ms(510)) }));
    graph.set_window(id, ms(995), None).unwrap();
    assert_eq!(window(&graph).unwrap().end, None);
    assert!(sample - window(&graph).unwrap().start >= MIN_CLIP);
    // at twice the speed, 10ms of sound is 20ms of the sample
    graph.set_window(id, Duration::ZERO, None).unwrap();
    graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { pitch: 12., ..Default::default() });
    graph.set_window(id, ms(500), Some(ms(501))).unwrap();
    assert!((window(&graph).unwrap().end.unwrap().as_secs_f64() - 0.520).abs() < 1e-6);
    graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings::default());

    // fades are kept within what's left
    graph.set_window(id, Duration::ZERO, None).unwrap();
    graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { attack_ms: 600., decay_ms: 50., ..Default::default() });
    graph.set_window(id, ms(100), Some(ms(400))).unwrap();
    let settings = graph.get_node_by_id(id).unwrap().settings();
    assert!((settings.attack_ms - 300.).abs() < 0.01);
    assert_eq!(settings.decay_ms, 50.);
    // and opening the clip out again doesn't bring them back
    graph.set_window(id, Duration::ZERO, None).unwrap();
    assert!((graph.get_node_by_id(id).unwrap().settings().attack_ms - 300.).abs() < 0.01);

    // a clip can't start before the playlist does: moved to the start with
    // its first half hidden, its left edge has nowhere to go
    graph.set_window(id, ms(500), None).unwrap();
    graph.move_node(id, Duration::ZERO, 0).unwrap();
    graph.set_window(id, ms(200), None).unwrap();
    let node = graph.get_node_by_id(id).unwrap();
    assert_eq!(node.start_offset, Duration::ZERO);
    assert!(node.window().unwrap().start.abs_diff(ms(500)) < Duration::from_nanos(3));

    assert!(graph.set_window(9_999, Duration::ZERO, None).is_err());
  }

  #[test]
  fn test_changing_a_clip_changes_no_other_clip_of_the_sample() {
    let path = noise_wav("others", 44_100, 2, 44_100);
    let (mut graph, id) = graph_with_clip(&path, 0.);
    let other = graph.construct_and_add_node(path.clone(), ms(5_000), 1).unwrap();
    let heard = |graph: &AudioGraph<'static>| -> Vec<f32> {
      let node = graph.get_node_by_id(other).unwrap();
      node.voice(48_000, node.start_offset.as_nanos() as i128, 0).collect()
    };
    let before = graph.get_node_by_id(other).unwrap().clone();
    let sound = heard(&graph);
    assert!(sound.iter().any(|&sample| sample != 0.));

    graph.set_window(id, ms(100), Some(ms(700))).unwrap();
    let start = graph.get_node_by_id(id).unwrap().start_offset;
    let half = graph.split_node(id, start + ms(300)).unwrap().unwrap();
    graph.get_mut_node_by_id(id).unwrap().set_settings(daw::ClipSettings { pitch: 7., gain_db: -12., ..Default::default() });
    graph.set_window(half, ms(450), Some(ms(600))).unwrap();

    let after = graph.get_node_by_id(other).unwrap();
    assert!(after.same_clip(&before));
    assert_eq!(after.start_offset, before.start_offset);
    assert_eq!(after.window(), None);
    assert_eq!(differences(&heard(&graph), &sound), 0);
  }

  // a two bar pattern with the kick on steps 0, 8 and 20, placed at 2s
  // (a step is 125ms at 120bpm): (graph, pattern, clip)
  fn graph_with_pattern_clip() -> (AudioGraph<'static>, u64, u64) {
    let (mut graph, kick, pattern) = graph_with_pattern();
    {
      let mut seq = graph.sequencer.lock().unwrap();
      seq.set_pattern_bars(pattern, 2, 4).unwrap();
      seq.set_step(pattern, kick, 20, true).unwrap();
    }
    let clip = graph.add_pattern_node::<daw::timing::QuarterNote>(pattern, ms(2_000), 1, None).unwrap();
    (graph, pattern, clip)
  }

  // the frames a sound starts on: where silence ends
  fn onsets(samples: &[f32]) -> Vec<usize> {
    let left: Vec<f32> = samples.chunks(2).map(|frame| frame[0]).collect();
    // (the kick is a lot shorter than the gaps between its hits)
    let quiet = 2_000;
    (0..left.len())
      .filter(|&i| left[i] != 0. && (i < quiet || left[i - quiet..i].iter().all(|&s| s == 0.)))
      .collect()
  }

  #[test]
  fn test_a_pattern_clip_plays_the_hits_in_its_window() {
    let rate = 44_100;
    let total = Duration::from_secs(8);
    let (mut graph, pattern, clip) = graph_with_pattern_clip();
    let whole = play(&mut graph, rate, Duration::ZERO, total);
    let hits = onsets(&whole);
    assert_eq!(hits.len(), 3);

    // from step 4 on: the hit on step 0 is gone, the others stay where
    // they were, and the clip starts half a second later
    graph.set_pattern_window(clip, 4., None).unwrap();
    assert_eq!(graph.get_pattern_node_by_id(clip).unwrap().start_offset, ms(2_500));
    assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), hits[1..]);
    assert_eq!(graph.end(), ms(6_000));

    // steps 4 up to 20: a hit on the step the window ends on isn't in it
    graph.set_pattern_window(clip, 4., Some(20.)).unwrap();
    assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), hits[1..2]);
    assert_eq!(graph.end(), ms(4_500));
    graph.set_pattern_window(clip, 4., Some(20.5)).unwrap();
    assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), hits[1..]);

    // the pattern made a bar long: the clip ends with it, and gets its
    // steps back when the pattern is two bars again
    let span = |graph: &AudioGraph<'static>| {
      let length = graph.sequencer.lock().unwrap().pattern(pattern).unwrap().length;
      graph.get_pattern_node_by_id(clip).unwrap().span(length)
    };
    graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 1, 4).unwrap();
    assert_eq!(span(&graph), (4., 16.));
    assert_eq!(graph.end(), ms(4_000));
    assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), hits[1..2]);
    graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();
    assert_eq!(span(&graph), (4., 20.5));

    // a window past the pattern's end: a clip a step long, playing nothing
    graph.set_pattern_window(clip, 20., None).unwrap();
    graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 1, 4).unwrap();
    assert_eq!(span(&graph), (20., 21.));
    assert!(play(&mut graph, rate, Duration::ZERO, total).iter().all(|&sample| sample == 0.));
    graph.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();

    // an end that's no number at all is the pattern's end
    graph.set_pattern_window(clip, 4., Some(f64::NAN)).unwrap();
    assert_eq!(span(&graph), (4., 32.));

    // never under a step long, nor past the pattern's ends
    graph.set_pattern_window(clip, 10., Some(10.25)).unwrap();
    assert_eq!(span(&graph), (10., 11.));
    graph.set_pattern_window(clip, 31.5, None).unwrap();
    assert_eq!(span(&graph), (31., 32.));
    graph.set_pattern_window(clip, 0., Some(99.)).unwrap();
    assert_eq!(graph.get_pattern_node_by_id(clip).unwrap().window, None);
    assert_eq!(graph.get_pattern_node_by_id(clip).unwrap().start_offset, ms(2_000));

    assert!(graph.set_pattern_window(9_999, 0., None).is_err());

    // exploded, each new clip plays the part of its pattern the clip played
    graph.set_pattern_window(clip, 4., Some(20.5)).unwrap();
    let before = onsets(&play(&mut graph, rate, Duration::ZERO, total));
    let made = graph.explode_pattern_node(clip).unwrap();
    assert!(!made.is_empty());
    for id in &made {
      assert_eq!(graph.get_pattern_node_by_id(*id).unwrap().window, Some(PatternWindow { start: 4., end: Some(20.5) }));
    }
    assert_eq!(onsets(&play(&mut graph, rate, Duration::ZERO, total)), before);
  }

  #[test]
  fn test_a_cut_pattern_clip_plays_exactly_what_the_whole_clip_did() {
    let rate = 48_000;
    let total = Duration::from_secs(8);
    let (mut graph, pattern, clip) = graph_with_pattern_clip();
    // off the grid, at a tempo whose steps aren't whole frames
    graph.set_tempo(97.);
    graph.pattern_nodes.iter_mut().find(|node| node.id == clip).unwrap().start_offset = Duration::from_nanos(1_234_567_891);
    let starts = [Duration::ZERO, ms(1_500), ms(3_000)];
    let whole: Vec<Vec<f32>> = starts.iter().map(|&start| play(&mut graph, rate, start, total)).collect();
    assert_eq!(onsets(&whole[0]).len(), 3);

    // between steps 8 and 9, then the second half again between 19 and 20
    let step = graph.beat_interval() / 4;
    let start = graph.get_pattern_node_by_id(clip).unwrap().start_offset;
    let right = graph.split_node(clip, start + step.mul_f64(8.5)).unwrap().expect("a cut inside the clip");
    let later = graph.split_node(right, start + step.mul_f64(19.75)).unwrap().expect("a cut inside the half");
    assert_eq!(graph.pattern_nodes.len(), 3);

    let node = |id: u64| graph.get_pattern_node_by_id(id).unwrap().clone();
    assert_eq!(node(clip).window, Some(PatternWindow { start: 0., end: Some(8.5) }));
    assert_eq!(node(later).window.unwrap().end, None);
    assert!(node(clip).pattern_id == pattern && node(right).pattern_id == pattern && node(later).track_number == 1);
    assert_eq!(node(clip).origin(graph.beat_interval()), node(later).origin(graph.beat_interval()));

    for (&start, whole) in starts.iter().zip(&whole) {
      assert_eq!(differences(&play(&mut graph, rate, start, total), whole), 0, "played from {:?}", start);
    }

    // too near an edge, nothing is cut
    assert_eq!(graph.split_node(clip, start + step.mul_f64(0.5)).unwrap(), None);
    assert_eq!(graph.split_node(clip, start + step.mul_f64(8.)).unwrap(), None);
    assert_eq!(graph.pattern_nodes.len(), 3);
  }
}
