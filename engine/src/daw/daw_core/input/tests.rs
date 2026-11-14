use super::*;
use std::path::Path;

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

// one channel of interleaved frames, `channels` wide
fn channel(frames: &[f32], channels: usize, channel: usize) -> Vec<f32> {
  frames.chunks_exact(channels).map(|frame| frame[channel]).collect()
}

fn file(path: &Path, channels: &[usize]) -> TakeFile {
  TakeFile { path: path.to_path_buf(), channels: channels.to_vec() }
}

const MS: Duration = Duration::from_millis(1);

#[test]
fn test_a_take_holds_what_was_captured_while_recording() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 4);
  let path = temp_take("captured");
  let t0 = Instant::now();

  // before recording: metered, not kept
  capturer.process(&frames(0, 10, 4), 4, t0);
  let recording = input.start_take(&[file(&path, &[0, 1])]).unwrap();
  recording.keep_from(t0, None);
  capturer.process(&frames(10, 10, 4), 4, t0 + 10 * MS);
  capturer.process(&frames(20, 5, 4), 4, t0 + 20 * MS);
  let take = input.stop_take(recording, None).unwrap();
  // after: not kept
  capturer.process(&frames(25, 10, 4), 4, t0 + 25 * MS);

  // kept from the first frame captured
  assert_eq!(take.started, Some(t0 + 10 * MS));
  assert_eq!(take.dropped, 0);
  assert_eq!(take.files.len(), 1);
  let file = &take.files[0];
  assert_eq!((file.frames, file.channels, file.sample_rate), (15, 2, RATE));

  let (spec, samples) = read(&path);
  assert_eq!(spec.sample_format, hound::SampleFormat::Float);
  assert_eq!(spec.bits_per_sample, 32);
  assert_eq!(samples, frames(10, 15, 2));

  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_frames_before_the_take_starts_are_dropped() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 1);
  let path = temp_take("keep-from");
  let t0 = Instant::now();

  let recording = input.start_take(&[file(&path, &[0])]).unwrap();
  // captured before the transport was heard
  capturer.process(&frames(0, 10, 1), 1, t0);
  capturer.process(&frames(10, 10, 1), 1, t0 + 10 * MS);
  // the run is heard from 13.5ms: the first frame at or after that is 14
  recording.keep_from(t0 + Duration::from_micros(13_500), None);
  capturer.process(&frames(20, 5, 1), 1, t0 + 20 * MS);
  let take = input.stop_take(recording, None).unwrap();

  assert_eq!(read(&path).1, frames(14, 11, 1));
  assert_eq!(take.started, Some(t0 + 14 * MS));
  assert_eq!(take.files[0].frames, 11);

  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_a_take_stops_at_its_limit() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 1);
  let path = temp_take("limit");
  let t0 = Instant::now();

  let recording = input.start_take(&[file(&path, &[0])]).unwrap();
  recording.keep_from(t0 + 2 * MS, None);
  capturer.process(&frames(0, 30, 1), 1, t0);
  // the loop ended 20 frames (20ms at the test's rate) in
  let take = input.stop_take(recording, Some(20 * MS)).unwrap();

  assert_eq!(read(&path).1, frames(2, 20, 1));
  assert_eq!(take.files[0].frames, 20);

  let _ = std::fs::remove_file(&path);
}

// a loop recording cut into passes at `first`, then every `every`, its
// files named `name`-1, -2, and so on after the first
fn passes(first: Duration, every: Duration, name: &'static str) -> (Passes, mpsc::Receiver<Takes>) {
  let (finished, done) = mpsc::channel();
  let mut n = 0;
  let plan = Passes {
    first,
    every,
    next_paths: Box::new(move || { n += 1; vec![temp_take(&format!("{}-{}", name, n))] }),
    finished,
  };
  (plan, done)
}

#[test]
fn test_a_loop_recording_is_cut_into_passes() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 2);
  let path = temp_take("passes");
  let t0 = Instant::now();

  let recording = input.start_take(&[file(&path, &[1])]).unwrap();
  // started 10 frames from the loop's end, which is 25 frames long
  let (plan, done) = passes(10 * MS, 25 * MS, "passes");
  let live = recording.keep_from(t0, Some(plan));
  capturer.process(&frames(0, 50, 2), 2, t0);
  capturer.process(&frames(50, 20, 2), 2, t0 + 50 * MS);
  let last = input.stop_take(recording, None).unwrap();

  // each pass in its own file, starting where the loop comes round
  let finished: Vec<Takes> = done.try_iter().collect();
  assert_eq!(finished.iter().map(|takes| (takes.pass, takes.files[0].frames)).collect::<Vec<_>>(), vec![(0, 10), (1, 25), (2, 25)]);
  let second = channel(&frames(10, 25, 2), 2, 1);
  assert_eq!(read(&finished[1].files[0].path).1, second);
  assert_eq!(read(&finished[1].files[0].path).0.channels, 1);
  // the last, as far as it got, is what stopping returns
  assert_eq!((last.pass, last.files[0].frames), (3, 10));
  assert_eq!(read(&last.files[0].path).1, channel(&frames(60, 10, 2), 2, 1));

  // the newest passes can be played as they were recorded
  assert!(live.pass(0).is_none());
  let pass = live.pass(2).unwrap();
  assert_eq!(pass.files[0].frames(), 25);
  assert_eq!(pass.files[0].written_frames(), 25);
  assert_eq!(live.pass(3).unwrap().files[0].written_frames(), 10);

  for takes in finished.iter().chain([&last]) { let _ = std::fs::remove_file(&takes.files[0].path); }
}

#[test]
fn test_a_late_start_still_cuts_where_the_loop_comes_round() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 1);
  let path = temp_take("late-passes");
  let t0 = Instant::now();

  let recording = input.start_take(&[file(&path, &[0])]).unwrap();
  let (plan, done) = passes(20 * MS, 30 * MS, "late-passes");
  // the input's first frame came 5 frames after the take should have
  // started: the loop still comes round 20 frames after that should have
  let _ = recording.keep_from(t0, Some(plan));
  capturer.process(&frames(0, 40, 1), 1, t0 + 5 * MS);
  let last = input.stop_take(recording, None).unwrap();

  let first = done.try_recv().unwrap();
  assert_eq!(first.started, Some(t0 + 5 * MS));
  assert_eq!(first.files[0].frames, 15);
  assert_eq!(last.files[0].frames, 25);
  assert_eq!(read(&last.files[0].path).1, frames(15, 25, 1));

  let _ = std::fs::remove_file(&first.files[0].path);
  let _ = std::fs::remove_file(&last.files[0].path);
}

#[test]
fn test_a_take_at_another_rate_is_limited_in_time() {
  // an input at twice the test's usual rate (like a 24kHz headset
  // microphone, while the output runs at 48kHz)
  let (input, mut capturer) = Input::new("headset".into(), RATE * 2, 1);
  let path = temp_take("other-rate");
  let t0 = Instant::now();

  let recording = input.start_take(&[file(&path, &[0])]).unwrap();
  recording.keep_from(t0, None);
  capturer.process(&frames(0, 100, 1), 1, t0);
  // the loop ended 20ms in: 40 of this input's frames
  let take = input.stop_take(recording, Some(20 * MS)).unwrap();

  assert_eq!(take.files[0].frames, 40);
  assert_eq!(take.files[0].sample_rate, RATE * 2);
  let (spec, samples) = read(&path);
  assert_eq!(spec.sample_rate, RATE * 2);
  assert_eq!(samples, frames(0, 40, 1));

  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_each_armed_track_gets_its_own_channels() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 4);
  let stereo = temp_take("pair");
  let mono = temp_take("mono");

  let recording = input.start_take(&[file(&stereo, &[2, 3]), file(&mono, &[0])]).unwrap();
  recording.keep_from(Instant::now() - Duration::from_secs(1), None);
  capturer.process(&frames(0, 3, 4), 4, Instant::now());
  let take = input.stop_take(recording, None).unwrap();

  assert_eq!(read(&stereo).1, vec![0.2, 0.3, 1.2, 1.3, 2.2, 2.3]);
  assert_eq!(read(&mono).1, vec![0., 1., 2.]);
  assert_eq!(take.files.iter().map(|f| f.channels).collect::<Vec<_>>(), vec![2, 1]);

  // channels the device doesn't have
  let refused = input.start_take(&[file(&mono, &[4])]).err().unwrap();
  assert!(refused.contains("4 input channels"), "{}", refused);

  let _ = std::fs::remove_file(&stereo);
  let _ = std::fs::remove_file(&mono);
}

#[test]
fn test_stopped_before_its_start_was_known_keeps_everything() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 1);
  let path = temp_take("unknown-start");
  let t0 = Instant::now();

  let recording = input.start_take(&[file(&path, &[0])]).unwrap();
  capturer.process(&frames(0, 5, 1), 1, t0);
  let take = input.stop_take(recording, None).unwrap();

  assert_eq!(take.files[0].frames, 5);
  assert_eq!(take.started, Some(t0));

  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_takes_follow_one_another() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 1);
  let first = temp_take("first");
  let second = temp_take("second");

  let recording = input.start_take(&[file(&first, &[0])]).unwrap();
  capturer.process(&frames(0, 8, 1), 1, Instant::now());
  let one = input.stop_take(recording, None).unwrap();

  // a second take starts from nothing, though the queue was reused
  let recording = input.start_take(&[file(&second, &[0])]).unwrap();
  // and can't be started twice
  assert!(input.start_take(&[file(&temp_take("third"), &[0])]).is_err());
  capturer.process(&frames(100, 3, 1), 1, Instant::now());
  let two = input.stop_take(recording, None).unwrap();

  assert_eq!((one.files[0].frames, two.files[0].frames), (8, 3));
  assert_eq!(read(&second).1, frames(100, 3, 1));

  let _ = std::fs::remove_file(&first);
  let _ = std::fs::remove_file(&second);
}

#[test]
fn test_what_does_not_fit_is_counted_not_blocked_on() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 1);
  let path = temp_take("overflow");
  let recording = input.start_take(&[file(&path, &[0])]).unwrap();

  // more than the queue holds, faster than the writer drains it
  let capacity = RATE as usize * QUEUE_SECONDS;
  capturer.process(&frames(0, capacity + 500, 1), 1, Instant::now());
  let take = input.stop_take(recording, None).unwrap();

  assert_eq!(take.dropped, (capacity + 500) as u64);
  assert_eq!(take.files[0].frames, 0);
  assert_eq!(take.started, None);

  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_a_missing_input_falls_back_to_the_default() {
  // what each input gives: the chosen one is unplugged
  let open = |name: Option<&str>| match name {
    Some("Interface") => Err("there's no audio input named 'Interface'".to_string()),
    Some(other) => Ok(other.to_string()),
    None => Ok("default".to_string()),
  };

  let (used, notice) = with_fallback(Some("Interface"), open).unwrap();
  assert_eq!(used, "default");
  let notice = notice.unwrap();
  assert!(notice.contains("'Interface'") && notice.contains("default input"), "{}", notice);

  // one that's there is used, with nothing to say
  assert_eq!(with_fallback(Some("Mic"), open).unwrap(), ("Mic".to_string(), None));
  assert_eq!(with_fallback(None, open).unwrap(), ("default".to_string(), None));

  // without a default either, the default's problem is the one given
  let nothing = |_: Option<&str>| Err::<String, _>("there's no audio input device".to_string());
  assert_eq!(with_fallback(Some("Interface"), nothing).unwrap_err(), "there's no audio input device");
  assert_eq!(with_fallback(None, nothing).unwrap_err(), "there's no audio input device");
}

#[test]
fn test_the_meter_reads_every_channel() {
  let (input, mut capturer) = Input::new("test".into(), RATE, 3);
  capturer.process(&[0.1, -0.5, 0.2, -0.3, 0.4, 0.], 3, Instant::now());

  assert_eq!(input.take_peaks(), vec![0.3, 0.5, 0.2]);
  // read once
  assert_eq!(input.take_peaks(), vec![0., 0., 0.]);
}

#[test]
fn test_a_take_that_cannot_be_created_leaves_the_input_usable() {
  let (input, _capturer) = Input::new("test".into(), RATE, 1);
  let blocker = temp_take("not-a-folder");
  std::fs::write(&blocker, b"in the way").unwrap();

  assert!(input.start_take(&[file(&blocker.join("take.wav"), &[0])]).is_err());
  let path = temp_take("after");
  let recording = input.start_take(&[file(&path, &[0])]).unwrap();
  input.stop_take(recording, None).unwrap();

  let _ = std::fs::remove_file(&blocker);
  let _ = std::fs::remove_file(&path);
}

// records from the real default input: run with `cargo test -- --ignored`
// (macOS asks for the microphone first)
#[test]
#[ignore = "needs an audio input device"]
fn test_the_device_input_records() {
  let rate = daw::engine().expect("could not open the output").sample_rate();
  let input = open_input(None, rate).expect("could not open the input");
  let path = temp_take("device");

  let recording = input.start_take(&[file(&path, &[0])]).unwrap();
  recording.keep_from(Instant::now(), None);
  thread::sleep(Duration::from_millis(300));
  let take = input.stop_take(recording, None).unwrap();
  let frames = take.files[0].frames;

  println!(
    "{}: {} channels at {} Hz, recorded {} frames, input latency {:?}, dropped {}",
    input.device_name(), input.device_channels(), rate, frames, input.latency(), take.dropped,
  );
  assert!(take.started.is_some());
  assert_eq!(take.dropped, 0);
  let expected = rate as f64 * 0.3;
  assert!((frames as f64 - expected).abs() < expected * 0.25, "{} frames", frames);
  assert_eq!(hound::WavReader::open(&path).unwrap().duration() as u64, frames);

  let _ = std::fs::remove_file(&path);
}
