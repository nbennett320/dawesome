use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync;
use std::sync::atomic;
use std::thread;
use std::time;
use tauri;

use crate::daw::{daw_core, state};

pub async fn play_sample(path: &str) {
  let path = String::from(path);
  thread::spawn(move || {
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
  println!("before spawn");
  let state = state_ref.clone();

  if state.playlist_current_beat.load(atomic::Ordering::SeqCst) > 0 {
    // default metronome tick
    futures::executor::block_on(play_sample(
      "assets/assets_66-hh-01-or.wav",
    ));
  } else {
    // play accented metronome tick
    futures::executor::block_on(play_sample(
      "assets/assets_66-hh-01-or-2.wav",
    ));
  }
}

pub fn run_playlist(state_ref: &sync::Arc<state::InnerState>) {
  println!("before spawn");
  let state = state_ref.clone();
  let pool = daw_core::threadpool::ThreadPool::new(4);
  let tempo = *state_ref.global_tempo_bpm.lock().unwrap();
  let tempo_intrv_ms = daw_core::timing::tempo_to_intrv_ms(tempo);

  pool.exec(move || {
    thread::spawn(move || {
      loop {
        println!("tick");

        if state.metronome_enabled.load(atomic::Ordering::SeqCst) {
          play_metronome(&state);
        }

        thread::sleep(time::Duration::from_millis(tempo_intrv_ms));
        
        let current_time_signature = state.playlist_time_signature.lock().unwrap();
        let current_beat = state.playlist_current_beat.load(atomic::Ordering::SeqCst);
        let next_beat = (current_beat + 1) % current_time_signature.numerator;
        state.playlist_current_beat.store(next_beat, atomic::Ordering::SeqCst);

        state.playlist_total_beats.fetch_add(1, atomic::Ordering::SeqCst);
        println!("current beat: {}, total beats played: {}",state.playlist_current_beat.load(atomic::Ordering::SeqCst), state.playlist_total_beats.load(atomic::Ordering::SeqCst));
        
        if !state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
          break;
        }
      }
    });
  });

  println!("continuing");
}

pub fn pause_playlist(state: tauri::State<'_, sync::Arc<state::InnerState>>) {
  println!("pausing playlist");
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

  // toggle metronome if enabled
  if state.metronome_enabled.load(atomic::Ordering::SeqCst) {
    let state_ref = state.inner();
    run_playlist(state_ref);
  }
}
