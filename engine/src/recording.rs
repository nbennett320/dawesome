// recording onto the playlist: tracks are armed, each with the input
// channels it records. Record starts the transport from the playhead (or,
// while it's already playing, records in from where it's got to), and
// every armed track records a take of its channels. the input stays open
// while a track is armed, so Record doesn't wait for the device. when playback stops (or
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
use crate::commands::clip::clip_info;
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
  // where on the playlist it started, and in beats (the tempo can't
  // change while recording)
  start: Duration,
  start_beat: f64,
  // when the first frame kept was meant to be captured
  keep_from: Instant,
  // the loop's length, if it was looping (the take ends where it comes round)
  loop_len: Option<Duration>,
  // where the loop ended when the recording started, looping or not: it
  // stays there until the recording's over
  loop_end: Duration,
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

// where the loop ends while a recording is going: where it did when the
// recording started
pub fn loop_len_recording(state: &daw::InnerState) -> Option<Duration> {
  state.recorder.lock().unwrap().session.as_ref().map(|session| session.loop_end)
}

// the clips recorded since this was last asked, and what went wrong
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Recorded {
  pub recording: bool,
  pub clips: Vec<LoadedClip>,
  pub problems: Vec<String>,
  // the take being recorded now, if one is, for the playlist to show
  pub take: Option<TakeInProgress>,
}

// a take being recorded: where on the playlist it started, and the tracks
// it's recording onto. it reaches as far as the playhead
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TakeInProgress {
  pub start_beat: f64,
  pub tracks: Vec<u32>,
}

pub fn collect(state: &daw::InnerState) -> Recorded {
  let mut recorder = state.recorder.lock().unwrap();
  let take = recorder.session.as_ref().map(|session| TakeInProgress {
    start_beat: session.start_beat,
    tracks: session.tracks.iter().map(|(track, _)| *track).collect(),
  });
  Recorded {
    recording: recorder.busy(),
    clips: std::mem::take(&mut recorder.clips),
    problems: std::mem::take(&mut recorder.problems),
    take,
  }
}

// keep the input open while any track is armed (or let it close when none
// is), so pressing Record doesn't wait for the device to open. an input
// that can't be opened is said when recording, or in the track's choices
pub fn hold_input_for_armed(state: &daw::InnerState) {
  let any = !state.recorder.lock().unwrap().armed.is_empty();
  if let Err(e) = daw::hold_input(any) {
    println!("could not open the audio input for the armed tracks: {}", e);
  }
}

// start recording every armed track, playing from the playhead. already
// playing the playlist: record in from where it's got to, without starting
// it again
pub fn start(state: &Arc<daw::InnerState>) -> Result<(), String> {
  let began = Instant::now();
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
  let ready = began.elapsed();

  // when the take starts being heard, and where on the playlist that is
  let (heard, start) = if state.playlist.playing_playlist() {
    // already playing: from now (or, still counting in, from when the
    // playlist's first frame is heard)
    let (_, started) = daw::run_anchor(state);
    let heard = started.map_or(Instant::now(), |started| started.max(Instant::now()));
    (heard, daw::playhead_at(state, heard))
  } else {
    // a pattern playing on its own stops first
    if state.playlist.playing() { daw::pause_playlist(state); }
    daw::start_playlist_after(state, count_in(state));

    let heard = *state.playlist.started_time.lock().unwrap();
    match heard {
      Some(heard) if state.playlist.playing_playlist() => (heard, *state.playlist.start_offset.lock().unwrap()),
      _ => {
        let _ = daw::stop_recording(None);
        for file in &files { let _ = std::fs::remove_file(&file.path); }
        return Err("playback couldn't start, so nothing was recorded".into());
      }
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
    start_beat: if interval.is_zero() { 0. } else { start.as_secs_f64() / interval.as_secs_f64() },
    tracks: armed.iter().zip(&files).map(|(track, file)| (track.track_number, file.path.clone())).collect(),
    start,
    keep_from,
    loop_len: daw::loop_len(state),
    loop_end: daw::loop_end(state),
  };
  println!(
    "recording {} tracks from {:?} (input ready in {:?}, playing in {:?})",
    session.tracks.len(), session.start, ready, began.elapsed());
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
mod tests;
