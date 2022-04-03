use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic;
use std::sync;
use std::thread;
use std::time;
use futures;
use chrono;
use tauri;

mod timer;
mod threadpool;

struct InnerState {
  global_tempo_bpm: atomic::AtomicU32,
  playlist_is_playing: atomic::AtomicBool,
  playlist_started_time: atomic::AtomicI64,
  metronome_enabled: atomic::AtomicBool,
}

impl InnerState {
  pub fn new() -> Self {
    InnerState {
      playlist_is_playing: atomic::AtomicBool::from(false),
      global_tempo_bpm: atomic::AtomicU32::from(60),
      playlist_started_time: atomic::AtomicI64::from(0),
      metronome_enabled: atomic::AtomicBool::from(true),
    }
  }
}

async fn play_sample(path: &str) {
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

fn run_metronome(state_ref: &sync::Arc<InnerState>, tempo: u32) {
  println!("before spawn");
  let state = state_ref.clone();
  let pool = threadpool::ThreadPool::new(4);

  pool.exec(move || {
    thread::spawn(move || {
      loop {
        // let timer = timer::Timer::new(); 
        // timer.timeout(time::Duration::new((tempo / 60) as u64, 0));
        println!("tick");
        futures::executor::block_on(play_sample("assets/assets_66-hh-01-or.wav"));
        thread::sleep(time::Duration::new((tempo / 60) as u64, 0));
        if !state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
          break;
        }
      }
    });
  });

  println!("continuing");
}

#[tauri::command]
fn toggle_playlist(state: tauri::State<'_, sync::Arc<InnerState>>) {
  if state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
    println!("pausing playlist");
    state.playlist_is_playing.store(false, atomic::Ordering::SeqCst);
  } else {
    // start playlist 
    println!("playing playlist");
    state.playlist_is_playing.store(true, atomic::Ordering::SeqCst);
   
    // set playlist start time 
    let now = chrono::offset::Utc::now();
    let timestamp = now.naive_utc().timestamp();
    state.playlist_started_time.store(timestamp, atomic::Ordering::SeqCst);
   
    let tempo = state.global_tempo_bpm.load(atomic::Ordering::SeqCst);
    
    // toggle metronome if enabled 
    if state.metronome_enabled.load(atomic::Ordering::SeqCst) {
      let state_ref = state.inner();
      run_metronome(state_ref, tempo);
    }
  }
}

#[tauri::command]
fn get_playlist_playing(state: tauri::State<'_, sync::Arc<InnerState>>) -> Result<bool, String> {
  Ok(
    state.playlist_is_playing.load(atomic::Ordering::SeqCst)
  )
}

#[tauri::command]
fn get_playlist_start_time(state: tauri::State<'_, sync::Arc<InnerState>>) -> Result<i64, String> {
  Ok(
    state.playlist_started_time.load(atomic::Ordering::SeqCst)
  )
}

fn main() {
  tauri::Builder::default()
    .manage(sync::Arc::new(InnerState::new()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      toggle_playlist,
      get_playlist_start_time
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
