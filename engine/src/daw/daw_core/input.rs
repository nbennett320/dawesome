// recording: an input stream whose callback meters what comes in and, while
// a take is being recorded, copies it into a lock-free queue with when it
// was captured. a writer thread drains the queue into a WAV file, so the
// audio thread never waits on the disk
//
// the input is opened at the output's rate when it can be. an input that
// can't (like a Bluetooth headset's microphone) records at its own rate, and
// its takes are converted to the output's rate as they play, like any sample
// at another rate. it's only open while something needs
// it: the Audio tab's meter, a track armed to record (so pressing Record
// doesn't wait for the device to open), or a take being recorded. on macOS
// that's when the microphone light shows
use crate::daw::{self, AtomicF32};
use rodio::cpal::{
  self,
  traits::{DeviceTrait, HostTrait, StreamTrait},
  FromSample,
  SizedSample,
};
use rtrb::{Consumer, Producer, RingBuffer};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// the most input channels metered
pub const MAX_INPUT_CHANNELS: usize = 32;
// how much audio the queue holds before the writer must have caught up
const QUEUE_SECONDS: usize = 4;
// how often the writer drains the queue
const WRITE_EVERY: Duration = Duration::from_millis(10);

// when a block of input was captured: the take's frame it starts on, and
// when that frame came into the device
#[derive(Clone, Copy, Debug, PartialEq)]
struct Mark {
  frame: u64,
  captured: Instant,
}

// what the callback reports, readable from any thread
struct InputShared {
  // set while a take is being recorded
  capturing: AtomicBool,
  // loudest sample of each channel since the meter last read it
  peaks: Vec<AtomicF32>,
  // from a frame coming into the device to the callback getting it
  latency_ns: AtomicU64,
  // samples that didn't fit in the queue (the writer fell behind)
  dropped: AtomicU64,
}

// the callback's side: meters every buffer, and queues the recorded channels
// while capturing
struct Capturer {
  shared: Arc<InputShared>,
  samples: Producer<f32>,
  marks: Producer<Mark>,
  // the device channels recorded, in order
  record: Vec<usize>,
  // frames queued in this take
  frame: u64,
  was_capturing: bool,
}

impl Capturer {
  // one buffer of interleaved input with `channels` channels, whose first
  // frame was captured at `captured`
  fn process(&mut self, data: &[f32], channels: usize, captured: Instant) {
    let channels = channels.max(1);

    for frame in data.chunks_exact(channels) {
      for (peak, &sample) in self.shared.peaks.iter().zip(frame) {
        peak.fetch_max(sample.abs());
      }
    }

    let capturing = self.shared.capturing.load(Ordering::Acquire);
    if capturing && !self.was_capturing { self.frame = 0; }
    self.was_capturing = capturing;
    if !capturing { return; }

    // the writer reads marks as it goes. without room for one, the timing
    // of the take is still known from an earlier mark
    let _ = self.marks.push(Mark { frame: self.frame, captured });

    let frames = data.len() / channels;
    let wanted = frames * self.record.len();
    match self.samples.write_chunk_uninit(wanted) {
      Ok(chunk) => {
        let samples = data.chunks_exact(channels)
          .flat_map(|frame| self.record.iter().map(move |&ch| frame.get(ch).copied().unwrap_or(0.)));
        chunk.fill_from_iter(samples);
      }
      Err(_) => {
        self.shared.dropped.fetch_add(wanted as u64, Ordering::Relaxed);
      }
    }
    self.frame += frames as u64;
  }
}

// a file a take is recorded into, from some of the input's channels
#[derive(Clone, Debug, PartialEq)]
pub struct TakeFile {
  pub path: PathBuf,
  // the device channels it records, in order (one, or a stereo pair)
  pub channels: Vec<usize>,
}

// a file of a recorded take
#[derive(Clone, Debug, PartialEq)]
pub struct Take {
  pub path: PathBuf,
  pub channels: u16,
  pub sample_rate: u32,
  pub frames: u64,
}

// a recorded take: a file for each part of the input recorded
#[derive(Clone, Debug, PartialEq)]
pub struct Takes {
  pub files: Vec<Take>,
  // when the first frame kept came into the input (none if nothing was)
  pub started: Option<Instant>,
  // samples lost because the disk couldn't keep up
  pub dropped: u64,
  // which pass of a loop recording it is, from 0 (always 0 otherwise)
  pub pass: u32,
}

// how a loop recording is cut into passes: the first ends `first` after
// the take starts (at the loop's end), and each after it `every` later (a
// loop's length). each pass is a new set of files
pub struct Passes {
  pub first: Duration,
  pub every: Duration,
  // the paths of the next pass's files, one for each of the first pass's,
  // in order
  pub next_paths: Box<dyn FnMut() -> Vec<PathBuf> + Send>,
  // each pass's takes as soon as it's finished (but the last, which
  // stopping returns)
  pub finished: mpsc::Sender<Takes>,
}

// a pass being recorded, playable as it's recorded: what each of its files
// has had so far
#[derive(Clone)]
pub struct LivePass {
  pub index: u32,
  pub files: Vec<Arc<daw::LiveSamples>>,
}

// the newest passes of a loop recording, as they're recorded
#[derive(Default)]
pub struct LivePasses {
  passes: Mutex<VecDeque<LivePass>>,
}

impl LivePasses {
  // how many are kept: the one being recorded, and the one before (which
  // may still be about to play)
  const KEPT: usize = 3;

  pub fn pass(&self, index: u32) -> Option<LivePass> {
    self.passes.lock().unwrap().iter().find(|pass| pass.index == index).cloned()
  }

  fn add(&self, pass: LivePass) {
    let mut passes = self.passes.lock().unwrap();
    passes.push_back(pass);
    while passes.len() > Self::KEPT { passes.pop_front(); }
  }
}

// the queues the writer reads, handed back when a take is done
struct Queues {
  samples: Consumer<f32>,
  marks: Consumer<Mark>,
}

// what the writer's told while it runs
#[derive(Default)]
struct TakeControl {
  // frames captured before this aren't kept. until it's known, nothing is
  // written (the queue holds what comes in meanwhile)
  keep_from: Mutex<Option<Instant>>,
  // the most frames kept
  limit: Mutex<Option<u64>>,
  // a loop recording's passes (set with keep_from)
  passes: Mutex<Option<Passes>>,
  live: Arc<LivePasses>,
}

// an open input
pub struct Input {
  shared: Arc<InputShared>,
  queues: Arc<Mutex<Option<Queues>>>,
  device_name: String,
  sample_rate: u32,
  device_channels: u16,
  // the channels queued while capturing (all of them, up to the most metered)
  captured_channels: u16,
  _close: Option<mpsc::Sender<()>>,
}

// a take being recorded
pub struct Recording {
  writer: thread::JoinHandle<Result<Takes, String>>,
  control: Arc<TakeControl>,
}

impl Recording {
  // keep what was captured from `at` on (when the run's first frame was
  // heard, and so when the performer started playing along), cut into
  // passes if it's a loop recording. returns its passes as they're recorded
  pub fn keep_from(&self, at: Instant, passes: Option<Passes>) -> Arc<LivePasses> {
    // (before the start, so the writer has both once it knows the start)
    *self.control.passes.lock().unwrap() = passes;
    *self.control.keep_from.lock().unwrap() = Some(at);
    self.control.live.clone()
  }
}

impl Input {
  // an input and the capturer its callback runs, without a device (the
  // input stream drives the capturer, or a test does)
  fn new(name: String, sample_rate: u32, device_channels: u16) -> (Input, Capturer) {
    let channels = (device_channels as usize).clamp(1, MAX_INPUT_CHANNELS);
    let shared = Arc::new(InputShared {
      capturing: AtomicBool::new(false),
      peaks: (0..channels).map(|_| AtomicF32::new(0.)).collect(),
      latency_ns: AtomicU64::new(0),
      dropped: AtomicU64::new(0),
    });
    let capacity = sample_rate as usize * QUEUE_SECONDS * channels;
    let (samples_tx, samples_rx) = RingBuffer::new(capacity.max(1));
    // a mark a buffer: plenty for a few seconds of tiny buffers
    let (marks_tx, marks_rx) = RingBuffer::new(4096);

    let input = Input {
      shared: shared.clone(),
      queues: Arc::new(Mutex::new(Some(Queues { samples: samples_rx, marks: marks_rx }))),
      device_name: name,
      sample_rate,
      device_channels,
      captured_channels: channels as u16,
      _close: None,
    };
    let capturer = Capturer {
      shared,
      samples: samples_tx,
      marks: marks_tx,
      record: (0..channels).collect(),
      frame: 0,
      was_capturing: false,
    };

    (input, capturer)
  }

  pub fn device_name(&self) -> &str {
    &self.device_name
  }

  // what it records at: the output's rate, unless it can't
  pub fn sample_rate(&self) -> u32 {
    self.sample_rate
  }

  pub fn device_channels(&self) -> u16 {
    self.device_channels
  }

  // from a frame coming into the device to the callback getting it
  pub fn latency(&self) -> Duration {
    Duration::from_nanos(self.shared.latency_ns.load(Ordering::Relaxed))
  }

  // each channel's loudest sample since this was last asked
  pub fn take_peaks(&self) -> Vec<f32> {
    self.shared.peaks.iter().map(|peak| peak.take()).collect()
  }

  // start recording a take into WAV files, each from some of the channels
  fn start_take(&self, files: &[TakeFile]) -> Result<Recording, String> {
    if self.queues.lock().unwrap().is_none() {
      return Err("a take is already being recorded".into());
    }
    if files.is_empty() {
      return Err("there's nothing to record into".into());
    }

    for file in files {
      if file.channels.is_empty() || file.channels.iter().any(|&ch| ch >= self.captured_channels as usize) {
        return Err(format!(
          "'{}' only has {} input channels", self.device_name, self.captured_channels));
      }
    }
    let writers = create_writers(files, self.sample_rate)?;

    // taken only once the files are made, so a failure leaves them in place
    let mut queues = self.queues.lock().unwrap().take()
      .ok_or("a take is already being recorded")?;
    // whatever's left from before belongs to no take
    while queues.samples.pop().is_ok() {}
    while queues.marks.pop().is_ok() {}
    self.shared.dropped.store(0, Ordering::Relaxed);

    let shared = self.shared.clone();
    let returned = self.queues.clone();
    let control = Arc::new(TakeControl::default());
    let writer_control = control.clone();
    let channels = self.captured_channels as usize;
    let sample_rate = self.sample_rate;

    self.shared.capturing.store(true, Ordering::Release);
    let writer = thread::Builder::new()
      .name("take writer".into())
      .spawn(move || {
        let result = write_take(writers, &mut queues, &shared, &writer_control, channels, sample_rate);
        *returned.lock().unwrap() = Some(queues);
        result
      })
      .map_err(|e| format!("could not start writing the take: {}", e))?;

    Ok(Recording { writer, control })
  }

  // stop recording, keeping at most `limit` of it, once what's been
  // captured is written
  fn stop_take(&self, recording: Recording, limit: Option<Duration>) -> Result<Takes, String> {
    // in this input's frames, which may not be the output's
    let frames = limit.map(|limit| (limit.as_secs_f64() * self.sample_rate as f64).round() as u64);
    *recording.control.limit.lock().unwrap() = frames;
    self.shared.capturing.store(false, Ordering::Release);
    recording.writer.join().map_err(|_| "the take's writer stopped unexpectedly".to_string())?
  }
}

// a take's files, ready to write
type Writers = Vec<(TakeFile, hound::WavWriter<std::io::BufWriter<std::fs::File>>)>;

fn create_writers(files: &[TakeFile], sample_rate: u32) -> Result<Writers, String> {
  let mut writers = Vec::new();
  for file in files {
    let spec = hound::WavSpec {
      channels: file.channels.len() as u16,
      sample_rate,
      bits_per_sample: 32,
      sample_format: hound::SampleFormat::Float,
    };
    if let Some(dir) = file.path.parent() {
      std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not make the folder '{}': {}", dir.display(), e))?;
    }
    let writer = hound::WavWriter::create(&file.path, spec)
      .map_err(|e| format!("could not create '{}': {}", file.path.display(), e))?;
    writers.push((file.clone(), writer));
  }
  Ok(writers)
}

// finish a take's files, `frames` long
fn finalize(writers: Writers, frames: u64, sample_rate: u32) -> Result<Vec<Take>, String> {
  let mut files = Vec::new();
  for (file, writer) in writers {
    writer.finalize().map_err(|e| format!("could not write the take: {}", e))?;
    files.push(Take { path: file.path, channels: file.channels.len() as u16, sample_rate, frames });
  }
  Ok(files)
}

// a loop recording being cut into passes, as the writer goes
struct Cutting {
  plan: Passes,
  // the pass being written, the kept frame it started on, and the one it
  // ends before
  index: u32,
  from: u64,
  until: u64,
  // frames the take started late by (it's cut where the loop comes round,
  // which was planned from when it should have started)
  late: u64,
  sample_rate: u32,
  // what's been written of the pass, to be played as it's recorded
  live: Vec<Arc<daw::LiveSamples>>,
}

impl Cutting {
  fn new(plan: Passes, late: u64, sample_rate: u32, files: &[TakeFile], live: &LivePasses) -> Self {
    let mut cutting = Cutting { plan, index: 0, from: 0, until: 0, late, sample_rate, live: Vec::new() };
    cutting.until = cutting.end_of(0);
    cutting.start_live(files, live);
    cutting
  }

  // the kept frame pass `index` ends before: where the loop comes round
  // for the (index + 1)th time, counted from the take's start
  fn end_of(&self, index: u32) -> u64 {
    let at = self.plan.first + self.plan.every * index;
    let frames = (at.as_secs_f64() * self.sample_rate as f64).round() as u64;
    frames.saturating_sub(self.late).max(self.from + 1)
  }

  fn start_live(&mut self, files: &[TakeFile], live: &LivePasses) {
    let frames = (self.until - self.from) as usize;
    self.live = files.iter()
      .map(|file| Arc::new(daw::LiveSamples::new(frames, file.channels.len() as u16, self.sample_rate)))
      .collect();
    live.add(LivePass { index: self.index, files: self.live.clone() });
  }

  // the pass is over at kept frame `kept`: finish its files, hand them on,
  // and start the next pass's
  fn next(&mut self, writers: Writers, kept: u64, started: Option<Instant>, live: &LivePasses) -> Result<Writers, String> {
    let files: Vec<TakeFile> = writers.iter().map(|(file, _)| file.clone()).collect();
    let done = finalize(writers, kept - self.from, self.sample_rate)?;
    let _ = self.plan.finished.send(Takes { files: done, started, dropped: 0, pass: self.index });

    let paths = (self.plan.next_paths)();
    let next: Vec<TakeFile> = files.iter().zip(paths)
      .map(|(file, path)| TakeFile { path, channels: file.channels.clone() })
      .collect();
    let writers = create_writers(&next, self.sample_rate)?;

    self.index += 1;
    self.from = kept;
    self.until = self.end_of(self.index);
    self.start_live(&next, live);
    Ok(writers)
  }
}

// the first frame captured at or after `at`, and when it was captured, from
// the marks so far (none until a mark has come in)
fn first_kept(marks: &[Mark], at: Instant, sample_rate: u32) -> Option<(u64, Instant)> {
  let rate = sample_rate.max(1) as f64;
  let first = marks.first()?;
  // captured before the take started: keep it all
  if first.captured >= at { return Some((first.frame, first.captured)); }

  let mark = marks.iter().rev().find(|mark| mark.captured <= at).unwrap_or(first);
  let into = (at - mark.captured).as_secs_f64() * rate;
  let frames = into.ceil() as u64;
  Some((mark.frame + frames, mark.captured + Duration::from_secs_f64(frames as f64 / rate)))
}

// drain the queues into the files until capturing stops and what's left has
// been written, keeping frames from the take's start (once that's known) up
// to its limit
fn write_take(
  mut writers: Writers,
  queues: &mut Queues,
  shared: &InputShared,
  control: &TakeControl,
  channels: usize,
  sample_rate: u32,
) -> Result<Takes, String> {
  let failed = |e: hound::Error| format!("could not write the take: {}", e);
  let mut marks: Vec<Mark> = Vec::new();
  // the first frame kept, once known, and when it was captured
  let mut skip: Option<u64> = None;
  let mut started: Option<Instant> = None;
  // a loop recording's passes, once the take's start is known
  let mut cutting: Option<Cutting> = None;
  // frames read from the queue, and kept
  let mut read = 0u64;
  let mut kept = 0u64;

  loop {
    // once capturing has stopped, the callback may still be finishing the
    // buffer it was on: one more pass after a pause catches that
    let stopping = !shared.capturing.load(Ordering::Acquire);
    if stopping { thread::sleep(WRITE_EVERY); }

    while let Ok(mark) = queues.marks.pop() {
      marks.push(mark);
    }

    if skip.is_none() {
      let keep_from = *control.keep_from.lock().unwrap();
      let found = match keep_from {
        Some(at) => first_kept(&marks, at, sample_rate),
        None => None,
      };
      match found {
        Some((frame, at)) => {
          skip = Some(frame);
          started = Some(at);
          if let (Some(plan), Some(keep_from)) = (control.passes.lock().unwrap().take(), keep_from) {
            let late = at.saturating_duration_since(keep_from).as_secs_f64() * sample_rate as f64;
            let files: Vec<TakeFile> = writers.iter().map(|(file, _)| file.clone()).collect();
            cutting = Some(Cutting::new(plan, late.round() as u64, sample_rate, &files, &control.live));
          }
        }
        // stopped before the take's start was known: keep it all
        None if stopping => {
          skip = Some(0);
          started = marks.first().map(|mark| mark.captured);
        }
        None => {}
      }
    }

    if let Some(skip) = skip {
      let limit = control.limit.lock().unwrap().unwrap_or(u64::MAX);
      let ready = queues.samples.slots() / channels * channels;
      if ready > 0 {
        let chunk = queues.samples.read_chunk(ready).expect("slots that are ready");
        let (a, b) = chunk.as_slices();
        let samples: Vec<f32> = a.iter().chain(b).copied().collect();
        chunk.commit_all();

        for frame in samples.chunks_exact(channels) {
          if read >= skip && kept < limit {
            // the loop's come round: a new pass
            if let Some(cut) = cutting.as_mut() {
              if kept >= cut.until {
                writers = cut.next(writers, kept, started, &control.live)?;
              }
            }
            for (n, (file, writer)) in writers.iter_mut().enumerate() {
              for &ch in &file.channels {
                writer.write_sample(frame[ch]).map_err(failed)?;
              }
              if let Some(live) = cutting.as_ref().and_then(|cut| cut.live.get(n)) {
                live.push(file.channels.iter().map(|&ch| frame[ch]));
              }
            }
            kept += 1;
          }
          read += 1;
        }
      }
    }

    if stopping { break; }
    thread::sleep(WRITE_EVERY);
  }

  // the last pass (or the whole take)
  let (from, pass) = cutting.as_ref().map_or((0, 0), |cut| (cut.from, cut.index));
  let files = finalize(writers, kept - from, sample_rate)?;

  Ok(Takes {
    files,
    started: if kept > 0 { started } else { None },
    dropped: shared.dropped.load(Ordering::Relaxed),
    pass,
  })
}

// an input device by name, or the system's default
pub fn input_device(name: Option<&str>) -> Result<cpal::Device, String> {
  let host = cpal::default_host();
  match name {
    None => host.default_input_device().ok_or_else(|| "there's no audio input device".to_string()),
    Some(name) => host
      .input_devices()
      .map_err(|e| format!("could not list the audio inputs: {}", e))?
      .find(|device| device.name().map(|n| n == name).unwrap_or(false))
      .ok_or_else(|| format!("there's no audio input named '{}'", name)),
  }
}

// open an input at `sample_rate` and start metering it. the stream lives on
// its own thread until the input is dropped
fn open_input(name: Option<&str>, sample_rate: u32) -> Result<Input, String> {
  let (opened_tx, opened_rx) = mpsc::channel();
  let (close_tx, close_rx) = mpsc::channel::<()>();
  let name = name.map(str::to_string);

  thread::Builder::new()
    .name("audio input".into())
    .spawn(move || {
      let stream = match build_input(name.as_deref(), sample_rate) {
        Ok((input, stream)) => {
          let _ = opened_tx.send(Ok(input));
          stream
        }
        Err(e) => {
          let _ = opened_tx.send(Err(e));
          return;
        }
      };

      let _ = close_rx.recv();
      drop(stream);
      println!("audio input closed");
    })
    .map_err(|e| format!("could not start the audio input thread: {}", e))?;

  let mut input = opened_rx.recv().map_err(|_| "the audio input thread exited".to_string())??;
  input._close = Some(close_tx);
  Ok(input)
}

fn build_input(name: Option<&str>, sample_rate: u32) -> Result<(Input, cpal::Stream), String> {
  let device = input_device(name)?;
  let device_name = device.name().unwrap_or_else(|_| "Unknown input".into());
  let default_channels = device.default_input_config().map(|c| c.channels()).unwrap_or(2);

  // a config at the output's rate: the device's usual channels if it can,
  // and samples as floats if it can
  let configs: Vec<_> = device.supported_input_configs()
    .map_err(|e| format!("could not read the settings of '{}': {}", device_name, e))?
    .filter(|c| c.min_sample_rate().0 <= sample_rate && c.max_sample_rate().0 >= sample_rate)
    .collect();
  let rank = |c: &cpal::SupportedStreamConfigRange| (
    c.channels() == default_channels,
    c.sample_format() == cpal::SampleFormat::F32,
  );
  let supported = match configs.iter().max_by_key(|c| rank(c)) {
    Some(config) => config.clone().with_sample_rate(cpal::SampleRate(sample_rate)),
    // it can't: its own rate, converted as its takes play
    None => device.default_input_config()
      .map_err(|e| format!("could not read the settings of '{}': {}", device_name, e))?,
  };

  let format = supported.sample_format();
  let config: cpal::StreamConfig = supported.into();
  let channels = config.channels;
  let rate = config.sample_rate.0;

  println!("audio input: {} ({} Hz, {} channels, {:?})", device_name, rate, channels, format);
  if rate != sample_rate {
    println!("  (it can't record at the output's {} Hz: its takes are converted as they play)", sample_rate);
  }

  let (input, capturer) = Input::new(device_name, rate, channels);
  let stream = match format {
    cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, capturer),
    cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, capturer),
    cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, capturer),
    cpal::SampleFormat::I32 => build_stream::<i32>(&device, &config, capturer),
    other => return Err(format!("unsupported input sample format: {:?}", other)),
  }?;
  stream.play().map_err(|e| format!("could not start the audio input: {}", e))?;

  Ok((input, stream))
}

fn build_stream<T>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  mut capturer: Capturer,
) -> Result<cpal::Stream, String>
where
  T: SizedSample,
  f32: FromSample<T>,
{
  let channels = config.channels.max(1) as usize;
  // converted here first, allocated once (and again only for an unusually
  // big buffer)
  let mut floats: Vec<f32> = Vec::with_capacity(8192 * channels);

  device.build_input_stream(
    config,
    move |data: &[T], info: &cpal::InputCallbackInfo| {
      let now = Instant::now();
      let stamp = info.timestamp();
      let latency = stamp.callback.duration_since(&stamp.capture).unwrap_or_default();
      capturer.shared.latency_ns.store(latency.as_nanos() as u64, Ordering::Relaxed);

      floats.clear();
      floats.extend(data.iter().map(|&s| s.to_sample::<f32>()));
      capturer.process(&floats, channels, now.checked_sub(latency).unwrap_or(now));
    },
    |e| eprintln!("audio input error: {}", e),
    None,
  ).map_err(|e| format!("could not open the audio input: {}", e))
}

// the app's input, opened while something needs it
struct Inputs {
  open: Option<Arc<Input>>,
  // the meters looking at it
  listeners: u32,
  // kept open while a track is armed, ready to record
  held: bool,
  recording: Option<(Arc<Input>, Recording)>,
  // why the chosen input isn't the one open, if it isn't
  notice: Option<String>,
}

lazy_static! {
  static ref INPUTS: Mutex<Inputs> = Mutex::new(Inputs { open: None, listeners: 0, held: false, recording: None, notice: None });
}

// use the chosen input, or (when it's gone, or won't work) the system's
// default, saying why
fn with_fallback<T>(
  chosen: Option<&str>,
  use_input: impl Fn(Option<&str>) -> Result<T, String>,
) -> Result<(T, Option<String>), String> {
  match (use_input(chosen), chosen) {
    (Ok(input), _) => Ok((input, None)),
    (Err(e), Some(name)) => {
      let input = use_input(None)?;
      Ok((input, Some(format!(
        "'{}' couldn't be used ({}), so the system's default input is used instead", name, e))))
    }
    (Err(e), None) => Err(e),
  }
}

fn open_or_reuse(inputs: &mut Inputs) -> Result<Arc<Input>, String> {
  if let Some(input) = &inputs.open { return Ok(input.clone()); }

  let rate = daw::engine()?.sample_rate();
  let settings = daw::audio_settings();
  let (input, notice) = with_fallback(settings.input_device.as_deref(), |name| open_input(name, rate))?;
  if let Some(notice) = &notice { println!("{}", notice); }

  let input = Arc::new(input);
  inputs.open = Some(input.clone());
  inputs.notice = notice;
  Ok(input)
}

// why the chosen input isn't the one in use, if it isn't
pub fn input_notice() -> Option<String> {
  INPUTS.lock().unwrap().notice.clone()
}

fn close_if_unused(inputs: &mut Inputs) {
  if inputs.listeners == 0 && !inputs.held && inputs.recording.is_none() {
    inputs.open = None;
  }
}

// start or stop metering the input (the Audio tab, while it's showing)
pub fn listen_to_input(on: bool) -> Result<(), String> {
  let mut inputs = INPUTS.lock().unwrap();
  if on {
    open_or_reuse(&mut inputs)?;
    inputs.listeners += 1;
  } else {
    inputs.listeners = inputs.listeners.saturating_sub(1);
    close_if_unused(&mut inputs);
  }
  Ok(())
}

// keep the input open while any track is armed, so a recording starts
// straight away, or (off) let it close once nothing else needs it
pub fn hold_input(on: bool) -> Result<(), String> {
  let mut inputs = INPUTS.lock().unwrap();
  inputs.held = on;
  if on {
    open_or_reuse(&mut inputs)?;
  } else {
    close_if_unused(&mut inputs);
  }
  Ok(())
}

// the open input, if any
pub fn input_if_open() -> Option<Arc<Input>> {
  INPUTS.lock().unwrap().open.clone()
}

// how many channels the chosen input has, without opening it
pub fn input_channel_count() -> Result<u16, String> {
  if let Some(input) = input_if_open() { return Ok(input.device_channels()); }

  // the same input opening it would use
  let settings = daw::audio_settings();
  let (channels, _) = with_fallback(settings.input_device.as_deref(), |name| {
    input_device(name)?
      .default_input_config()
      .map(|config| config.channels())
      .map_err(|e| format!("could not read its settings: {}", e))
  })?;
  Ok(channels)
}

// start recording a take into these files. nothing's kept until it's told
// when the take starts (see keep_recording_from)
pub fn start_recording(files: &[TakeFile]) -> Result<(), String> {
  let mut inputs = INPUTS.lock().unwrap();
  if inputs.recording.is_some() { return Err("already recording".into()); }

  let input = open_or_reuse(&mut inputs)?;
  match input.start_take(files) {
    Ok(recording) => {
      inputs.recording = Some((input, recording));
      Ok(())
    }
    Err(e) => {
      close_if_unused(&mut inputs);
      Err(e)
    }
  }
}

// keep what's captured from `at` on, cut into passes if it's a loop
// recording. returns its passes as they're recorded (none if nothing's
// recording)
pub fn keep_recording_from(at: Instant, passes: Option<Passes>) -> Option<Arc<LivePasses>> {
  INPUTS.lock().unwrap().recording.as_ref().map(|(_, recording)| recording.keep_from(at, passes))
}

// stop recording, keeping at most `limit` of it, returning the take (none if
// nothing was recording)
pub fn stop_recording(limit: Option<Duration>) -> Result<Option<Takes>, String> {
  let mut inputs = INPUTS.lock().unwrap();
  let (input, recording) = match inputs.recording.take() {
    Some(recording) => recording,
    None => return Ok(None),
  };

  let takes = input.stop_take(recording, limit);
  close_if_unused(&mut inputs);
  takes.map(Some)
}

// the input device or output changed: stop recording, and reopen the input
// (at the output's rate) the next time it's needed
pub fn reset_input() -> Option<Takes> {
  let takes = stop_recording(None).ok().flatten();
  let mut inputs = INPUTS.lock().unwrap();
  inputs.open = None;
  inputs.notice = None;
  if inputs.listeners > 0 || inputs.held {
    if let Err(e) = open_or_reuse(&mut inputs) {
      println!("could not reopen the audio input: {}", e);
    }
  }
  takes
}

#[cfg(test)]
mod tests;
