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
  #[cfg(test)]
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
  #[cfg(test)]
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

  #[cfg(test)]
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
  #[cfg(test)]
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
  #[cfg(test)]
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
  #[cfg(test)]
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
  #[cfg(test)]
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
mod tests;
