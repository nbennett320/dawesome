use std::io;
use std::sync::Arc;
use rodio::{Decoder, Source};
use rodio::source::SamplesConverter;

#[derive(Clone)]
pub struct SampleBuffer (Arc<Vec<u8>>);

impl AsRef<[u8]> for SampleBuffer {
  fn as_ref(&self) -> &[u8] {
    &self.0
  }
}

impl SampleBuffer {
  // (only the tests read a file this way: sample rows decode theirs once,
  // see sample_voice.rs)
  #[cfg(test)]
  pub fn load(path: &str) -> io::Result<SampleBuffer> {
    Ok(SampleBuffer(Arc::new(std::fs::read(path)?)))
  }

  pub fn from_bytes(bytes: &[u8]) -> SampleBuffer {
    SampleBuffer(Arc::new(bytes.to_vec()))
  }

  pub fn cursor(&self) -> io::Cursor<SampleBuffer> {
    io::Cursor::new(SampleBuffer(self.0.clone()))
  }

  pub fn decoder(&self) -> Decoder<io::Cursor<SampleBuffer>> {
    Decoder::new(self.cursor()).unwrap()
  }

  pub fn convert_samples(&self) -> SamplesConverter<Decoder<io::Cursor<SampleBuffer>>, f32> {
    self.decoder().convert_samples()
  }
}
