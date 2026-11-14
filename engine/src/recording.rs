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
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use crate::daw;
use crate::commands::clip::clip_info;
use crate::history;
use crate::preferences::LoopRecording;
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
  // when that was heard
  heard: Instant,
  // when the first frame kept was meant to be captured
  keep_from: Instant,
  // what it does when the loop comes round
  at_loop_end: LoopRecording,
  // while looping and layering: its passes, playable as they're recorded,
  // the thread putting each finished pass on its tracks (it returns the
  // clips it made), and the pass last sent to play on the pass after it
  live: Option<Arc<daw::LivePasses>>,
  placing: Option<thread::JoinHandle<Vec<u64>>>,
  playing_pass: Option<u32>,
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

// a take being recorded: where on the playlist it started (a loop
// recording's pass: the loop's start, once it's come round), and the tracks
// it's recording onto. it reaches as far as the playhead
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TakeInProgress {
  pub start_beat: f64,
  pub tracks: Vec<u32>,
}

pub fn collect(state: &daw::InnerState) -> Recorded {
  let mut recorder = state.recorder.lock().unwrap();
  let take = recorder.session.as_ref().map(|session| {
    let first = session.loop_end.saturating_sub(session.start);
    let came_round = session.live.is_some() && Instant::now() >= session.heard + first;
    TakeInProgress {
      start_beat: if came_round { 0. } else { session.start_beat },
      tracks: session.tracks.iter().map(|(track, _)| *track).collect(),
    }
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
// it again. `at_loop_end` is what it does when the loop comes round
pub fn start(state: &Arc<daw::InnerState>, at_loop_end: LoopRecording) -> Result<(), String> {
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
  let (dir, name) = {
    let mut location = state.project.lock().unwrap();
    (location.recordings_dir()?, location.name())
  };
  let paths = project_files::next_take_paths(&dir, &name, &project_files::today(), armed.len());
  let files: Vec<daw::TakeFile> = armed.iter().zip(paths)
    .map(|(track, path)| daw::TakeFile { path, channels: track.input.fit(channels).channels() })
    .collect();

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

  // looping and layering: a new pass each time the loop comes round, each
  // put on its tracks as soon as it's done
  let loop_len = daw::loop_len(state);
  let loop_end = daw::loop_end(state);
  let tracks: Vec<u32> = armed.iter().map(|track| track.track_number).collect();
  let layering = at_loop_end == LoopRecording::Layer && loop_len.is_some() && start < loop_end;
  let (passes, finished) = if layering {
    let (finished_tx, finished_rx) = mpsc::channel();
    let count = files.len();
    let passes = daw::Passes {
      first: loop_end - start,
      every: loop_end,
      next_paths: Box::new(move || project_files::next_take_paths(&dir, &name, &project_files::today(), count)),
      finished: finished_tx,
    };
    (Some(passes), Some(finished_rx))
  } else {
    (None, None)
  };
  let live = daw::keep_recording_from(keep_from, passes);
  let placing = finished.map(|finished| {
    let state = state.clone();
    let tracks = tracks.clone();
    thread::spawn(move || place_passes(&state, finished, &tracks, start, keep_from))
  });

  let interval = daw::beat_interval(state);
  let session = Session {
    start_beat: if interval.is_zero() { 0. } else { start.as_secs_f64() / interval.as_secs_f64() },
    tracks: tracks.iter().zip(&files).map(|(&track, file)| (track, file.path.clone())).collect(),
    start,
    heard,
    keep_from,
    at_loop_end,
    live: if layering { live } else { None },
    placing,
    playing_pass: None,
    loop_len,
    loop_end,
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

// where a pass of a recording goes on the playlist: the first where the
// recording started (as late as its take started, if capture hadn't begun
// when the run was heard), the rest at the loop's start
fn pass_start(takes: &daw::Takes, start: Duration, keep_from: Instant) -> Duration {
  if takes.pass > 0 { return Duration::ZERO; }
  // (by less than a frame is just the take starting on a whole frame)
  let rate = takes.files.first().map(|take| take.sample_rate).unwrap_or(48_000);
  let frame = Duration::from_secs_f64(1. / rate.max(1) as f64);
  let late = takes.started
    .map(|started| started.saturating_duration_since(keep_from))
    .filter(|&late| late > frame)
    .unwrap_or_default();
  start + late
}

// put a take's files on their tracks (in order), as clips from `start`.
// while layering, a pass whose sound is playing as it was recorded is held
// back until the loop comes round again, so it isn't heard twice
fn place(state: &daw::InnerState, takes: &daw::Takes, tracks: &[u32], start: Duration) -> (Vec<LoadedClip>, Vec<String>) {
  let mut problems = Vec::new();

  // decoded before taking any lock, as playback needs the graph's
  let mut nodes = Vec::new();
  for (take, &track_number) in takes.files.iter().zip(tracks) {
    if take.frames == 0 {
      let _ = std::fs::remove_file(&take.path);
      continue;
    }
    let path = take.path.to_string_lossy().into_owned();
    match daw::AudioNode::new(0, path.clone(), start, track_number) {
      Ok(node) => nodes.push((path, track_number, node)),
      Err(e) => problems.push(format!("the take couldn't be added: {}", e)),
    }
  }

  // (the recorder's lock first, so the loop can't come round in between)
  let recorder = state.recorder.lock().unwrap();
  let hold = recorder.session.as_ref().is_some_and(|session| session.playing_pass == Some(takes.pass));
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();
  let clips = nodes.into_iter().map(|(path, track_number, node)| {
    let id = audiograph.insert_node(node);
    if hold { audiograph.held_back.insert(id); }
    let node = audiograph.get_node_by_id(id).expect("a node that was just added");
    LoadedClip {
      id,
      path,
      track_number,
      start_beat: if interval.is_zero() { 0. } else { (start.as_secs_f64() / interval.as_secs_f64()) as f32 },
      info: clip_info(node),
    }
  }).collect();
  drop(audiograph);
  drop(recorder);

  (clips, problems)
}

// while layering: put each pass on its tracks as it's finished, so the
// playlist shows it straight away. returns the clips made
fn place_passes(
  state: &daw::InnerState,
  finished: mpsc::Receiver<daw::Takes>,
  tracks: &[u32],
  start: Duration,
  keep_from: Instant,
) -> Vec<u64> {
  let mut ids = Vec::new();
  for takes in finished {
    let (clips, problems) = place(state, &takes, tracks, pass_start(&takes, start, keep_from));
    ids.extend(clips.iter().map(|clip| clip.id));
    let mut recorder = state.recorder.lock().unwrap();
    recorder.clips.extend(clips);
    recorder.problems.extend(problems);
  }
  ids
}

// whether the recording stops when the loop comes round (and playback with
// it), rather than carrying on. one that can't layer stops too: it wasn't
// looping when it started
pub fn stops_at_loop_end(state: &daw::InnerState) -> bool {
  state.recorder.lock().unwrap().session.as_ref()
    .is_some_and(|session| session.at_loop_end == LoopRecording::Stop || session.live.is_none())
}

// where the recording started on the playlist, if one's going
pub fn started_at(state: &daw::InnerState) -> Option<Duration> {
  state.recorder.lock().unwrap().session.as_ref().map(|session| session.start)
}

// which pass of a loop recording ends where the loop comes round, `into`
// the recording: the loop comes round `first` after its start, then every
// `every`. None if it hasn't come round since the recording started
fn ended_pass(first: Duration, every: Duration, into: Duration) -> Option<u32> {
  let (first, every, into) = (first.as_secs_f64(), every.as_secs_f64(), into.as_secs_f64());
  if every <= 0. || into < first - every / 2. { return None; }
  Some(((into - first) / every).round().max(0.) as u32)
}

// the loop's about to come round, heard at `heard`, `at` into the run.
// while layering, the pass ending there is played on the next pass as it's
// recorded (its last moments are in by the time they're read), and the pass
// before it plays from the playlist from now on. returns what to play
pub fn layer_voices(state: &daw::InnerState, engine: &daw::Engine, run: u64, at: Duration, heard: Instant) -> Vec<Box<daw::Voice>> {
  let (pass, start) = {
    let mut recorder = state.recorder.lock().unwrap();
    let Some(session) = recorder.session.as_mut() else { return Vec::new() };
    let Some(live) = session.live.clone() else { return Vec::new() };

    // the pass that was playing as it was recorded is on the playlist now
    state.playlist.audiograph.lock().unwrap().held_back.clear();

    let first = session.loop_end.saturating_sub(session.start);
    let into = heard.saturating_duration_since(session.heard);
    let Some(ended) = ended_pass(first, session.loop_end, into) else { return Vec::new() };
    let Some(pass) = live.pass(ended) else { return Vec::new() };

    session.playing_pass = Some(ended);
    let start = if ended == 0 { session.start } else { Duration::ZERO };
    let tracks: Vec<u32> = session.tracks.iter().map(|(track, _)| *track).collect();
    ((pass, tracks), start)
  };

  let ((pass, tracks), rate) = (pass, engine.sample_rate());
  pass.files.into_iter().zip(tracks).map(|(samples, track)| {
    let pcm = Arc::new(daw::Pcm::live(samples));
    let reader = daw::ClipReader::new(pcm, &daw::ClipSettings::default(), rate, (Duration::ZERO, Duration::MAX), 0, None);
    let route = daw::Route::Track(state.playlist.mixer.track_route(track));
    engine.boxed_voice(Box::new(reader), Some(run), engine.frames_in(at + start), route)
  }).collect()
}

// stop recording, and put the takes on their tracks. at a loop's end, the
// takes end there. a loop recording's passes are already on their tracks:
// this puts the last on. one undo takes the whole recording off
pub fn finish(state: &daw::InnerState, at_loop_end: bool) {
  let mut session = match state.recorder.lock().unwrap().begin_finish() {
    Some(session) => session,
    None => return,
  };

  let limit = match (at_loop_end, session.loop_len) {
    (true, Some(len)) => Some(len.saturating_sub(session.start)),
    _ => None,
  };

  let mut clips = Vec::new();
  let mut problems = Vec::new();

  let stopped = daw::stop_recording(limit);
  // the passes before the last are on their tracks once that's done (the
  // writer's done, so no more are coming)
  let mut ids = session.placing.take()
    .map(|placing| placing.join().unwrap_or_default())
    .unwrap_or_default();

  match stopped {
    Err(e) => problems.push(format!("the take couldn't be finished: {}", e)),
    Ok(None) => problems.push("the input stopped before the take was finished".into()),
    Ok(Some(takes)) => {
      if takes.dropped > 0 {
        problems.push("some audio was lost because the disk couldn't keep up".into());
      }
      let tracks: Vec<u32> = session.tracks.iter().map(|(track, _)| *track).collect();
      let (placed, trouble) = place(state, &takes, &tracks, pass_start(&takes, session.start, session.keep_from));
      clips = placed;
      problems.extend(trouble);
    }
  }
  ids.extend(clips.iter().map(|clip| clip.id));

  // every pass plays from the playlist now
  state.playlist.audiograph.lock().unwrap().held_back.clear();
  // and one undo takes them all off
  if !ids.is_empty() {
    history::record(state, history::Step::new().with_clips(Vec::new(), history::clips(state, &ids)));
  }

  for problem in &problems { println!("recording: {}", problem); }
  state.recorder.lock().unwrap().end_finish(clips, problems);
}

#[cfg(test)]
mod tests;
