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
    recording: recorder.recording(),
    clips: std::mem::take(&mut recorder.clips),
    problems: std::mem::take(&mut recorder.problems),
  }
}

// start recording every armed track, playing from the playhead
pub fn start(state: &Arc<daw::InnerState>) -> Result<(), String> {
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

  let files: Vec<daw::TakeFile> = {
    let mut location = state.project.lock().unwrap();
    let dir = location.recordings_dir()?;
    let paths = project_files::next_take_paths(&dir, &location.name(), &project_files::today(), armed.len());
    armed.iter().zip(paths)
      .map(|(track, path)| daw::TakeFile { path, channels: track.input.channels() })
      .collect()
  };

  // capturing starts first, so nothing's missed while playback gets going.
  // what comes before the run is heard is dropped (see keep_recording_from)
  daw::start_recording(&files)?;
  daw::start_playlist(state);

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
  state.recorder.lock().unwrap().session = Some(session);
  Ok(())
}

// stop recording, and put the takes on their tracks. at a loop's end, the
// takes end there
pub fn finish(state: &daw::InnerState, at_loop_end: bool) {
  let session = match state.recorder.lock().unwrap().session.take() {
    Some(session) => session,
    None => return,
  };

  let rate = daw::engine_if_open().map(|engine| engine.sample_rate()).unwrap_or(48_000);
  let limit = match (at_loop_end, session.loop_len) {
    (true, Some(len)) => Some(((len.saturating_sub(session.start)).as_secs_f64() * rate as f64).round() as u64),
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
  let mut recorder = state.recorder.lock().unwrap();
  recorder.clips.extend(clips);
  recorder.problems.extend(problems);
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
    assert!(recorded.problems.is_empty());
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
