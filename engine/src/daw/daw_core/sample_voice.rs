// a sample row's notes as they sound (see sequencer.rs): its sample, decoded
// once, read at each note's pitch the way a clip's pitch is read (tape-style:
// higher is faster and shorter, see clip_reader.rs), at the row's volume
// times the note's velocity, and panned as the row is
//
// a one-shot row's note plays to the sample's end, whatever its length. a
// gated row's stops at the note's end, fading out over the last FADE of it so
// it doesn't click. a key held on a gated row (an audition) fades out the
// same way once it's let go (see HeldKeys)
//
// live playback, the mixdown and Deep copy all play notes through this, so
// what they play can't differ. reading a voice only reads the decoded sample:
// the audio thread never decodes, locks or allocates for one
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use rodio::Source;

use super::clip::ClipSettings;
use super::clip_reader::{pitched_rate, ClipReader, Pcm};

// how long a gated note fades out for, before its end
pub const FADE: Duration = Duration::from_millis(5);

// a sample row, as what plays its notes
#[derive(Clone)]
pub struct SampleInstrument {
  pub pcm: Arc<Pcm>,
  // the pitch that plays the sample unchanged
  pub sample_pitch: u8,
  // whether a note stops at its end, rather than playing the sample out
  pub gated: bool,
  // the row's volume (a gain) and pan
  pub volume: f32,
  pub pan: f32,
}

impl SampleInstrument {
  // how a note at `pitch` reads the sample: as a clip pitched by the
  // semitones from the sample's own pitch, and panned as the row is
  fn settings(&self, pitch: u8) -> ClipSettings {
    ClipSettings { pitch: pitch as f32 - self.sample_pitch as f32, pan: self.pan, ..Default::default() }
  }

  // how long a note at `pitch` sounds for, if it plays to the sample's end
  pub fn played_len(&self, pitch: u8) -> Duration {
    let rate = pitched_rate(self.pcm.rate(), &self.settings(pitch));
    Duration::from_secs_f64(self.pcm.frames() as f64 / rate as f64)
  }

  // the longest any of `notes` can sound for: the sample played to its end
  // at the lowest of their pitches, the slowest
  pub fn longest(&self, notes: &[crate::daw::PatternNote]) -> Duration {
    notes.iter().map(|note| note.pitch).min().map_or(Duration::ZERO, |pitch| self.played_len(pitch))
  }

  // a note at `pitch` and `velocity` (0 to 1), read out at `rate`
  // - lead: frames from the note's start to where it's played from (one
  //   picked up partway through)
  // - end: the frame, from the note's start, a gated note stops at. none
  //   plays to the sample's end
  pub fn voice(&self, pitch: u8, velocity: f32, rate: u32, lead: u64, end: Option<u64>) -> SampleVoice {
    let fades = (Duration::ZERO, Duration::ZERO);
    let reader = ClipReader::new(self.pcm.clone(), &self.settings(pitch), rate, fades, lead, None);
    let fade = frames_in(FADE, rate);
    SampleVoice {
      reader,
      gain: self.volume * velocity,
      next: lead,
      end,
      // (a note shorter than a fade fades over all of it)
      fade: end.map_or(fade, |end| fade.min(end).max(1)),
      release: None,
      pending_right: None,
    }
  }

  // a key held on the row: a note that plays until it's let go of (then
  // fading out) or to the sample's end. a one-shot row's plays to the end
  // whatever, so it has nothing to let it go
  pub fn held(&self, pitch: u8, velocity: f32, rate: u32) -> (SampleVoice, Option<Release>) {
    let mut voice = self.voice(pitch, velocity, rate, 0, None);
    if !self.gated { return (voice, None); }
    let release = Release(Arc::new(AtomicBool::new(false)));
    voice.release = Some(release.clone());
    (voice, Some(release))
  }

  // the sample once, as it is, at the row's volume and pan
  pub fn preview(&self, rate: u32) -> SampleVoice {
    self.voice(self.sample_pitch, 1., rate, 0, None)
  }
}

// a duration in frames at `rate`, at least one
fn frames_in(dur: Duration, rate: u32) -> u64 {
  ((dur.as_secs_f64() * rate as f64).round() as u64).max(1)
}

// a note playing, in stereo at the rate it was made for
pub struct SampleVoice {
  reader: ClipReader,
  gain: f32,
  // the next frame out, counted from the note's start
  next: u64,
  // the frame it stops at, counted the same way, having faded out over the
  // `fade` frames before it. none: at the sample's end
  end: Option<u64>,
  fade: u64,
  // what lets a held key's voice go
  release: Option<Release>,
  // the right side of the frame whose left side was just given
  pending_right: Option<f32>,
}

impl SampleVoice {
  fn next_frame(&mut self) -> Option<[f32; 2]> {
    // let go of: it fades out from here. (the release stays with the voice,
    // which the engine drops off the audio thread: dropping it here could
    // free it here)
    if self.end.is_none() && self.release.as_ref().is_some_and(Release::is_let_go) {
      self.end = Some(self.next + self.fade);
    }
    if self.end.is_some_and(|end| self.next >= end) { return None; }

    let [left, right] = self.reader.next_frame()?;
    let level = self.gain * self.fading();
    self.next += 1;
    Some([left * level, right * level])
  }

  // how far the next frame is faded: 1 until the last `fade` frames before
  // the end, then down in a straight line, to nothing at the end
  fn fading(&self) -> f32 {
    match self.end {
      Some(end) => ((end - self.next) as f32 / self.fade as f32).min(1.),
      None => 1.,
    }
  }
}

impl Iterator for SampleVoice {
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

impl Source for SampleVoice {
  fn current_frame_len(&self) -> Option<usize> { None }
  fn channels(&self) -> u16 { 2 }
  fn sample_rate(&self) -> u32 { self.reader.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { None }
}

// what lets go of a held key's voice (see SampleInstrument::held). the voice
// sees it on the audio thread, without a lock
#[derive(Clone)]
pub struct Release(Arc<AtomicBool>);

impl Release {
  pub fn let_go(&self) {
    self.0.store(true, Ordering::Relaxed);
  }

  fn is_let_go(&self) -> bool {
    self.0.load(Ordering::Relaxed)
  }

  // whether its voice is still there (the engine drops one that's finished)
  fn voice_alive(&self) -> bool {
    Arc::strong_count(&self.0) > 1
  }
}

// the keys held on gated sample rows, by row and pitch, so letting go of one
// fades out what it's playing. (a synth row's keys are its plugin's to hold)
#[derive(Default)]
pub struct HeldKeys(Mutex<HashMap<(u64, u8), Vec<Release>>>);

impl HeldKeys {
  pub fn hold(&self, row: u64, pitch: u8, release: Release) {
    let mut held = self.0.lock().unwrap();
    // (keys whose voices have played out are forgotten)
    held.retain(|_, releases| {
      releases.retain(Release::voice_alive);
      !releases.is_empty()
    });
    held.entry((row, pitch)).or_default().push(release);
  }

  // a key let go of: every voice it's playing fades out
  pub fn let_go(&self, row: u64, pitch: u8) {
    let releases = self.0.lock().unwrap().remove(&(row, pitch)).unwrap_or_default();
    for release in releases {
      release.let_go();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // a mono sample at 44.1kHz, `frames` long: a sweep, so every frame
  // differs from the next
  fn sweep(frames: usize) -> Arc<Pcm> {
    let samples = (0..frames).map(|n| ((n as f32 * 0.05).sin() * 20_000.) as i16).collect();
    Arc::new(Pcm::new(samples, 1, 44_100))
  }

  // a sample that's the same all the way through, so a fade is all that
  // changes its level
  fn steady(frames: usize) -> Arc<Pcm> {
    Arc::new(Pcm::new(vec![16_384; frames], 1, 44_100))
  }

  fn instrument(pcm: Arc<Pcm>, gated: bool) -> SampleInstrument {
    SampleInstrument { pcm, sample_pitch: 60, gated, volume: 1., pan: 0. }
  }

  // the left side of a voice's frames
  fn left(voice: SampleVoice) -> Vec<f32> {
    voice.step_by(2).collect()
  }

  #[test]
  fn test_an_octave_up_plays_every_other_frame_in_half_the_time() {
    let row = instrument(sweep(10_000), false);
    let unchanged = left(row.voice(60, 1., 44_100, 0, None));
    let octave = left(row.voice(72, 1., 44_100, 0, None));

    assert_eq!(unchanged.len(), 10_000);
    assert_eq!(octave.len(), 5_000);
    // twice the rate: frame n is the sample's frame 2n
    assert!(octave.iter().enumerate().all(|(n, s)| *s == unchanged[2 * n]));
    let (octave_len, unchanged_len) = (row.played_len(72).as_secs_f64(), row.played_len(60).as_secs_f64());
    assert!((octave_len * 2. - unchanged_len).abs() < 1e-8);
    // and an octave down, twice as long (read out at another rate too)
    assert_eq!(left(row.voice(48, 1., 44_100, 0, None)).len(), 19_999);
    assert!(left(row.voice(48, 1., 48_000, 0, None)).len() > 21_700);
  }

  #[test]
  fn test_velocity_and_volume_scale_it() {
    let row = instrument(sweep(2_000), false);
    let full = left(row.voice(60, 1., 44_100, 0, None));
    let quarter = left(row.voice(60, 0.25, 44_100, 0, None));
    let peak = |samples: &[f32]| samples.iter().fold(0f32, |m, s| m.max(s.abs()));

    assert_eq!(peak(&quarter), peak(&full) * 0.25);
    assert!(quarter.iter().zip(&full).all(|(q, f)| *q == f * 0.25));
    let louder = SampleInstrument { volume: 2., ..row.clone() };
    assert!(left(louder.voice(60, 0.25, 44_100, 0, None)).iter().zip(&full).all(|(l, f)| *l == f * 0.5));
    assert!(left(row.voice(60, 0., 44_100, 0, None)).iter().all(|s| *s == 0.));
  }

  #[test]
  fn test_a_gated_note_ends_at_its_end_faded_out() {
    let fade = frames_in(FADE, 48_000) as usize;
    assert_eq!(fade, 240);
    let row = instrument(steady(44_100), true);
    let full = 16_384. / 32_768.;

    for (rate, end) in [(48_000, 3_000), (48_000, 1_001), (44_100, 9_999)] {
      let heard = left(row.voice(64, 1., rate, 0, Some(end)));
      let fade = frames_in(FADE, rate) as usize;
      assert_eq!(heard.len(), end as usize);
      // full until the fade, then down to nothing in even steps
      assert!(heard[..end as usize - fade].iter().all(|s| (s - full).abs() < 1e-6));
      assert!(heard.windows(2).all(|pair| (pair[1] - pair[0]).abs() <= full / fade as f32 + 1e-6));
      assert!(heard[end as usize - 1] <= full / fade as f32 + 1e-6);
    }

    // shorter than a fade: down from full over all of it
    let short = left(row.voice(60, 1., 48_000, 0, Some(100)));
    assert_eq!(short.len(), 100);
    assert!((short[0] - full).abs() < 1e-6 && short[99] < full / 50.);
    // past the sample's end: it ends there, as a one-shot would
    assert_eq!(left(row.voice(60, 1., 44_100, 0, Some(1_000_000))).len(), 44_100);
  }

  #[test]
  fn test_a_note_picked_up_partway_plays_the_rest_of_it() {
    let row = instrument(sweep(5_000), true);
    for (pitch, end) in [(60, Some(4_000)), (67, Some(2_500)), (55, None)] {
      let whole: Vec<f32> = row.voice(pitch, 0.7, 48_000, 0, end).collect();
      for lead in [1, 777, 2_399] {
        let rest: Vec<f32> = row.voice(pitch, 0.7, 48_000, lead, end).collect();
        assert!(rest == whole[lead as usize * 2..], "pitch {} from {}", pitch, lead);
      }
    }
    assert_eq!(row.voice(60, 1., 48_000, 4_000, Some(4_000)).count(), 0);
  }

  #[test]
  fn test_a_panned_row_turns_the_far_side_down() {
    let row = SampleInstrument { pan: -1., ..instrument(sweep(500), false) };
    let frames: Vec<f32> = row.voice(60, 1., 44_100, 0, None).collect();
    assert!(frames.chunks(2).all(|frame| frame[1] == 0.));
    assert!(frames.chunks(2).any(|frame| frame[0] != 0.));
    let half_right = SampleInstrument { pan: 0.5, ..row.clone() };
    let centered = SampleInstrument { pan: 0., ..row.clone() };
    let half: Vec<f32> = half_right.voice(60, 1., 44_100, 0, None).collect();
    let center: Vec<f32> = centered.voice(60, 1., 44_100, 0, None).collect();
    assert!(half.chunks(2).zip(center.chunks(2)).all(|(h, c)| h[1] == c[1] && h[0] == c[0] * 0.5));
  }

  #[test]
  fn test_a_held_key_fades_out_once_its_let_go_of() {
    let row = instrument(steady(44_100), true);
    let keys = HeldKeys::default();
    let (mut voice, release) = row.held(60, 1., 48_000);
    keys.hold(3, 60, release.unwrap());
    let mut heard: Vec<f32> = voice.by_ref().take(2_000).step_by(2).collect();

    // another key, and the same key on another row, aren't let go of
    keys.let_go(3, 61);
    keys.let_go(4, 60);
    heard.extend(voice.by_ref().take(2_000).step_by(2));
    assert_eq!(heard.len(), 2_000);
    keys.let_go(3, 60);
    heard.extend(voice.step_by(2));

    // 240 frames of fade (5ms), no step bigger than one of its own
    assert_eq!(heard.len(), 2_240);
    let full = 0.5;
    assert!(heard[..2_000].iter().all(|s| (s - full).abs() < 1e-6));
    assert!(heard.windows(2).all(|pair| (pair[1] - pair[0]).abs() <= full / 240. + 1e-6));

    // a one-shot row's key plays to the end whatever
    let one_shot = instrument(steady(1_000), false);
    let (voice, release) = one_shot.held(60, 1., 44_100);
    assert!(release.is_none());
    assert_eq!(voice.count(), 2_000);
  }

  #[test]
  fn test_keys_whose_voices_are_gone_are_forgotten() {
    let row = instrument(steady(100), true);
    let keys = HeldKeys::default();
    let (voice, release) = row.held(60, 1., 44_100);
    keys.hold(1, 60, release.unwrap());
    drop(voice);
    let (_playing, release) = row.held(62, 1., 44_100);
    keys.hold(1, 62, release.unwrap());
    let held = keys.0.lock().unwrap();
    assert_eq!(held.keys().copied().collect::<Vec<_>>(), vec![(1, 62)]);
  }
}
