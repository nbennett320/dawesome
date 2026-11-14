use crate::daw::{
  daw_core::{
    self,
    SampleBuffer,
  }, 
  state, 
};
use std::fs::File;
use std::io::BufReader;
use std::sync::{
  mpsc,
  Arc, 
};
use std::sync::atomic::{Ordering};
use std::thread;
use std::time::{Duration, Instant};
use rodio::{Decoder, Source};


// (the click's file, for tests that put it in the playlist)
#[cfg(test)]
pub const METRONOME_TICK_PATH: &str = "assets/assets_66-hh-01-or.wav";

lazy_static! {
  pub static ref METRONOME_TICK_SOURCE: SampleBuffer = {
    SampleBuffer::from_bytes(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/assets_66-hh-01-or.wav")))
  };
}

lazy_static! {
  pub static ref METRONOME_TICK_ACCENTED_SOURCE: SampleBuffer = {
    SampleBuffer::from_bytes(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/assets_66-hh-01-or-2.wav")))
  };
}

// TODO(linux): the Linux preview below played through PulseAudio directly.
// it's kept as a reminder to get Linux builds working again (see the note
// on AudioNode in audiograph.rs)
// #[cfg(target_os = "linux")]
// use psimple;
// #[cfg(target_os = "linux")]
// use pulse;
//
// /**
// Play a single sound sample for its entire length
// - Compiled only on Linux systems,
// for use with Alsa, Jack, and PulseAudio drivers
// */
// #[cfg(target_os = "linux")]
// pub async fn play_sample(path: &str) {
//   let path = String::from(path);
//   thread::spawn(move || {
//     // let (_stream, stream_handle) = OutputStream::try_default().unwrap();
//     let file = BufReader::new(File::open(path).unwrap());
//     let source = Decoder::new(file).unwrap();
//
//     let spec = pulse::sample::Spec {
//       format: pulse::sample::Format::S16NE,
//       channels: 2,
//       rate: 44_100,
//     };
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
//     )
//     .unwrap();
//
//     let raw_source: rodio::source::SamplesConverter<_, i16> =
//       source.convert_samples();
//     let raw_vec: std::vec::Vec<i16> = raw_source.collect();
//
//     unsafe {
//       let raw_slice = raw_vec.align_to::<u8>().1;
//       sink.write(raw_slice).unwrap();
//       sink.drain().unwrap();
//     }
//   });
// }

// play a sample file once, straight to the output
pub fn play_sample(path: &str) -> Result<(), String> {
  let file = File::open(path).map_err(|e| format!("could not open '{}': {}", path, e))?;
  let source = Decoder::new(BufReader::new(file))
    .map_err(|e| format!("could not decode '{}': {}", path, e))?;

  daw_core::engine()?.preview(source.convert_samples::<f32>())
}

// length of one beat at the playlist's tempo
pub fn beat_interval(state: &state::InnerState) -> Duration {
  state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .interval_of_subdivision(daw_core::timing::QuarterNote::new())
}

// where the playhead is now: moving with the current run while playing, or
// where the next run will start while stopped or paused
pub fn playhead(state: &state::InnerState) -> Duration {
  playhead_at(state, Instant::now())
}

// where the playhead is (or was, or will be) heard at `at`, as playhead.
// before the run's first frame is heard (counting in), where it starts
pub fn playhead_at(state: &state::InnerState, at: Instant) -> Duration {
  let (start, started) = run_anchor(state);
  // a pattern playing on its own leaves the playlist's playhead where it is
  if !state.playlist.playing_playlist() { return start; }

  let started = match started {
    Some(started) => started,
    None => return start,
  };

  state::timeline_position(start, at.saturating_duration_since(started), loop_len(state))
}

// where on the timeline the playhead counts from, and when that was heard
// (None before the first run). read together, as they're moved together each
// time the loop comes round (see run_playlist)
pub fn run_anchor(state: &state::InnerState) -> (Duration, Option<std::time::Instant>) {
  let start = state.playlist.start_offset.lock().unwrap();
  let started = *state.playlist.started_time.lock().unwrap();
  (*start, started)
}

// where the loop ends (whether or not looping is on): at the end of the last
// clip, on a bar line. while recording, where it was when the recording
// started, so the loop doesn't move under a take
pub fn loop_end(state: &state::InnerState) -> Duration {
  if let Some(frozen) = crate::recording::loop_len_recording(state) { return frozen; }

  let interval = beat_interval(state);
  let beats_per_bar = state.playlist.time_signature.lock().unwrap().numerator;
  let end = state.playlist.audiograph.lock().unwrap().end();
  state::loop_end_after(end, interval, beats_per_bar)
}

// how long the playlist plays before jumping back to the start, or None when
// looping is off
pub fn loop_len(state: &state::InnerState) -> Option<Duration> {
  if !state.playlist.loop_enabled.load(Ordering::SeqCst) { return None; }

  let len = loop_end(state);
  if len.is_zero() { None } else { Some(len) }
}

// how long a pattern plays for at the playlist's tempo, or None if there's no
// such pattern, or it has no steps
pub fn pattern_len(state: &state::InnerState, id: u64) -> Option<Duration> {
  let interval = beat_interval(state);
  state.playlist.sequencer.lock().unwrap()
    .pattern_len(id, interval)
    .filter(|len| !len.is_zero())
}

// where the song ends (the last clip's end), or None with no clips
pub fn song_end(state: &state::InnerState) -> Option<Duration> {
  let end = state.playlist.audiograph.lock().unwrap().end();
  if end.is_zero() { None } else { Some(end) }
}

// how many beats long the playlist is: clips start within it, and the
// playhead stays within it
pub fn playlist_beats(state: &state::InnerState) -> u64 {
  state.playlist.ui.lock().unwrap().beats
}

// work out the playlist's length again (see state::playlist_beats_for), when
// the time signature changes or a project is loaded. moving or pasting clips
// doesn't change it: a clip can only start within it, and one that ends past
// it hangs over the end. (takes the graph's and sequencer's locks: call it
// holding none)
pub fn fit_playlist(state: &state::InnerState) {
  let beats_per_bar = state.playlist.time_signature.lock().unwrap().numerator;
  let song_end = song_end(state).unwrap_or_default();
  let beats = state::playlist_beats_for(song_end, beat_interval(state), beats_per_bar);
  state.playlist.ui.lock().unwrap().beats = beats;
}

// the beat counters (total beats in, and the beat of the bar) at an offset
fn set_beat_counters(state: &state::InnerState, offset: Duration, interval: Duration) {
  let total = if interval.is_zero() { 0 } else { (offset.as_nanos() / interval.as_nanos()) as u64 };
  let numerator = state.playlist.time_signature.lock().unwrap().numerator.max(1);

  state.playlist.total_beats.store(total, Ordering::SeqCst);
  state.playlist.current_beat.store((total % numerator as u64) as u16, Ordering::SeqCst);
}

// how far ahead of the engine each slice is sent: enough for building it, and
// for the scheduler's thread to wake up late. on top of this, two of the
// device's buffers, which the engine renders a whole one at a time
const SCHEDULE_AHEAD: Duration = Duration::from_millis(50);

// play the run's mode (the playlist, or a pattern on its own) from the
// playhead, until it's stopped or replaced
//
// the audio engine's output is the clock: a run starts on whatever frame the
// engine is at when it gets the run's first slice, and every later slice's
// sounds are sent to it a little early, each tagged with the frame it starts
// on. this thread only decides what plays in each slice (a beat), and keeps
// the beat counters in step with what's heard
//
// a slice's notes (pattern clips', for their tracks' copies of the synths,
// or a pattern's on its own, for the master's) go with its sounds, each
// note's end with its start. a stop, a seek or the run
// coming to the end of the song ends the run in the engine, which ends any
// note still held
// the clicks of a count-in: one on each beat of the bar before playback
// starts, from the run's start, the first accented
pub fn count_in_clicks(beats: u16, beat: Duration) -> Vec<(Duration, bool)> {
  (0..beats as u32).map(|n| (beat * n, n == 0)).collect()
}

// - count_in: how long to count in for (none for no count-in). the run
//   starts with that long of clicks, and the playlist after them
pub fn run_playlist(
  state_ref: &Arc<state::InnerState>,
  ready: mpsc::Sender<Result<(), String>>,
  count_in: Duration,
) {
  let state = state_ref.clone();
  let tempo_interval = beat_interval(&state);
  // what this run plays, fixed for the run
  let mode = state.playlist.mode();

  // taken before the thread starts, so a stop sent right after this (before
  // the thread gets going) still ends this run
  let run = state.playlist.run_id.fetch_add(1, Ordering::SeqCst) + 1;

  println!("TEMPO INTERVAL {:?}", tempo_interval);

  thread::spawn(move || {
    let engine = match daw_core::engine() {
      Ok(engine) => engine,
      Err(e) => {
        let _ = ready.send(Err(e));
        return;
      }
    };

    // the run plays from the playhead, or a pattern from the step it was
    // started from
    let first_step = match mode {
      state::PlayMode::Playlist => None,
      state::PlayMode::Pattern(id) => Some(pattern_start_step(&state, id)),
    };
    let start = match first_step {
      None => *state.playlist.start_offset.lock().unwrap(),
      Some(step) => steps_time(step, tempo_interval),
    };
    // the sounds of the slice `dur` long, `at` into the run, as voices, and
    // its notes. while the playlist loops (`loop_len`), its notes end by the
    // loop's end (see run_notes). a pattern's notes already end at its length
    let slice_voices = |at: Duration, dur: Duration, on_beat: bool, resume: bool, loop_len: Option<Duration>| {
      let audiograph_ref = state.playlist.audiograph.lock().unwrap();
      let metronome = on_beat && state.metronome_enabled.load(Ordering::SeqCst);
      let (sounds, notes) = match mode {
        state::PlayMode::Playlist => (
          audiograph_ref.run_sounds_at(at, engine.sample_rate(), dur, metronome, resume),
          audiograph_ref.run_notes(dur, loop_len),
        ),
        state::PlayMode::Pattern(id) => (
          audiograph_ref.pattern_sounds(id, at, engine.sample_rate(), dur, metronome, resume),
          audiograph_ref.pattern_notes(id, dur),
        ),
      };

      let voices = sounds
        .into_iter()
        .map(|(delay, source, route)| engine.boxed_voice(source, Some(run), engine.frames_in(at + delay), route))
        .collect::<Vec<_>>();
      (voices, notes)
    };
    let send_notes = |at: Duration, notes: Vec<daw_core::SliceNote>| {
      if notes.is_empty() { return; }
      let copies = state.playlist.effects.copy_ids();
      if let Err(e) = daw_core::play_slice_notes(&engine, run, at, notes, &copies) {
        println!("could not schedule a beat's notes: {}", e);
      }
    };

    {
      let mut audiograph_ref = state.playlist.audiograph.lock().unwrap();
      audiograph_ref.init();
      audiograph_ref.set_current_offset(Some(start));
    }

    // a run from between beats plays the part beat up to the next one first,
    // so every later slice (and metronome tick) stays on the beat
    let lead = state::until_next_beat(start, tempo_interval);
    let mut slice = if lead.is_zero() { tempo_interval } else { lead };

    // a count-in's clicks come first (whether the metronome's on or not),
    // then the playlist
    let mut first: Vec<_> = if count_in.is_zero() { Vec::new() } else {
      let beats = (count_in.as_secs_f64() / tempo_interval.as_secs_f64()).round() as u16;
      count_in_clicks(beats, tempo_interval)
        .into_iter()
        .map(|(at, accented)| {
          let click: &SampleBuffer = if accented { &METRONOME_TICK_ACCENTED_SOURCE } else { &METRONOME_TICK_SOURCE };
          engine.voice(click.convert_samples(), Some(run), engine.frames_in(at), daw_core::Route::Direct)
        })
        .collect()
    };
    let first_loop = match mode {
      state::PlayMode::Playlist => loop_len(&state),
      state::PlayMode::Pattern(_) => None,
    };
    // a pattern's length as the last slice played it
    let mut played_len = match mode {
      state::PlayMode::Playlist => Duration::ZERO,
      state::PlayMode::Pattern(id) => pattern_len(&state, id).unwrap_or_default(),
    };
    let (voices, first_notes) = slice_voices(count_in, slice, lead.is_zero(), true, first_loop);
    first.extend(voices);

    // where the run is in the song, for the effects: its frame 0 is the
    // playhead, less the count-in
    let signature = *state.playlist.time_signature.lock().unwrap();
    let run_start = daw_core::RunStart {
      tempo: state.tempo() as f64,
      beats_per_bar: signature.numerator,
      beat_unit: signature.denominator,
      origin: engine.frames_in(start) as i64 - engine.frames_in(count_in) as i64,
    };

    // the run starts with its first slice, so nothing in it can arrive late
    // (its notes straight after it: sent before, they'd be for a run that
    // hasn't started)
    let anchor = match engine
      .start_run_at(run, state.playlist.mixer.master(), run_start, first)
      .map(|_| send_notes(count_in, first_notes))
      .and_then(|_| engine.wait_for_anchor(run, Duration::from_secs(2)))
    {
      Ok(anchor) => anchor,
      Err(e) => {
        let _ = engine.stop_run(run);
        let _ = ready.send(Err(e));
        return;
      }
    };

    // stopped before it got going: the stop has already been dealt with
    if state.playlist.run_id.load(Ordering::SeqCst) != run {
      let _ = engine.stop_run(run);
      let _ = ready.send(Ok(()));
      return;
    }

    // the start time is when the run's first frame is heard, so the playhead
    // (and the frontend cursor, which uses the same time) follows what's
    // heard. it's set before flagging the playlist as playing, so anything
    // that sees `playing == true` also sees this run's start time
    // (after a count-in, when the playlist's first frame is heard). a
    // pattern counts its steps from the one it starts on
    {
      let mut from = state.playlist.pattern_start.lock().unwrap();
      if let Some(step) = first_step { *from = step; }
      *state.playlist.started_time.lock().unwrap() = Some(anchor.heard + count_in);
    }
    // the counters follow the playlist, which a pattern doesn't move
    let counts_beats = mode == state::PlayMode::Playlist;
    if counts_beats { set_beat_counters(&state, start, tempo_interval); }
    state.playlist.set_playing(true);
    let _ = ready.send(Ok(()));

    // still the run that should be playing: not stopped, and not replaced by
    // a newer run (a seek while playing stops this one and starts another)
    let current = || is_current(&state, run);
    // the engine's frame `at` into the run, and when it's sent ahead / heard
    let frame_at = |at: Duration| anchor.frame + engine.frames_in(at);
    let send_by = |at: Duration| {
      let ahead = engine.frames_in(SCHEDULE_AHEAD) + 2 * engine.buffer_frames();
      frame_at(at).saturating_sub(ahead)
    };
    let heard_at = |at: Duration| frame_at(at) + engine.frames_in(engine.latency());

    // where the current slice starts in the run
    let mut slice_at = count_in;

    loop {
      let next_at = slice_at + slice;

      // wake up a little before the engine gets to the next slice. stopped,
      // paused or replaced meanwhile: leave the shared state to whoever did that
      if !engine.wait_for_frame(send_by(next_at), &current) || !current() { break; }

      // move on, jumping back to the start at the loop's end (looping is read
      // each slice, so toggling it applies from the next one)
      // a pattern always loops, at its length now (it can be changed while
      // it plays). a pattern deleted while playing ends the run
      let loop_len = match mode {
        state::PlayMode::Playlist => loop_len(&state),
        state::PlayMode::Pattern(id) => match pattern_len(&state, id) {
          Some(len) => Some(len),
          None => {
            println!("the pattern that was playing is gone");
            end_run(&state);
            break;
          }
        },
      };
      let current_offset = state.playlist.audiograph.lock().unwrap().current_offset.unwrap_or_default();
      let next = match (mode, loop_len) {
        (state::PlayMode::Pattern(_), Some(len)) => {
          let next = next_pattern_slice(&state, run, current_offset, slice, len, played_len, anchor.heard + next_at);
          played_len = len;
          next
        }
        _ => state::advance(current_offset, slice, loop_len),
      };

      // without looping, the tape runs out at the end of the song: stop
      // there, once the last clip has been heard. not while recording,
      // which can carry on past it
      if loop_len.is_none() && counts_beats && !crate::recording::is_recording(&state) {
        if let Some(end) = song_end(&state) {
          if next >= end {
            if engine.wait_for_frame(heard_at(next_at), &current) && current() {
              println!("reached the end of the song");
              end_run(&state);
              set_playhead(&state, end);
            }
            break;
          }
        }
      }

      // the playlist's loop comes round with this slice
      let wraps = counts_beats && next < current_offset;

      // a recording that stops at the loop's end stops playback there too,
      // once what's before it has been heard, and the playhead goes back to
      // where the recording started, ready for another take
      if wraps && crate::recording::stops_at_loop_end(&state) {
        if engine.wait_for_frame(heard_at(next_at), &current) && current() {
          println!("recorded to the loop's end");
          let back_to = crate::recording::started_at(&state).unwrap_or_default();
          crate::recording::finish(&state, true);
          end_run(&state);
          set_playhead(&state, back_to);
        }
        break;
      }

      // send the next slice
      state.playlist.audiograph.lock().unwrap().set_current_offset(Some(next));
      slice = tempo_interval;
      let (mut voices, notes) = slice_voices(next_at, slice, true, false, loop_len);
      // and, while a recording layers its passes, the pass that's ending
      if wraps {
        voices.extend(crate::recording::layer_voices(&state, &engine, run, next_at, anchor.heard + next_at));
      }
      if let Err(e) = engine.play(voices) {
        println!("could not schedule the next beat: {}", e);
      }
      send_notes(next_at, notes);

      // and move the counters on once it's heard
      if !engine.wait_for_frame(heard_at(next_at), &current) || !current() { break; }
      if counts_beats { set_beat_counters(&state, next, tempo_interval); }

      // the playlist came round: the playhead counts from here now, so it
      // stays right when the loop's length changes (the last clip was moved)
      // on a later pass
      // (both moved under the first's lock, and only by the run that's still
      // playing: a seek that got in first has already moved the playhead)
      if counts_beats && next < current_offset {
        let mut start_offset = state.playlist.start_offset.lock().unwrap();
        if current() {
          *start_offset = next;
          *state.playlist.started_time.lock().unwrap() = Some(anchor.heard + next_at);
        }
      }

      slice_at = next_at;
    }

    // stopping (or pausing) has already stopped the run in the engine. this
    // covers anything else that ends it. a newer run isn't affected
    let _ = engine.stop_run(run);
  });

  println!("continuing");
}

// end the current run, if any, leaving the playhead where it is
fn end_run(state: &state::InnerState) {
  // a recording ends with playback, its takes put on their tracks
  crate::recording::finish(state, false);

  // tell the run's thread it's done, even before it's flagged as playing
  let run = state.playlist.run_id.fetch_add(1, Ordering::SeqCst);

  // set playlist state to paused
  state
    .playlist
    .set_playing(false);

  // fade out what's playing now, rather than at the next beat
  if let Some(engine) = daw_core::engine_if_open() {
    if let Err(e) = engine.stop_run(run) {
      println!("could not stop playback: {}", e);
    }
  }

  // pause the audiograph
  let mut audiograph_ref = state.playlist.audiograph.lock().unwrap();
  audiograph_ref.pause();
}

// move the playhead (with no run going)
fn set_playhead(state: &state::InnerState, offset: Duration) {
  let interval = beat_interval(state);
  *state.playlist.start_offset.lock().unwrap() = offset;
  set_beat_counters(state, offset, interval);
}

// stop playing, keeping the playhead where it got to, so playing again
// carries on from there
pub fn pause_playlist(state: &state::InnerState) {
  println!("pausing playlist");

  // a pattern playing on its own just stops: the playlist's playhead
  // never moved
  if state.playlist.playing_pattern().is_some() {
    end_run(state);
    return;
  }

  // read before stopping, while the run's start time still applies
  let position = playhead(state);
  end_run(state);
  set_playhead(state, state::snap_to_beat(position, beat_interval(state)));
}

// stop playing and wind back to the start
pub fn stop_playlist(state: &state::InnerState) {
  println!("stopping playlist");

  // stopping a pattern leaves the playlist's playhead alone
  let pattern = state.playlist.playing_pattern().is_some();
  end_run(state);
  if !pattern { set_playhead(state, Duration::ZERO); }
}

pub fn start_playlist(state: &Arc<state::InnerState>) {
  start_playlist_after(state, Duration::ZERO);
}

// play the playlist from the playhead, after a count-in this long (see
// run_playlist)
pub fn start_playlist_after(state: &Arc<state::InnerState>, count_in: Duration) {
  println!("playing playlist");

  // a pattern playing on its own stops first
  if state.playlist.playing_pattern().is_some() { end_run(state); }
  *state.playlist.mode.lock().unwrap() = state::PlayMode::Playlist;

  // a playhead past the loop's end starts back at the top, so the run (and
  // the frontend cursor) stays inside the loop. without looping, playing
  // from the end of the song starts it again
  let end = match loop_len(state) {
    Some(len) => Some(len),
    None => song_end(state),
  };
  if let Some(end) = end {
    let mut start = state.playlist.start_offset.lock().unwrap();
    if *start >= end { *start = Duration::ZERO; }
  }

  start_run(state, count_in);
}

// start a run of whatever the mode is, and wait until the output stream is
// open and the start time is set, so a `get_playlist_playing` right after
// this sees the new run
fn start_run(state: &Arc<state::InnerState>, count_in: Duration) {
  // every synth's copies there, and in step with it, before anything plays
  // (here, as the run's thread can't reach the main thread: this one may be
  // it, waiting below)
  crate::commands::synth::before_playing(state);
  let (ready_tx, ready_rx) = mpsc::channel();
  run_playlist(state, ready_tx, count_in);

  match ready_rx.recv() {
    Ok(Ok(())) => {}
    Ok(Err(e)) => println!("could not start playback: {}", e),
    Err(_) => println!("could not start playback: playback thread exited"),
  }
}

// how long a number of steps (fractional) lasts, and how many steps a time
// is, with beats `beat` long
fn steps_time(steps: f64, beat: Duration) -> Duration {
  Duration::from_secs_f64(beat.as_secs_f64() * steps.max(0.) / daw_core::STEPS_PER_BEAT as f64)
}

fn time_steps(time: Duration, beat: Duration) -> f64 {
  offset_beats(time, beat) * daw_core::STEPS_PER_BEAT as f64
}

// the step a run of pattern `id` starts on: the step it was started from
// (the Piano Roll's start marker), or its start if that's past its end (it
// was made shorter)
pub fn pattern_start_step(state: &state::InnerState, id: u64) -> f64 {
  let step = state.playlist.pattern_start.lock().unwrap().max(0.);
  let length = state.playlist.sequencer.lock().unwrap().pattern(id).map_or(0, |p| p.length);
  if step < length as f64 { step } else { 0. }
}

// the step a pattern playing on its own counts from, and when that step is
// (or was) heard: None before its run has started. read together, as
// they're moved together (see next_pattern_slice)
pub fn pattern_anchor(state: &state::InnerState) -> (f64, Option<Instant>) {
  let step = state.playlist.pattern_start.lock().unwrap();
  let heard = *state.playlist.started_time.lock().unwrap();
  (*step, heard)
}

// the step (fractional) a pattern playing on its own has got to: counted
// from its anchor (see pattern_anchor), wrapped at its length now. None
// before it's started
pub fn pattern_step(state: &state::InnerState, id: u64) -> Option<f64> {
  let length = state.playlist.sequencer.lock().unwrap().pattern(id)?.length;
  let (from, heard) = pattern_anchor(state);
  let heard = heard?;
  if length == 0 { return None; }
  // (the anchor can be a moment ahead: it's moved as a slice is sent)
  let now = Instant::now();
  let beat = beat_interval(state);
  let since = if now >= heard { time_steps(now - heard, beat) } else { -time_steps(heard - now, beat) };
  Some((from + since).rem_euclid(length as f64))
}

// whether run `run` is still the one playing: not stopped, and not replaced
// by a newer run (a seek while playing stops one and starts another)
fn is_current(state: &state::InnerState, run: u64) -> bool {
  state.playlist.run_id.load(Ordering::SeqCst) == run && state.playlist.playing()
}

// a pattern playing on its own in run `run`, after a slice `slice` long
// from `offset`: where the next slice starts, at the pattern's length now
// (`len`: it can be changed while it plays), going round to its start at
// its end (see state::advance_round). when it goes round, or its length
// isn't what the last slice's was (`was`), it counts its steps from there,
// heard at `heard`. the cursors (pattern_step, and the frontend's from
// get_playlist_transport) work out the step from that and its length now,
// so they stay on the step heard: a pattern grown while it plays carries
// on from the same step
pub fn next_pattern_slice(
  state: &state::InnerState,
  run: u64,
  offset: Duration,
  slice: Duration,
  len: Duration,
  was: Duration,
  heard: Instant,
) -> Duration {
  let next = state::advance_round(offset, slice, len);
  if next != offset + slice || len != was {
    let step = time_steps(next, beat_interval(state));
    // (moved only by the run that's still playing: a seek or a tempo change
    // that got in first has already put its own step there)
    let mut from = state.playlist.pattern_start.lock().unwrap();
    if is_current(state, run) {
      *from = step;
      *state.playlist.started_time.lock().unwrap() = Some(heard);
    }
  }
  next
}

// loop a pattern on its own (Play in a sequencer tab, or the Piano Roll),
// stopping whatever was playing: from `from_step` (the Piano Roll's start
// marker, fractional) to its end, then the whole pattern over and over. its
// synth rows' notes are played by their plugins' master copies. the
// playlist's playhead stays where it is
pub fn start_pattern(state: &Arc<state::InnerState>, id: u64, from_step: f64) -> Result<(), String> {
  println!("playing pattern {} from step {}", id, from_step);
  if pattern_len(state, id).is_none() {
    return Err(format!("no pattern with id {}", id));
  }

  if state.playlist.playing() {
    if state.playlist.playing_playlist() {
      // keep the playlist's place, as pausing would
      pause_playlist(state);
    } else {
      end_run(state);
    }
  }
  // (once the run before has ended, so it can't move the step on)
  *state.playlist.pattern_start.lock().unwrap() = if from_step.is_finite() { from_step.max(0.) } else { 0. };
  *state.playlist.mode.lock().unwrap() = state::PlayMode::Pattern(id);
  start_run(state, Duration::ZERO);
  Ok(())
}

// while pattern `id` plays on its own, carry on from `step` instead (the
// Piano Roll's ruler was clicked). otherwise nothing changes: the marker the
// next Play starts from is kept by the frontend
pub fn seek_pattern(state: &Arc<state::InnerState>, id: u64, step: f64) -> Result<(), String> {
  if state.playlist.playing_pattern() != Some(id) { return Ok(()); }
  start_pattern(state, id, step)
}

// move the playhead to a beat, kept within the playlist. while playing,
// playback carries on from there. returns the beat it moved to
pub fn seek_playlist(state: &Arc<state::InnerState>, beat: f64) -> f64 {
  let interval = beat_interval(state);
  let end = playlist_beats(state) as f64;
  let beat = if beat.is_finite() { beat.clamp(0., end) } else { 0. };
  let offset = state::snap_to_beat(interval.mul_f64(beat), interval);

  // a pattern playing on its own carries on: only the playhead moves
  let playing = state.playlist.playing_playlist();
  if playing { end_run(state); }
  set_playhead(state, offset);
  if playing { start_playlist(state); }

  offset_beats(offset, interval)
}

// an offset in (fractional) beats
pub fn offset_beats(offset: Duration, interval: Duration) -> f64 {
  if interval.is_zero() { 0. } else { offset.as_secs_f64() / interval.as_secs_f64() }
}

// what was playing when the tempo changed, to carry on with
#[derive(Debug, PartialEq)]
pub enum Retimed {
  Stopped,
  Playlist,
  Pattern(u64),
}

// change the tempo, ending whatever's playing. every clip, and the playhead,
// stays on its beat: a playlist that was playing is left paused on the beat
// it had got to, and a pattern on the step it had got to. returns what was
// playing (see set_playlist_tempo)
pub fn retime(state: &state::InnerState, tempo: f32) -> Result<Retimed, String> {
  if !daw_core::timing::valid_tempo(tempo) {
    return Err(format!("the tempo can't be {}", tempo));
  }
  // carrying on at another tempo would cut a take short, and one being put
  // on its track is placed in the tempo it was recorded at
  if crate::recording::is_busy(state) {
    return Err("the tempo can't be changed while recording".into());
  }
  if state.tempo() == tempo { return Ok(Retimed::Stopped); }

  let was = match state.playlist.playing_pattern() {
    Some(id) => Retimed::Pattern(id),
    None if state.playlist.playing_playlist() => Retimed::Playlist,
    None => Retimed::Stopped,
  };
  match was {
    Retimed::Playlist => {
      // where it's got to (in the old tempo's time), before the run ends
      let position = playhead(state);
      end_run(state);
      *state.playlist.start_offset.lock().unwrap() = position;
    }
    Retimed::Pattern(id) => {
      // where it's got to (in the old tempo's steps), to carry on from
      let reached = pattern_step(state, id);
      end_run(state);
      if let Some(step) = reached { *state.playlist.pattern_start.lock().unwrap() = step; }
    }
    Retimed::Stopped => {}
  }

  state.set_tempo(tempo);
  // the counters follow the playhead's beat
  let position = *state.playlist.start_offset.lock().unwrap();
  set_playhead(state, position);

  Ok(was)
}

// change the tempo. every clip, and the playhead, stays on its beat. while
// the playlist plays, it carries on from the beat it had got to (after a
// short fade), and so does a pattern playing on its own. returns the tempo
pub fn set_playlist_tempo(state: &Arc<state::InnerState>, tempo: f32) -> Result<f32, String> {
  match retime(state, tempo)? {
    Retimed::Playlist => start_playlist(state),
    // (the tempo's changed even if the pattern has gone meanwhile). from the
    // step it had got to (read first: an `if let` holds its lock until its
    // end, and start_pattern takes it again)
    Retimed::Pattern(id) => {
      let from = *state.playlist.pattern_start.lock().unwrap();
      if let Err(e) = start_pattern(state, id, from) {
        println!("could not carry on with pattern {}: {}", id, e);
      }
    }
    Retimed::Stopped => {}
  }

  Ok(tempo)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Instant;

  #[test]
  fn test_count_in_clicks() {
    let beat = Duration::from_millis(500);
    assert_eq!(count_in_clicks(4, beat), vec![
      (Duration::ZERO, true),
      (beat, false),
      (beat * 2, false),
      (beat * 3, false),
    ]);
    assert!(count_in_clicks(0, beat).is_empty());
  }

  // where the playhead is heard at a moment: what a recording started
  // while playing counts from
  #[test]
  fn test_the_playhead_at_a_moment() {
    let state = state::InnerState::default();
    let started = Instant::now();
    *state.playlist.start_offset.lock().unwrap() = Duration::from_secs(1);
    *state.playlist.started_time.lock().unwrap() = Some(started);

    // stopped: where the next run starts
    assert_eq!(playhead_at(&state, started + Duration::from_secs(5)), Duration::from_secs(1));

    state.playlist.set_playing(true);
    assert_eq!(playhead_at(&state, started + Duration::from_millis(1_500)), Duration::from_millis(2_500));
    // still counting in: where it starts
    assert_eq!(playhead_at(&state, started - Duration::from_millis(200)), Duration::from_secs(1));
  }

  // plays through the real output: run with `cargo test -- --ignored`
  #[test]
  #[ignore = "needs an audio output device"]
  fn test_a_count_in_holds_the_playhead_for_a_bar() {
    let state = Arc::new(state::InnerState::default());
    *state.playlist.start_offset.lock().unwrap() = Duration::from_secs(1);
    let count_in = Duration::from_secs(2);

    start_playlist_after(&state, count_in);
    let started = state.playlist.started_time.lock().unwrap().expect("a start time");
    let heard = started.duration_since(Instant::now());
    println!("the playlist is heard {:?} after play", heard);
    // a bar off, less what's passed since (and plus the output's latency)
    assert!(heard > Duration::from_millis(1_900) && heard < Duration::from_millis(2_200), "{:?}", heard);

    // held at the playhead through the count-in
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(playhead(&state), Duration::from_secs(1));
    // and moving once it's over
    std::thread::sleep(Duration::from_millis(2_000));
    let moved = playhead(&state);
    assert!(moved > Duration::from_millis(1_300) && moved < Duration::from_millis(1_700), "{:?}", moved);

    stop_playlist(&state);
  }
}
