use std::convert::{AsRef};
use std::sync::{Arc};
use std::io::{self, Read};
use std::fs::{File};
use std::vec::{Vec};
use std::time::{Duration};
use rodio::{Decoder, Source};
use rodio::source::{Buffered, SamplesConverter};

#[derive(Clone)]
pub struct SoundBuffer (Arc<Vec<u8>>);

impl AsRef<[u8]> for SoundBuffer {
  fn as_ref(&self) -> &[u8] {
    &self.0
  }
}

impl SoundBuffer {
  pub fn load(path: &str) -> io::Result<SoundBuffer> {
    let mut buf = Vec::new();
    let mut file = File::open(path)?;
    
    file.read_to_end(&mut buf)?;
    
    Ok(SoundBuffer(Arc::new(buf)))
  }

  pub fn cursor(&self) -> io::Cursor<SoundBuffer> {
    io::Cursor::new(SoundBuffer(self.0.clone()))
  }

  pub fn decoder(&self) -> Decoder<io::Cursor<SoundBuffer>> {
    Decoder::new(self.cursor()).unwrap()
  }
}

impl Iterator for SoundBuffer {
  fn next(&mut self) -> Option<Self::Item> {
    self.next()
  }
}

impl Source for SoundBuffer {
  fn amplify(self, value: f32) -> rodio::source::Amplify<Self>
    where
      Self: Sized, {
    
  }

  fn buffered(self) -> Buffered<Self>
    where
      Self: Sized, {
    
  }

  fn current_frame_len(&self) -> Option<usize> {
    
  }

  fn channels(&self) -> u16 {
    
  }

  fn convert_samples<D>(self) -> SamplesConverter<Self, D>
    where
        Self: Sized,
        D: rodio::Sample, {
    
  }

  fn delay(self, duration: Duration) -> rodio::source::Delay<Self>
    where
        Self: Sized, {
    
  }
}