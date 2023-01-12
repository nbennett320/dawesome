use std::convert::{AsRef};
use std::sync::{Arc};
use std::io::{self, Read};
use std::fs::{File};
use std::vec::{Vec};
use std::time::{Duration};
use rodio::{Decoder, Source, Sample};
use rodio::source::{Buffered, SamplesConverter};

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

trait SizedSample: rodio::cpal::Sample + Sized {
  fn size_hint();
}

#[derive(Clone)]
pub struct SoundSource {
  buffer: SampleBuffer,
}

// impl Sample for SoundSource {
//   fn amplify(self, value: f32) -> Self {
    
//   }

//   fn lerp(
//     first: Self, 
//     second: Self, 
//     numerator: u32, 
//     denominator: u32
//   ) -> Self {
    
//   }

//   fn saturating_add(self, other: Self) -> Self {
    
//   }

//   fn zero_value() -> Self {
    
//   }
// }

impl SoundSource {
  pub fn decoder(&self) -> Decoder<io::Cursor<SampleBuffer>> {
    self.decoder()
  }
}
