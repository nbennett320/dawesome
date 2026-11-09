// recording onto the playlist: tracks are armed, each with the input
// channels it records. Record starts the transport from the playhead, and
// every armed track records a take of its channels. when playback stops (or
// a loop comes round) the takes become clips on their tracks, starting where
// the recording did
//
// a take lines up with what was heard while it was played: the engine knows
// when the run's first frame was heard, and the input when each frame was
// captured, so the take keeps frames from the moment the run was heard (plus
// the recording offset in the audio settings, for devices that don't report
// all their latency, like Bluetooth headphones)
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::daw;
use crate::event_dispatch::clip_info;
use crate::project::LoadedClip;
use crate::project_files;

// the input channels a track records: one, or a pair from `first`
// (counted from 0)
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InputChoice {
  pub first: u16,
  pub stereo: bool,
}

impl InputChoice {
  // the first two channels, or the only one
  pub fn default_for(device_channels: u16) -> Self {
    InputChoice { first: 0, stereo: device_channels >= 2 }
  }

  // this choice, kept to the channels an input has: a pair past them is
  // the first pair (or, on a mono input, its channel), and a channel past
  // them is the first
  pub fn fit(self, device_channels: u16) -> Self {
    let last = self.first as u32 + self.stereo as u32;
    if device_channels == 0 || last < device_channels as u32 { return self; }
    if self.stereo { InputChoice::default_for(device_channels) } else { InputChoice { first: 0, stereo: false } }
  }

  pub fn channels(&self) -> Vec<usize> {
    let first = self.first as usize;
    if self.stereo { vec![first, first + 1] } else { vec![first] }
  }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArmedTrack {
  pub track_number: u32,
  pub input: InputChoice,
}

// a recording in progress
struct Session {
  // each armed track, and the file its take goes in
  tracks: Vec<(u32, PathBuf)>,
  // where on the playlist it started
  start: Duration,
  // when the first frame kept was meant to be captured
  keep_from: Instant,
  // the loop's length, if it was looping (the take ends where it comes round)
  loop_len: Option<Duration>,
}

// what's armed, what's recording, and the clips recorded since the frontend
// last asked
#[derive(Default)]
pub struct Recorder {
  armed: BTreeMap<u32, InputChoice>,
  session: Option<Session>,
  // recordings that have stopped, whose takes aren't on their tracks yet
  finishing: u32,
  clips: Vec<LoadedClip>,
  problems: Vec<String>,
}

impl Recorder {
  pub fn armed(&self) -> Vec<ArmedTrack> {
    self.armed.iter().map(|(&track_number, &input)| ArmedTrack { track_number, input }).collect()
  }

  pub fn set_armed(&mut self, tracks: &[ArmedTrack]) {
    self.armed = tracks.iter().map(|t| (t.track_number, t.input)).collect();
  }

  pub fn arm(&mut self, track_number: u32, input: Option<InputChoice>) {
    match input {
      Some(input) => { self.armed.insert(track_number, input); }
      None => { self.armed.remove(&track_number); }
    }
  }

  pub fn recording(&self) -> bool {
    self.session.is_some()
  }

  // recording, or putting the takes of a recording that's stopped on their
  // tracks. the frontend keeps asking until this is over, so it gets them
  fn busy(&self) -> bool {
    self.session.is_some() || self.finishing > 0
  }

  // stop the recording, counting it as finishing until its takes are in
  fn begin_finish(&mut self) -> Option<Session> {
    let session = self.session.take()?;
    self.finishing += 1;
    Some(session)
  }

  // its takes are on their tracks
  fn end_finish(&mut self, clips: Vec<LoadedClip>, problems: Vec<String>) {
    self.clips.extend(clips);
    self.problems.extend(problems);
    self.finishing = self.finishing.saturating_sub(1);
  }
}

// recording, or still putting a stopped recording's takes on their tracks:
// its files shouldn't be renamed or deleted until that's over
pub fn is_busy(state: &daw::InnerState) -> bool {
  state.recorder.lock().unwrap().busy()
}

pub fn is_recording(state: &daw::InnerState) -> bool {
  state.recorder.lock().unwrap().recording()
}

// the clips recorded since this was last asked, and what went wrong
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Recorded {
  pub recording: bool,
  pub clips: Vec<LoadedClip>,
  pub problems: Vec<String>,
}

pub fn collect(state: &daw::InnerState) -> Recorded {
  let mut recorder = state.recorder.lock().unwrap();
  Recorded {
    recording: recorder.busy(),
    clips: std::mem::take(&mut recorder.clips),
    problems: std::mem::take(&mut recorder.problems),
  }
}

// start recording every armed track, playing from the playhead
pub fn start(state: &Arc<daw::InnerState>) -> Result<(), String> {
  // the last recording's takes may still be going on their tracks (it
  // stopped with playback, or at a loop's end): it has the input until then
  let waited = Instant::now();
  while state.recorder.lock().unwrap().finishing > 0 {
    if waited.elapsed() > Duration::from_secs(5) {
      return Err("the last recording is still being saved".into());
    }
    std::thread::sleep(Duration::from_millis(5));
  }

  let armed = {
    let recorder = state.recorder.lock().unwrap();
    if recorder.recording() { return Err("already recording".into()); }
    recorder.armed()
  };
  if armed.is_empty() {
    return Err("Arm a track first, with the R key on its mixer strip".into());
  }

  // already playing: start again from where it's got to, so the take and
  // the run start together
  if state.playlist.playing() { daw::pause_playlist(state); }

  // each track records what it was armed with, kept to the channels the
  // input has (it may have been armed on another input, or in a project
  // made with one)
  let channels = daw::input_channel_count()?;
  let files: Vec<daw::TakeFile> = {
    let mut location = state.project.lock().unwrap();
    let dir = location.recordings_dir()?;
    let paths = project_files::next_take_paths(&dir, &location.name(), &project_files::today(), armed.len());
    armed.iter().zip(paths)
      .map(|(track, path)| daw::TakeFile { path, channels: track.input.fit(channels).channels() })
      .collect()
  };

  // capturing starts first, so nothing's missed while playback gets going.
  // what comes before the run is heard is dropped (see keep_recording_from)
  daw::start_recording(&files)?;
  daw::start_playlist_after(state, count_in(state));

  let heard = *state.playlist.started_time.lock().unwrap();
  let heard = match heard {
    Some(heard) if state.playlist.playing_playlist() => heard,
    _ => {
      let _ = daw::stop_recording(None);
      for file in &files { let _ = std::fs::remove_file(&file.path); }
      return Err("playback couldn't start, so nothing was recorded".into());
    }
  };

  let offset_ms = daw::audio_settings().record_offset_ms;
  let offset = Duration::from_secs_f64(offset_ms.abs() / 1000.);
  let keep_from = if offset_ms >= 0. {
    heard + offset
  } else {
    heard.checked_sub(offset).unwrap_or(heard)
  };
  daw::keep_recording_from(keep_from);

  let interval = daw::beat_interval(state);
  let session = Session {
    tracks: armed.iter().zip(&files).map(|(track, file)| (track.track_number, file.path.clone())).collect(),
    start: *state.playlist.start_offset.lock().unwrap(),
    keep_from,
    loop_len: state.playlist.loop_len(interval),
  };
  println!("recording {} tracks from {:?}", session.tracks.len(), session.start);
  let mut recorder = state.recorder.lock().unwrap();
  recorder.session = Some(session);
  // recording from another input than the one chosen is worth knowing
  if let Some(notice) = daw::input_notice() { recorder.problems.push(notice); }
  drop(recorder);
  Ok(())
}

// how long recording counts in for: a bar, when counting in is on. what's
// captured meanwhile is dropped with the rest from before the playhead's heard
pub fn count_in(state: &daw::InnerState) -> Duration {
  if !state.playlist.count_in.load(std::sync::atomic::Ordering::SeqCst) {
    return Duration::ZERO;
  }
  let beats = state.playlist.time_signature.lock().unwrap().numerator.max(1);
  daw::beat_interval(state) * beats as u32
}

// stop recording, and put the takes on their tracks. at a loop's end, the
// takes end there
pub fn finish(state: &daw::InnerState, at_loop_end: bool) {
  let session = match state.recorder.lock().unwrap().begin_finish() {
    Some(session) => session,
    None => return,
  };

  let limit = match (at_loop_end, session.loop_len) {
    (true, Some(len)) => Some(len.saturating_sub(session.start)),
    _ => None,
  };

  let mut clips = Vec::new();
  let mut problems = Vec::new();

  match daw::stop_recording(limit) {
    Err(e) => problems.push(format!("the take couldn't be finished: {}", e)),
    Ok(None) => problems.push("the input stopped before the take was finished".into()),
    Ok(Some(takes)) => {
      if takes.dropped > 0 {
        problems.push("some audio was lost because the disk couldn't keep up".into());
      }
      // it started late (capture hadn't begun when the run was heard): the
      // clip starts as late
      // (by less than a frame is just the take starting on a whole frame)
      let rate = takes.files.first().map(|take| take.sample_rate).unwrap_or(48_000);
      let frame = Duration::from_secs_f64(1. / rate.max(1) as f64);
      let late = takes.started
        .map(|started| started.saturating_duration_since(session.keep_from))
        .filter(|&late| late > frame)
        .unwrap_or_default();
      let start = session.start + late;

      for (take, (track_number, _)) in takes.files.iter().zip(&session.tracks) {
        if take.frames == 0 {
          let _ = std::fs::remove_file(&take.path);
          continue;
        }

        // decoded before taking the graph's lock, which playback needs
        let path = take.path.to_string_lossy().into_owned();
        match daw::AudioNode::new(0, path.clone(), start, *track_number) {
          Ok(node) => {
            let mut audiograph = state.playlist.audiograph.lock().unwrap();
            let interval = audiograph.beat_interval();
            let id = audiograph.insert_node(node);
            let node = audiograph.get_node_by_id(id).expect("a node that was just added");
            clips.push(LoadedClip {
              id,
              path,
              track_number: *track_number,
              start_beat: if interval.is_zero() { 0. } else { (start.as_secs_f64() / interval.as_secs_f64()) as f32 },
              info: clip_info(node),
            });
          }
          Err(e) => problems.push(format!("the take couldn't be added: {}", e)),
        }
      }
    }
  }

  for problem in &problems { println!("recording: {}", problem); }
  state.recorder.lock().unwrap().end_finish(clips, problems);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_input_choices() {
    assert_eq!(InputChoice::default_for(1), InputChoice { first: 0, stereo: false });
    assert_eq!(InputChoice::default_for(8), InputChoice { first: 0, stereo: true });
    assert_eq!(InputChoice { first: 2, stereo: true }.channels(), vec![2, 3]);
    assert_eq!(InputChoice { first: 5, stereo: false }.channels(), vec![5]);
  }

  #[test]
  fn test_choices_fit_the_input() {
    let mono = |first| InputChoice { first, stereo: false };
    let pair = |first| InputChoice { first, stereo: true };

    // ones that fit stay as they are
    assert_eq!(pair(0).fit(2), pair(0));
    assert_eq!(pair(2).fit(4), pair(2));
    assert_eq!(mono(3).fit(4), mono(3));
    // a pair on a mono input records its channel
    assert_eq!(pair(0).fit(1), mono(0));
    // a pair past an input's channels is its first pair
    assert_eq!(pair(2).fit(2), pair(0));
    assert_eq!(pair(1).fit(2), pair(0));
    // a channel past them is the first
    assert_eq!(mono(1).fit(1), mono(0));
    assert_eq!(mono(5).fit(4), mono(0));
    // nothing known about the input: as chosen
    assert_eq!(pair(4).fit(0), pair(4));
  }

  #[test]
  fn test_arming() {
    let mut recorder = Recorder::default();
    recorder.arm(3, Some(InputChoice { first: 0, stereo: true }));
    recorder.arm(1, Some(InputChoice { first: 1, stereo: false }));
    recorder.arm(3, Some(InputChoice { first: 2, stereo: false }));

    // by track, the latest choice
    assert_eq!(recorder.armed(), vec![
      ArmedTrack { track_number: 1, input: InputChoice { first: 1, stereo: false } },
      ArmedTrack { track_number: 3, input: InputChoice { first: 2, stereo: false } },
    ]);

    recorder.arm(1, None);
    assert_eq!(recorder.armed().len(), 1);
    recorder.set_armed(&[]);
    assert!(recorder.armed().is_empty());
  }

  // a stand-in for a recording in progress
  fn session() -> Session {
    Session { tracks: vec![], start: Duration::ZERO, keep_from: Instant::now(), loop_len: None }
  }

  #[test]
  fn test_still_recording_until_the_takes_are_in() {
    let state = daw::InnerState::default();
    state.recorder.lock().unwrap().session = Some(session());
    assert!(collect(&state).recording);

    // stopped, but its takes aren't on their tracks yet: asking now used to
    // say it was over, with no clips, so the frontend stopped asking and
    // the clips only showed up with the next recording
    let stopped = state.recorder.lock().unwrap().begin_finish();
    assert!(stopped.is_some());
    assert!(!is_recording(&state));
    let meanwhile = collect(&state);
    assert!(meanwhile.recording);
    assert!(meanwhile.clips.is_empty());

    state.recorder.lock().unwrap().end_finish(vec![], vec!["a problem".into()]);
    assert_eq!(collect(&state), Recorded { recording: false, clips: vec![], problems: vec!["a problem".into()] });
  }

  #[test]
  fn test_a_new_recording_waits_for_the_last_to_finish() {
    let state = Arc::new(daw::InnerState::default());
    state.recorder.lock().unwrap().session = Some(session());
    state.recorder.lock().unwrap().begin_finish();

    let finisher = state.clone();
    let done = std::thread::spawn(move || {
      std::thread::sleep(Duration::from_millis(50));
      finisher.recorder.lock().unwrap().end_finish(vec![], vec![]);
    });

    // nothing's armed, so it's refused, but only once the last one's in
    let asked = Instant::now();
    assert!(start(&state).unwrap_err().contains("Arm a track"));
    assert!(asked.elapsed() >= Duration::from_millis(45));
    done.join().unwrap();
  }

  #[test]
  fn test_a_count_in_is_a_bar() {
    let state = daw::InnerState::default();
    // off unless it's turned on
    assert_eq!(count_in(&state), Duration::ZERO);

    state.playlist.count_in.store(true, std::sync::atomic::Ordering::SeqCst);
    // 4/4 at 120bpm
    assert_eq!(count_in(&state), Duration::from_secs(2));
    state.playlist.time_signature.lock().unwrap().numerator = 3;
    assert_eq!(count_in(&state), Duration::from_millis(1_500));
  }

  #[test]
  fn test_nothing_armed_records_nothing() {
    let state = Arc::new(daw::InnerState::default());
    assert!(start(&state).unwrap_err().contains("Arm a track"));
    assert!(!is_recording(&state));
  }

  #[test]
  fn test_finishing_without_a_recording_does_nothing() {
    let state = daw::InnerState::default();
    finish(&state, false);
    assert_eq!(collect(&state), Recorded { recording: false, clips: vec![], problems: vec![] });
  }

  // records from the real input onto an armed track, playing through the
  // real output: run with `cargo test -- --ignored` (macOS asks for the
  // microphone first)
  #[test]
  #[ignore = "needs audio input and output devices"]
  fn test_recording_onto_the_playlist() {
    let dir = std::env::temp_dir().join(format!("dawesome-recording-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let state = Arc::new(daw::InnerState::default());
    state.project.lock().unwrap().reset(Some(dir.join("song").join("song.dawesome")));
    state.playlist.loop_enabled.store(false, std::sync::atomic::Ordering::SeqCst);
    let channels = daw::input_channel_count().unwrap();
    state.recorder.lock().unwrap().arm(1, Some(InputChoice::default_for(channels)));
    // from beat 2
    let beat = daw::beat_interval(&state);
    *state.playlist.start_offset.lock().unwrap() = beat * 2;

    start(&state).unwrap();
    assert!(is_recording(&state) && state.playlist.playing());
    std::thread::sleep(Duration::from_millis(1_000));
    daw::pause_playlist(&state);

    let recorded = collect(&state);
    println!("{:?}", recorded);
    assert!(!recorded.recording);
    // (a saved input that isn't connected is said, and the default used)
    assert!(recorded.problems.iter().all(|p| p.contains("default input")), "{:?}", recorded.problems);
    let clip = &recorded.clips[0];
    assert_eq!(clip.track_number, 1);
    assert_eq!(clip.start_beat, 2.);
    assert!(clip.path.ends_with("recordings/song_") || clip.path.contains("/song/recordings/song_"));
    // about a second (less the time to get going)
    assert!(clip.info.sample_duration > 0.8 && clip.info.sample_duration < 1.1, "{}", clip.info.sample_duration);
    assert!(state.playlist.audiograph.lock().unwrap().get_node_by_id(clip.id).is_some());

    let _ = std::fs::remove_dir_all(&dir);
  }
}
