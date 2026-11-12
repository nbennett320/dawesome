// the mixer: a channel strip for each playlist track, all summed into a master
// bus
//
// every clip's audio goes through its track's strip, and the whole mix through
// the master strip. the strips run in the audio engine, one on each track's
// bus (the sum of that track's clips) and one on the master bus (see
// engine.rs). they read their controls while the audio plays (shared atomics,
// no locks in the audio code), so moving a fader is heard straight away, even
// partway through a clip
//
// signal flow: clip settings (clip gain, pitch, fades, clip pan) → track bus
// → track effects → track strip (volume, pan, mute, solo) → master bus →
// master effects → master strip (volume) → limiter
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(test)]
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

// a track as the engine sees it: the bus its voices are summed on, and the
// controls of that bus's strip
#[derive(Clone, Debug)]
pub struct TrackRoute {
  pub number: u32,
  pub controls: Arc<ChannelControls>,
  // the mixer's count of soloed tracks
  pub solo_count: Arc<AtomicUsize>,
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

  // where a track's voices go in the engine: its bus, through its strip
  pub fn track_route(&self, track_number: u32) -> TrackRoute {
    TrackRoute {
      number: track_number,
      controls: self.track(track_number),
      solo_count: self.solo_count.clone(),
    }
  }

  // a source through a track's strip, a sample at a time (the engine runs
  // the strips on its buses instead)
  #[cfg(test)]
  pub fn track_strip<S: Source<Item = f32>>(&self, track_number: u32, source: S) -> ChannelStrip<S> {
    ChannelStrip::new(source, self.track(track_number), Some(self.solo_count.clone()))
  }

  // a source through the master strip
  #[cfg(test)]
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
  // a track's name, if it's been given one
  pub fn name(&self, track_number: u32) -> Option<String> {
    self.names.lock().unwrap().get(&track_number).cloned()
  }

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

  // only what's been changed: the tracks that aren't at their defaults or
  // have a name, and the master. `apply` puts the whole mixer back from it,
  // so it's what the undo history keeps of the mixer
  pub fn changed(&self) -> MixerSettings {
    let names = self.names.lock().unwrap().clone();
    let tracks = self.tracks.lock().unwrap();
    let mut numbers: Vec<u32> = tracks.keys().chain(names.keys()).copied().collect();
    numbers.sort_unstable();
    numbers.dedup();

    MixerSettings {
      tracks: numbers.into_iter()
        .map(|track_number| TrackMix {
          track_number,
          name: names.get(&track_number).cloned(),
          settings: tracks.get(&track_number).map(|controls| controls.settings()).unwrap_or_default(),
        })
        .filter(|track| track.name.is_some() || track.settings != ChannelSettings::default())
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

// a strip's workings: volume, pan, mute and solo, read from its controls as
// it plays, and the loudest sample for the meter. the engine runs one on each
// bus, a block at a time (ChannelStrip runs one on a source, in tests)
//
// pan uses the same balance law as a clip's pan: at the centre both sides are
// unchanged, and panning turns the opposite side down. sources that aren't
// stereo only get the volume
pub struct Strip {
  controls: Arc<ChannelControls>,
  // for track strips: how many tracks are soloed
  solo_count: Option<Arc<AtomicUsize>>,
  // whether it pans (only stereo does)
  stereo: bool,
  // where the volume of each side is heading, and where it is now
  target: [f32; 2],
  gain: [f32; 2],
  // how far towards the target each frame moves
  step: f32,
  // frames until the controls are read again
  until_read: u32,
  peak: f32,
}

impl Strip {
  pub fn new(
    controls: Arc<ChannelControls>,
    solo_count: Option<Arc<AtomicUsize>>,
    sample_rate: u32,
    stereo: bool,
  ) -> Self {
    let rate = sample_rate.max(1) as f32;
    let mut strip = Strip {
      controls,
      solo_count,
      stereo,
      target: [0.; 2],
      gain: [0.; 2],
      step: 1. - (-1. / (SMOOTHING.as_secs_f32() * rate)).exp(),
      until_read: CONTROL_FRAMES,
      peak: 0.,
    };

    // start at the right volume, so a clip's first hit isn't faded in
    strip.jump();
    strip
  }

  // go straight to the volume the controls are at now, without smoothing
  // (a bus that has been idle starts as a new strip would)
  pub fn jump(&mut self) {
    self.read_controls();
    self.gain = self.target;
    self.until_read = CONTROL_FRAMES;
  }

  pub fn controls(&self) -> &Arc<ChannelControls> {
    &self.controls
  }

  // play by other controls from now on. returns the old ones
  pub fn set_controls(
    &mut self,
    controls: Arc<ChannelControls>,
    solo_count: Option<Arc<AtomicUsize>>,
  ) -> (Arc<ChannelControls>, Option<Arc<AtomicUsize>>) {
    self.record_peak();
    let old_controls = std::mem::replace(&mut self.controls, controls);
    let old_solo_count = std::mem::replace(&mut self.solo_count, solo_count);
    (old_controls, old_solo_count)
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

    self.target = if self.stereo {
      [volume * (1. - pan).min(1.), volume * (1. + pan).min(1.)]
    } else {
      [volume, volume]
    };
  }

  // once per frame: read the controls now and then, and move the volume
  // towards where it's heading
  fn next_frame(&mut self) {
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

  // one sample of the frame, on `side`
  fn apply(&mut self, side: usize, sample: f32) -> f32 {
    let out = sample * self.gain[side];
    self.peak = self.peak.max(out.abs());
    out
  }

  // a block of stereo, in place
  pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
    for (l, r) in left.iter_mut().zip(right.iter_mut()) {
      self.next_frame();
      *l = self.apply(0, *l);
      *r = self.apply(1, *r);
    }
    // so the meter isn't kept waiting by a bus that goes quiet
    self.record_peak();
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

impl Drop for Strip {
  fn drop(&mut self) {
    self.record_peak();
  }
}

// plays a source through a strip, a sample at a time
#[cfg(test)]
pub struct ChannelStrip<S> {
  inner: S,
  strip: Strip,
  // the next sample's channel
  channel: u16,
}

#[cfg(test)]
impl<S: Source<Item = f32>> ChannelStrip<S> {
  pub fn new(inner: S, controls: Arc<ChannelControls>, solo_count: Option<Arc<AtomicUsize>>) -> Self {
    let strip = Strip::new(controls, solo_count, inner.sample_rate(), inner.channels() == 2);
    ChannelStrip { inner, strip, channel: 0 }
  }
}

#[cfg(test)]
impl<S: Source<Item = f32>> Iterator for ChannelStrip<S> {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    let sample = match self.inner.next() {
      Some(sample) => sample,
      None => {
        self.strip.record_peak();
        return None;
      }
    };
    let channels = self.inner.channels().max(1);

    if self.channel == 0 { self.strip.next_frame(); }

    let side = if channels == 2 { self.channel as usize } else { 0 };
    let out = self.strip.apply(side, sample);
    self.channel = (self.channel + 1) % channels;

    Some(out)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.inner.size_hint()
  }
}

#[cfg(test)]
impl<S: Source<Item = f32>> Source for ChannelStrip<S> {
  fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
  fn channels(&self) -> u16 { self.inner.channels() }
  fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

// the master's limiter: below LIMIT_KNEE the mix passes unchanged, above it
// peaks are rounded off smoothly so they never pass 1.0, instead of being
// cut off harshly by the output (the clip light still shows when this works)
const LIMIT_KNEE: f32 = 0.9;

pub fn soft_limit(sample: f32) -> f32 {
  let level = sample.abs();
  if level <= LIMIT_KNEE { return sample; }

  // the part above the knee is squeezed into what's left below 1.0
  let room = 1. - LIMIT_KNEE;
  let over = (level - LIMIT_KNEE) / room;
  (LIMIT_KNEE + room * over.tanh()).copysign(sample)
}

#[cfg(test)]
mod tests;
