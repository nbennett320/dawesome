use crate::daw::{
  daw_core::{
    self,
    SampleBuffer,
  }, 
  state, 
  MusicalTiming,
};
use std::fs::File;
use std::io::BufReader;
use std::sync::{
  Arc, 
  Mutex,
};
use std::sync::atomic::{Ordering};
use std::thread;
use std::time::{
  Duration, 
  Instant,
};
use rodio::{Source};
use rodio::{
  Decoder, 
  OutputStream, 
  Sink,
};
use tauri;

#[cfg(target_os = "linux")]
use psimple;
#[cfg(target_os = "linux")]
use pulse;

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

#[cfg(not(target_os = "linux"))]
pub async fn play_buffer<S>(
  buf: S,
) 
  where
    S: Source<Item = f32> + Send + 'static,
{
  thread::spawn(move || {
    // read and decode audio file, and append to a sound sink
    let dur = buf.total_duration().unwrap();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    
    println!("System time (M): {:?}", std::time::Instant::now());
    stream_handle.play_raw(buf).unwrap();
    thread::sleep(dur);
  });
}

pub fn play_metronome(state_ref: &Arc<state::InnerState>) {
  let state = state_ref.clone();

  if state.playlist.current_beat.load(Ordering::SeqCst) > 0 {
    // default metronome tick
    let buf = METRONOME_TICK_SOURCE.convert_samples();
    futures::executor::block_on(play_buffer(buf));
  } else {
    // play accented metronome tick
    let buf = METRONOME_TICK_ACCENTED_SOURCE.convert_samples();
    futures::executor::block_on(play_buffer(buf));
  }
}

pub fn run_playlist(state_ref: &Arc<state::InnerState>) {
  let state = state_ref.clone();
  let pool = daw_core::threadpool::ThreadPool::new(8);
  let tempo_interval = state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .interval_of_subdivision(daw_core::timing::WholeNote::new());

  pool.exec(move || {
    thread::spawn(move || loop {
      println!("tick: {}ms", &state.playlist.audiograph.lock().unwrap().current_offset.unwrap().as_millis());

      // play metronome if enabled
      // if state.metronome_enabled.load(Ordering::SeqCst) 
      //   && state.playlist.playing() {
      //   play_metronome(&state);
      // }

      // run ahead n milliseconds and schedule the next
      // samples in the audio graph to be played
      let mut audiograph_ref = state.playlist.audiograph.lock().unwrap();

      audiograph_ref.run_for(tempo_interval);
      // let m = audiograph_ref.buffer_for(tempo_interval);
      // match m {
      //   Some(x) => { 
      //     futures::executor::block_on(play_buffer(x.take_duration(tempo_interval).buffered()));
      //   }
      //   None => {}
      // }

      // sleep this thread for the length of a single beat
      thread::sleep(tempo_interval);
      
      let curr = audiograph_ref.current_offset.unwrap();

      if state.playlist.playing() {
        audiograph_ref.set_current_offset(Some(curr + tempo_interval));
      } else {
        audiograph_ref.set_current_offset(Some(curr));
      }

      // increment the beat counter
      let current_time_signature =
        state.playlist.time_signature.lock().unwrap();
      let current_beat =
        state.playlist.current_beat.load(Ordering::SeqCst);
      let next_beat = (current_beat + 1) % current_time_signature.numerator;
      
      if state.playlist.playing() {
        // loop the playlist by setting the current offset to zero,
        // if looping is enabled, otherwise, add and store the next 
        // current beat
        if state.playlist.loop_enabled.load(Ordering::SeqCst)
          && ((state.playlist.total_beats.load(Ordering::SeqCst) + 1) as u64 
          >= state.playlist.max_beats.load(Ordering::SeqCst)) {
          audiograph_ref.set_current_offset(Some(Duration::ZERO));

          state
            .playlist
            .current_beat
            .store(next_beat, Ordering::SeqCst);
          state
            .playlist
            .total_beats
            .store(0, Ordering::SeqCst);
        } else {
          state
            .playlist.current_beat
            .store(next_beat, Ordering::SeqCst);
          state
            .playlist.total_beats
            .fetch_add(1, Ordering::SeqCst);
        }
      } else {
        state.playlist.current_beat.store(0, Ordering::SeqCst);
      }

      println!(
        "current beat: {}, total beats played: {}, looping: {}",
        state.playlist.current_beat.load(Ordering::SeqCst),
        state.playlist.total_beats.load(Ordering::SeqCst),
        state.playlist.loop_enabled.load(Ordering::SeqCst)
      );

      if !state.playlist.playing() {
        break;
      }
    });
  });

  println!("continuing");
}

pub fn pause_playlist(state: tauri::State<'_, Arc<state::InnerState>>) {
  println!("pausing playlist");
  
  // reset playlist to 0 beats
  state
    .playlist
    .current_beat
    .store(0, Ordering::SeqCst);
  state
    .playlist
    .total_beats
    .store(0, Ordering::SeqCst);

  // set playlist state to paused
  state
    .playlist
    .set_playing(false);

  // pause the audiograph, clearing all start times
  let mut audiograph_ref = state.playlist.audiograph.lock().unwrap();
  audiograph_ref.pause();
}

pub fn start_playlist(state: tauri::State<'_, Arc<state::InnerState>>) {
  // start playlist
  println!("playing playlist");
  state
    .playlist
    .set_playing(true);

  // set playlist start time
  let now = Instant::now();
  *state
    .playlist
    .started_time
    .lock()
    .unwrap() = Some(now);

  // start audio graph
  state
    .playlist
    .audiograph
    .lock()
    .unwrap()
    .init(now);

  let state_ref = state.inner();
  run_playlist(state_ref);
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
