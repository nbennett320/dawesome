// a mixdown's file: WAV or MP3, written as the mix is rendered
//
// written beside its place under another name first, so a file that's half
// written (or cancelled) never has the name. the finished file is given its
// name by a link, which fails if the name's taken, and then the next free
// one is tried: "song.wav", "song 2.wav", ... nothing is ever written over
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use mp3lame_encoder::{Bitrate, Builder, Encoder, FlushNoGap, InterleavedPcm, Quality};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Bits {
  Int16,
  Int24,
  Float32,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Format {
  Wav { bits: Bits },
  Mp3 { kbps: u32 },
}

impl Format {
  fn extension(&self) -> &'static str {
    match self {
      Format::Wav { .. } => "wav",
      Format::Mp3 { .. } => "mp3",
    }
  }

  // the rate to render at for this format, on an engine at `rate`. MP3
  // goes no higher than 48kHz: above it, the song is rendered at 44.1 or
  // 48kHz (whichever the rate is a multiple of) rather than resampled after
  pub fn render_rate(&self, rate: u32) -> u32 {
    match self {
      Format::Mp3 { .. } if rate > 48_000 => if rate % 44_100 == 0 { 44_100 } else { 48_000 },
      _ => rate,
    }
  }
}

pub const NEEDS_NAME: &str = "Give the mixdown a name";
// the longest name kept, in characters (well inside what a file system takes)
const MAX_NAME: usize = 200;

// a name fit for a file: nothing that could be a folder or isn't allowed in
// a file name, no leading dots (a hidden file), and without the format's
// extension if it was typed (it's added back)
pub fn file_stem(name: &str, format: Format) -> Result<String, String> {
  let ext = format.extension();
  let mut name = name.trim();
  // (compared byte for byte: lower-casing can change a name's length)
  let dotted = ext.len() + 1;
  if name.len() >= dotted && name.is_char_boundary(name.len() - dotted) {
    let (stem, end) = name.split_at(name.len() - dotted);
    if end.starts_with('.') && end[1..].eq_ignore_ascii_case(ext) { name = stem.trim_end(); }
  }

  let cleaned: String = name.chars()
    .map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '-' } else { c })
    .take(MAX_NAME)
    .collect();
  let cleaned = cleaned.trim().trim_start_matches('.').trim().to_string();
  if cleaned.is_empty() { Err(NEEDS_NAME.into()) } else { Ok(cleaned) }
}

enum Output {
  Wav { writer: hound::WavWriter<BufWriter<File>>, bits: Bits, seed: u32 },
  Mp3 { encoder: Box<Encoder>, file: BufWriter<File>, buffer: Vec<u8> },
}

pub struct MixdownFile {
  part: PathBuf,
  dir: PathBuf,
  stem: String,
  ext: &'static str,
  // None once finished
  output: Option<Output>,
}

impl MixdownFile {
  // start the file `name` in `dir` (made if it isn't there), stereo at `rate`
  pub fn create(dir: &Path, name: &str, format: Format, rate: u32) -> Result<MixdownFile, String> {
    let stem = file_stem(name, format)?;
    let ext = format.extension();
    std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't make the folder {}: {}", dir.display(), e))?;
    let part = dir.join(format!(".{}.{}.part", stem, ext));
    let file = File::create(&part).map_err(|e| format!("Couldn't write to {}: {}", dir.display(), e))?;
    let file = BufWriter::new(file);

    let output = match format {
      Format::Wav { bits } => {
        let (bits_per_sample, sample_format) = match bits {
          Bits::Int16 => (16, hound::SampleFormat::Int),
          Bits::Int24 => (24, hound::SampleFormat::Int),
          Bits::Float32 => (32, hound::SampleFormat::Float),
        };
        let spec = hound::WavSpec { channels: 2, sample_rate: rate, bits_per_sample, sample_format };
        let writer = hound::WavWriter::new(file, spec).map_err(|e| e.to_string());
        Output::Wav { writer: writer.map_err(|e| { let _ = std::fs::remove_file(&part); e })?, bits, seed: 0x9e37_79b9 }
      }
      Format::Mp3 { kbps } => match mp3_encoder(kbps, rate) {
        Ok(encoder) => Output::Mp3 { encoder: Box::new(encoder), file, buffer: Vec::new() },
        Err(e) => {
          let _ = std::fs::remove_file(&part);
          return Err(e);
        }
      },
    };

    Ok(MixdownFile { part, dir: dir.to_path_buf(), stem, ext, output: Some(output) })
  }

  // add stereo samples, interleaved
  pub fn write(&mut self, samples: &[f32]) -> Result<(), String> {
    match self.output.as_mut().ok_or("the file is finished")? {
      Output::Wav { writer, bits, seed } => {
        let result = match bits {
          Bits::Int16 => samples.iter().try_for_each(|&s| writer.write_sample(dithered_16(s, seed))),
          Bits::Int24 => samples.iter().try_for_each(|&s| writer.write_sample((s.clamp(-1., 1.) * 8_388_607.).round() as i32)),
          Bits::Float32 => samples.iter().try_for_each(|&s| writer.write_sample(s)),
        };
        result.map_err(|e| e.to_string())
      }
      Output::Mp3 { encoder, file, buffer } => {
        buffer.clear();
        buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples.len() / 2));
        encoder.encode_to_vec(InterleavedPcm(samples), buffer).map_err(|e| format!("MP3: {}", e))?;
        file.write_all(buffer).map_err(|e| e.to_string())
      }
    }
  }

  // finish the file and give it its name: the first of "stem.ext",
  // "stem 2.ext", ... that's free. returns where it is
  pub fn finish(mut self) -> Result<PathBuf, String> {
    match self.output.take().ok_or("the file is finished")? {
      Output::Wav { writer, .. } => writer.finalize().map_err(|e| e.to_string())?,
      Output::Mp3 { mut encoder, mut file, mut buffer } => {
        buffer.clear();
        buffer.reserve(7_200);
        encoder.flush_to_vec::<FlushNoGap>(&mut buffer).map_err(|e| format!("MP3: {}", e))?;
        file.write_all(&buffer).and_then(|_| file.flush()).map_err(|e| e.to_string())?;
      }
    }

    for n in 1..10_000u32 {
      let name = if n == 1 { format!("{}.{}", self.stem, self.ext) } else { format!("{} {}.{}", self.stem, n, self.ext) };
      let path = self.dir.join(name);
      match std::fs::hard_link(&self.part, &path) {
        Ok(()) => return Ok(path),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
        Err(e) => return Err(format!("Couldn't name the file {}: {}", path.display(), e)),
      }
    }
    Err(format!("There are too many files named {} in {}", self.stem, self.dir.display()))
  }
}

impl Drop for MixdownFile {
  // the part-written file never stays: once finished, it has its name by a
  // link, and when not (cancelled, or failed) it's not wanted
  fn drop(&mut self) {
    self.output = None;
    let _ = std::fs::remove_file(&self.part);
  }
}

fn mp3_encoder(kbps: u32, rate: u32) -> Result<Encoder, String> {
  let bitrate = match kbps {
    128 => Bitrate::Kbps128,
    192 => Bitrate::Kbps192,
    256 => Bitrate::Kbps256,
    320 => Bitrate::Kbps320,
    _ => return Err(format!("MP3s are made at 128, 192, 256 or 320 kbps, not {}", kbps)),
  };
  let error = |e: mp3lame_encoder::BuildError| format!("MP3 at {}Hz: {}", rate, e);
  let mut builder = Builder::new().ok_or("the MP3 encoder couldn't start")?;
  builder.set_num_channels(2).map_err(error)?;
  builder.set_sample_rate(rate).map_err(error)?;
  builder.set_brate(bitrate).map_err(error)?;
  builder.set_quality(Quality::Best).map_err(error)?;
  builder.build().map_err(error)
}

// a sample as 16-bit, with TPDF dither (two random values' difference, a
// step either way at most), so the quietest sounds and fades are a little
// hiss rather than grainy steps
fn dithered_16(sample: f32, seed: &mut u32) -> i16 {
  let mut random = || {
    // xorshift: fast, and plenty random for dither
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed as f32 / u32::MAX as f32
  };
  let dither = random() - random();
  (sample.clamp(-1., 1.) * 32_767. + dither).round().clamp(-32_768., 32_767.) as i16
}

#[cfg(test)]
mod tests {
  use super::*;
  use rodio::Source;

  const WAV16: Format = Format::Wav { bits: Bits::Int16 };

  fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dawesome-mixdown-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
  }

  // a second of a tone, stereo, the right channel half as loud
  fn tone(rate: u32) -> Vec<f32> {
    (0..rate).flat_map(|n| {
      let s = (n as f32 * 440. * std::f32::consts::TAU / rate as f32).sin() * 0.8;
      [s, s * 0.5]
    }).collect()
  }

  fn write(dir: &Path, name: &str, format: Format, rate: u32, samples: &[f32]) -> PathBuf {
    let mut file = MixdownFile::create(dir, name, format, rate).unwrap();
    // in uneven chunks, as the renderer might
    for chunk in samples.chunks(4_000 * 2 + 2) { file.write(chunk).unwrap(); }
    file.finish().unwrap()
  }

  fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir).unwrap()
      .map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
  }

  #[test]
  fn test_wavs_read_back_as_written() {
    let dir = folder("wav");
    let samples = tone(48_000);
    for (bits, size, step) in [(Bits::Int16, 16, 1. / 32_767.), (Bits::Int24, 24, 1. / 8_388_607.), (Bits::Float32, 32, 0.)] {
      let path = write(&dir, &format!("{:?}", bits), Format::Wav { bits }, 48_000, &samples);
      let mut reader = hound::WavReader::open(&path).unwrap();
      let spec = reader.spec();
      assert_eq!((spec.channels, spec.sample_rate, spec.bits_per_sample), (2, 48_000, size));
      let read: Vec<f32> = match bits {
        Bits::Float32 => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
        _ => reader.samples::<i32>().map(|s| s.unwrap() as f32 * step).collect(),
      };
      assert_eq!(read.len(), samples.len());
      let off = read.iter().zip(&samples).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
      // dithered 16-bit: within two steps; 24-bit: within one; float: exact
      assert!(off <= step * 2. + 1e-7, "{:?} off by {}", bits, off);
    }
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_mp3s_are_mp3s_of_the_mix() {
    let dir = folder("mp3");
    for (kbps, rate) in [(320, 48_000), (128, 44_100)] {
      let path = write(&dir, &format!("song {}", kbps), Format::Mp3 { kbps }, rate, &tone(rate));
      let decoder = rodio::Decoder::new(std::fs::File::open(&path).unwrap()).unwrap();
      assert_eq!((decoder.channels(), decoder.sample_rate()), (2, rate));
      let read: Vec<f32> = decoder.convert_samples().collect();
      // about a second (the encoder pads the ends a little), and loud
      let secs = read.len() as f32 / 2. / rate as f32;
      assert!((0.99..1.1).contains(&secs), "{} s", secs);
      let left = read.iter().step_by(2).fold(0f32, |m, s| m.max(s.abs()));
      let right = read.iter().skip(1).step_by(2).fold(0f32, |m, s| m.max(s.abs()));
      assert!((left - 0.8).abs() < 0.08 && (right - 0.4).abs() < 0.08, "{} {}", left, right);
      // about the size its bitrate makes it
      let bytes = std::fs::metadata(&path).unwrap().len() as f32;
      assert!((bytes / (kbps as f32 * 125.) - 1.).abs() < 0.15, "{} bytes at {}", bytes, kbps);
    }
    assert!(MixdownFile::create(&dir, "x", Format::Mp3 { kbps: 100 }, 48_000).is_err());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_mp3s_render_at_a_rate_they_take() {
    let mp3 = Format::Mp3 { kbps: 320 };
    assert_eq!(mp3.render_rate(44_100), 44_100);
    assert_eq!(mp3.render_rate(48_000), 48_000);
    assert_eq!(mp3.render_rate(88_200), 44_100);
    assert_eq!(mp3.render_rate(96_000), 48_000);
    assert_eq!(mp3.render_rate(192_000), 48_000);
    assert_eq!(WAV16.render_rate(96_000), 96_000);
  }

  #[test]
  fn test_a_name_thats_taken_gets_the_next_number_and_nothing_is_left_behind() {
    let dir = folder("numbers");
    let samples = tone(8_000);
    let first = write(&dir, "song", WAV16, 8_000, &samples);
    std::fs::write(dir.join("song 2.wav"), b"someone else's").unwrap();
    let second = write(&dir, "song", WAV16, 8_000, &samples);
    let third = write(&dir, "song", WAV16, 8_000, &samples);
    assert_eq!(first, dir.join("song.wav"));
    assert_eq!(second, dir.join("song 3.wav"));
    assert_eq!(third, dir.join("song 4.wav"));
    // the file that was there is as it was
    assert_eq!(std::fs::read(dir.join("song 2.wav")).unwrap(), b"someone else's");
    // and the same name as another format is free
    write(&dir, "song", Format::Mp3 { kbps: 128 }, 8_000, &samples);
    assert_eq!(files_in(&dir), ["song 2.wav", "song 3.wav", "song 4.wav", "song.mp3", "song.wav"]);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_an_unfinished_file_leaves_nothing() {
    let dir = folder("unfinished");
    let mut file = MixdownFile::create(&dir, "song", WAV16, 8_000).unwrap();
    file.write(&tone(8_000)).unwrap();
    drop(file);
    let mut file = MixdownFile::create(&dir, "song", Format::Mp3 { kbps: 320 }, 48_000).unwrap();
    file.write(&tone(48_000)).unwrap();
    drop(file);
    assert!(files_in(&dir).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_names_are_fit_for_files() {
    let stem = |name: &str| file_stem(name, WAV16);
    assert_eq!(stem("My Song").unwrap(), "My Song");
    assert_eq!(stem("  a/b:c  ").unwrap(), "a-b-c");
    assert_eq!(stem("song.wav").unwrap(), "song");
    assert_eq!(stem("song.WAV").unwrap(), "song");
    assert_eq!(file_stem("song.wav", Format::Mp3 { kbps: 320 }).unwrap(), "song.wav");
    assert_eq!(stem("..hidden").unwrap(), "hidden");
    assert_eq!(stem("tab\there").unwrap(), "tab-here");
    // (a Kelvin sign lower-cases to a shorter k)
    assert_eq!(stem("\u{212A}.wav").unwrap(), "\u{212A}");
    assert_eq!(stem("\u{212A}\u{212A}\u{212A}").unwrap(), "\u{212A}\u{212A}\u{212A}");
    assert_eq!(stem("ü.wav").unwrap(), "ü");
    assert_eq!(stem(&"x".repeat(500)).unwrap().len(), MAX_NAME);
    for empty in ["", "   ", ".wav", "...", " . "] {
      assert_eq!(stem(empty).err().unwrap(), NEEDS_NAME, "{:?}", empty);
    }
  }
}
