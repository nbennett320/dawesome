use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync;
use std::sync::atomic;
use std::thread;
use std::time;

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

pub fn run_metronome(state_ref: &sync::Arc<state::InnerState>) {
  println!("before spawn");
  let state = state_ref.clone();
  let pool = daw_core::threadpool::ThreadPool::new(4);
  let tempo = *state_ref.global_tempo_bpm.lock().unwrap();
  let tempo_intrv_ms = daw_core::timing::tempo_to_intrv_ms(tempo);

  pool.exec(move || {
    thread::spawn(move || {
      loop {
        // let timer = timer::Timer::new();
        // timer.timeout(time::Duration::new((tempo / 60) as u64, 0));

        println!("tick");
        futures::executor::block_on(play_sample("assets/assets_66-hh-01-or.wav"));
        thread::sleep(time::Duration::from_millis(tempo_intrv_ms));
        if !state.playlist_is_playing.load(atomic::Ordering::SeqCst) {
          break;
        }
      }
    });
  });

  println!("continuing");
}
