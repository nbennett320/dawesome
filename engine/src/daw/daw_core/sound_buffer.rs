use std::fs::File;
use std::io::{self, Read};
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
  pub fn load(path: &str) -> io::Result<SampleBuffer> {
    let mut buf = Vec::new();
    let mut file = File::open(path)?;

    file.read_to_end(&mut buf)?;

    Ok(SampleBuffer(Arc::new(buf)))
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
