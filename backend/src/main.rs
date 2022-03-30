use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicBool, Ordering};
use std::boxed::Box;
use std::sync::mpsc;
// use std::sync::Mutex;
use tauri::State;
// use tauri::async_runtime::RwLock;
use std::fs::File;
use std::io::BufReader;
// use std::collections::HashMap;
// use std::vec::Vec;
use rodio::{Decoder, OutputStream, Sink};
// use rodio::source::{Source};

// mod state;
// mod sound;

struct InnerState {
  counter: AtomicI32,
  sound_threads: AtomicPtr<Result<Sink, mpsc::RecvError>>,
  is_playing: AtomicBool
}

// fn init_sink (
//   state: State<InnerState>, 
//   delta: i32
// ) -> Result<CounterResponse, String> {

//   if !state.is_playing.load(Ordering::SeqCst) {

//   let (_stream, stream_handle) = OutputStream::try_default().unwrap();
//   let sink = Sink::try_new(&stream_handle).unwrap();
//   // let sink_ptr: Box<Sink> = Box::new(sink);
  
//   let file = BufReader::new(File::open("assets/test.mp3").unwrap());
//   let source = Decoder::new(file).unwrap();
//   sink.append(source);

//   state.sound_threads.store(&mut sink, Ordering::SeqCst);

//   Ok(
//     CounterResponse {
//       data: 0
//     }
//   )
      
// }

#[tauri::command]
fn toggle_play_sound (
  state: State<InnerState>, 
  delta: i32
) -> Result<CounterResponse, String> {

  
  if !state.is_playing.load(Ordering::SeqCst) {
    let (tx, rx) = mpsc::channel();
    
    // create thread for sound handle
    std::thread::spawn(move || {
      println!("playing sound");
      let (_stream, stream_handle) = OutputStream::try_default().unwrap();
      let sink = Sink::try_new(&stream_handle).unwrap();
      // let sink_ptr: Box<Sink> = Box::new(sink);
      
      // read sound and attach it to a sink
      println!("reading sound file");
      let file = BufReader::new(File::open("assets/test.mp3").unwrap());
      let source = Decoder::new(file).unwrap();
      sink.append(source);
      
      // toggle play/pause
      sink.play();
      // state.is_playing.store(true, Ordering::SeqCst);
      println!("is paused: {}", sink.is_paused());
      if sink.is_paused() {
        sink.play();
      } else {
        // sink.pause();
      }

      // add handle to state
      // state.sound_handles.store(sink_ptr.as_mut(), Ordering::SeqCst);

      // stream_handle.play_raw(source.convert_samples());
      sink.sleep_until_end();
      tx.send(sink).unwrap();
    });

    let mut recieved_sink = rx.recv();
    // recieved_sink.pause();
    state.is_playing.store(true, Ordering::SeqCst);
    state.sound_threads.store(&mut recieved_sink, Ordering::SeqCst);
    println!("playing: {}", state.is_playing.load(Ordering::SeqCst));
  } else {
    println!("not playing: {}", state.is_playing.load(Ordering::SeqCst));
  }

  // state.sound_threads.store(&mut sink_thread, Ordering::SeqCst);
  //std::thread::sleep(std::time::Duration::from_secs(5));
  Ok(
    CounterResponse {
      data: 0,
    }
  )
}

#[derive(serde::Serialize)]
struct CounterResponse {
  data: i32
}

#[tauri::command]
fn increment_counter (
  state: State<'_, InnerState>, 
  delta: i32
) -> Result<CounterResponse, String> {
  println!("incrementing counter by {}", delta);

  Ok(
    CounterResponse {
      data: state.counter.fetch_add(delta, Ordering::SeqCst) + delta,
    }
  )
  // Ok(state.fetch_add(delta, Ordering::SeqCst) + delta)
}

#[tauri::command]
fn get_counter (
  state: State<'_, InnerState>, 
) -> Result<CounterResponse, String> {

  println!("Getting counter value: {}", state.counter.load(Ordering::SeqCst));

  Ok(
    CounterResponse {
      data: state.counter.load(Ordering::SeqCst)
    }
  )
}

fn main() {
  tauri::Builder::default()
    .manage(InnerState {
      counter: AtomicI32::from(0),
      sound_threads: AtomicPtr::default(),
      is_playing: AtomicBool::from(false),
    })
    .invoke_handler(tauri::generate_handler![
      increment_counter,
      get_counter,
      toggle_play_sound 
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
