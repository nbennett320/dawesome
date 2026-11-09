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
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{
  Duration,
  Instant,
};
use rodio::{
  Decoder, 
  OutputStream, 
  Sink,
};
use tauri;


pub const METRONOME_TICK_PATH: &str = "assets/assets_66-hh-01-or.wav";
pub const METRONOME_TICK_ACCENTED_PATH: &str = "assets/assets_66-hh-01-or-2.wav";

lazy_static! {
  pub static ref METRONOME_TICK_SOURCE: SampleBuffer = {
    SampleBuffer::load(METRONOME_TICK_PATH).unwrap()
  };
}

lazy_static! {
  static ref METRONOME_TICK_ACCENTED_SOURCE: SampleBuffer = {
    SampleBuffer::load(METRONOME_TICK_ACCENTED_PATH).unwrap()
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

/**
Play a single sound sample for its entire length
- Compiled only on Windows and MacOS systems,
for use with ALSA and CoreAudio
*/
#[cfg(not(target_os = "linux"))]
pub async fn play_sample(path: &str) {
  let path = String::from(path);
  thread::spawn(move || {
    // read and decode audio file, and append to a sound sink
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let file = BufReader::new(File::open(path).unwrap());
    let source = Decoder::new(file).unwrap();

    sink.append(source);
    sink.play();
    sink.sleep_until_end();
  });
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
  let start = *state.playlist.start_offset.lock().unwrap();
  // a pattern playing on its own leaves the playlist's playhead where it is
  if !state.playlist.playing_playlist() { return start; }

  let started = match *state.playlist.started_time.lock().unwrap() {
    Some(started) => started,
    None => return start,
  };

  let loop_len = state.playlist.loop_len(beat_interval(state));
  state::timeline_position(start, started.elapsed(), loop_len)
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

// the beat counters (total beats in, and the beat of the bar) at an offset
fn set_beat_counters(state: &state::InnerState, offset: Duration, interval: Duration) {
  let total = if interval.is_zero() { 0 } else { (offset.as_nanos() / interval.as_nanos()) as u64 };
  let numerator = state.playlist.time_signature.lock().unwrap().numerator.max(1);

  state.playlist.total_beats.store(total, Ordering::SeqCst);
  state.playlist.current_beat.store((total % numerator as u64) as u16, Ordering::SeqCst);
}

pub fn run_playlist(
  state_ref: &Arc<state::InnerState>,
  ready: mpsc::Sender<Result<(), String>>,
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
    // output stream for the whole run
    let (_stream, stream_handle) = match OutputStream::try_default() {
      Ok(s) => s,
      Err(e) => {
        let _ = ready.send(Err(format!("could not open audio output: {}", e)));
        return;
      }
    };

    // the master bus: every beat's clips are added to it, and it plays through
    // the master strip and limiter. it's kept going with silence until playback stops
    // (rodio's mixer ends as soon as it has nothing to play)
    let sample_rate = state.playlist.audiograph.lock().unwrap().sample_rate();
    let (bus, bus_out) = rodio::dynamic_mixer::mixer::<f32>(2, sample_rate);
    bus.add(rodio::source::Zero::<f32>::new(2, sample_rate));

    let stop = Arc::new(AtomicBool::new(false));
    let master = state.playlist.mixer.master_strip(bus_out);
    if let Err(e) = stream_handle.play_raw(daw_core::FadeOnStop::new(daw_core::SoftLimit::new(master), stop.clone())) {
      let _ = ready.send(Err(format!("could not play the master bus: {}", e)));
      return;
    }
    // stop the previous run's bus, if it's still going
    if let Some(previous) = state.playlist.bus_stop.lock().unwrap().replace(stop.clone()) {
      previous.store(true, Ordering::SeqCst);
    }

    // the run plays from the playhead, or a pattern from its start
    let start = match mode {
      state::PlayMode::Playlist => *state.playlist.start_offset.lock().unwrap(),
      state::PlayMode::Pattern(_) => Duration::ZERO,
    };

    // every tick is scheduled relative to the start time, so time spent
    // building each slice doesn't accumulate into drift
    // the frontend cursor uses the same start time, so the two stay aligned
    // the start time is set before flagging the playlist as playing, so
    // anything that sees `playing == true` also sees this run's start time
    let started = Instant::now();
    *state.playlist.started_time.lock().unwrap() = Some(started);
    {
      let mut audiograph_ref = state.playlist.audiograph.lock().unwrap();
      audiograph_ref.init();
      audiograph_ref.set_current_offset(Some(start));
    }
    // the counters follow the playlist, which a pattern doesn't move
    let counts_beats = mode == state::PlayMode::Playlist;
    if counts_beats { set_beat_counters(&state, start, tempo_interval); }
    state.playlist.set_playing(true);
    let _ = ready.send(Ok(()));

    // still the run that should be playing: not stopped, and not replaced by
    // a newer run (a seek while playing stops this one and starts another)
    let current = || {
      state.playlist.run_id.load(Ordering::SeqCst) == run && state.playlist.playing()
    };

    // a run from between beats plays the part beat up to the next one first,
    // so every later slice (and metronome tick) stays on the beat
    let lead = state::until_next_beat(start, tempo_interval);
    let mut slice = if lead.is_zero() { tempo_interval } else { lead };
    let mut on_beat = lead.is_zero();
    let mut resume = true;
    let mut next_tick = started;

    loop {
      // schedule this slice of the audio graph to be played
      {
        let audiograph_ref = state.playlist.audiograph.lock().unwrap();
        println!("tick: {}ms", audiograph_ref.current_offset.unwrap_or_default().as_millis());

        let metronome = on_beat && state.metronome_enabled.load(Ordering::SeqCst);
        match mode {
          state::PlayMode::Playlist =>
            audiograph_ref.run_for(&stream_handle, &bus, slice, metronome, resume),
          state::PlayMode::Pattern(id) =>
            audiograph_ref.run_pattern_for(&stream_handle, &bus, id, slice, metronome, resume),
        }
      } // release the audiograph while sleeping so commands aren't blocked

      // sleep until the start of the next slice
      next_tick += slice;
      thread::sleep(next_tick.saturating_duration_since(Instant::now()));

      // stopped, paused or replaced while sleeping: leave the shared state to
      // whoever did that
      if !current() { break; }

      // move on, jumping back to the start at the loop's end (looping is read
      // each slice, so toggling it applies right away)
      // a pattern always loops, at its length now (it can be changed while
      // it plays). a pattern deleted while playing ends the run
      let loop_len = match mode {
        state::PlayMode::Playlist => state.playlist.loop_len(tempo_interval),
        state::PlayMode::Pattern(id) => match pattern_len(&state, id) {
          Some(len) => Some(len),
          None => {
            println!("the pattern that was playing is gone");
            end_run(&state);
            break;
          }
        },
      };
      let next = {
        let mut audiograph_ref = state.playlist.audiograph.lock().unwrap();
        let next = state::advance(audiograph_ref.current_offset.unwrap_or_default(), slice, loop_len);
        audiograph_ref.set_current_offset(Some(next));
        next
      };
      if counts_beats { set_beat_counters(&state, next, tempo_interval); }

      // without looping, the tape runs out at the end of the song: stop
      // there, once the last clip has played out
      if loop_len.is_none() && counts_beats {
        if let Some(end) = song_end(&state) {
          if next >= end {
            println!("reached the end of the song");
            end_run(&state);
            set_playhead(&state, end);
            break;
          }
        }
      }

      println!(
        "current beat: {}, total beats played: {}, looping: {}",
        state.playlist.current_beat.load(Ordering::SeqCst),
        state.playlist.total_beats.load(Ordering::SeqCst),
        loop_len.is_some(),
      );

      slice = tempo_interval;
      on_beat = true;
      resume = false;
    }

    // pausing already stopped the bus. this covers anything else that ends
    // the run
    stop.store(true, Ordering::SeqCst);
  });

  println!("continuing");
}

// end the current run, if any, leaving the playhead where it is
fn end_run(state: &state::InnerState) {
  // tell the run's thread it's done, even before it's flagged as playing
  state.playlist.run_id.fetch_add(1, Ordering::SeqCst);

  // set playlist state to paused
  state
    .playlist
    .set_playing(false);

  // fade out what's playing now, rather than at the next beat
  if let Some(stop) = state.playlist.bus_stop.lock().unwrap().take() {
    stop.store(true, Ordering::SeqCst);
  }

  // pause the audiograph, clearing all start times
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
  println!("playing playlist");

  // a pattern playing on its own stops first
  if state.playlist.playing_pattern().is_some() { end_run(state); }
  *state.playlist.mode.lock().unwrap() = state::PlayMode::Playlist;

  // a playhead past the loop's end starts back at the top, so the run (and
  // the frontend cursor) stays inside the loop. without looping, playing
  // from the end of the song starts it again
  let end = match state.playlist.loop_len(beat_interval(state)) {
    Some(len) => Some(len),
    None => song_end(state),
  };
  if let Some(end) = end {
    let mut start = state.playlist.start_offset.lock().unwrap();
    if *start >= end { *start = Duration::ZERO; }
  }

  start_run(state);
}

// start a run of whatever the mode is, and wait until the output stream is
// open and the start time is set, so a `get_playlist_playing` right after
// this sees the new run
fn start_run(state: &Arc<state::InnerState>) {
  let (ready_tx, ready_rx) = mpsc::channel();
  run_playlist(state, ready_tx);

  match ready_rx.recv() {
    Ok(Ok(())) => {}
    Ok(Err(e)) => println!("could not start playback: {}", e),
    Err(_) => println!("could not start playback: playback thread exited"),
  }
}

// loop a pattern on its own from its start (Play in the Sequencer tab),
// stopping whatever was playing. the playlist's playhead stays where it is
pub fn start_pattern(state: &Arc<state::InnerState>, id: u64) -> Result<(), String> {
  println!("playing pattern {}", id);
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
  *state.playlist.mode.lock().unwrap() = state::PlayMode::Pattern(id);
  start_run(state);
  Ok(())
}

// move the playhead to a beat, kept within the playlist. while playing,
// playback carries on from there. returns the beat it moved to
pub fn seek_playlist(state: &Arc<state::InnerState>, beat: f64) -> f64 {
  let interval = beat_interval(state);
  let end = state.playlist.ui.lock().unwrap().max_beats_displayed as f64;
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

pub fn set_playlist_tempo(
  state: tauri::State<'_, Arc<state::InnerState>>,
  val: f32,
) {
  state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .set_tempo(val);
}

pub fn get_playlist_track_count(
  state: &tauri::State<'_, Arc<state::InnerState>>,
) -> u32 {
  let audiograph = state
    .playlist
    .audiograph
    .lock()
    .unwrap();
  let count: u32 = audiograph
    .track_numbers()
    .len()
    .try_into()
    .unwrap();
  
  if count > 1 { count } else { state::defaults::NUM_OF_TRACKS }
}
