// recording: an input stream whose callback meters what comes in and, while
// a take is being recorded, copies it into a lock-free queue with when it
// was captured. a writer thread drains the queue into a WAV file, so the
// audio thread never waits on the disk
//
// the input is opened at the output's rate (the engine follows its devices
// rather than resampling what's recorded), and only while something needs
// it: the Audio tab's meter, or a take being recorded. on macOS that's when
// the microphone light shows
use crate::daw::{self, AtomicF32};
use rodio::cpal::{
  self,
  traits::{DeviceTrait, HostTrait, StreamTrait},
  FromSample,
  Sample,
  SizedSample,
};
use rtrb::{Consumer, Producer, RingBuffer};
use std::path::{Path, PathBuf};
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

// a recorded take
#[derive(Clone, Debug, PartialEq)]
pub struct Take {
  pub path: PathBuf,
  pub channels: u16,
  pub sample_rate: u32,
  pub frames: u64,
  // when its first frame came into the input (none if nothing was recorded)
  pub started: Option<Instant>,
  // samples lost because the disk couldn't keep up
  pub dropped: u64,
}

impl Take {
  pub fn duration(&self) -> Duration {
    Duration::from_secs_f64(self.frames as f64 / self.sample_rate.max(1) as f64)
  }
}

// the queues the writer reads, handed back when a take is done
struct Queues {
  samples: Consumer<f32>,
  marks: Consumer<Mark>,
}

// an open input
pub struct Input {
  shared: Arc<InputShared>,
  queues: Arc<Mutex<Option<Queues>>>,
  device_name: String,
  sample_rate: u32,
  device_channels: u16,
  record_channels: u16,
  _close: Option<mpsc::Sender<()>>,
}

// a take being recorded
struct Recording {
  writer: thread::JoinHandle<Result<Take, String>>,
}

impl Input {
  // an input and the capturer its callback runs, without a device (the
  // input stream drives the capturer, or a test does)
  fn new(name: String, sample_rate: u32, device_channels: u16, record: Vec<usize>) -> (Input, Capturer) {
    let shared = Arc::new(InputShared {
      capturing: AtomicBool::new(false),
      peaks: (0..(device_channels as usize).min(MAX_INPUT_CHANNELS)).map(|_| AtomicF32::new(0.)).collect(),
      latency_ns: AtomicU64::new(0),
      dropped: AtomicU64::new(0),
    });
    let capacity = (sample_rate as usize * QUEUE_SECONDS * record.len()).max(1);
    let (samples_tx, samples_rx) = RingBuffer::new(capacity);
    // a mark a buffer: plenty for a few seconds of tiny buffers
    let (marks_tx, marks_rx) = RingBuffer::new(4096);

    let input = Input {
      shared: shared.clone(),
      queues: Arc::new(Mutex::new(Some(Queues { samples: samples_rx, marks: marks_rx }))),
      device_name: name,
      sample_rate,
      device_channels,
      record_channels: record.len() as u16,
      _close: None,
    };
    let capturer = Capturer {
      shared,
      samples: samples_tx,
      marks: marks_tx,
      record,
      frame: 0,
      was_capturing: false,
    };

    (input, capturer)
  }

  pub fn device_name(&self) -> &str {
    &self.device_name
  }

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

  // start recording a take into a WAV file at `path`
  fn start_take(&self, path: &Path) -> Result<Recording, String> {
    if self.queues.lock().unwrap().is_none() {
      return Err("a take is already being recorded".into());
    }

    let spec = hound::WavSpec {
      channels: self.record_channels,
      sample_rate: self.sample_rate,
      bits_per_sample: 32,
      sample_format: hound::SampleFormat::Float,
    };
    if let Some(dir) = path.parent() {
      std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not make the folder '{}': {}", dir.display(), e))?;
    }
    let file = hound::WavWriter::create(path, spec)
      .map_err(|e| format!("could not create '{}': {}", path.display(), e))?;

    // taken only once the file's made, so a failure leaves them in place
    let mut queues = self.queues.lock().unwrap().take()
      .ok_or("a take is already being recorded")?;
    // whatever's left from before belongs to no take
    while queues.samples.pop().is_ok() {}
    while queues.marks.pop().is_ok() {}
    self.shared.dropped.store(0, Ordering::Relaxed);

    let shared = self.shared.clone();
    let returned = self.queues.clone();
    let path = path.to_path_buf();
    let channels = self.record_channels;
    let sample_rate = self.sample_rate;

    self.shared.capturing.store(true, Ordering::Release);
    let writer = thread::Builder::new()
      .name("take writer".into())
      .spawn(move || {
        let result = write_take(file, &mut queues, &shared);
        *returned.lock().unwrap() = Some(queues);
        result.map(|(frames, started)| Take {
          path,
          channels,
          sample_rate,
          frames,
          started,
          dropped: shared.dropped.load(Ordering::Relaxed),
        })
      })
      .map_err(|e| format!("could not start writing the take: {}", e))?;

    Ok(Recording { writer })
  }

  // stop recording, once what's been captured is written
  fn stop_take(&self, recording: Recording) -> Result<Take, String> {
    self.shared.capturing.store(false, Ordering::Release);
    recording.writer.join().map_err(|_| "the take's writer stopped unexpectedly".to_string())?
  }
}

// drain the queues into the file until capturing stops and what's left has
// been written. returns the frames written and when the first was captured
fn write_take<W: std::io::Write + std::io::Seek>(
  mut file: hound::WavWriter<W>,
  queues: &mut Queues,
  shared: &InputShared,
) -> Result<(u64, Option<Instant>), String> {
  let mut samples = 0u64;
  let mut first: Option<Mark> = None;
  let channels = file.spec().channels.max(1) as u64;
  let failed = |e: hound::Error| format!("could not write the take: {}", e);

  loop {
    // once capturing has stopped, the callback may still be finishing the
    // buffer it was on: one more pass after a pause catches that
    let stopping = !shared.capturing.load(Ordering::Acquire);
    if stopping { thread::sleep(WRITE_EVERY); }

    while let Ok(mark) = queues.marks.pop() {
      first.get_or_insert(mark);
    }

    let ready = queues.samples.slots();
    if ready > 0 {
      let chunk = queues.samples.read_chunk(ready).expect("slots that are ready");
      let (a, b) = chunk.as_slices();
      for &sample in a.iter().chain(b) {
        file.write_sample(sample).map_err(failed)?;
      }
      samples += ready as u64;
      chunk.commit_all();
    }

    if stopping { break; }
    thread::sleep(WRITE_EVERY);
  }

  file.finalize().map_err(failed)?;
  Ok((samples / channels, first.map(|mark| mark.captured)))
}

// which of a device's channels a take records: the first two, or the only one
fn record_channels(device_channels: u16) -> Vec<usize> {
  (0..(device_channels as usize).min(2)).collect()
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
  let supported = configs.iter().max_by_key(|c| rank(c))
    .ok_or_else(|| format!(
      "'{}' can't record at {} Hz, the output's rate. Choose an input and output that share a rate",
      device_name, sample_rate))?
    .clone()
    .with_sample_rate(cpal::SampleRate(sample_rate));

  let format = supported.sample_format();
  let config: cpal::StreamConfig = supported.into();
  let channels = config.channels;

  println!("audio input: {} ({} Hz, {} channels, {:?})", device_name, sample_rate, channels, format);

  let (input, capturer) = Input::new(device_name, sample_rate, channels, record_channels(channels));
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
  recording: Option<(Arc<Input>, Recording)>,
}

lazy_static! {
  static ref INPUTS: Mutex<Inputs> = Mutex::new(Inputs { open: None, listeners: 0, recording: None });
}

fn open_or_reuse(inputs: &mut Inputs) -> Result<Arc<Input>, String> {
  if let Some(input) = &inputs.open { return Ok(input.clone()); }

  let rate = daw::engine()?.sample_rate();
  let settings = daw::audio_settings();
  let input = Arc::new(open_input(settings.input_device.as_deref(), rate)?);
  inputs.open = Some(input.clone());
  Ok(input)
}

fn close_if_unused(inputs: &mut Inputs) {
  if inputs.listeners == 0 && inputs.recording.is_none() {
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

// the open input, if any
pub fn input_if_open() -> Option<Arc<Input>> {
  INPUTS.lock().unwrap().open.clone()
}

pub fn recording() -> bool {
  INPUTS.lock().unwrap().recording.is_some()
}

// start recording a take into `path`
pub fn start_recording(path: &Path) -> Result<(), String> {
  let mut inputs = INPUTS.lock().unwrap();
  if inputs.recording.is_some() { return Err("already recording".into()); }

  let input = open_or_reuse(&mut inputs)?;
  let started = input.start_take(path);
  match started {
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

// stop recording, returning the take (none if nothing was recording)
pub fn stop_recording() -> Result<Option<Take>, String> {
  let mut inputs = INPUTS.lock().unwrap();
  let (input, recording) = match inputs.recording.take() {
    Some(recording) => recording,
    None => return Ok(None),
  };

  let take = input.stop_take(recording);
  close_if_unused(&mut inputs);
  take.map(Some)
}

// the input device or output changed: stop recording, and reopen the input
// (at the output's rate) the next time it's needed
pub fn reset_input() -> Option<Take> {
  let take = stop_recording().ok().flatten();
  let mut inputs = INPUTS.lock().unwrap();
  inputs.open = None;
  if inputs.listeners > 0 {
    if let Err(e) = open_or_reuse(&mut inputs) {
      println!("could not reopen the audio input: {}", e);
    }
  }
  take
}

#[cfg(test)]
mod tests {
  use super::*;

  const RATE: u32 = 1_000;

  fn temp_take(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("dawesome-take-{}-{}.wav", name, std::process::id()))
  }

  // interleaved frames whose samples count up, `channels` wide: frame n,
  // channel c is n + c / 10
  fn frames(start: usize, count: usize, channels: usize) -> Vec<f32> {
    (start..start + count)
      .flat_map(|n| (0..channels).map(move |c| n as f32 + c as f32 / 10.))
      .collect()
  }

  fn read(path: &Path) -> (hound::WavSpec, Vec<f32>) {
    let mut reader = hound::WavReader::open(path).unwrap();
    let spec = reader.spec();
    (spec, reader.samples::<f32>().map(|s| s.unwrap()).collect())
  }

  #[test]
  fn test_a_take_holds_what_was_captured_while_recording() {
    // a 4 channel device, recording its first two
    let (input, mut capturer) = Input::new("test".into(), RATE, 4, record_channels(4));
    let path = temp_take("captured");
    let t0 = Instant::now();

    // before recording: metered, not kept
    capturer.process(&frames(0, 10, 4), 4, t0);
    let recording = input.start_take(&path).unwrap();
    capturer.process(&frames(10, 10, 4), 4, t0 + Duration::from_millis(10));
    capturer.process(&frames(20, 5, 4), 4, t0 + Duration::from_millis(20));
    let take = input.stop_take(recording).unwrap();
    // after: not kept
    capturer.process(&frames(25, 10, 4), 4, t0 + Duration::from_millis(25));

    assert_eq!(take.frames, 15);
    assert_eq!(take.channels, 2);
    assert_eq!(take.sample_rate, RATE);
    assert_eq!(take.started, Some(t0 + Duration::from_millis(10)));
    assert_eq!(take.dropped, 0);
    assert_eq!(take.duration(), Duration::from_millis(15));

    let (spec, samples) = read(&path);
    assert_eq!(spec.sample_format, hound::SampleFormat::Float);
    assert_eq!(spec.bits_per_sample, 32);
    assert_eq!(spec.channels, 2);
    assert_eq!(samples, frames(10, 15, 2));

    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_takes_follow_one_another() {
    let (input, mut capturer) = Input::new("test".into(), RATE, 1, record_channels(1));
    let first = temp_take("first");
    let second = temp_take("second");

    let recording = input.start_take(&first).unwrap();
    capturer.process(&frames(0, 8, 1), 1, Instant::now());
    let one = input.stop_take(recording).unwrap();

    // a second take starts from nothing, though the queue was reused
    let recording = input.start_take(&second).unwrap();
    // and can't be started twice
    assert!(input.start_take(&temp_take("third")).is_err());
    capturer.process(&frames(100, 3, 1), 1, Instant::now());
    let two = input.stop_take(recording).unwrap();

    assert_eq!((one.frames, two.frames), (8, 3));
    assert_eq!(read(&second).1, frames(100, 3, 1));
    assert_eq!(read(&second).0.channels, 1);

    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
  }

  #[test]
  fn test_what_does_not_fit_is_counted_not_blocked_on() {
    let (input, mut capturer) = Input::new("test".into(), RATE, 1, record_channels(1));
    let path = temp_take("overflow");
    let recording = input.start_take(&path).unwrap();

    // more than the queue holds, faster than the writer drains it
    let capacity = RATE as usize * QUEUE_SECONDS;
    capturer.process(&frames(0, capacity + 500, 1), 1, Instant::now());
    let take = input.stop_take(recording).unwrap();

    assert_eq!(take.dropped, (capacity + 500) as u64);
    // the frames counted are those captured, so the take keeps its timing
    assert_eq!(take.frames, 0);

    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_the_meter_reads_every_channel() {
    let (input, mut capturer) = Input::new("test".into(), RATE, 3, record_channels(3));
    capturer.process(&[0.1, -0.5, 0.2, -0.3, 0.4, 0.], 3, Instant::now());

    assert_eq!(input.take_peaks(), vec![0.3, 0.5, 0.2]);
    // read once
    assert_eq!(input.take_peaks(), vec![0., 0., 0.]);
  }

  #[test]
  fn test_a_take_that_cannot_be_created_leaves_the_input_usable() {
    let (input, _capturer) = Input::new("test".into(), RATE, 1, record_channels(1));
    let file = temp_take("not-a-folder");
    std::fs::write(&file, b"in the way").unwrap();

    assert!(input.start_take(&file.join("take.wav")).is_err());
    let path = temp_take("after");
    let recording = input.start_take(&path).unwrap();
    input.stop_take(recording).unwrap();

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_record_channels() {
    assert_eq!(record_channels(1), vec![0]);
    assert_eq!(record_channels(2), vec![0, 1]);
    assert_eq!(record_channels(18), vec![0, 1]);
  }

  // records from the real default input: run with `cargo test -- --ignored`
  // (macOS asks for the microphone first)
  #[test]
  #[ignore = "needs an audio input device"]
  fn test_the_device_input_records() {
    let rate = daw::engine().expect("could not open the output").sample_rate();
    let input = open_input(None, rate).expect("could not open the input");
    let path = temp_take("device");

    let recording = input.start_take(&path).unwrap();
    thread::sleep(Duration::from_millis(300));
    let take = input.stop_take(recording).unwrap();

    println!(
      "{}: {} channels at {} Hz, recorded {:?} ({} frames), input latency {:?}, dropped {}",
      input.device_name(), input.device_channels(), rate, take.duration(), take.frames, input.latency(), take.dropped,
    );
    assert!(take.started.is_some());
    assert_eq!(take.dropped, 0);
    let expected = rate as f64 * 0.3;
    assert!((take.frames as f64 - expected).abs() < expected * 0.25, "{} frames", take.frames);
    assert_eq!(hound::WavReader::open(&path).unwrap().duration() as u64, take.frames);

    let _ = std::fs::remove_file(&path);
  }
}
