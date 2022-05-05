use rodio::Source;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync;
use std::sync::atomic;
use std::thread;
use std::time;
use tauri;

use crate::daw::{daw_core, state};

#[cfg(target_os = "linux")]
use psimple;
#[cfg(target_os = "linux")]
use pulse;

use super::audiograph;

/**
Play a single sound sample for its entire length
- Compiled only on Linux systems,
for use with Alsa, Jack, and PulseAudio drivers
*/
#[cfg(target_os = "linux")]
pub async fn play_sample(path: &str) {
  let path = String::from(path);
  thread::spawn(move || {
    // let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let file = BufReader::new(File::open(path).unwrap());
    let source = Decoder::new(file).unwrap();

    let spec = pulse::sample::Spec {
      format: pulse::sample::Format::S16NE,
      channels: 2,
      rate: 44_100,
    };

    let sink = psimple::Simple::new(
      None,
      "dawesome",
      pulse::stream::Direction::Playback,
      None,
      "dawesome output",
      &spec,
      None,
      None,
    )
    .unwrap();

    let raw_source: rodio::source::SamplesConverter<_, i16> =
      source.convert_samples();
    let raw_vec: std::vec::Vec<i16> = raw_source.collect();

    unsafe {
      let raw_slice = raw_vec.align_to::<u8>().1;
      sink.write(raw_slice).unwrap();
      sink.drain().unwrap();
    }
  });
}

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

pub fn play_metronome(state_ref: &sync::Arc<state::InnerState>) {
  let state = state_ref.clone();

  if state.playlist_current_beat.load(atomic::Ordering::SeqCst) > 0 {
    // default metronome tick
    futures::executor::block_on(play_sample("assets/assets_66-hh-01-or.wav"));
  } else {
    // play accented metronome tick
    futures::executor::block_on(play_sample("assets/assets_66-hh-01-or-2.wav"));
  }
}

pub fn run_playlist(state_ref: &sync::Arc<state::InnerState>) {
  let state = state_ref.clone();
  let pool = daw_core::threadpool::ThreadPool::new(4);
  let tempo = *state_ref.global_tempo_bpm.lock().unwrap();
  let tempo_intrv_ms = daw_core::timing::tempo_to_intrv_ms(tempo);

  pool.exec(move || {
    thread::spawn(move || loop {
      println!("tick: {}ms", &state.playlist_audiograph.lock().unwrap().current_offset);

      // play metronome if enabled
      if state.metronome_enabled.load(atomic::Ordering::SeqCst) && state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
        play_metronome(&state);
      }

      // run ahead n milliseconds and schedule the next
      // samples in the audio graph to be played
      let mut audiograph_ref = state.playlist_audiograph.lock().unwrap();
      audiograph_ref.run_for(tempo_intrv_ms);
      let curr = audiograph_ref.current_offset;

      if state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
        audiograph_ref.set_current_offset(curr + tempo_intrv_ms);
      } else {
        audiograph_ref.set_current_offset(curr);
      }

      // sleep this thread for the length of a single beat
      thread::sleep(time::Duration::from_millis(tempo_intrv_ms));

      // increment the beat counter
      let current_time_signature =
        state.playlist_time_signature.lock().unwrap();
      let current_beat =
        state.playlist_current_beat.load(atomic::Ordering::SeqCst);
      let next_beat = (current_beat + 1) % current_time_signature.numerator;
      
      if state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
        state
          .playlist_current_beat
          .store(next_beat, atomic::Ordering::SeqCst);
        state
          .playlist_total_beats
          .fetch_add(1, atomic::Ordering::SeqCst);
      } else {
        state.playlist_current_beat.store(0, atomic::Ordering::SeqCst);
      }

      println!(
        "current beat: {}, total beats played: {}",
        state.playlist_current_beat.load(atomic::Ordering::SeqCst),
        state.playlist_total_beats.load(atomic::Ordering::SeqCst)
      );

      if !state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
        break;
      }
    });
  });

  println!("continuing");
}

pub fn pause_playlist(state: tauri::State<'_, sync::Arc<state::InnerState>>) {
  println!("pausing playlist");

  // pause the audiograph, clearing all start times
  let mut audiograph_ref = state.playlist_audiograph.lock().unwrap();
  audiograph_ref.pause();

  // reset playlist to 0 beats
  state
    .playlist_current_beat
    .store(0, atomic::Ordering::SeqCst);
  state
    .playlist_total_beats
    .store(0, atomic::Ordering::SeqCst);

  // set playlist state to paused
  state
    .playlist_is_playing
    .store(false, atomic::Ordering::SeqCst);
}

pub fn start_playlist(state: tauri::State<'_, sync::Arc<state::InnerState>>) {
  // start playlist
  println!("playing playlist");
  state
    .playlist_is_playing
    .store(true, atomic::Ordering::SeqCst);

  // set playlist start time
  let now = chrono::offset::Utc::now();
  let timestamp = now.naive_utc().timestamp();
  state
    .playlist_started_time
    .store(timestamp, atomic::Ordering::SeqCst);

  // start audio graph
  state
    .playlist_audiograph
    .lock()
    .unwrap()
    .init(timestamp.try_into().unwrap());

  let state_ref = state.inner();
  run_playlist(state_ref);
}

pub fn set_playlist_tempo(
  state: tauri::State<'_, sync::Arc<state::InnerState>>,
  val: f32,
) {
  let old_tempo = *state.global_tempo_bpm.lock().unwrap();
  state.playlist_audiograph.lock().unwrap().fit_nodes_to_tempo(val, old_tempo);
  *state.global_tempo_bpm.lock().unwrap() = val;
}
