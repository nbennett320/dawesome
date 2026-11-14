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
// plays whatever the pattern holds now, but it keeps a length of its own:
// longer than its pattern, it plays the pattern over and over (see
// pattern_step_at). its sample rows' notes go to its track's bus (see
// sample_voice.rs), and its synth rows' notes to its track's copy of each
// synth (see run_notes)
#[derive(Clone, Debug, PartialEq)]
pub struct PatternNode {
  pub id: u64,
  pub pattern_id: u64,
  // where the clip starts in the playlist
  pub start_offset: Duration,
  pub track_number: u32,
  // where in its pattern the clip starts, in steps. it isn't wrapped at the
  // pattern's length, so the second half of a clip cut on its pattern's
  // second time round still plays what the whole clip did there, whatever
  // the pattern's length becomes later
  pub start: f64,
  // how long the clip is, in steps
  pub length: f64,
}

// the shortest a pattern clip can be trimmed or cut to, in steps
pub const MIN_PATTERN_CLIP: f64 = 1.;
// the longest a pattern clip can be, and the furthest into its pattern it
// can start, in steps: as far as a note can start, past the longest
// playlist there can be
pub const MAX_PATTERN_CLIP: f64 = daw::MAX_NOTE_START;

// how long a number of steps lasts
fn steps_time(steps: f64, beat_interval: Duration) -> Duration {
  Duration::from_secs_f64(beat_interval.as_secs_f64() * steps.max(0.) / daw::STEPS_PER_BEAT as f64)
}

// a pattern clip's steps to a millionth, which is what they're kept to: the
// end of the first half of a cut clip is then the very step the second half
// starts on, however the two were worked out
fn micro_steps(steps: f64) -> f64 {
  (steps * 1e6).round() / 1e6
}

// the step of its pattern, `length` steps long, that a clip starting
// `start` steps into it plays `t` steps after the clip's start: the clip
// plays the pattern end to end, over and over. (the remainder is exact, so
// the clip's step less this is a whole number of patterns to the bit: see
// PatternNode::first_pass_at)
pub fn pattern_step_at(start: f64, t: f64, length: u32) -> f64 {
  let step = start + t;
  // (most often in the pattern's first time round: no division then)
  if (0. ..length as f64).contains(&step) { step } else { step.rem_euclid(length as f64) }
}

// one time through its pattern that a clip plays: the part of the pattern
// it plays then, from `from` up to `to` (steps of the pattern, `to` no
// further than its end), and where that time through starts, in steps from
// the clip's origin (see PatternNode::origin): a whole number of patterns
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pass {
  pub from: f64,
  pub to: f64,
  pub at: f64,
}

impl Pass {
  // which of a row's notes (sorted by start: see sort_notes) it plays: the
  // ones that start in its part of the pattern. found by binary search, as a
  // pattern can hold tens of thousands of notes
  pub fn notes<'a>(&self, notes: &'a [daw::PatternNote]) -> &'a [daw::PatternNote] {
    let first = notes.partition_point(|note| note.start < self.from);
    let past = notes.partition_point(|note| note.start < self.to).max(first);
    &notes[first..past]
  }

  // where one of those notes starts and ends as it plays it, in steps from
  // the clip's origin: it ends at its own end, or at the end of the pass,
  // whichever comes first
  pub fn plays(&self, note: &daw::PatternNote) -> (f64, f64) {
    (self.at + note.start, self.at + note.end().min(self.to))
  }
}

impl PatternNode {
  // a clip of all of a pattern `length` steps long
  pub fn new(id: u64, pattern_id: u64, start_offset: Duration, track_number: u32, length: u32) -> Self {
    PatternNode { id, pattern_id, start_offset, track_number, start: 0., length: length as f64 }
  }

  // a pattern played on its own, which plays as a clip of all of it at the
  // playlist's start would (its id and track aren't used)
  fn alone(pattern: &daw::Pattern) -> Self {
    PatternNode::new(0, pattern.id, Duration::ZERO, 0, pattern.length)
  }

  // where in its pattern the clip ends, in steps (counted as `start` is)
  pub fn end(&self) -> f64 {
    micro_steps(self.start + self.length)
  }

  // play `length` steps of the pattern from `start` steps into it, kept to
  // what a clip can be: from step 0, at least MIN_PATTERN_CLIP long, no
  // further than MAX_PATTERN_CLIP either way, and to a millionth of a step.
  // a number that makes no sense is the pattern's start, or the shortest
  // clip. (the clip's place in the playlist is the caller's: see
  // AudioGraph::set_pattern_window)
  pub fn set_span(&mut self, start: f64, length: f64) {
    let start = if start.is_finite() { start.clamp(0., MAX_PATTERN_CLIP) } else { 0. };
    let length = if length.is_finite() { length.clamp(MIN_PATTERN_CLIP, MAX_PATTERN_CLIP) } else { MIN_PATTERN_CLIP };
    self.start = micro_steps(start);
    self.length = micro_steps(length);
  }

  // where the pattern's first step would be heard, were all of it played,
  // in nanoseconds (it can be before the playlist's start). the halves of a
  // cut clip have the same origin, to the nanosecond
  pub fn origin(&self, beat_interval: Duration) -> i128 {
    self.start_offset.as_nanos() as i128 - steps_time(self.start, beat_interval).as_nanos() as i128
  }

  // where in the playlist each point of the clip is heard, given in steps
  // from its origin. (the origin is worked out once, for all of a slice's
  // notes)
  fn heard(&self, beat_interval: Duration) -> impl Fn(f64) -> i128 + Copy {
    let origin = self.origin(beat_interval);
    move |steps| origin + steps_time(steps, beat_interval).as_nanos() as i128
  }

  // where in the playlist a point of the clip is heard, in steps from its
  // origin
  fn heard_at(&self, steps: f64, beat_interval: Duration) -> i128 {
    self.heard(beat_interval)(steps)
  }

  // how long the clip is
  pub fn duration(&self, beat_interval: Duration) -> Duration {
    steps_time(self.end(), beat_interval).saturating_sub(steps_time(self.start, beat_interval))
  }

  // where the clip's first time through its pattern, `length` steps long,
  // starts: the start of the pattern it starts in, in steps from its origin
  // (a whole number of patterns)
  fn first_pass_at(&self, length: u32) -> f64 {
    self.start - pattern_step_at(self.start, 0., length)
  }

  // the times through its pattern, `length` steps long, that the clip
  // plays, in order, from the `k`th (the first is 0), the first of them
  // starting `first` steps from its origin (see first_pass_at): the first
  // starts where the clip does in the pattern, and the last ends where the
  // clip ends
  fn passes_from(&self, first: f64, k: u64, length: u32) -> impl Iterator<Item = Pass> + Clone {
    let (start, end, whole) = (self.start, self.end(), length as f64);
    (k..).map(move |k| first + k as f64 * whole)
      .take_while(move |&at| length > 0 && at < end)
      .map(move |at| Pass { from: (start - at).max(0.), to: (end - at).min(whole), at })
  }

  // every time through its pattern that the clip plays, in order
  pub fn passes(&self, length: u32) -> impl Iterator<Item = Pass> + Clone {
    self.passes_from(self.first_pass_at(length), 0, length)
  }

  // the times through it that are heard, if only in part, from `from` up to
  // `to` (nanoseconds in the playlist), and one more either side: they're
  // found from steps, which can be a hair out at a pass's edges, so which of
  // their notes are heard then is worked out from when each one is (see
  // notes_around). (cheap to go through again: clone it)
  fn passes_heard(&self, length: u32, beat: Duration, from: i128, to: i128) -> impl Iterator<Item = Pass> + Clone {
    let step = beat.as_secs_f64() * 1e9 / daw::STEPS_PER_BEAT as f64;
    let (first, origin) = (self.first_pass_at(length), self.origin(beat));
    let pass_of = |at: i128| ((at.saturating_sub(origin) as f64 / step - first) / length as f64).floor();
    // (a float that's too big or isn't a number becomes the nearest whole
    // number or 0, and the passes stop where the clip does)
    let (low, high) = ((pass_of(from) - 1.).max(0.) as u64, (pass_of(to) + 1.).max(0.) as u64);
    self.passes_from(first, low, length).take(high.saturating_sub(low).saturating_add(1) as usize)
  }

  // which of a row's notes (sorted by start: see sort_notes) the clip plays
  // in a time through its pattern, split by when they're heard: those heard
  // before `from`, and those heard from `from` up to `to` (nanoseconds in
  // the playlist)
  // - heard: where a point of the clip is heard (see PatternNode::heard)
  fn notes_around<'a>(
    notes: &'a [daw::PatternNote],
    pass: &Pass,
    heard: impl Fn(f64) -> i128,
    from: i128,
    to: i128,
  ) -> (&'a [daw::PatternNote], &'a [daw::PatternNote]) {
    let played = pass.notes(notes);
    // (each is where the notes stop passing a test that a note later than
    // one that fails it fails too)
    let starting = played.partition_point(|note| heard(pass.plays(note).0) < from);
    let past = played.partition_point(|note| heard(pass.plays(note).0) < to).max(starting);
    (&played[..starting], &played[starting..past])
  }

  // the voices of the notes the clip plays on its pattern's sample rows
  // that start in a slice of a run, each with how far into the slice it
  // starts. on a resume, those that started before the slice and are still
  // sounding are played from where it starts in them
  fn sample_voices(
    &self,
    sequencer: &daw::Sequencer,
    pattern: &daw::Pattern,
    beat: Duration,
    slice: &Slice,
  ) -> Vec<(Duration, daw::SampleVoice)> {
    let (in_slice, heard) = (self.passes_heard(pattern.length, beat, slice.start, slice.end), self.heard(beat));
    let mut voices = Vec::new();
    for (instrument, notes) in sequencer.sample_notes(pattern) {
      // (on a resume, the times through as far back as one of the row's
      // notes can still be sounding: none, for a clip over by then)
      let passes = if slice.resume {
        let back = instrument.longest(notes).as_nanos() as i128;
        self.passes_heard(pattern.length, beat, slice.start - back, slice.end)
      } else {
        in_slice.clone()
      };
      for pass in passes {
        let (before, starting) = Self::notes_around(notes, &pass, heard, slice.start, slice.end);
        let resumed = if slice.resume { before } else { &[] };
        for note in resumed.iter().chain(starting) {
          voices.extend(Self::sample_voice(&instrument, note, &pass, heard, slice));
        }
      }
    }
    voices
  }

  // a sample row's note as the clip plays it in a slice: from its start, or
  // from where the slice starts in it. a gated row's stops at the note's
  // end, or at the end of the time through the pattern if that's sooner
  // (see Pass::plays). none if it started before the slice and is over by
  // then
  fn sample_voice(
    instrument: &daw::SampleInstrument,
    note: &daw::PatternNote,
    pass: &Pass,
    heard: impl Fn(f64) -> i128,
    slice: &Slice,
  ) -> Option<(Duration, daw::SampleVoice)> {
    let (from, to) = pass.plays(note);
    let on = heard(from);
    let stop = instrument.gated.then(|| heard(to));
    if on < slice.start {
      let played_out = on + instrument.played_len(note.pitch).as_nanos() as i128;
      let sounds_until = stop.map_or(played_out, |stop| stop.min(played_out));
      if sounds_until <= slice.start { return None; }
    }

    // in frames from the note's start, as the run has them
    let first = slice.frame(on);
    let starts = on.max(slice.start);
    let lead = (slice.frame(starts) - first) as u64;
    let end = stop.map(|stop| (slice.frame(stop) - first).max(0) as u64);
    let voice = instrument.voice(note.pitch, note.velocity, slice.rate, lead, end);
    Some((Duration::from_nanos((starts - slice.start) as u64), voice))
  }

  // the notes the clip plays on its pattern's synth rows that are heard
  // from `from` up to `to` (nanoseconds in the playlist), for the synths'
  // copies on `route` (none: the master's). each ends where it ends, at the
  // end of its time through the pattern, or at `cut`, whichever comes first
  #[allow(clippy::too_many_arguments)]
  fn synth_slice_notes(
    &self,
    sequencer: &daw::Sequencer,
    pattern: &daw::Pattern,
    beat: Duration,
    from: i128,
    to: i128,
    cut: i128,
    route: Option<daw::TrackRoute>,
  ) -> Vec<SliceNote> {
    let (passes, heard) = (self.passes_heard(pattern.length, beat, from, to), self.heard(beat));
    let mut notes = Vec::new();
    for (slot, synth_notes) in sequencer.synth_notes(pattern) {
      for pass in passes.clone() {
        let (_, starting) = Self::notes_around(synth_notes, &pass, heard, from, to);
        for note in starting {
          let (starts, ends) = pass.plays(note);
          let (on, off) = (heard(starts), heard(ends).min(cut));
          notes.push(SliceNote::new(on - from, off - from, slot, route.clone(), note));
        }
      }
    }
    notes
  }
}

// a slice of a run being scheduled: from `start` up to `end`, in
// nanoseconds of the playlist (or of a pattern playing on its own). every
// sound's frames are counted from where the run's frame 0 is (`run_zero`),
// so two sounds worked out from the same time land on the same frame
#[derive(Clone, Copy, Debug)]
struct Slice {
  start: i128,
  end: i128,
  run_zero: i128,
  // the engine's rate
  rate: u32,
  // the run's first slice: sounds that started before it and are still
  // sounding are played from where it starts in them
  resume: bool,
}

impl Slice {
  // the slice `dur` long from `start`, which is `at` into the run
  fn new(start: Duration, at: Duration, dur: Duration, rate: u32, resume: bool) -> Slice {
    let start = start.as_nanos() as i128;
    Slice { start, end: start + dur.as_nanos() as i128, run_zero: start - at.as_nanos() as i128, rate, resume }
  }

  // the frame of the run a time falls on
  fn frame(&self, at: i128) -> i64 {
    run_frame(at - self.run_zero, self.rate)
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
  // the synth row's slot
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
  next_id: u64,
  // every clip plays through its track's strip (shared with the playlist
  // state, so the mixer's controls don't need the graph's lock)
  pub mixer: Arc<daw::Mixer>,
  // the patterns that pattern clips play (shared with the playlist state).
  // always locked after the graph, never before it
  pub sequencer: Arc<Mutex<daw::Sequencer>>,
  // the fewest tracks the playlist shows, even with no clips on them: the
  // default number, or more once tracks are added (see add_track)
  pub min_tracks: u32,
  // clips that don't start playing for now: a loop recording's latest pass,
  // which is played as it was recorded until the loop comes round again
  // (see recording.rs)
  pub held_back: std::collections::BTreeSet<u64>,
  _phantom: PhantomData<&'a str>,
}

impl AudioGraph<'static> {
  pub fn new(sample_rate: u32, tempo: f32) -> Self {
    AudioGraph {
      nodes: Vec::<AudioNode>::new(),
      pattern_nodes: Vec::new(),
      running: false,
      current_offset: Some(Duration::ZERO),
      sample_rate,
      tempo,
      next_id: 0,
      mixer: Arc::new(daw::Mixer::default()),
      sequencer: Arc::new(Mutex::new(daw::Sequencer::default())),
      min_tracks: daw::defaults::NUM_OF_TRACKS,
      held_back: Default::default(),
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
    if !self.has_clips() || !self.running { return Vec::new(); }

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
      node.start_offset >= window_start && node.start_offset < window_end && !self.held_back.contains(&node.id))
  }

  // nodes that started before `at` and are still playing then
  fn nodes_sounding_at(&self, at: Duration) -> impl Iterator<Item = &AudioNode> {
    self.nodes.iter().filter(move |node|
      node.start_offset < at && node.start_offset + node.duration() > at && !self.held_back.contains(&node.id))
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
    let slice = Slice::new(window_start, at, dur, rate, resume);

    if resume {
      for node in self.nodes_sounding_at(window_start) {
        let voice = node.voice(rate, at.as_nanos() as i128, slice.run_zero);
        add(Duration::ZERO, Box::new(voice), node.track_number);
      }
    }

    // every node that starts within this time slice
    for node in self.nodes_in_window(window_start, dur) {
      let delay_offset = node.start_offset - window_start;
      let voice = node.voice(rate, (at + delay_offset).as_nanos() as i128, slice.run_zero);
      add(delay_offset, Box::new(voice), node.track_number);
    }

    // and every pattern clip's sample rows' notes, on their track's bus
    if !self.pattern_nodes.is_empty() {
      let sequencer = self.sequencer.lock().unwrap();
      let beat = self.beat_interval();
      for node in &self.pattern_nodes {
        let Some(pattern) = sequencer.pattern(node.pattern_id) else { continue };
        for (delay, voice) in node.sample_voices(&sequencer, pattern, beat, &slice) {
          add(delay, Box::new(voice), node.track_number);
        }
      }
    }

    sounds
  }

  // the sounds of the next `dur` of a pattern played on its own (in the
  // Sequencer tab), the slice starting `at` into the run, at the engine's
  // `rate`. the graph's current offset is where the run is in the pattern.
  // like run_sounds_at, but the notes go straight to the master bus
  // (there's no playlist track), and only the pattern plays
  pub fn pattern_sounds(
    &self,
    pattern_id: u64,
    at: Duration,
    rate: u32,
    dur: Duration,
    metronome: bool,
    resume: bool,
  ) -> Vec<SliceSound> {
    let mut sounds = Vec::new();
    if metronome { sounds.push(metronome_tick()); }

    let sequencer = self.sequencer.lock().unwrap();
    let Some(pattern) = sequencer.pattern(pattern_id) else { return sounds };
    let slice = Slice::new(self.current_offset.unwrap_or(Duration::ZERO), at, dur, rate, resume);
    let voices = PatternNode::alone(pattern).sample_voices(&sequencer, pattern, self.beat_interval(), &slice);
    for (delay, voice) in voices {
      sounds.push((delay, Box::new(voice), daw::Route::Master));
    }
    sounds
  }

  // the voices of every note a pattern clip plays on its pattern's sample
  // rows, as a run from the playlist's start at `rate` plays them, each with
  // how far into the clip it starts (what Deep copy writes out)
  pub fn pattern_clip_voices(&self, node: &PatternNode, rate: u32) -> Vec<(Duration, daw::SampleVoice)> {
    let sequencer = self.sequencer.lock().unwrap();
    let Some(pattern) = sequencer.pattern(node.pattern_id) else { return Vec::new() };
    let start = node.start_offset.as_nanos() as i128;
    let whole = Slice { start, end: i128::MAX, run_zero: 0, rate, resume: false };
    node.sample_voices(&sequencer, pattern, self.beat_interval(), &whole)
  }

  // the notes that start in the next `dur` of a run of the graph, for the
  // synths' copies on the pattern clips' tracks: each pattern clip's synth
  // rows' notes that start in the part of its pattern it plays, each ending
  // where it ends or at the clip's end, whichever comes first. a note that
  // started before the slice isn't played (an instrument can't start
  // partway through a note), so a run from the middle of one leaves it out
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
      let Some(pattern) = sequencer.pattern(node.pattern_id) else { continue };
      let route = Some(self.mixer.track_route(node.track_number));
      notes.extend(node.synth_slice_notes(&sequencer, pattern, beat, window_start, window_end.min(loop_end), loop_end, route));
    }
    notes
  }

  // which synths each track plays: (slot, track) for every pattern clip
  // with notes of that synth row that it plays some of (or would, were its
  // pattern longer). each needs a copy of the synth on that track
  pub fn synth_tracks(&self) -> BTreeSet<(u64, u32)> {
    let sequencer = self.sequencer.lock().unwrap();
    let mut pairs = BTreeSet::new();
    for node in &self.pattern_nodes {
      let Some(pattern) = sequencer.pattern(node.pattern_id) else { continue };
      for (slot, _) in sequencer.synth_notes(pattern) {
        pairs.insert((slot, node.track_number));
      }
    }
    pairs
  }

  // the notes of the next `dur` of a pattern played on its own (see
  // pattern_sounds), for the synths' copies on the master
  pub fn pattern_notes(&self, pattern_id: u64, dur: Duration) -> Vec<SliceNote> {
    let window_start = self.current_offset.unwrap_or(Duration::ZERO).as_nanos() as i128;
    let window_end = window_start + dur.as_nanos() as i128;
    let sequencer = self.sequencer.lock().unwrap();
    let Some(pattern) = sequencer.pattern(pattern_id) else { return Vec::new() };
    PatternNode::alone(pattern)
      .synth_slice_notes(&sequencer, pattern, self.beat_interval(), window_start, window_end, i128::MAX, None)
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
    self.held_back.clear();
    self.current_offset = Some(Duration::ZERO);
    self.min_tracks = daw::defaults::NUM_OF_TRACKS;
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

  // place a pattern in the playlist, snapped when `subdivision` is given: a
  // clip as long as the pattern is now. returns the clip's id
  pub fn add_pattern_node<T: daw::timing::MusicalTiming>(
    &mut self,
    pattern_id: u64,
    start_offset: Duration,
    track_number: u32,
    subdivision: Option<T>,
  ) -> Result<u64, String> {
    let length = self.sequencer.lock().unwrap().pattern(pattern_id)
      .map(|pattern| pattern.length)
      .ok_or_else(|| format!("no pattern with id {}", pattern_id))?;
    let start_offset = match subdivision {
      Some(s) => self.find_nearest_offset_subdivision(start_offset, s),
      None => start_offset,
    };
    let id = self.next_id;
    self.add_pattern_node_with_id(PatternNode::new(id, pattern_id, start_offset, track_number, length));
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

  // split a pattern clip into a clip per row it plays, sample or synth: each
  // plays a pattern holding just that row's notes (see
  // Sequencer::split_pattern), on a track of its own (see explode_tracks),
  // so each row gets its own mixer strip. a synth row's copy on that track
  // is made when the synths next follow the clips. unnamed tracks are named
  // after their rows. returns the new clips' ids, in row order
  pub fn explode_pattern_node(&mut self, id: u64) -> Result<Vec<u64>, String> {
    let clip = self.get_pattern_node_by_id(id).cloned()
      .ok_or_else(|| format!("no pattern clip with id {}", id))?;

    let (parts, tracks) = {
      let mut sequencer = self.sequencer.lock().unwrap();
      let parts = sequencer.split_pattern(clip.pattern_id)?;
      let sounds: Vec<daw::Sound> = parts.iter()
        .map(|part| sequencer.row(part.row).expect("a row the split found").sound())
        .collect();
      // every other clip, with what it plays when that's one sound
      let placed: Vec<(u32, Option<daw::Sound>)> = self.nodes.iter()
        .map(|n| (n.track_number, Some(daw::Sound::Sample(n.sample_path()))))
        .chain(self.pattern_nodes.iter()
          .filter(|n| n.id != id)
          .map(|n| (n.track_number, sequencer.sole_sound(n.pattern_id))))
        .collect();
      let tracks = explode_tracks(&sounds, clip.track_number, &placed);
      (parts, tracks)
    };

    self.remove_node(id)?;
    let mut ids = Vec::new();
    for (part, &track) in parts.iter().zip(&tracks) {
      let new_id = self.next_id;
      // each plays the same steps of its pattern as the clip did of its,
      // for as long (so a looping clip makes looping clips)
      self.add_pattern_node_with_id(PatternNode {
        id: new_id,
        pattern_id: part.pattern,
        track_number: track,
        ..clip.clone()
      });
      ids.push(new_id);
      if self.mixer.name(track).is_none() {
        self.mixer.set_name(track, &part.name);
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

  // where the last clip finishes playing (zero with no clips). a pattern
  // clip ends where its own length takes it (its last hits ring past it)
  pub fn end(&self) -> Duration {
    let beat = self.beat_interval();
    self.nodes
      .iter()
      .map(|node| node.start_offset + node.duration())
      .chain(self.pattern_nodes.iter().map(|node| node.start_offset + node.duration(beat)))
      .max()
      .unwrap_or(Duration::ZERO)
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

  // the same for a pattern clip, in steps of its pattern (counted as
  // PatternNode::start is): from `start` to `end` (None: where it ends now).
  // the end isn't kept to the pattern: past it, the clip plays the pattern
  // again (see pattern_step_at)
  pub fn set_pattern_window(&mut self, id: u64, start: f64, end: Option<f64>) -> Result<(), String> {
    let beat = self.beat_interval();
    let node = self.pattern_nodes.iter_mut().find(|n| n.id == id)
      .ok_or_else(|| format!("no pattern clip with id {}", id))?;
    let origin = node.origin(beat);
    // the steps before the playlist's start can't be played
    let earliest = if origin >= 0 || beat.is_zero() { 0. } else {
      (-origin) as f64 / 1e9 / beat.as_secs_f64() * daw::STEPS_PER_BEAT as f64
    };

    // (an end that makes no sense is where it ends now, and one too near the
    // start is moved later, where there's room)
    let start = start.max(earliest);
    let end = end.filter(|end| end.is_finite()).unwrap_or(node.end());
    node.set_span(start, end - start);
    node.start_offset = Duration::from_nanos((origin + steps_time(node.start, beat).as_nanos() as i128).max(0) as u64);
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
    let (from, to) = (node.start, node.end());
    // (a step's length in seconds, not rounded to a whole nanosecond)
    let step = beat.as_secs_f64() / daw::STEPS_PER_BEAT as f64;
    if !(step > 0.) { return Ok(None); }
    // (without the rounding noise, so a cut on a step is on it). it can be
    // past the pattern's end: the second half goes on from there
    let cut = micro_steps(from + (at.as_secs_f64() - node.start_offset.as_secs_f64()) / step);
    if cut < from + MIN_PATTERN_CLIP || cut + MIN_PATTERN_CLIP > to { return Ok(None); }

    let mut right = node.clone();
    right.id = new_id;
    right.start_offset = Duration::from_nanos(node.heard_at(cut, beat).max(0) as u64);
    right.set_span(cut, to - cut);

    let left = self.pattern_nodes.iter_mut().find(|n| n.id == id).expect("the clip that was just found");
    left.set_span(from, cut - from);

    self.add_pattern_node_with_id(right);
    Ok(Some(new_id))
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
  
  // how many tracks the playlist shows: min_tracks, or enough to reach the
  // highest track a clip is on. (counting only the tracks that have clips
  // would hide a clip on track 4 when just two tracks are used)
  pub fn track_count(&self) -> u32 {
    let needed = self.nodes.iter().map(|node| node.track_number)
      .chain(self.pattern_nodes.iter().map(|node| node.track_number))
      .map(|track_number| track_number + 1)
      .max()
      .unwrap_or(0);

    needed.max(self.min_tracks)
  }

  // an empty track below the last one. it stays when clips come and go
  // above it. returns the new track count
  pub fn add_track(&mut self) -> u32 {
    self.min_tracks = self.track_count() + 1;
    self.min_tracks
  }

}

// which track each sound of an exploded pattern goes on, so every sound
// gets a track (and mixer strip) of its own. a sound is whatever tells them
// apart (a Sound in the app, a name in the tests)
// - sounds: the pattern's, in row order
// - clip_track: the track the pattern clip was on
// - placed: every other clip in the playlist, as (track, sound), with None
//   for a pattern clip of more than one
//
// a sound goes on the track already holding only clips of it (its track
// from an earlier explode), if there is one. otherwise the first sound
// stays on the clip's track, unless that has other sounds on it, and the
// rest go on the next empty tracks below it (new ones past the last
// track), so they stay together under the clip
pub fn explode_tracks<S: Copy + PartialEq>(
  sounds: &[S],
  clip_track: u32,
  placed: &[(u32, Option<S>)],
) -> Vec<u32> {
  let mut placed: Vec<(u32, Option<S>)> = placed.to_vec();
  let mut tracks = Vec::new();

  for (i, &sound) in sounds.iter().enumerate() {
    let has_other_sound = |placed: &[(u32, Option<S>)], track: u32| placed.iter()
      .any(|&(t, s)| t == track && s.map_or(false, |s| s != sound));
    let dedicated = placed.iter()
      .filter(|&&(_, s)| s == Some(sound))
      .map(|&(t, _)| t)
      .filter(|&t| !has_other_sound(&placed, t))
      .min();

    let track = dedicated
      .or_else(|| if i == 0 && !has_other_sound(&placed, clip_track) { Some(clip_track) } else { None })
      .unwrap_or_else(|| (clip_track + 1..)
        .find(|&t| !placed.iter().any(|&(pt, _)| pt == t))
        .expect("an empty track below the clip"));

    placed.push((track, Some(sound)));
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
