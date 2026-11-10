// reads a sample clip's audio for the engine: the clip's settings applied
// (see clip.rs) and its rate changed to the engine's, for the whole sample or
// for a window of it
//
// every clip of a sample is read on the same grid: output frame `n`, counted
// from where the sample's first frame would be heard (the clip's origin, see
// AudioNode::origin), is always the sample at `n * from / to` frames, however
// much of the sample the clip plays. so a clip cut in two plays exactly the
// frames the uncut clip did: the right half carries on at the frame after
// the left half's last, with nothing started over at the cut
//
// a whole clip comes out as it did when rodio's own converters read it
// (Speed, then UniformSourceIterator): the same frames, interpolated the
// same way
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;
use rodio::cpal::Sample as _;
use rodio::{Decoder, Source};

use super::clip::ClipSettings;

// a sample file's audio, decoded
pub struct Pcm {
  // every channel's samples, a frame after another
  samples: Vec<i16>,
  channels: u16,
  rate: u32,
}

impl Pcm {
  pub fn new(samples: Vec<i16>, channels: u16, rate: u32) -> Self {
    Pcm { samples, channels: channels.max(1), rate: rate.max(1) }
  }

  pub fn load(path: &str) -> Result<Self, String> {
    let file = BufReader::new(File::open(path)
      .map_err(|e| format!("could not open sample '{}': {}", path, e))?);
    let source = Decoder::new(file)
      .map_err(|e| format!("could not decode sample '{}': {}", path, e))?;
    let channels = source.channels();
    let rate = source.sample_rate();

    Ok(Pcm::new(source.collect(), channels, rate))
  }

  pub fn samples(&self) -> &[i16] {
    &self.samples
  }

  pub fn channels(&self) -> u16 {
    self.channels
  }

  // the file's own rate
  pub fn rate(&self) -> u32 {
    self.rate
  }

  pub fn frames(&self) -> u64 {
    (self.samples.len() / self.channels as usize) as u64
  }

  // a frame's left and right (a mono sample is on both sides; channels past
  // the first two aren't played)
  fn frame(&self, index: u64) -> [f32; 2] {
    let at = index as usize * self.channels as usize;
    let left: f32 = self.samples[at].to_sample();
    let right = if self.channels >= 2 { self.samples[at + 1].to_sample() } else { left };
    [left, right]
  }
}

// the rate a sample is read at once its pitch is changed (a whole number,
// as rodio's Speed has it)
pub fn pitched_rate(file_rate: u32, settings: &ClipSettings) -> u32 {
  ((file_rate as f32 * settings.playback_rate()) as u32).max(1)
}

fn gcd(a: u64, b: u64) -> u64 {
  if b == 0 { a } else { gcd(b, a % b) }
}

// the part of a sample a clip plays, in the sample's own time (before any
// change of speed)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
  pub start: Duration,
  // None: to the sample's end
  pub end: Option<Duration>,
}

pub struct ClipReader {
  pcm: Arc<Pcm>,
  // output frame n is the sample at n * from / to frames
  from: u64,
  to: u64,
  // the next output frame, counted from the clip's origin
  next: u64,
  // the output frame the clip stops before (None: at the sample's end)
  end: Option<u64>,
  // the rate it's read out at
  rate: u32,
  gain: f32,
  pan: [f32; 2],
  fades: Fades,
  // the right side of the frame whose left side was just returned
  pending_right: Option<f32>,
  // the two sample frames last used, with the settings applied
  held: Option<(u64, [f32; 2], Option<[f32; 2]>)>,
  done: bool,
}

// the clip's fade in and out, along the sample's own frames
//
// times are counted from the sample's start, at the clip's speed, whatever
// part of the sample the clip plays: a frame's time is worked out the same
// way in every clip of the sample, so the half of a cut clip that keeps a
// fade plays it exactly as the uncut clip did
struct Fades {
  attack: f32,
  decay: f32,
  // where the clip starts and ends
  start: f32,
  end: f32,
  // frames a second, at the clip's pitch
  rate: f32,
}

impl Fades {
  // the level, 0 to 1, of a frame of the sample
  fn level(&self, frame: u64) -> f32 {
    let t = frame as f32 / self.rate;
    let fade_in = if self.attack > 0. { (t - self.start) / self.attack } else { 1. };
    let fade_out = if self.decay > 0. { (self.end - t) / self.decay } else { 1. };

    fade_in.min(fade_out).clamp(0., 1.)
  }
}

impl ClipReader {
  // - rate: the rate to read it out at (the engine's)
  // - fades: where the clip starts and ends, in played time from the
  //   sample's start: what its fades are measured from
  // - lead: output frames from the clip's origin to where reading starts
  // - end: the output frame, from the origin, it stops before. None for a
  //   clip that plays to the sample's end
  pub fn new(
    pcm: Arc<Pcm>,
    settings: &ClipSettings,
    rate: u32,
    fades: (Duration, Duration),
    lead: u64,
    end: Option<u64>,
  ) -> Self {
    let rate = rate.max(1);
    let pitched = pitched_rate(pcm.rate(), settings);
    let divisor = gcd(pitched as u64, rate as u64);
    let pan = settings.pan.clamp(-1., 1.);
    // (through a Duration and back, as the fades always were)
    let secs = |ms: f32| Duration::from_secs_f32(ms.max(0.) / 1000.).as_secs_f32();

    ClipReader {
      from: pitched as u64 / divisor,
      to: rate as u64 / divisor,
      next: lead,
      end,
      rate,
      gain: settings.gain_factor(),
      pan: [(1. - pan).min(1.), (1. + pan).min(1.)],
      fades: Fades {
        attack: secs(settings.attack_ms),
        decay: secs(settings.decay_ms),
        start: fades.0.as_secs_f32(),
        end: fades.1.as_secs_f32(),
        rate: pitched as f32,
      },
      pending_right: None,
      held: None,
      done: false,
      pcm,
    }
  }

  // a frame of the sample with the clip's settings applied: louder or
  // quieter, faded, then panned
  fn played(&self, index: u64) -> [f32; 2] {
    let level = self.fades.level(index);
    let [left, right] = self.pcm.frame(index);
    [
      left * self.gain * level * self.pan[0],
      right * self.gain * level * self.pan[1],
    ]
  }

  // the sample frame at `index` and the one after it (None past the end)
  fn pair(&mut self, index: u64) -> ([f32; 2], Option<[f32; 2]>) {
    let frames = self.pcm.frames();
    match self.held {
      Some((held, current, next)) if held == index => return (current, next),
      // moved on by one: the frame after is the frame now
      Some((held, _, Some(next))) if held + 1 == index => {
        let after = if index + 1 < frames { Some(self.played(index + 1)) } else { None };
        self.held = Some((index, next, after));
      }
      _ => {
        let after = if index + 1 < frames { Some(self.played(index + 1)) } else { None };
        self.held = Some((index, self.played(index), after));
      }
    }

    let (_, current, next) = self.held.unwrap();
    (current, next)
  }

  // the next output frame
  fn next_frame(&mut self) -> Option<[f32; 2]> {
    if self.done { return None; }
    if self.end.is_some_and(|end| self.next >= end) {
      self.done = true;
      return None;
    }

    let at = self.next as u128 * self.from as u128;
    let index = (at / self.to as u128) as u64;
    if index >= self.pcm.frames() {
      self.done = true;
      return None;
    }
    let numerator = (at % self.to as u128) as u32;
    self.next += 1;

    Some(match self.pair(index) {
      (current, Some(next)) => {
        let lerp = |a: f32, b: f32| a + (b - a) * numerator as f32 / self.to as f32;
        [lerp(current[0], next[0]), lerp(current[1], next[1])]
      }
      // the sample's last frame is played as it is, and ends the clip
      (current, None) => {
        self.done = true;
        current
      }
    })
  }
}

impl Iterator for ClipReader {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    if let Some(right) = self.pending_right.take() {
      return Some(right);
    }

    let [left, right] = self.next_frame()?;
    self.pending_right = Some(right);
    Some(left)
  }
}

impl Source for ClipReader {
  fn current_frame_len(&self) -> Option<usize> { None }
  fn channels(&self) -> u16 { 2 }
  fn sample_rate(&self) -> u32 { self.rate }
  fn total_duration(&self) -> Option<Duration> { None }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rodio::buffer::SamplesBuffer;
  use rodio::source::UniformSourceIterator;

  // a sample that's different at every frame: a sweep with some noise on
  // it, the right side not the left
  fn pcm(channels: u16, rate: u32, frames: usize) -> Arc<Pcm> {
    let mut seed = 12_345u32;
    let mut samples = Vec::with_capacity(frames * channels as usize);
    for frame in 0..frames {
      for channel in 0..channels {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let noise = ((seed >> 16) % 2_000) as f32 - 1_000.;
        let t = frame as f32 / rate as f32;
        let tone = (t * (200. + 900. * t) * std::f32::consts::TAU + channel as f32).sin() * 20_000.;
        samples.push((tone + noise) as i16);
      }
    }
    Arc::new(Pcm::new(samples, channels, rate))
  }

  fn length(pcm: &Pcm, settings: &ClipSettings) -> Duration {
    settings.played_duration(Duration::from_secs_f64(pcm.frames() as f64 / pcm.rate() as f64))
  }

  // the sample as the engine used to read it: rodio's Speed, the clip's
  // fades and pan, then rodio's converter to the engine's rate
  fn old_way(pcm: &Pcm, settings: &ClipSettings, rate: u32) -> Vec<f32> {
    let source = SamplesBuffer::new(pcm.channels(), pcm.rate(), pcm.samples().to_vec());
    let sample_duration = Duration::from_secs_f64(pcm.frames() as f64 / pcm.rate() as f64);
    let played = settings.apply(source.convert_samples::<f32>(), sample_duration);
    let mut converted = UniformSourceIterator::<_, f32>::new(played, 2, rate);
    // (not collected: rodio's size hint, which collect asks for, can
    // overflow)
    let mut out = Vec::new();
    while let Some(sample) = converted.next() { out.push(sample); }
    out
  }

  fn new_way(pcm: &Arc<Pcm>, settings: &ClipSettings, rate: u32) -> Vec<f32> {
    ClipReader::new(pcm.clone(), settings, rate, (Duration::ZERO, length(pcm, settings)), 0, None).collect()
  }

  fn pitched(pitch: f32) -> ClipSettings {
    ClipSettings { pitch, ..Default::default() }
  }

  #[test]
  fn test_a_whole_clip_is_read_as_it_always_was() {
    // file rate, engine rate, channels
    for (file, engine, channels) in [
      (44_100, 44_100, 2), (48_000, 48_000, 1), (44_100, 48_000, 2), (48_000, 44_100, 2),
      (44_100, 48_000, 1), (22_050, 48_000, 2), (96_000, 44_100, 2),
    ] {
      let pcm = pcm(channels, file, 5_000);
      for pitch in [0., -7., 5., 12., -12., 0.37, -24., 24.] {
        let settings = pitched(pitch);
        let old = old_way(&pcm, &settings, engine);
        let new = new_way(&pcm, &settings, engine);
        assert_eq!(new.len(), old.len(), "{}Hz to {}Hz, {} channels, pitch {}", file, engine, channels, pitch);
        // not one sample differs
        assert!(new == old, "{}Hz to {}Hz, {} channels, pitch {}", file, engine, channels, pitch);
      }
    }
  }

  #[test]
  fn test_a_whole_clip_with_every_setting_is_within_a_16_bit_step() {
    let step = 1. / 32_768.;
    for (file, engine) in [(44_100, 44_100), (44_100, 48_000), (48_000, 44_100)] {
      let pcm = pcm(2, file, 9_000);
      let settings = ClipSettings { gain_db: -4.5, pan: 0.3, pitch: -3., attack_ms: 40., decay_ms: 75. }
        .clamped(Duration::from_secs_f64(9_000. / file as f64));
      let old = old_way(&pcm, &settings, engine);
      let new = new_way(&pcm, &settings, engine);

      assert_eq!(new.len(), old.len());
      let worst = new.iter().zip(&old).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
      assert!(worst < step, "off by {} at {}Hz to {}Hz", worst, file, engine);
      // it fades in from nothing, and out to it
      assert_eq!(new[0], 0.);
      assert!(new[new.len() - 2].abs() < 0.01);
    }
  }

  #[test]
  fn test_a_mono_sample_is_on_both_sides_and_pan_turns_one_down() {
    let pcm = pcm(1, 44_100, 100);
    let centre = new_way(&pcm, &ClipSettings::default(), 44_100);
    assert!(centre.chunks(2).all(|frame| frame[0] == frame[1]));

    let left = new_way(&pcm, &ClipSettings { pan: -1., ..Default::default() }, 44_100);
    assert!(left.chunks(2).all(|frame| frame[1] == 0.));
    assert!(left.chunks(2).zip(centre.chunks(2)).all(|(a, b)| a[0] == b[0]));
  }

  // the output frames [lead, end) of a clip
  fn part(pcm: &Arc<Pcm>, settings: &ClipSettings, rate: u32, lead: u64, end: Option<u64>) -> Vec<f32> {
    ClipReader::new(pcm.clone(), settings, rate, (Duration::ZERO, length(pcm, settings)), lead, end).collect()
  }

  #[test]
  fn test_two_parts_are_the_whole_frame_for_frame() {
    for (file, engine) in [(44_100, 48_000), (48_000, 48_000), (44_100, 44_100), (48_000, 44_100)] {
      let pcm = pcm(2, file, 6_000);
      for pitch in [-7., 0., 5.] {
        let settings = pitched(pitch);
        let whole = new_way(&pcm, &settings, engine);
        let frames = whole.len() as u64 / 2;

        // cut anywhere, including at the very ends
        for cut in [0, 1, 2, 777, 3_001, frames - 1, frames] {
          let mut halves = part(&pcm, &settings, engine, 0, Some(cut));
          assert_eq!(halves.len() as u64, cut * 2);
          halves.extend(part(&pcm, &settings, engine, cut, None));
          assert!(halves == whole, "cut at {} of {}, {}Hz to {}Hz, pitch {}", cut, frames, file, engine, pitch);
        }

        // and in three
        let mut thirds = part(&pcm, &settings, engine, 0, Some(1_000));
        thirds.extend(part(&pcm, &settings, engine, 1_000, Some(1_001)));
        thirds.extend(part(&pcm, &settings, engine, 1_001, None));
        assert!(thirds == whole);
      }
    }
  }

  #[test]
  fn test_parts_keep_the_whole_clips_fades_exactly() {
    for (file, engine) in [(44_100, 48_000), (48_000, 48_000)] {
      let pcm = pcm(2, file, 20_000);
      for pitch in [-7., 0., 5.] {
        let settings = ClipSettings { pitch, attack_ms: 30., decay_ms: 45., ..Default::default() };
        let whole = new_way(&pcm, &settings, engine);
        let ends = length(&pcm, &settings);

        // cut past the fade in and before the fade out: the first part
        // keeps the one and the second the other, measured from where the
        // whole clip's were
        let cut = 9_000u64;
        let at = Duration::from_secs_f64(cut as f64 / engine as f64);
        let first = ClipSettings { decay_ms: 0., ..settings };
        let second = ClipSettings { attack_ms: 0., ..settings };
        let mut parts: Vec<f32> = ClipReader::new(pcm.clone(), &first, engine, (Duration::ZERO, at), 0, Some(cut)).collect();
        parts.extend(ClipReader::new(pcm.clone(), &second, engine, (at, ends), cut, None));
        assert!(parts == whole, "{}Hz to {}Hz, pitch {}", file, engine, pitch);
      }
    }
  }

  #[test]
  fn test_a_part_past_the_sample_is_silent() {
    let pcm = pcm(2, 44_100, 500);
    assert!(part(&pcm, &ClipSettings::default(), 48_000, 10_000, None).is_empty());
    assert!(part(&pcm, &ClipSettings::default(), 48_000, 100, Some(100)).is_empty());
    assert!(part(&pcm, &ClipSettings::default(), 48_000, 100, Some(50)).is_empty());
    // an end past the sample's is the sample's
    let whole = new_way(&pcm, &ClipSettings::default(), 48_000);
    assert!(part(&pcm, &ClipSettings::default(), 48_000, 0, Some(1_000_000)) == whole);
  }

  #[test]
  fn test_fades_are_measured_from_the_window() {
    // a second of full-scale sound, read from 0.25s to 0.75s with 100ms fades
    let rate = 1_000;
    let pcm = Arc::new(Pcm::new(vec![i16::MAX; 1_000], 1, rate));
    let settings = ClipSettings { attack_ms: 100., decay_ms: 100., ..Default::default() };
    let window = (Duration::from_millis(250), Duration::from_millis(750));
    let out: Vec<f32> = ClipReader::new(pcm, &settings, rate, window, 250, Some(750))
      .step_by(2)
      .collect();

    assert_eq!(out.len(), 500);
    let full = i16::MAX as f32 / 32_768.;
    // silent at the window's start, half way up 50ms in, full by 100ms
    assert!(out[0].abs() < 1e-4);
    assert!((out[50] - full * 0.5).abs() < 1e-3);
    assert!((out[100] - full).abs() < 1e-3);
    assert!((out[250] - full).abs() < 1e-6);
    // and down to silence at the window's end
    assert!((out[450] - full * 0.5).abs() < 1e-3);
    assert!(out[499] < full * 0.02);
  }
}
