use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::time;
use futures;
use std::sync::atomic;
use std::sync;
use std::thread;
use tauri;

mod timer;
mod threadpool;

// #[derive(Copy, Clone)]
struct InnerState {
  global_tempo_bpm: atomic::AtomicU32,
  playlist_is_playing: atomic::AtomicBool,
  metronome_enabled: atomic::AtomicBool,
}

impl InnerState {
  pub fn new() -> Self {
    InnerState {
      playlist_is_playing: atomic::AtomicBool::from(false),
      global_tempo_bpm: atomic::AtomicU32::from(60),
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
  // let state = state.as_ref();
  if state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
    println!("pausing playlist");
    state.playlist_is_playing.store(false, atomic::Ordering::SeqCst);
  } else {
    // start playlist 
    state.playlist_is_playing.store(true, atomic::Ordering::SeqCst);
    println!("playing playlist");
    let tempo = state.global_tempo_bpm.load(atomic::Ordering::SeqCst);
   
    // toggle metronome if enabled 
    if state.metronome_enabled.load(atomic::Ordering::SeqCst) {
      let state_ref = state.inner();
      run_metronome(state_ref, tempo);
    }
  }
}

#[tauri::command]
fn get_paused(state: tauri::State<'_, sync::Arc<InnerState>>) -> Result<bool, String> {
  Ok(
    state.playlist_is_playing.load(atomic::Ordering::SeqCst)
  )
}

fn main() {
  tauri::Builder::default()
    .manage(sync::Arc::new(InnerState::new()))
    .invoke_handler(tauri::generate_handler![
      get_paused,
      toggle_playlist,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
