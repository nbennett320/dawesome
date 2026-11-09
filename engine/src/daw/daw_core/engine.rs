// the audio engine: one output stream for the whole app, whose callback is
// the clock everything plays by
//
// the callback renders sounds (voices) it's been sent, each starting on an
// exact frame of a run (a stretch of playback from play to stop). a run's
// frame 0 is wherever the callback is when it gets the run's start, and the
// scheduler (see daw::run_playlist) sends each beat's voices a little before
// the callback reaches them, tagged with the frame they start on
//
// the callback never takes a lock, touches the disk, or frees memory: voices
// arrive and leave through lock-free queues, and finished ones are sent back
// to be freed by whoever sends the next command
use crate::daw;
use rodio::cpal::{
  self,
  traits::{DeviceTrait, HostTrait, StreamTrait},
  FromSample,
  SizedSample,
};
use rodio::source::UniformSourceIterator;
use rodio::Source;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// a sound sent to the engine
pub type VoiceSource = Box<dyn Source<Item = f32> + Send>;

// where a voice is heard
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
  // through the master strip and its limiter (clips and pattern hits)
  Master,
  // straight to the output (the metronome and previews)
  Direct,
}

pub struct Voice {
  // the run it belongs to, and the frame of that run it starts on. a voice
  // without a run (a preview) starts as soon as the engine gets it, and
  // isn't stopped with the run
  run: Option<u64>,
  at: u64,
  route: Route,
  // stereo, at the engine's rate
  samples: UniformSourceIterator<VoiceSource, f32>,
  // the engine's frame it starts on, once it's been placed
  start: u64,
  // frames left in its fade out, once it's been stopped
  fade_left: Option<u32>,
}

enum Command {
  // start a run, stopping any other, with the master strip's controls and
  // its first voices
  Start { run: u64, master: Arc<daw::ChannelControls>, voices: Vec<Box<Voice>> },
  // stop a run (if it's the one playing), fading out what's sounding
  Stop { run: u64 },
  Play(Vec<Box<Voice>>),
}

// what the callback is done with, sent back to be freed off the audio thread
// (only held, to be dropped)
#[allow(dead_code)]
enum Garbage {
  Voice(Box<Voice>),
  Voices(Vec<Box<Voice>>),
}

// how many voices can sound at once. more are dropped (and counted)
const MAX_VOICES: usize = 1024;
const QUEUE_LEN: usize = 4096;
// how long stopping takes to fade out, so it doesn't click
const STOP_FADE: Duration = Duration::from_millis(10);
// the most frames rendered in one go. longer buffers are rendered in parts
const MAX_CHUNK: usize = 4096;

// what the callback reports, readable from any thread
struct Shared {
  // when the engine started, which the instants below count from
  epoch: Instant,
  // frames rendered so far
  frames: AtomicU64,
  // from rendering a frame to hearing it
  latency_ns: AtomicU64,
  // how many frames the device asks for at a time
  buffer_frames: AtomicU64,
  // the last run started (plus one, zero for none), its first frame, and
  // when that frame is heard (ns since `epoch`). the run is stored last, so
  // whoever sees it sees the rest
  anchor_run: AtomicU64,
  anchor_frame: AtomicU64,
  anchor_heard_ns: AtomicU64,
  // voices dropped because too many were sounding, and voices that arrived
  // after they should have started
  dropped: AtomicU64,
  late: AtomicU64,
}

// where a run started, in the engine's frames and in time
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
  pub frame: u64,
  // when its first frame comes out of the speakers
  pub heard: Instant,
}

// the side of the engine everything but the callback uses
pub struct Engine {
  commands: Mutex<Producer<Command>>,
  garbage: Mutex<Consumer<Garbage>>,
  shared: Arc<Shared>,
  sample_rate: u32,
}

impl Engine {
  // an engine and the renderer its callback runs, without a device
  // (the output stream drives the renderer, or a test does)
  pub fn new(sample_rate: u32) -> (Engine, Renderer) {
    let (commands_tx, commands_rx) = RingBuffer::new(QUEUE_LEN);
    let (garbage_tx, garbage_rx) = RingBuffer::new(QUEUE_LEN);
    let shared = Arc::new(Shared {
      epoch: Instant::now(),
      frames: AtomicU64::new(0),
      latency_ns: AtomicU64::new(0),
      buffer_frames: AtomicU64::new(0),
      anchor_run: AtomicU64::new(0),
      anchor_frame: AtomicU64::new(0),
      anchor_heard_ns: AtomicU64::new(0),
      dropped: AtomicU64::new(0),
      late: AtomicU64::new(0),
    });

    let engine = Engine {
      commands: Mutex::new(commands_tx),
      garbage: Mutex::new(garbage_rx),
      shared: shared.clone(),
      sample_rate,
    };
    let renderer = Renderer::new(sample_rate, commands_rx, garbage_tx, shared);

    (engine, renderer)
  }

  pub fn sample_rate(&self) -> u32 {
    self.sample_rate
  }

  // frames the callback has rendered
  pub fn frames(&self) -> u64 {
    self.shared.frames.load(Ordering::Acquire)
  }

  // from rendering a frame to hearing it, as last reported by the device
  pub fn latency(&self) -> Duration {
    Duration::from_nanos(self.shared.latency_ns.load(Ordering::Relaxed))
  }

  // how many frames the device asks for at a time (the most seen)
  pub fn buffer_frames(&self) -> u64 {
    self.shared.buffer_frames.load(Ordering::Relaxed)
  }

  // a duration in frames at the engine's rate
  pub fn frames_in(&self, dur: Duration) -> u64 {
    (dur.as_secs_f64() * self.sample_rate as f64).round() as u64
  }

  // a sound for the engine to play, `at` frames into a run (or right away,
  // without one)
  pub fn voice<S>(&self, source: S, run: Option<u64>, at: u64, route: Route) -> Box<Voice>
  where
    S: Source<Item = f32> + Send + 'static,
  {
    self.boxed_voice(Box::new(source), run, at, route)
  }

  pub fn boxed_voice(&self, source: VoiceSource, run: Option<u64>, at: u64, route: Route) -> Box<Voice> {
    Box::new(Voice {
      run,
      at,
      route,
      samples: UniformSourceIterator::new(source, 2, self.sample_rate),
      start: 0,
      fade_left: None,
    })
  }

  // start a run with its first voices, played through the master strip with
  // these controls. any run that's playing stops
  pub fn start_run(
    &self,
    run: u64,
    master: Arc<daw::ChannelControls>,
    voices: Vec<Box<Voice>>,
  ) -> Result<(), String> {
    self.send(Command::Start { run, master, voices })
  }

  // stop a run, if it's the one playing
  pub fn stop_run(&self, run: u64) -> Result<(), String> {
    self.send(Command::Stop { run })
  }

  pub fn play(&self, voices: Vec<Box<Voice>>) -> Result<(), String> {
    if voices.is_empty() { return Ok(()); }
    self.send(Command::Play(voices))
  }

  // play a sound once, straight to the output
  pub fn preview<S>(&self, source: S) -> Result<(), String>
  where
    S: Source<Item = f32> + Send + 'static,
  {
    self.play(vec![self.voice(source, None, 0, Route::Direct)])
  }

  fn send(&self, command: Command) -> Result<(), String> {
    self.collect_garbage();
    self.commands.lock().unwrap()
      .push(command)
      .map_err(|_| "the audio engine is busy (its queue is full)".to_string())
  }

  // free what the callback is done with
  fn collect_garbage(&self) {
    let mut garbage = self.garbage.lock().unwrap();
    while let Ok(item) = garbage.pop() {
      drop(item);
    }

    let dropped = self.shared.dropped.swap(0, Ordering::Relaxed);
    if dropped > 0 { println!("audio engine: dropped {} voices (too many at once)", dropped); }
    let late = self.shared.late.swap(0, Ordering::Relaxed);
    if late > 0 { println!("audio engine: {} voices arrived late", late); }
  }

  // where a run started, once the callback has started it
  pub fn anchor(&self, run: u64) -> Option<Anchor> {
    if self.shared.anchor_run.load(Ordering::Acquire) != run + 1 { return None; }

    let at = |ns: &AtomicU64| self.shared.epoch + Duration::from_nanos(ns.load(Ordering::Relaxed));
    Some(Anchor {
      frame: self.shared.anchor_frame.load(Ordering::Relaxed),
      heard: at(&self.shared.anchor_heard_ns),
    })
  }

  // wait for the callback to start a run
  pub fn wait_for_anchor(&self, run: u64, timeout: Duration) -> Result<Anchor, String> {
    let deadline = Instant::now() + timeout;
    loop {
      if let Some(anchor) = self.anchor(run) { return Ok(anchor); }
      if Instant::now() >= deadline {
        return Err("the audio output isn't running".to_string());
      }
      thread::sleep(Duration::from_millis(1));
    }
  }

  // sleep until the callback has rendered up to `frame`, or `keep_waiting`
  // says to give up. returns whether it got there
  pub fn wait_for_frame(&self, frame: u64, keep_waiting: impl Fn() -> bool) -> bool {
    // checked now and then, so a stop isn't kept waiting
    const MAX_NAP: Duration = Duration::from_millis(20);

    loop {
      let now = self.frames();
      if now >= frame { return true; }
      if !keep_waiting() { return false; }

      let left = Duration::from_secs_f64((frame - now) as f64 / self.sample_rate as f64);
      thread::sleep(left.min(MAX_NAP).max(Duration::from_micros(500)));
    }
  }
}

// the master bus, as a source the master strip can play: one frame of the
// mix at a time, set by the renderer before it pulls the strip
struct MasterMix {
  frame: [f32; 2],
  channel: usize,
  sample_rate: u32,
}

impl Iterator for MasterMix {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    let sample = self.frame[self.channel];
    self.channel = (self.channel + 1) % 2;
    Some(sample)
  }
}

impl Source for MasterMix {
  fn current_frame_len(&self) -> Option<usize> { None }
  fn channels(&self) -> u16 { 2 }
  fn sample_rate(&self) -> u32 { self.sample_rate }
  fn total_duration(&self) -> Option<Duration> { None }
}

// the callback's side of the engine
pub struct Renderer {
  commands: Consumer<Command>,
  garbage: Producer<Garbage>,
  shared: Arc<Shared>,
  voices: Vec<Box<Voice>>,
  // the run playing, and the engine's frame it started on
  run: Option<(u64, u64)>,
  // frames rendered
  frame: u64,
  // the master strip, set by each run (the limiter comes after it)
  master: Option<daw::ChannelStrip<MasterMix>>,
  sample_rate: u32,
  fade_frames: u32,
}

impl Renderer {
  fn new(
    sample_rate: u32,
    commands: Consumer<Command>,
    garbage: Producer<Garbage>,
    shared: Arc<Shared>,
  ) -> Self {
    Renderer {
      commands,
      garbage,
      shared,
      voices: Vec::with_capacity(MAX_VOICES),
      run: None,
      frame: 0,
      master: None,
      sample_rate,
      fade_frames: ((STOP_FADE.as_secs_f64() * sample_rate as f64) as u32).max(1),
    }
  }

  // the device asked for a buffer this many frames long
  pub fn saw_buffer(&self, frames: usize) {
    self.shared.buffer_frames.fetch_max(frames as u64, Ordering::Relaxed);
  }

  // fill `out` (interleaved stereo) with what plays next
  // - now: when the callback was called
  // - latency: from now until the first frame of `out` is heard
  pub fn render(&mut self, out: &mut [f32], now: Instant, latency: Duration) {
    self.shared.latency_ns.store(latency.as_nanos() as u64, Ordering::Relaxed);

    // what's arrived since the last buffer
    while let Ok(command) = self.commands.pop() {
      self.apply(command, now, latency);
    }

    for frame in out.chunks_exact_mut(2) {
      let mut master = [0f32; 2];
      let mut direct = [0f32; 2];

      for voice in self.voices.iter_mut() {
        if voice.start > self.frame { continue; }

        let (l, r) = match next_frame(&mut voice.samples) {
          Some(lr) => lr,
          None => {
            // done: it's taken out after this buffer
            voice.fade_left = Some(0);
            continue;
          }
        };
        let gain = match voice.fade_left {
          Some(0) => continue,
          Some(left) => {
            voice.fade_left = Some(left - 1);
            left as f32 / self.fade_frames as f32
          }
          None => 1.,
        };

        let bus = if voice.route == Route::Master { &mut master } else { &mut direct };
        bus[0] += l * gain;
        bus[1] += r * gain;
      }

      let [l, r] = match self.master.as_mut() {
        Some(strip) => {
          strip.inner_mut().frame = master;
          [strip.next().unwrap_or(0.), strip.next().unwrap_or(0.)]
        }
        None => master,
      };
      frame[0] = daw::soft_limit(l) + direct[0];
      frame[1] = daw::soft_limit(r) + direct[1];

      self.frame += 1;
    }

    // take out finished voices, and those that have faded out
    let mut i = 0;
    while i < self.voices.len() {
      if self.voices[i].fade_left == Some(0) {
        let voice = self.voices.swap_remove(i);
        self.throw_away(Garbage::Voice(voice));
      } else {
        i += 1;
      }
    }

    self.shared.frames.store(self.frame, Ordering::Release);
  }

  fn apply(&mut self, command: Command, now: Instant, latency: Duration) {
    match command {
      Command::Start { run, master, voices } => {
        if let Some((playing, _)) = self.run { self.stop(playing); }
        self.run = Some((run, self.frame));

        let mix = MasterMix { frame: [0.; 2], channel: 0, sample_rate: self.sample_rate };
        self.master = Some(daw::ChannelStrip::new(mix, master, None));

        let since_epoch = |t: Instant| t.duration_since(self.shared.epoch).as_nanos() as u64;
        self.shared.anchor_frame.store(self.frame, Ordering::Relaxed);
        self.shared.anchor_heard_ns.store(since_epoch(now + latency), Ordering::Relaxed);
        self.shared.anchor_run.store(run + 1, Ordering::Release);

        self.add(voices);
      }
      Command::Stop { run } => self.stop(run),
      Command::Play(voices) => self.add(voices),
    }
  }

  // stop a run if it's playing: what's sounding fades out, what hasn't
  // started yet won't
  fn stop(&mut self, run: u64) {
    match self.run {
      Some((playing, _)) if playing == run => self.run = None,
      _ => return,
    }

    for voice in self.voices.iter_mut() {
      if voice.run != Some(run) { continue; }

      if voice.start > self.frame {
        voice.fade_left = Some(0);
      } else if voice.fade_left.is_none() {
        voice.fade_left = Some(self.fade_frames);
      }
    }
  }

  fn add(&mut self, mut voices: Vec<Box<Voice>>) {
    for mut voice in voices.drain(..) {
      voice.start = match (voice.run, self.run) {
        // a preview starts now
        (None, _) => self.frame,
        (Some(run), Some((playing, origin))) if run == playing => origin + voice.at,
        // its run has stopped, or been replaced
        _ => {
          self.throw_away(Garbage::Voice(voice));
          continue;
        }
      };

      // it should have started already: play it from where it would be now
      if voice.start < self.frame {
        self.shared.late.fetch_add(1, Ordering::Relaxed);
        for _ in voice.start..self.frame {
          if next_frame(&mut voice.samples).is_none() { break; }
        }
        voice.start = self.frame;
      }

      if self.voices.len() < MAX_VOICES {
        self.voices.push(voice);
      } else {
        self.shared.dropped.fetch_add(1, Ordering::Relaxed);
        self.throw_away(Garbage::Voice(voice));
      }
    }

    // the list itself is freed elsewhere too
    self.throw_away(Garbage::Voices(voices));
  }

  fn throw_away(&mut self, garbage: Garbage) {
    // with the queue full there's no choice but to free it here
    let _ = self.garbage.push(garbage);
  }
}

fn next_frame(samples: &mut impl Iterator<Item = f32>) -> Option<(f32, f32)> {
  let l = samples.next()?;
  let r = samples.next()?;
  Some((l, r))
}

// the app's engine, opened the first time it's needed. if the output can't
// be opened, the next call tries again
lazy_static! {
  static ref ENGINE: Mutex<Option<Arc<Engine>>> = Mutex::new(None);
}

pub fn engine() -> Result<Arc<Engine>, String> {
  let mut engine = ENGINE.lock().unwrap();
  if let Some(engine) = engine.as_ref() { return Ok(engine.clone()); }

  let opened = Arc::new(open_output()?);
  *engine = Some(opened.clone());
  Ok(opened)
}

// the engine, if it's been opened (so stopping doesn't open it)
pub fn engine_if_open() -> Option<Arc<Engine>> {
  ENGINE.lock().unwrap().clone()
}

// open the default output device and start rendering to it. the stream
// lives on its own thread for as long as the app runs (it can't be moved
// between threads)
fn open_output() -> Result<Engine, String> {
  let (opened_tx, opened_rx) = mpsc::channel();

  thread::Builder::new()
    .name("audio output".into())
    .spawn(move || {
      let stream = match build_output() {
        Ok((engine, stream)) => {
          let _ = opened_tx.send(Ok(engine));
          stream
        }
        Err(e) => {
          let _ = opened_tx.send(Err(e));
          return;
        }
      };

      let _keep = stream;
      loop { thread::park(); }
    })
    .map_err(|e| format!("could not start the audio output thread: {}", e))?;

  opened_rx.recv().map_err(|_| "the audio output thread exited".to_string())?
}

fn build_output() -> Result<(Engine, cpal::Stream), String> {
  let device = cpal::default_host()
    .default_output_device()
    .ok_or("no audio output device")?;
  let supported = device.default_output_config()
    .map_err(|e| format!("could not read the output device's settings: {}", e))?;
  let format = supported.sample_format();
  let config: cpal::StreamConfig = supported.into();

  println!(
    "audio output: {} ({} Hz, {} channels, {:?})",
    device.name().unwrap_or_default(), config.sample_rate.0, config.channels, format,
  );

  let (engine, renderer) = Engine::new(config.sample_rate.0);
  let stream = match format {
    cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, renderer),
    cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, renderer),
    cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, renderer),
    cpal::SampleFormat::I32 => build_stream::<i32>(&device, &config, renderer),
    other => return Err(format!("unsupported output sample format: {:?}", other)),
  }?;
  stream.play().map_err(|e| format!("could not start the audio output: {}", e))?;

  Ok((engine, stream))
}

fn build_stream<T>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  mut renderer: Renderer,
) -> Result<cpal::Stream, String>
where
  T: SizedSample + FromSample<f32>,
{
  let channels = config.channels.max(1) as usize;
  let sample_rate = config.sample_rate.0 as f64;
  // rendered here first, allocated once
  let mut mix = vec![0f32; MAX_CHUNK * 2];

  device.build_output_stream(
    config,
    move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
      let now = Instant::now();
      let stamp = info.timestamp();
      let latency = stamp.playback.duration_since(&stamp.callback).unwrap_or_default();
      renderer.saw_buffer(data.len() / channels);

      for (i, out) in data.chunks_mut(MAX_CHUNK * channels).enumerate() {
        let frames = out.len() / channels;
        let mix = &mut mix[..frames * 2];
        let offset = Duration::from_secs_f64((i * MAX_CHUNK) as f64 / sample_rate);
        renderer.render(mix, now, latency + offset);

        for (frame, lr) in out.chunks_mut(channels).zip(mix.chunks_exact(2)) {
          match frame.len() {
            1 => frame[0] = T::from_sample((lr[0] + lr[1]) * 0.5),
            _ => {
              frame[0] = T::from_sample(lr[0]);
              frame[1] = T::from_sample(lr[1]);
              for extra in frame[2..].iter_mut() { *extra = T::from_sample(0f32); }
            }
          }
        }
      }
    },
    |e| eprintln!("audio output error: {}", e),
    None,
  ).map_err(|e| format!("could not open the audio output: {}", e))
}

#[cfg(test)]
mod tests {
  use super::*;
  use rodio::buffer::SamplesBuffer;

  const RATE: u32 = 1_000;

  fn master() -> Arc<daw::ChannelControls> {
    daw::Mixer::default().master()
  }

  // a stereo click: `frames` frames of 1.0 at the test rate
  fn click(frames: usize) -> SamplesBuffer<f32> {
    SamplesBuffer::new(2, RATE, vec![1.; frames * 2])
  }

  // render `frames` frames, returning the left channel
  fn render(renderer: &mut Renderer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.; frames * 2];
    renderer.render(&mut out, Instant::now(), Duration::ZERO);
    out.chunks(2).map(|f| f[0]).collect()
  }

  fn first_sound(samples: &[f32]) -> Option<usize> {
    samples.iter().position(|&s| s != 0.)
  }

  #[test]
  fn test_voices_start_on_their_frame() {
    let (engine, mut renderer) = Engine::new(RATE);
    // the run starts after some frames have played
    render(&mut renderer, 37);

    engine.start_run(1, master(), vec![engine.voice(click(3), Some(1), 10, Route::Direct)]).unwrap();
    // a voice sent later lands on its frame too, even mid-buffer
    engine.play(vec![engine.voice(click(3), Some(1), 45, Route::Direct)]).unwrap();
    let out = render(&mut renderer, 64);

    assert_eq!(engine.anchor(1).unwrap().frame, 37);
    assert_eq!(&out[8..14], &[0., 0., 1., 1., 1., 0.]);
    assert_eq!(first_sound(&out[14..]), Some(45 - 14));
    assert_eq!(engine.frames(), 37 + 64);
  }

  #[test]
  fn test_voices_can_start_in_a_later_buffer() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![engine.voice(click(1), Some(1), 100, Route::Direct)]).unwrap();

    let mut out = render(&mut renderer, 64);
    out.extend(render(&mut renderer, 64));

    assert_eq!(first_sound(&out), Some(100));
  }

  #[test]
  fn test_a_late_voice_plays_from_where_it_would_be() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![]).unwrap();
    render(&mut renderer, 10);

    // should have started at frame 5: its first 5 frames are skipped
    let ramp = SamplesBuffer::new(2, RATE, (0..40).map(|i| (i / 2 + 1) as f32).collect::<Vec<_>>());
    engine.play(vec![engine.voice(ramp, Some(1), 5, Route::Direct)]).unwrap();
    let out = render(&mut renderer, 4);

    assert_eq!(out, vec![6., 7., 8., 9.]);
  }

  #[test]
  fn test_stopping_fades_out_and_drops_what_has_not_started() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![
      engine.voice(click(500), Some(1), 0, Route::Direct),
      engine.voice(click(500), Some(1), 200, Route::Direct),
    ]).unwrap();
    render(&mut renderer, 50);

    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 300);

    // a 10 frame fade at 1kHz, then nothing, and the later voice never plays
    assert!(out[0] > 0.8 && out[0] <= 1.);
    assert!(out[5] < out[0]);
    assert!(out[10..].iter().all(|&s| s == 0.));
    assert!(renderer.voices.is_empty());
  }

  #[test]
  fn test_a_new_run_replaces_the_old_one_and_stale_voices_are_dropped() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![]).unwrap();
    render(&mut renderer, 10);

    engine.start_run(2, master(), vec![engine.voice(click(1), Some(2), 5, Route::Direct)]).unwrap();
    // the old run's scheduler hasn't noticed yet
    engine.play(vec![engine.voice(click(1), Some(1), 12, Route::Direct)]).unwrap();
    // stopping the old run again does nothing to the new one
    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 20);

    assert_eq!(engine.anchor(2).unwrap().frame, 10);
    assert!(engine.anchor(1).is_none());
    assert_eq!(out.iter().filter(|&&s| s != 0.).count(), 1);
    assert_eq!(first_sound(&out), Some(5));
  }

  #[test]
  fn test_previews_play_now_and_ignore_runs() {
    let (engine, mut renderer) = Engine::new(RATE);
    render(&mut renderer, 10);
    engine.preview(click(3)).unwrap();
    engine.stop_run(0).unwrap();

    let out = render(&mut renderer, 8);
    assert_eq!(&out[..4], &[1., 1., 1., 0.]);
  }

  #[test]
  fn test_the_anchor_is_when_the_run_is_heard() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(3, master(), vec![]).unwrap();
    let now = Instant::now();
    let mut out = vec![0.; 16];
    renderer.render(&mut out, now, Duration::from_millis(12));

    let heard = engine.anchor(3).unwrap().heard.duration_since(now);
    assert!(heard >= Duration::from_micros(11_999) && heard <= Duration::from_micros(12_001));
    assert_eq!(engine.latency(), Duration::from_millis(12));
  }

  #[test]
  fn test_voices_are_resampled_to_the_engine_rate() {
    let (engine, mut renderer) = Engine::new(RATE);
    // 10 frames at twice the engine's rate last 5 of its frames
    let fast = SamplesBuffer::new(2, RATE * 2, vec![1f32; 20]);
    engine.preview(fast).unwrap();

    let out = render(&mut renderer, 10);
    let sounding = out.iter().filter(|&&s| s != 0.).count();
    assert!((4..=6).contains(&sounding), "{} frames sounded", sounding);
  }

  // plays through the real output: run with `cargo test -- --ignored`
  #[test]
  #[ignore = "needs an audio output device"]
  fn test_the_device_output_runs() {
    let engine = engine().expect("could not open the output");
    let start = engine.frames();
    engine.start_run(99, master(), vec![
      engine.voice(daw::METRONOME_TICK_SOURCE.convert_samples(), Some(99), 0, Route::Direct),
    ]).unwrap();

    let anchor = engine.wait_for_anchor(99, Duration::from_secs(2)).unwrap();
    assert!(anchor.frame >= start);
    assert!(engine.wait_for_frame(anchor.frame + engine.frames_in(Duration::from_millis(300)), || true));
    engine.stop_run(99).unwrap();

    println!(
      "rate {} Hz, buffer {} frames, output latency {:?}, anchored at frame {}",
      engine.sample_rate(), engine.buffer_frames(), engine.latency(), anchor.frame,
    );
    assert!(engine.buffer_frames() > 0);
  }
}
