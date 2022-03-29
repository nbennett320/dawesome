
use std::sync::atomic::{AtomicI32, Ordering};
use tauri::State;
use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, source::Source};

#[tauri::command]
fn play_sound(state: State<AtomicI32>, delta: i32) -> Result<i32, String> {
  println!("playing sound");
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();

  println!("reading sound file");
  let file = BufReader::new(File::open("assets/test.mp3").unwrap());
  let source = Decoder::new(file).unwrap();

  println!("playing raw");
  stream_handle.play_raw(source.convert_samples());

  std::thread::sleep(std::time::Duration::from_secs(5));
  Ok(state.fetch_add(delta, Ordering::SeqCst) + delta)
}

#[tauri::command]
fn increment_counter(state: State<AtomicI32>, delta: i32) -> Result<i32, String> {
  println!("incrementing counter by {}", delta);
  Ok(state.fetch_add(delta, Ordering::SeqCst) + delta)
}

#[tauri::command]
fn get_counter(state: State<AtomicI32>) -> Result<i32, String> {
  println!("Getting counter value");
  Ok(state.load(Ordering::SeqCst))
}

fn main() {
  tauri::Builder::default()
    .manage(AtomicI32::from(0))
    .invoke_handler(tauri::generate_handler![
      increment_counter,
      get_counter,
      play_sound
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

