// how a clip in the playlist is played back, set from the frontend's Sample
// tab. mirrors src/types/clip.ts
//
// pitch and speed are linked, like a tape or record: raising the pitch plays
// the clip faster and shorter
use std::ops::RangeInclusive;
use std::time::Duration;

use crate::util::math::clamp_or;
#[cfg(test)]
use rodio::Source;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipSettings {
  // volume change in decibels, 0 is unchanged
  pub gain_db: f32,
  // stereo position, -1 is hard left, 0 is center, 1 is hard right
  pub pan: f32,
  // semitones up or down, which also changes the speed
  pub pitch: f32,
  // fade in from silence over this many milliseconds at the clip's start
  pub attack_ms: f32,
  // fade out to silence over this many milliseconds at the clip's end
  pub decay_ms: f32,
}

impl Default for ClipSettings {
  fn default() -> Self {
    ClipSettings {
      gain_db: 0.,
      pan: 0.,
      pitch: 0.,
      attack_ms: 0.,
      decay_ms: 0.,
    }
  }
}

// how loud each side is at a pan of -1 (hard left) to 1 (hard right): a
// balance law, as a clip's pan (and a sample or synth row's) has it. at the
// center both sides are unchanged, and panning turns the far side down, to
// silence at hard left or right
pub fn pan_gains(pan: f32) -> [f32; 2] {
  let pan = pan.clamp(-1., 1.);
  [(1. - pan).min(1.), (1. + pan).min(1.)]
}

// the range of each setting (attack and decay are also kept within the clip)
const GAIN_DB: RangeInclusive<f32> = -60.0..=24.0;
const PAN: RangeInclusive<f32> = -1.0..=1.0;
const PITCH: RangeInclusive<f32> = -24.0..=24.0;
const FADE_MS: RangeInclusive<f32> = 0.0..=10_000.0;

impl ClipSettings {
  // how much faster than the original the clip plays: 12 semitones up is
  // twice as fast, 12 down is half as fast
  pub fn playback_rate(&self) -> f32 {
    2f32.powf(self.pitch / 12.)
  }

  // the clip's length once it's sped up or slowed down
  pub fn played_duration(&self, sample_duration: Duration) -> Duration {
    sample_duration.div_f32(self.playback_rate())
  }

  // the same, to the nanosecond (played_duration rounds to a float's seven
  // or so digits, which is fine for a length to show, but not for working
  // out where a point of a sample is heard)
  pub fn played_exactly(&self, of_sample: Duration) -> Duration {
    Duration::from_secs_f64(of_sample.as_secs_f64() / self.playback_rate() as f64)
  }

  // how much of the sample goes by in a length of played time
  pub fn sample_time(&self, played: Duration) -> Duration {
    Duration::from_secs_f64(played.as_secs_f64() * self.playback_rate() as f64)
  }

  // the volume multiplier for the gain
  pub fn gain_factor(&self) -> f32 {
    10f32.powf(self.gain_db / 20.)
  }

  // keep every setting in range, and the fades within the clip, given the
  // sample's length before any speed change
  pub fn clamped(&self, sample_duration: Duration) -> Self {
    let d = ClipSettings::default();
    let mut s = ClipSettings {
      gain_db: clamp_or(self.gain_db, GAIN_DB, d.gain_db),
      pan: clamp_or(self.pan, PAN, d.pan),
      pitch: clamp_or(self.pitch, PITCH, d.pitch),
      attack_ms: clamp_or(self.attack_ms, FADE_MS, d.attack_ms),
      decay_ms: clamp_or(self.decay_ms, FADE_MS, d.decay_ms),
    };

    let length_ms = s.played_duration(sample_duration).as_secs_f32() * 1000.;
    s.attack_ms = s.attack_ms.min(length_ms);
    s.decay_ms = s.decay_ms.min(length_ms);

    s
  }

  // a source played with these settings: sped up or slowed down, louder or
  // quieter, faded in and out, and panned (always stereo)
  // - sample_duration: the source's length before any speed change
  #[cfg(test)]
  pub fn apply<S>(&self, source: S, sample_duration: Duration) -> Pan<Envelope<rodio::source::Amplify<rodio::source::Speed<S>>>>
  where
    S: Source<Item = f32>,
  {
    let envelope = Envelope::new(
      source.speed(self.playback_rate()).amplify(self.gain_factor()),
      Duration::from_secs_f32(self.attack_ms / 1000.),
      Duration::from_secs_f32(self.decay_ms / 1000.),
      self.played_duration(sample_duration),
    );

    Pan::new(envelope, self.pan)
  }
}

// fades a source in from silence over `attack` and out to silence over
// `decay` before `length` (the source's played length). where the fades
// overlap, the quieter one wins
#[cfg(test)]
pub struct Envelope<S> {
  inner: S,
  attack: f32,
  decay: f32,
  length: f32,
  // samples so far, across all channels
  position: u64,
}

#[cfg(test)]
impl<S: Source<Item = f32>> Envelope<S> {
  pub fn new(inner: S, attack: Duration, decay: Duration, length: Duration) -> Self {
    Envelope {
      inner,
      attack: attack.as_secs_f32(),
      decay: decay.as_secs_f32(),
      length: length.as_secs_f32(),
      position: 0,
    }
  }

  // the envelope's level, 0 to 1, at `t` seconds
  fn level_at(&self, t: f32) -> f32 {
    let fade_in = if self.attack > 0. { t / self.attack } else { 1. };
    let fade_out = if self.decay > 0. { (self.length - t) / self.decay } else { 1. };

    fade_in.min(fade_out).clamp(0., 1.)
  }
}

#[cfg(test)]
impl<S: Source<Item = f32>> Iterator for Envelope<S> {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    let sample = self.inner.next()?;
    let channels = self.inner.channels().max(1) as u64;
    let rate = self.inner.sample_rate().max(1) as f32;
    let t = (self.position / channels) as f32 / rate;
    self.position += 1;

    Some(sample * self.level_at(t))
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.inner.size_hint()
  }
}

#[cfg(test)]
impl<S: Source<Item = f32>> Source for Envelope<S> {
  fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
  fn channels(&self) -> u16 { self.inner.channels() }
  fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

// places a source in the stereo field. always outputs two channels: a mono
// source goes to both, a stereo source keeps its own left and right (any
// channels past the first two are dropped)
//
// uses a balance law: at the center both sides are unchanged, and panning
// turns the opposite side down, to silence at hard left or right
#[cfg(test)]
pub struct Pan<S> {
  inner: S,
  left_gain: f32,
  right_gain: f32,
  // the right sample of the current frame, returned after the left one
  pending_right: Option<f32>,
}

#[cfg(test)]
impl<S: Source<Item = f32>> Pan<S> {
  pub fn new(inner: S, pan: f32) -> Self {
    let pan = pan.clamp(-1., 1.);

    Pan {
      inner,
      left_gain: (1. - pan).min(1.),
      right_gain: (1. + pan).min(1.),
      pending_right: None,
    }
  }
}

#[cfg(test)]
impl<S: Source<Item = f32>> Iterator for Pan<S> {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    if let Some(right) = self.pending_right.take() {
      return Some(right);
    }

    // read one frame of the source
    let channels = self.inner.channels().max(1);
    let left = self.inner.next()?;
    let right = if channels >= 2 { self.inner.next().unwrap_or(left) } else { left };
    for _ in 2..channels {
      self.inner.next();
    }

    self.pending_right = Some(right * self.right_gain);
    Some(left * self.left_gain)
  }
}

#[cfg(test)]
impl<S: Source<Item = f32>> Source for Pan<S> {
  fn current_frame_len(&self) -> Option<usize> {
    // the same number of frames, now two samples each
    let channels = self.inner.channels().max(1) as usize;
    self.inner.current_frame_len().map(|len| len / channels * 2)
  }
  fn channels(&self) -> u16 { 2 }
  fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rodio::buffer::SamplesBuffer;

  // a constant signal: `secs` seconds of 1.0 at 1kHz
  fn ones(channels: u16, secs: f32) -> SamplesBuffer<f32> {
    let len = (1000. * secs) as usize * channels as usize;
    SamplesBuffer::new(channels, 1000, vec![1.; len])
  }

  #[test]
  fn test_playback_rate_and_duration() {
    let up = ClipSettings { pitch: 12., ..Default::default() };
    let down = ClipSettings { pitch: -12., ..Default::default() };

    assert_eq!(ClipSettings::default().playback_rate(), 1.);
    assert!((up.playback_rate() - 2.).abs() < 1e-5);
    assert!((down.playback_rate() - 0.5).abs() < 1e-5);
    assert_eq!(
      up.played_duration(Duration::from_secs(4)).as_millis(),
      2000);
  }

  #[test]
  fn test_gain_factor() {
    assert_eq!(ClipSettings::default().gain_factor(), 1.);
    let quiet = ClipSettings { gain_db: -20., ..Default::default() };
    assert!((quiet.gain_factor() - 0.1).abs() < 1e-5);
  }

  #[test]
  fn test_clamped() {
    let wild = ClipSettings {
      gain_db: 40.,
      pan: -3.,
      pitch: f32::NAN,
      attack_ms: 900.,
      decay_ms: 300.,
    };

    assert_eq!(wild.clamped(Duration::from_millis(500)), ClipSettings {
      gain_db: 24.,
      pan: -1.,
      pitch: 0.,
      attack_ms: 500.,
      decay_ms: 300.,
    });
  }

  #[test]
  fn test_serde_names_match_the_frontend() {
    let json = serde_json::to_value(ClipSettings::default()).unwrap();
    assert_eq!(json, serde_json::json!({
      "gainDb": 0.,
      "pan": 0.,
      "pitch": 0.,
      "attackMs": 0.,
      "decayMs": 0.,
    }));
  }

  #[test]
  fn test_envelope_fades_in_and_out() {
    // 1s, 100ms attack, 200ms decay
    let out: Vec<f32> = Envelope::new(
      ones(1, 1.),
      Duration::from_millis(100),
      Duration::from_millis(200),
      Duration::from_secs(1),
    ).collect();

    assert_eq!(out.len(), 1000);
    assert_eq!(out[0], 0.);
    assert!((out[50] - 0.5).abs() < 0.01);
    assert_eq!(out[500], 1.);
    assert!((out[900] - 0.5).abs() < 0.01);
    assert!(out[999] < 0.01);
  }

  #[test]
  fn test_envelope_counts_time_per_frame() {
    // stereo: both samples of a frame get the same level
    let out: Vec<f32> = Envelope::new(
      ones(2, 1.),
      Duration::from_millis(100),
      Duration::ZERO,
      Duration::from_secs(1),
    ).collect();

    assert_eq!(out[100], out[101]);
    assert!((out[100] - 0.5).abs() < 0.01);
  }

  #[test]
  fn test_default_settings_leave_audio_unchanged() {
    let source = SamplesBuffer::new(1, 1000, vec![0.25, -0.5, 1., 0.]);
    let out: Vec<f32> = ClipSettings::default()
      .apply(source, Duration::from_millis(4))
      .collect();

    // mono becomes stereo, at full level on both sides
    assert_eq!(out, vec![0.25, 0.25, -0.5, -0.5, 1., 1., 0., 0.]);
  }

  #[test]
  fn test_pan() {
    let frame = |pan: f32, channels: u16| -> Vec<f32> {
      Pan::new(ones(channels, 0.001), pan).take(2).collect()
    };

    assert_eq!(frame(0., 1), vec![1., 1.]);
    assert_eq!(frame(-1., 1), vec![1., 0.]);
    assert_eq!(frame(1., 1), vec![0., 1.]);
    assert_eq!(frame(0.5, 1), vec![0.5, 1.]);
    assert_eq!(frame(0.5, 2), vec![0.5, 1.]);
  }

  #[test]
  fn test_pan_keeps_stereo_sides() {
    let source = SamplesBuffer::new(2, 1000, vec![0.1, 0.9, 0.2, 0.8]);
    let out: Vec<f32> = Pan::new(source, 0.).collect();

    assert_eq!(out, vec![0.1, 0.9, 0.2, 0.8]);
  }

  #[test]
  fn test_apply_speeds_up_and_fades() {
    let settings = ClipSettings { pitch: 12., attack_ms: 100., gain_db: -6., ..Default::default() };
    let applied = settings.apply(ones(1, 1.), Duration::from_secs(1));

    // twice as fast: the same samples, reported at twice the rate
    assert_eq!(applied.sample_rate(), 2000);
    let out: Vec<f32> = applied.collect();
    assert_eq!(out[0], 0.);
    // past the attack: just the gain
    let half = settings.gain_factor();
    assert!((out[1000] - half).abs() < 1e-5);
  }
}
