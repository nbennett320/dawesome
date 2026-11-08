// the mixer: a channel strip for each playlist track, all summed into a master
// bus
//
// every clip's audio goes through its track's strip, and the whole mix through
// the master strip. clips are scheduled a beat ahead and play to their end
// once scheduled, so the strips read their controls while the audio plays
// (shared atomics, no locks in the audio code). moving a fader is heard
// straight away, even partway through a clip
//
// signal flow: clip settings (clip gain, pitch, fades, clip pan) → track
// strip (volume, pan, mute, solo) → master strip (volume) → limiter
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use rodio::Source;
use serde::{Deserialize, Serialize};

// the quietest a fader goes. at or below this it's silent (shown as -∞)
// (JSON has no infinity, so silence is stored as this)
pub const MIN_DB: f32 = -60.;
// the loudest a fader goes
pub const MAX_DB: f32 = 6.;
// the longest a track's name can be
pub const MAX_NAME_CHARS: usize = 40;

// how often a strip reads its controls, in frames
const CONTROL_FRAMES: u32 = 64;
// how long a strip takes to reach a new volume or pan, so changes don't click
const SMOOTHING: Duration = Duration::from_millis(10);

// the volume multiplier for a fader position in decibels
pub fn db_to_gain(db: f32) -> f32 {
  if db.is_nan() || db <= MIN_DB { 0. } else { 10f32.powf(db.min(MAX_DB) / 20.) }
}

// an f32 that can be shared between threads
#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
  pub fn new(value: f32) -> Self {
    AtomicF32(AtomicU32::new(value.to_bits()))
  }

  pub fn load(&self) -> f32 {
    f32::from_bits(self.0.load(Ordering::Relaxed))
  }

  pub fn store(&self, value: f32) {
    self.0.store(value.to_bits(), Ordering::Relaxed)
  }

  // keep the larger of the stored value and `value` (both must be 0 or more:
  // for those, a larger float has larger bits)
  pub fn fetch_max(&self, value: f32) {
    self.0.fetch_max(value.max(0.).to_bits(), Ordering::Relaxed);
  }

  // read the value and set it back to 0
  pub fn take(&self) -> f32 {
    f32::from_bits(self.0.swap(0f32.to_bits(), Ordering::Relaxed))
  }
}

// a strip's settings, as the frontend and project files see them
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ChannelSettings {
  // fader position in decibels, MIN_DB (silent) to MAX_DB
  pub volume_db: f32,
  // -1 is hard left, 0 is centre, 1 is hard right
  pub pan: f32,
  pub mute: bool,
  pub solo: bool,
}

impl Default for ChannelSettings {
  fn default() -> Self {
    ChannelSettings { volume_db: 0., pan: 0., mute: false, solo: false }
  }
}

fn clamp(value: f32, min: f32, max: f32, default: f32) -> f32 {
  if value.is_nan() { default } else { value.clamp(min, max) }
}

// a strip's live controls, read by the audio while it plays
#[derive(Debug)]
pub struct ChannelControls {
  volume_db: AtomicF32,
  pan: AtomicF32,
  mute: AtomicBool,
  solo: AtomicBool,
  // the loudest sample since the meter was last read
  peak: AtomicF32,
}

impl Default for ChannelControls {
  fn default() -> Self {
    let d = ChannelSettings::default();
    ChannelControls {
      volume_db: AtomicF32::new(d.volume_db),
      pan: AtomicF32::new(d.pan),
      mute: AtomicBool::new(d.mute),
      solo: AtomicBool::new(d.solo),
      peak: AtomicF32::new(0.),
    }
  }
}

impl ChannelControls {
  pub fn settings(&self) -> ChannelSettings {
    ChannelSettings {
      volume_db: self.volume_db.load(),
      pan: self.pan.load(),
      mute: self.mute.load(Ordering::Relaxed),
      solo: self.solo.load(Ordering::Relaxed),
    }
  }

  pub fn volume_db(&self) -> f32 {
    self.volume_db.load()
  }

  pub fn pan(&self) -> f32 {
    self.pan.load()
  }

  pub fn muted(&self) -> bool {
    self.mute.load(Ordering::Relaxed)
  }

  pub fn soloed(&self) -> bool {
    self.solo.load(Ordering::Relaxed)
  }

  // the loudest sample since the last call, then start again
  pub fn take_peak(&self) -> f32 {
    self.peak.take()
  }

  fn set_volume_db(&self, db: f32) -> f32 {
    let db = clamp(db, MIN_DB, MAX_DB, 0.);
    self.volume_db.store(db);
    db
  }

  fn set_pan(&self, pan: f32) -> f32 {
    let pan = clamp(pan, -1., 1., 0.);
    self.pan.store(pan);
    pan
  }

  fn set_mute(&self, mute: bool) {
    self.mute.store(mute, Ordering::Relaxed);
  }

  // returns whether it changed, so the mixer can count soloed tracks
  fn set_solo(&self, solo: bool) -> bool {
    self.solo.swap(solo, Ordering::Relaxed) != solo
  }
}

// every track's strip and the master strip
#[derive(Debug, Default)]
pub struct Mixer {
  // made the first time a track is used
  tracks: Mutex<BTreeMap<u32, Arc<ChannelControls>>>,
  master: Arc<ChannelControls>,
  // how many tracks are soloed. while any are, the others are silent
  solo_count: Arc<AtomicUsize>,
  // names given to tracks. unnamed tracks are shown as "Track n"
  names: Mutex<BTreeMap<u32, String>>,
}

// the whole mixer's settings, as the frontend and project files see them
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MixerSettings {
  pub tracks: Vec<TrackMix>,
  pub master: ChannelSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackMix {
  pub track_number: u32,
  // the name it was given, if any
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(flatten)]
  pub settings: ChannelSettings,
}

// each strip's meter reading, 0 to 1 (above 1 is clipping)
#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MixerLevels {
  // [track number, peak]
  pub tracks: Vec<(u32, f32)>,
  pub master: f32,
}

impl Mixer {
  pub fn track(&self, track_number: u32) -> Arc<ChannelControls> {
    self.tracks.lock().unwrap().entry(track_number).or_default().clone()
  }

  pub fn master(&self) -> Arc<ChannelControls> {
    self.master.clone()
  }

  // a clip's audio, through its track's strip
  pub fn track_strip<S: Source<Item = f32>>(&self, track_number: u32, source: S) -> ChannelStrip<S> {
    ChannelStrip::new(source, self.track(track_number), Some(self.solo_count.clone()))
  }

  // the whole mix, through the master strip
  pub fn master_strip<S: Source<Item = f32>>(&self, source: S) -> ChannelStrip<S> {
    ChannelStrip::new(source, self.master(), None)
  }

  pub fn set_volume_db(&self, track_number: u32, db: f32) -> f32 {
    self.track(track_number).set_volume_db(db)
  }

  pub fn set_pan(&self, track_number: u32, pan: f32) -> f32 {
    self.track(track_number).set_pan(pan)
  }

  pub fn set_mute(&self, track_number: u32, mute: bool) {
    self.track(track_number).set_mute(mute)
  }

  pub fn set_solo(&self, track_number: u32, solo: bool) {
    if self.track(track_number).set_solo(solo) {
      if solo {
        self.solo_count.fetch_add(1, Ordering::Relaxed);
      } else {
        self.solo_count.fetch_sub(1, Ordering::Relaxed);
      }
    }
  }

  pub fn set_master_volume_db(&self, db: f32) -> f32 {
    self.master.set_volume_db(db)
  }

  // name a track, or clear its name with an empty one. returns the name kept
  pub fn set_name(&self, track_number: u32, name: &str) -> Option<String> {
    let name: String = name.trim().chars().take(MAX_NAME_CHARS).collect();
    let mut names = self.names.lock().unwrap();

    if name.is_empty() {
      names.remove(&track_number);
      None
    } else {
      names.insert(track_number, name.clone());
      Some(name)
    }
  }

  // the settings of every track from 0 to `track_count` (and any others that
  // have been used), and the master
  pub fn settings(&self, track_count: u32) -> MixerSettings {
    for track_number in 0..track_count {
      self.track(track_number);
    }

    // named tracks are listed even if they haven't been used yet
    let names = self.names.lock().unwrap().clone();
    for &track_number in names.keys() {
      self.track(track_number);
    }

    let tracks = self.tracks.lock().unwrap();
    MixerSettings {
      tracks: tracks.iter()
        .map(|(&track_number, controls)| TrackMix {
          track_number,
          name: names.get(&track_number).cloned(),
          settings: controls.settings(),
        })
        .collect(),
      master: self.master.settings(),
    }
  }

  // set every strip and name, e.g. from a project. tracks not listed go back
  // to their defaults, without a name
  pub fn apply(&self, settings: &MixerSettings) {
    self.reset();
    self.names.lock().unwrap().clear();

    for track in &settings.tracks {
      self.set_channel(track.track_number, track.settings);
      if let Some(name) = &track.name {
        self.set_name(track.track_number, name);
      }
    }
    self.master.set_volume_db(settings.master.volume_db);
  }

  fn set_channel(&self, track_number: u32, s: ChannelSettings) {
    self.set_volume_db(track_number, s.volume_db);
    self.set_pan(track_number, s.pan);
    self.set_mute(track_number, s.mute);
    self.set_solo(track_number, s.solo);
  }

  // every strip back to its defaults (names are kept)
  pub fn reset(&self) {
    let track_numbers: Vec<u32> = self.tracks.lock().unwrap().keys().copied().collect();
    for track_number in track_numbers {
      self.set_channel(track_number, ChannelSettings::default());
    }
    self.master.set_volume_db(ChannelSettings::default().volume_db);
  }

  // each strip's loudest sample since the last call
  pub fn take_levels(&self) -> MixerLevels {
    let tracks = self.tracks.lock().unwrap();
    MixerLevels {
      tracks: tracks.iter().map(|(&n, controls)| (n, controls.take_peak())).collect(),
      master: self.master.take_peak(),
    }
  }
}

// plays a source through a strip: volume, pan, mute and solo, read from the
// strip's controls as it plays. records the loudest sample for the meter
//
// pan uses the same balance law as a clip's pan: at the centre both sides are
// unchanged, and panning turns the opposite side down. sources that aren't
// stereo only get the volume
pub struct ChannelStrip<S> {
  inner: S,
  controls: Arc<ChannelControls>,
  // for track strips: how many tracks are soloed
  solo_count: Option<Arc<AtomicUsize>>,
  // where the volume of each side is heading, and where it is now
  target: [f32; 2],
  gain: [f32; 2],
  // how far towards the target each frame moves
  step: f32,
  // frames until the controls are read again
  until_read: u32,
  // the next sample's channel
  channel: u16,
  peak: f32,
}

impl<S: Source<Item = f32>> ChannelStrip<S> {
  pub fn new(inner: S, controls: Arc<ChannelControls>, solo_count: Option<Arc<AtomicUsize>>) -> Self {
    let rate = inner.sample_rate().max(1) as f32;
    let mut strip = ChannelStrip {
      inner,
      controls,
      solo_count,
      target: [0.; 2],
      gain: [0.; 2],
      step: 1. - (-1. / (SMOOTHING.as_secs_f32() * rate)).exp(),
      until_read: CONTROL_FRAMES,
      channel: 0,
      peak: 0.,
    };

    // start at the right volume, so a clip's first hit isn't faded in
    strip.read_controls();
    strip.gain = strip.target;
    strip
  }

  // whether this strip is silenced by its mute, or by another track's solo
  fn silenced(&self) -> bool {
    if self.controls.muted() { return true; }

    match &self.solo_count {
      Some(count) => count.load(Ordering::Relaxed) > 0 && !self.controls.soloed(),
      None => false,
    }
  }

  fn read_controls(&mut self) {
    let volume = if self.silenced() { 0. } else { db_to_gain(self.controls.volume_db()) };
    let pan = self.controls.pan();

    self.target = if self.inner.channels() == 2 {
      [volume * (1. - pan).min(1.), volume * (1. + pan).min(1.)]
    } else {
      [volume, volume]
    };
  }

  // the meter reads the loudest sample in batches, so the audio isn't
  // writing to shared memory on every sample
  fn record_peak(&mut self) {
    if self.peak > 0. {
      self.controls.peak.fetch_max(self.peak);
      self.peak = 0.;
    }
  }
}

impl<S: Source<Item = f32>> Iterator for ChannelStrip<S> {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    let sample = match self.inner.next() {
      Some(sample) => sample,
      None => {
        self.record_peak();
        return None;
      }
    };
    let channels = self.inner.channels().max(1);

    // once per frame: read the controls now and then, and move the volume
    // towards where it's heading
    if self.channel == 0 {
      self.until_read -= 1;
      if self.until_read == 0 {
        self.until_read = CONTROL_FRAMES;
        self.read_controls();
        self.record_peak();
      }

      for side in 0..2 {
        self.gain[side] += (self.target[side] - self.gain[side]) * self.step;
      }
    }

    let side = if channels == 2 { self.channel as usize } else { 0 };
    let out = sample * self.gain[side];

    self.peak = self.peak.max(out.abs());
    self.channel = (self.channel + 1) % channels;

    Some(out)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.inner.size_hint()
  }
}

impl<S: Source<Item = f32>> Source for ChannelStrip<S> {
  fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
  fn channels(&self) -> u16 { self.inner.channels() }
  fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

impl<S> Drop for ChannelStrip<S> {
  fn drop(&mut self) {
    if self.peak > 0. {
      self.controls.peak.fetch_max(self.peak);
    }
  }
}

// the master's limiter: below LIMIT_KNEE the mix passes unchanged, above it
// peaks are rounded off smoothly so they never pass 1.0, instead of being
// cut off harshly by the output (the clip light still shows when this works)
pub struct SoftLimit<S> {
  inner: S,
}

const LIMIT_KNEE: f32 = 0.9;

impl<S> SoftLimit<S> {
  pub fn new(inner: S) -> Self {
    SoftLimit { inner }
  }
}

pub fn soft_limit(sample: f32) -> f32 {
  let level = sample.abs();
  if level <= LIMIT_KNEE { return sample; }

  // the part above the knee is squeezed into what's left below 1.0
  let room = 1. - LIMIT_KNEE;
  let over = (level - LIMIT_KNEE) / room;
  (LIMIT_KNEE + room * over.tanh()).copysign(sample)
}

impl<S: Source<Item = f32>> Iterator for SoftLimit<S> {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    self.inner.next().map(soft_limit)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.inner.size_hint()
  }
}

impl<S: Source<Item = f32>> Source for SoftLimit<S> {
  fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
  fn channels(&self) -> u16 { self.inner.channels() }
  fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

// plays a source until `stop` is set, then fades it out over a few
// milliseconds and ends, so stopping playback doesn't click
pub struct FadeOnStop<S> {
  inner: S,
  stop: Arc<AtomicBool>,
  // frames left in the fade, once it has started
  fade_left: Option<u32>,
  fade_frames: u32,
  channel: u16,
}

// how long stopping takes to fade out
const STOP_FADE: Duration = Duration::from_millis(10);

impl<S: Source<Item = f32>> FadeOnStop<S> {
  pub fn new(inner: S, stop: Arc<AtomicBool>) -> Self {
    let fade_frames = (STOP_FADE.as_secs_f32() * inner.sample_rate() as f32).max(1.) as u32;
    FadeOnStop { inner, stop, fade_left: None, fade_frames, channel: 0 }
  }
}

impl<S: Source<Item = f32>> Iterator for FadeOnStop<S> {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    let channels = self.inner.channels().max(1);

    if self.channel == 0 {
      match self.fade_left {
        Some(0) => return None,
        Some(left) => self.fade_left = Some(left - 1),
        None if self.stop.load(Ordering::Relaxed) => self.fade_left = Some(self.fade_frames),
        None => {}
      }
    }

    let sample = self.inner.next()?;
    self.channel = (self.channel + 1) % channels;

    Some(match self.fade_left {
      Some(left) => sample * left as f32 / self.fade_frames as f32,
      None => sample,
    })
  }
}

impl<S: Source<Item = f32>> Source for FadeOnStop<S> {
  fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
  fn channels(&self) -> u16 { self.inner.channels() }
  fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { None }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rodio::buffer::SamplesBuffer;

  #[test]
  fn test_soft_limit() {
    // quiet and moderate levels pass unchanged
    assert_eq!(soft_limit(0.5), 0.5);
    assert_eq!(soft_limit(-0.9), -0.9);
    // louder ones are rounded off below 1, keeping their sign and order
    assert!(soft_limit(1.2) < 1. && soft_limit(1.2) > 0.9);
    assert!(soft_limit(4.) > soft_limit(1.2));
    assert!(soft_limit(100.) <= 1.);
    assert_eq!(soft_limit(-1.2), -soft_limit(1.2));
    // smooth at the knee
    assert!((soft_limit(0.9001) - 0.9001).abs() < 1e-4);
  }

  #[test]
  fn test_fade_on_stop() {
    let stop = Arc::new(AtomicBool::new(false));
    // stereo at 1kHz: a 10 frame fade
    let mut out = FadeOnStop::new(ones(2, 1000), stop.clone());

    assert!(out.by_ref().take(100).all(|s| s == 1.));
    stop.store(true, Ordering::Relaxed);
    let rest: Vec<f32> = out.collect();

    // ends after the fade, long before the source does
    assert_eq!(rest.len(), 22);
    assert_eq!(rest[0], 1.);
    assert_eq!(rest[2], rest[3]);
    assert!(rest[2] < 1. && rest[2] > 0.8);
    assert_eq!(rest[20], 0.);
  }

  // `frames` frames of 1.0 at 1kHz
  fn ones(channels: u16, frames: usize) -> SamplesBuffer<f32> {
    SamplesBuffer::new(channels, 1000, vec![1.; frames * channels as usize])
  }

  fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
  }

  #[test]
  fn test_db_to_gain() {
    assert_eq!(db_to_gain(0.), 1.);
    assert!(close(db_to_gain(-20.), 0.1));
    assert!(close(db_to_gain(6.), 1.995));
    // silent at the bottom, and never louder than the top
    assert_eq!(db_to_gain(MIN_DB), 0.);
    assert_eq!(db_to_gain(f32::NAN), 0.);
    assert_eq!(db_to_gain(40.), db_to_gain(MAX_DB));
  }

  #[test]
  fn test_atomic_f32() {
    let peak = AtomicF32::new(0.);
    peak.fetch_max(0.5);
    peak.fetch_max(0.25);
    peak.fetch_max(-3.);
    assert_eq!(peak.load(), 0.5);
    assert_eq!(peak.take(), 0.5);
    assert_eq!(peak.load(), 0.);
  }

  #[test]
  fn test_unchanged_at_the_defaults() {
    let mixer = Mixer::default();
    let source = SamplesBuffer::new(2, 1000, vec![0.1, 0.9, -0.2, 0.8]);
    let out: Vec<f32> = mixer.master_strip(mixer.track_strip(0, source)).collect();

    assert_eq!(out, vec![0.1, 0.9, -0.2, 0.8]);
  }

  #[test]
  fn test_volume_pan_and_mute_start_straight_away() {
    let mixer = Mixer::default();
    mixer.set_volume_db(0, -20.);
    mixer.set_pan(1, 0.5);
    mixer.set_mute(2, true);

    let first = |track: u32| -> Vec<f32> { mixer.track_strip(track, ones(2, 4)).take(2).collect() };
    let quiet = first(0);
    assert!(close(quiet[0], 0.1) && close(quiet[1], 0.1));
    assert_eq!(first(1), vec![0.5, 1.]);
    assert_eq!(first(2), vec![0., 0.]);

    // mono only gets the volume
    let mono: Vec<f32> = mixer.track_strip(1, ones(1, 2)).collect();
    assert_eq!(mono, vec![1., 1.]);
  }

  #[test]
  fn test_solo_silences_the_other_tracks() {
    let mixer = Mixer::default();
    mixer.set_solo(1, true);
    // soloing twice still counts once
    mixer.set_solo(1, true);

    let level = |track: u32| -> f32 { mixer.track_strip(track, ones(2, 1)).next().unwrap() };
    assert_eq!(level(0), 0.);
    assert_eq!(level(1), 1.);

    mixer.set_solo(1, false);
    assert_eq!(level(0), 1.);
  }

  #[test]
  fn test_changes_are_heard_while_playing_without_a_click() {
    let mixer = Mixer::default();
    // 1s of audio at 1kHz
    let mut strip = mixer.track_strip(0, ones(2, 1000));

    let start: Vec<f32> = strip.by_ref().take(200).collect();
    assert!(start.iter().all(|&s| s == 1.));

    mixer.set_mute(0, true);
    let rest: Vec<f32> = strip.collect();

    // it fades out over a few samples rather than jumping to silence
    let lefts: Vec<f32> = rest.iter().step_by(2).copied().collect();
    let first_change = lefts.iter().position(|&s| s < 1.).unwrap();
    assert!(first_change <= CONTROL_FRAMES as usize);
    assert!(lefts[first_change] > 0.5);
    assert!(lefts.windows(2).all(|w| w[1] <= w[0]));
    assert!(*lefts.last().unwrap() < 1e-3);
  }

  #[test]
  fn test_master_volume() {
    let mixer = Mixer::default();
    mixer.set_master_volume_db(-20.);
    let out: Vec<f32> = mixer.master_strip(ones(2, 1)).collect();
    assert!(close(out[0], 0.1));
  }

  #[test]
  fn test_meters_record_the_loudest_sample() {
    let mixer = Mixer::default();
    mixer.set_volume_db(0, -20.);
    let source = SamplesBuffer::new(1, 1000, vec![0.5, -1., 0.25]);
    let _: Vec<f32> = mixer.master_strip(mixer.track_strip(0, source)).collect();

    let levels = mixer.take_levels();
    assert_eq!(levels.tracks.len(), 1);
    assert!(close(levels.tracks[0].1, 0.1));
    assert!(close(levels.master, 0.1));
    // reading the meters starts them again
    assert_eq!(mixer.take_levels().master, 0.);
  }

  #[test]
  fn test_settings_and_apply() {
    let mixer = Mixer::default();
    mixer.set_volume_db(1, -6.);
    mixer.set_solo(1, true);
    mixer.set_master_volume_db(3.);

    let settings = mixer.settings(3);
    assert_eq!(settings.tracks.len(), 3);
    assert_eq!(settings.tracks[1].settings, ChannelSettings { volume_db: -6., pan: 0., mute: false, solo: true });

    // a project with only track 2 changed: track 1 goes back to its defaults
    let other = Mixer::default();
    other.set_pan(2, -1.);
    mixer.apply(&other.settings(0));
    let applied = mixer.settings(3);
    assert_eq!(applied.tracks[1].settings, ChannelSettings::default());
    assert_eq!(applied.tracks[2].settings.pan, -1.);
    assert_eq!(applied.master.volume_db, 0.);
    assert_eq!(mixer.solo_count.load(Ordering::Relaxed), 0);
  }

  #[test]
  fn test_track_names() {
    let mixer = Mixer::default();
    assert_eq!(mixer.set_name(5, "  Drums  "), Some("Drums".into()));
    assert_eq!(mixer.set_name(1, &"x".repeat(100)).unwrap().len(), MAX_NAME_CHARS);

    // a named track is listed even before it's used
    let settings = mixer.settings(2);
    assert_eq!(settings.tracks.len(), 3);
    assert_eq!(settings.tracks[2].name.as_deref(), Some("Drums"));

    // resetting the mix keeps names, a project replaces them
    mixer.reset();
    assert_eq!(mixer.settings(0).tracks[2].name.as_deref(), Some("Drums"));
    mixer.apply(&MixerSettings::default());
    assert!(mixer.settings(0).tracks.iter().all(|t| t.name.is_none()));

    mixer.set_name(5, "Bass");
    assert_eq!(mixer.set_name(5, "   "), None);
    assert!(mixer.settings(0).tracks.iter().all(|t| t.name.is_none()));
  }

  #[test]
  fn test_values_kept_in_range() {
    let mixer = Mixer::default();
    assert_eq!(mixer.set_volume_db(0, 40.), MAX_DB);
    assert_eq!(mixer.set_volume_db(0, -500.), MIN_DB);
    assert_eq!(mixer.set_volume_db(0, f32::NAN), 0.);
    assert_eq!(mixer.set_pan(0, 3.), 1.);
  }

  #[test]
  fn test_serde_names_match_the_frontend() {
    let settings = MixerSettings {
      tracks: vec![
        TrackMix { track_number: 0, name: None, settings: ChannelSettings::default() },
        TrackMix { track_number: 1, name: Some("Drums".into()), settings: ChannelSettings::default() },
      ],
      master: ChannelSettings::default(),
    };
    assert_eq!(serde_json::to_value(&settings).unwrap(), serde_json::json!({
      "tracks": [
        { "trackNumber": 0, "volumeDb": 0., "pan": 0., "mute": false, "solo": false },
        { "trackNumber": 1, "name": "Drums", "volumeDb": 0., "pan": 0., "mute": false, "solo": false },
      ],
      "master": { "volumeDb": 0., "pan": 0., "mute": false, "solo": false },
    }));
  }
}
