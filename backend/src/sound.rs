use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

pub struct InnerState {
  sink: Sink
}

pub fn init() -> InnerState {
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();
  let sink = Sink::try_new(&stream_handle).unwrap();
  let file = BufReader::new(File::open("assets/test.mp3").unwrap());
  let source = Decoder::new(file).unwrap();
  sink.append(source);
  sink.pause();


  InnerState {
    sink
  }
}
