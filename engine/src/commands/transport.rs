use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use crate::{daw, history};

#[tauri::command]
pub fn toggle_playlist(state: daw::TState<'_>) {
  if state.playlist.playing() {
    daw::pause_playlist(state.inner());
  } else {
    // start playlist
    daw::start_playlist(state.inner());
  }
}

// play from the playhead (nothing if already playing)
#[tauri::command]
pub fn play_playlist(state: daw::TState<'_>) {
  // a pattern playing on its own is replaced by the playlist
  if !state.playlist.playing_playlist() {
    daw::start_playlist(state.inner());
  }
}

// stop where it is, so play carries on from there
#[tauri::command]
pub fn pause_playlist(state: daw::TState<'_>) {
  if state.playlist.playing() {
    daw::pause_playlist(state.inner());
  }
}

// stop and go back to the start
#[tauri::command]
pub fn stop_playlist(state: daw::TState<'_>) {
  daw::stop_playlist(state.inner());
}

// move the playhead to a beat (kept within the playlist), carrying on playing
// from there if playing. returns the beat it moved to
#[tauri::command]
pub fn seek_playlist(state: daw::TState<'_>, beat: f64) -> f64 {
  daw::seek_playlist(state.inner(), beat)
}

#[tauri::command]
pub fn get_playlist_playing(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.playing())
}

// convert the engine's monotonic start time into wall clock ms
// (since the unix epoch), which the frontend can compare to Date.now()
fn wall_clock_ms(started: Instant) -> Result<f64, String> {
  let now = Instant::now();
  // UNIX_EPOCH is the epoch on every platform, including windows
  let wall = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .map_err(|e| e.to_string())?
    .as_secs_f64() * 1000.;

  // the start time is when the run's first frame is heard, which is just
  // after it starts (by the output's latency)
  Ok(if started >= now {
    wall + started.duration_since(now).as_secs_f64() * 1000.
  } else {
    wall - now.duration_since(started).as_secs_f64() * 1000.
  })
}

// everything the frontend needs to draw the playback cursor in sync with
// the engine's beat loop, without polling every frame
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTransport {
  playing: bool,
  // engine start time of the current run, ms since the unix epoch (None
  // before the first run). for a pattern playing on its own, when step 0 of
  // its time round was (or would have been) heard: it may have started part
  // way in, from the Piano Roll's start marker, or changed length since
  // (see pattern_length)
  start_time_ms: Option<f64>,
  // where on the timeline the current run started, in ms. while playing,
  // the playhead is this plus the time since `start_time_ms` (wrapped at the
  // loop's end)
  start_offset_ms: f64,
  // where the playhead is now, in ms
  position_ms: f64,
  // length of one beat tick in the engine loop
  beat_interval_ms: f64,
  // where the loop starts and ends, in beats (whether or not looping is
  // on). it plays in from before its start, and goes round to its start at
  // its end (see daw::state::timeline_position)
  loop_start_beats: f64,
  loop_end_beats: f64,
  // whether there's a region drawn in the ruler (which is the loop), and
  // whether the end flag was dragged somewhere, rather than following the
  // last clip
  loop_region: bool,
  loop_end_pinned: bool,
  // beats spanned by the full playlist width
  beats_displayed: u64,
  loop_enabled: bool,
  // where the last clip finishes, in ms (0 with no clips)
  song_end_ms: f64,
  // the time signature's numerator
  beats_per_bar: u16,
  // the pattern playing on its own (Play in the Sequencer tab), or None
  // when the playlist is playing or nothing is. while a pattern plays, the
  // playlist's playhead stays still
  pattern_id: Option<u64>,
  // how long that pattern is, in steps (0 for none). it loops at this
  // length, counted from `start_time_ms`. when its length changes, its
  // steps are counted afresh from the one it's on (see
  // daw::next_pattern_slice), so `start_time_ms` moves with it
  pattern_length: u32,
}

fn playlist_transport(state: &daw::InnerState) -> PlaylistTransport {
  let beat_interval = daw::beat_interval(state);
  // read as a pair: both move when the loop comes round
  let (start_offset, started) = daw::run_anchor(state);
  // (worked out before the struct, whose fields hold other locks as they're read)
  let span = daw::loop_span(state);
  let loop_region = state.playlist.loop_region.lock().unwrap().is_some();
  let loop_end_pinned = state.playlist.loop_end_pin.lock().unwrap().is_some();
  let pattern = state.playlist.playing_pattern();
  // a pattern counts from the step it was on when it last came round,
  // changed length, or started (see daw::pattern_anchor)
  let (started, counted_from_ms) = match pattern {
    Some(_) => {
      let (step, heard) = daw::pattern_anchor(state);
      (heard, step * beat_interval.as_secs_f64() * 1000. / daw::STEPS_PER_BEAT as f64)
    }
    None => (started, 0.),
  };

  PlaylistTransport {
    playing: state.playlist.playing(),
    start_time_ms: started.and_then(|started| wall_clock_ms(started).ok()).map(|ms| ms - counted_from_ms),
    start_offset_ms: start_offset.as_secs_f64() * 1000.,
    position_ms: daw::playhead(state).as_secs_f64() * 1000.,
    beat_interval_ms: beat_interval.as_secs_f64() * 1000.,
    loop_start_beats: daw::offset_beats(span.start, beat_interval),
    loop_end_beats: daw::offset_beats(span.end, beat_interval),
    loop_region,
    loop_end_pinned,
    beats_displayed: daw::playlist_beats(state),
    loop_enabled: state.playlist.loop_enabled.load(Ordering::SeqCst),
    song_end_ms: daw::song_end(state).unwrap_or_default().as_secs_f64() * 1000.,
    beats_per_bar: state.playlist.time_signature.lock().unwrap().numerator,
    pattern_id: pattern,
    pattern_length: pattern
      .and_then(|id| state.playlist.sequencer.lock().unwrap().pattern(id).map(|p| p.length))
      .unwrap_or(0),
  }
}

#[tauri::command]
pub fn get_playlist_transport(
  state: daw::TState<'_>
) -> PlaylistTransport {
  playlist_transport(state.inner())
}

// change the tempo (see daw::set_playlist_tempo), as a step that can be
// undone. refused while recording
// - gesture: the visit to the tempo field it's part of, if any
pub fn change_tempo(state: &Arc<daw::InnerState>, tempo: f32, gesture: Option<u64>) -> Result<f32, String> {
  let before = state.tempo();
  let after = daw::set_playlist_tempo(state, tempo)?;
  history::record(state, history::Step::new().in_gesture(gesture).with_tempo(before, after));
  Ok(after)
}

#[tauri::command]
pub fn set_playlist_tempo(
  state: daw::TState<'_>,
  val: f32,
  gesture: Option<u64>,
) -> Result<f32, String> {
  println!("playlist tempo updated: {}", val);
  change_tempo(state.inner(), val, gesture)
}

#[tauri::command]
pub fn toggle_metronome_enabled(
  state: daw::TState<'_>
) {
  let val = !state.metronome_enabled.load(Ordering::SeqCst);
  state.metronome_enabled.store(val, Ordering::SeqCst);
}

// whether recording counts in a bar first. returns the new setting
#[tauri::command]
pub fn toggle_count_in(state: daw::TState<'_>) -> bool {
  !state.playlist.count_in.fetch_xor(true, Ordering::SeqCst)
}

#[tauri::command]
pub fn get_metronome_enabled(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.metronome_enabled.load(Ordering::SeqCst))
}

#[tauri::command]
pub fn set_playlist_time_signature(
  state: daw::TState<'_>,
  numerator: u16,
  denominator: u16,
) {
  set_time_signature(&state, daw::timing::TimeSignature { numerator, denominator });
}

// the playlist's length is counted in its bars, so it's worked out again
pub fn set_time_signature(state: &daw::InnerState, signature: daw::timing::TimeSignature) {
  *state.playlist.time_signature.lock().unwrap() = signature;
  daw::fit_playlist(state);
}

#[tauri::command]
pub fn toggle_loop_enabled(
  state: daw::TState<'_>
) {
  // a recording cuts its passes where the loop comes round: it stays as it
  // is until the recording's over
  if crate::recording::is_recording(&state) { return; }

  let val = !state
    .playlist
    .loop_enabled
    .load(Ordering::SeqCst);

  state
    .playlist
    .loop_enabled
    .store(val, Ordering::SeqCst);
}

// the loop as the playlist has it: the region drawn in the ruler, where the
// end flag was dragged to (each in beats), and whether looping is on
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoopSettings {
  region: Option<daw::LoopRegion>,
  end_pin: Option<f64>,
  enabled: bool,
}

fn loop_settings(state: &daw::InnerState) -> LoopSettings {
  LoopSettings {
    region: *state.playlist.loop_region.lock().unwrap(),
    end_pin: *state.playlist.loop_end_pin.lock().unwrap(),
    enabled: state.playlist.loop_enabled.load(Ordering::SeqCst),
  }
}

// draw the region the playlist loops, between two beats (in either order,
// kept to the playlist). on the same beat, there's no region. drawing one
// where there was none turns looping on. while recording, the loop stays as
// it is until it's over (see toggle_loop_enabled), and nothing changes.
// returns the loop as it is now
pub fn change_loop_region(state: &daw::InnerState, start: f64, end: f64) -> LoopSettings {
  if !crate::recording::is_recording(state) {
    let beats = daw::playlist_beats(state) as f64;
    let mut region = state.playlist.loop_region.lock().unwrap();
    let new = daw::LoopRegion::between(start, end, beats);
    if region.is_none() && new.is_some() {
      state.playlist.loop_enabled.store(true, Ordering::SeqCst);
    }
    *region = new;
  }
  loop_settings(state)
}

#[tauri::command]
pub fn set_loop_region(state: daw::TState<'_>, start: f64, end: f64) -> LoopSettings {
  change_loop_region(&state, start, end)
}

// take the region away: the loop's from the start to the end flag again.
// not while recording
pub fn remove_loop_region(state: &daw::InnerState) -> LoopSettings {
  if !crate::recording::is_recording(state) {
    *state.playlist.loop_region.lock().unwrap() = None;
  }
  loop_settings(state)
}

#[tauri::command]
pub fn clear_loop_region(state: daw::TState<'_>) -> LoopSettings {
  remove_loop_region(&state)
}

// pin the loop's end flag at a beat (at least one in, and within the
// playlist), so it stays there when clips move. not while recording
pub fn change_loop_end(state: &daw::InnerState, beat: f64) -> LoopSettings {
  if !crate::recording::is_recording(state) {
    let beats = daw::playlist_beats(state) as f64;
    if let Some(pin) = daw::valid_loop_end_pin(beat.min(beats)) {
      *state.playlist.loop_end_pin.lock().unwrap() = Some(pin);
    }
  }
  loop_settings(state)
}

#[tauri::command]
pub fn set_loop_end(state: daw::TState<'_>, beat: f64) -> LoopSettings {
  change_loop_end(&state, beat)
}

// let the loop's end flag follow the last clip again. not while recording
pub fn unpin_loop_end(state: &daw::InnerState) -> LoopSettings {
  if !crate::recording::is_recording(state) {
    *state.playlist.loop_end_pin.lock().unwrap() = None;
  }
  loop_settings(state)
}

#[tauri::command]
pub fn clear_loop_end(state: daw::TState<'_>) -> LoopSettings {
  unpin_loop_end(&state)
}

#[tauri::command]
pub fn get_loop_settings(state: daw::TState<'_>) -> LoopSettings {
  loop_settings(&state)
}

#[tauri::command]
pub fn get_loop_enabled(
  state: daw::TState<'_>
) -> Result<bool, String> {
  Ok(state.playlist.loop_enabled.load(Ordering::SeqCst))
}

#[cfg(test)]
mod tests;
