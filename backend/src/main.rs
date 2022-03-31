use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use tauri;

mod timer;

struct InnerState {
  sink: rodio::Sink,
}

#[tauri::command]
fn toggle_play(state: tauri::State<'_, InnerState>) {
  if state.sink.is_paused() {
    state.sink.play();
  } else {
    state.sink.pause();
  }
}

#[tauri::command]
fn get_paused(state: tauri::State<'_, InnerState>) -> Result<bool, String> {
  Ok(
    state.sink.is_paused()
  )
}

fn main() {
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();
  let sink = Sink::try_new(&stream_handle).unwrap();
  let file = BufReader::new(File::open("assets/test.mp3").unwrap());
  let source = Decoder::new(file).unwrap();
  sink.append(source);
  sink.pause();

  

  
  tauri::Builder::default()
    .manage(InnerState {
      sink
    })
    .invoke_handler(tauri::generate_handler![
      toggle_play,
      get_paused,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
