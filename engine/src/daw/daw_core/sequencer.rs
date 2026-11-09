// the pattern sequencer: a drum machine. rows are samples, shared by every
// pattern, and each pattern lights some of each row's steps. steps are 16th
// notes, and a pattern is a whole number of bars long
//
// a pattern plays on its own in the Sequencer tab (see daw::run_playlist),
// and as pattern clips in the playlist (PatternNode in audiograph.rs), which
// stay linked to it: editing a pattern changes every clip of it
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use rodio::Source;
use serde::{Deserialize, Serialize};

use crate::daw;

// a step is a 16th note
pub const STEPS_PER_BEAT: u32 = 4;
// the lengths a pattern can be, in bars
pub const BAR_CHOICES: [u32; 3] = [1, 2, 4];
// a row's volume when it's added, as a gain (1 leaves the sample as is)
pub const ROW_VOLUME: f32 = 1.;
pub const ROW_VOLUME_MAX: f32 = 2.;

// a sample in the sequencer
#[derive(Clone)]
pub struct SequencerRow {
  pub id: u64,
  pub path: String,
  pub volume: f32,
  pub muted: bool,
  buffer: daw::SampleBuffer,
  // how long the sample plays for
  duration: Duration,
}

impl SequencerRow {
  // the row's sample at its volume, ready to be played once
  pub fn source(&self) -> impl Source<Item = f32> + Send + 'static {
    self.buffer.convert_samples().amplify(self.volume)
  }

  pub fn duration(&self) -> Duration {
    self.duration
  }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
  pub id: u64,
  // unnamed patterns are shown by their number
  pub name: Option<String>,
  // how many steps long the pattern is: its bars times the steps in a bar,
  // worked out when its length is set, so changing the time signature later
  // doesn't change patterns
  pub length: u32,
  // the steps lit on each row, by row id
  pub steps: BTreeMap<u64, BTreeSet<u32>>,
}

impl Pattern {
  // whether a row's step is lit
  pub fn lit(&self, row_id: u64, step: u32) -> bool {
    self.steps.get(&row_id).map_or(false, |s| s.contains(&step))
  }
}

// a row as the frontend sees it
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowInfo {
  pub id: u64,
  pub path: String,
  pub volume: f32,
  pub muted: bool,
}

// the whole sequencer, as the frontend sees it. every editing command
// returns it, so the frontend's copy can't drift from the engine's
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequencerState {
  pub rows: Vec<RowInfo>,
  pub patterns: Vec<Pattern>,
}

// a row as saved in a project (its path is made relative by project.rs)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRow {
  pub id: u64,
  pub path: String,
  #[serde(default = "default_row_volume")]
  pub volume: f32,
  #[serde(default)]
  pub muted: bool,
}

fn default_row_volume() -> f32 { ROW_VOLUME }

// the sequencer as saved in a project
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedSequencer {
  pub rows: Vec<SavedRow>,
  pub patterns: Vec<Pattern>,
}

impl SavedSequencer {
  pub fn is_empty(&self) -> bool {
    self.rows.is_empty() && self.patterns.is_empty()
  }
}

#[derive(Default)]
pub struct Sequencer {
  rows: Vec<SequencerRow>,
  patterns: Vec<Pattern>,
  next_row_id: u64,
  next_pattern_id: u64,
}

// steps in `bars` bars of `beats_per_bar` beats
pub fn steps_in_bars(bars: u32, beats_per_bar: u16) -> u32 {
  bars * beats_per_bar.max(1) as u32 * STEPS_PER_BEAT
}

// where step `n` starts, from the start of the pattern
pub fn step_offset(n: u32, beat_interval: Duration) -> Duration {
  Duration::from_nanos((beat_interval.as_nanos() * n as u128 / STEPS_PER_BEAT as u128) as u64)
}

// a pattern's length in bars. needs to be one of BAR_CHOICES
fn check_bars(bars: u32) -> Result<(), String> {
  if BAR_CHOICES.contains(&bars) { Ok(()) } else {
    Err(format!("a pattern can be {:?} bars long, not {}", BAR_CHOICES, bars))
  }
}

impl Sequencer {
  pub fn rows(&self) -> &[SequencerRow] {
    &self.rows
  }

  pub fn patterns(&self) -> &[Pattern] {
    &self.patterns
  }

  pub fn row(&self, id: u64) -> Option<&SequencerRow> {
    self.rows.iter().find(|r| r.id == id)
  }

  pub fn pattern(&self, id: u64) -> Option<&Pattern> {
    self.patterns.iter().find(|p| p.id == id)
  }

  fn pattern_mut(&mut self, id: u64) -> Result<&mut Pattern, String> {
    self.patterns.iter_mut().find(|p| p.id == id)
      .ok_or_else(|| format!("no pattern with id {}", id))
  }

  fn row_mut(&mut self, id: u64) -> Result<&mut SequencerRow, String> {
    self.rows.iter_mut().find(|r| r.id == id)
      .ok_or_else(|| format!("no sequencer row with id {}", id))
  }

  pub fn state(&self) -> SequencerState {
    SequencerState {
      rows: self.rows.iter().map(|r| RowInfo {
        id: r.id,
        path: r.path.clone(),
        volume: r.volume,
        muted: r.muted,
      }).collect(),
      patterns: self.patterns.clone(),
    }
  }

  // forget every row and pattern, e.g. before loading a project
  pub fn clear(&mut self) {
    self.rows.clear();
    self.patterns.clear();
  }

  // add a row playing the sample at `path`, at the bottom. returns its id
  pub fn add_row(&mut self, path: &str, sample_rate: u32) -> Result<u64, String> {
    let id = self.next_row_id;
    self.insert_row(SavedRow { id, path: path.to_string(), volume: ROW_VOLUME, muted: false }, sample_rate)?;
    Ok(id)
  }

  // add a row with a known id (from a project)
  fn insert_row(&mut self, saved: SavedRow, sample_rate: u32) -> Result<(), String> {
    if self.row(saved.id).is_some() {
      return Err(format!("there's already a sequencer row with id {}", saved.id));
    }
    // decoded first, so a file that isn't audio is refused here rather than
    // when it's played
    let (duration, _) = daw::preview_sample(&saved.path, sample_rate)?;
    let buffer = daw::SampleBuffer::load(&saved.path)
      .map_err(|e| format!("could not read sample '{}': {}", saved.path, e))?;

    self.next_row_id = self.next_row_id.max(saved.id + 1);
    self.rows.push(SequencerRow {
      id: saved.id,
      path: saved.path,
      volume: clamp_volume(saved.volume),
      muted: saved.muted,
      buffer,
      duration,
    });
    Ok(())
  }

  // take a row out, and its steps out of every pattern
  pub fn remove_row(&mut self, id: u64) -> Result<(), String> {
    let idx = self.rows.iter().position(|r| r.id == id)
      .ok_or_else(|| format!("no sequencer row with id {}", id))?;
    self.rows.remove(idx);
    for pattern in &mut self.patterns {
      pattern.steps.remove(&id);
    }
    Ok(())
  }

  // a row's volume as a gain, kept between 0 and ROW_VOLUME_MAX
  pub fn set_row_volume(&mut self, id: u64, volume: f32) -> Result<(), String> {
    self.row_mut(id)?.volume = clamp_volume(volume);
    Ok(())
  }

  pub fn set_row_muted(&mut self, id: u64, muted: bool) -> Result<(), String> {
    self.row_mut(id)?.muted = muted;
    Ok(())
  }

  // a new, empty, unnamed pattern of `bars` bars, at the end. returns its id
  pub fn add_pattern(&mut self, bars: u32, beats_per_bar: u16) -> Result<u64, String> {
    check_bars(bars)?;
    let id = self.next_pattern_id;
    self.insert_pattern(Pattern {
      id,
      name: None,
      length: steps_in_bars(bars, beats_per_bar),
      steps: BTreeMap::new(),
    })?;
    Ok(id)
  }

  // add a pattern with a known id (from a project)
  fn insert_pattern(&mut self, pattern: Pattern) -> Result<(), String> {
    if self.pattern(pattern.id).is_some() {
      return Err(format!("there's already a pattern with id {}", pattern.id));
    }
    if pattern.length == 0 {
      return Err(format!("pattern {} has no steps", pattern.id));
    }
    self.next_pattern_id = self.next_pattern_id.max(pattern.id + 1);
    self.patterns.push(pattern);
    Ok(())
  }

  // a copy of a pattern, named after it, straight after it. returns its id
  pub fn duplicate_pattern(&mut self, id: u64) -> Result<u64, String> {
    let idx = self.patterns.iter().position(|p| p.id == id)
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    let new_id = self.next_pattern_id;
    self.next_pattern_id += 1;

    let mut copy = self.patterns[idx].clone();
    copy.id = new_id;
    copy.name = copy.name.map(|n| format!("{} copy", n));
    self.patterns.insert(idx + 1, copy);
    Ok(new_id)
  }

  pub fn remove_pattern(&mut self, id: u64) -> Result<(), String> {
    let idx = self.patterns.iter().position(|p| p.id == id)
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    self.patterns.remove(idx);
    Ok(())
  }

  // name a pattern. a blank name goes back to showing its number
  pub fn rename_pattern(&mut self, id: u64, name: &str) -> Result<(), String> {
    let name = name.trim();
    self.pattern_mut(id)?.name = if name.is_empty() { None } else { Some(name.to_string()) };
    Ok(())
  }

  // make a pattern `bars` long. steps past a shortened pattern's end are
  // kept, so making it longer again brings them back
  pub fn set_pattern_bars(&mut self, id: u64, bars: u32, beats_per_bar: u16) -> Result<(), String> {
    check_bars(bars)?;
    self.pattern_mut(id)?.length = steps_in_bars(bars, beats_per_bar);
    Ok(())
  }

  // light or clear one step of a row in a pattern
  pub fn set_step(&mut self, pattern_id: u64, row_id: u64, step: u32, on: bool) -> Result<(), String> {
    if self.row(row_id).is_none() {
      return Err(format!("no sequencer row with id {}", row_id));
    }
    let pattern = self.pattern_mut(pattern_id)?;
    if step >= pattern.length {
      return Err(format!("pattern {} has {} steps, there's no step {}", pattern_id, pattern.length, step));
    }

    let steps = pattern.steps.entry(row_id).or_default();
    if on { steps.insert(step); } else { steps.remove(&step); }
    if steps.is_empty() { pattern.steps.remove(&row_id); }
    Ok(())
  }

  // how long a pattern plays for, at the beat length given
  pub fn pattern_len(&self, id: u64, beat_interval: Duration) -> Option<Duration> {
    self.pattern(id).map(|p| step_offset(p.length, beat_interval))
  }

  // every hit of a pattern, as (row, time from the pattern's start), in row
  // order. muted rows, and steps past the pattern's end, don't play
  pub fn hits(&self, pattern_id: u64, beat_interval: Duration) -> Vec<(&SequencerRow, Duration)> {
    let pattern = match self.pattern(pattern_id) {
      Some(p) => p,
      None => return Vec::new(),
    };

    self.rows.iter()
      .filter(|row| !row.muted)
      .flat_map(|row| pattern.steps.get(&row.id)
        .into_iter()
        .flatten()
        .filter(|&&step| step < pattern.length)
        .map(move |&step| (row, step_offset(step, beat_interval))))
      .collect()
  }

  // the sequencer for a project file
  pub fn saved(&self) -> SavedSequencer {
    SavedSequencer {
      rows: self.rows.iter().map(|r| SavedRow {
        id: r.id,
        path: r.path.clone(),
        volume: r.volume,
        muted: r.muted,
      }).collect(),
      patterns: self.patterns.clone(),
    }
  }

  // replace everything with a project's sequencer. rows whose sample can't
  // be loaded are left out (and their steps with them), and returned as
  // (path, reason). patterns that don't make sense (no steps, or an id used
  // twice) are left out
  pub fn restore(&mut self, saved: SavedSequencer, sample_rate: u32) -> Vec<(String, String)> {
    self.clear();
    let mut missing = Vec::new();

    for row in saved.rows {
      // past missing rows' ids too, so a new row never takes one
      self.next_row_id = self.next_row_id.max(row.id + 1);
      let path = row.path.clone();
      if let Err(e) = self.insert_row(row, sample_rate) {
        missing.push((path, e));
      }
    }
    for mut pattern in saved.patterns {
      pattern.steps.retain(|row_id, steps| self.rows.iter().any(|r| r.id == *row_id) && !steps.is_empty());
      let id = pattern.id;
      if let Err(e) = self.insert_pattern(pattern) {
        println!("left out pattern {}: {}", id, e);
      }
    }
    missing
  }
}

fn clamp_volume(volume: f32) -> f32 {
  if volume.is_finite() { volume.clamp(0., ROW_VOLUME_MAX) } else { ROW_VOLUME }
}

#[cfg(test)]
mod tests {
  use super::*;

  const BEAT: Duration = Duration::from_millis(500);
  const KICK: &str = "assets/assets_66-bd-01.wav";
  const SNARE: &str = "assets/assets_66-sd-01.wav";
  const RATE: u32 = 44_100;

  fn ms(n: u64) -> Duration { Duration::from_millis(n) }

  // a sequencer with a kick and a snare row, and a one bar pattern
  fn kit() -> (Sequencer, u64, u64, u64) {
    let mut seq = Sequencer::default();
    let kick = seq.add_row(KICK, RATE).unwrap();
    let snare = seq.add_row(SNARE, RATE).unwrap();
    let pattern = seq.add_pattern(1, 4).unwrap();
    (seq, kick, snare, pattern)
  }

  fn hit_times(seq: &Sequencer, pattern: u64) -> Vec<(u64, Duration)> {
    seq.hits(pattern, BEAT).into_iter().map(|(row, at)| (row.id, at)).collect()
  }

  #[test]
  fn test_steps_are_16th_notes() {
    assert_eq!(steps_in_bars(1, 4), 16);
    assert_eq!(steps_in_bars(2, 3), 24);
    assert_eq!(step_offset(0, BEAT), ms(0));
    assert_eq!(step_offset(1, BEAT), ms(125));
    assert_eq!(step_offset(16, BEAT), ms(2_000));
  }

  #[test]
  fn test_a_new_pattern_is_empty_and_its_bars_long() {
    let (seq, _, _, pattern) = kit();
    let p = seq.pattern(pattern).unwrap();
    assert_eq!(p.length, 16);
    assert_eq!(p.name, None);
    assert!(p.steps.is_empty());
    assert_eq!(seq.pattern_len(pattern, BEAT), Some(ms(2_000)));
  }

  #[test]
  fn test_bars_must_be_a_choice() {
    let (mut seq, _, _, pattern) = kit();
    assert!(seq.add_pattern(3, 4).is_err());
    assert!(seq.set_pattern_bars(pattern, 0, 4).is_err());
    seq.set_pattern_bars(pattern, 4, 4).unwrap();
    assert_eq!(seq.pattern(pattern).unwrap().length, 64);
  }

  #[test]
  fn test_hits_follow_the_lit_steps() {
    let (mut seq, kick, snare, pattern) = kit();
    for step in [0, 8] { seq.set_step(pattern, kick, step, true).unwrap(); }
    seq.set_step(pattern, snare, 4, true).unwrap();

    assert_eq!(hit_times(&seq, pattern), vec![(kick, ms(0)), (kick, ms(1_000)), (snare, ms(500))]);

    // clearing a step takes its hit out
    seq.set_step(pattern, kick, 8, false).unwrap();
    assert_eq!(hit_times(&seq, pattern), vec![(kick, ms(0)), (snare, ms(500))]);
  }

  #[test]
  fn test_muted_rows_dont_play() {
    let (mut seq, kick, snare, pattern) = kit();
    seq.set_step(pattern, kick, 0, true).unwrap();
    seq.set_step(pattern, snare, 4, true).unwrap();
    seq.set_row_muted(kick, true).unwrap();
    assert_eq!(hit_times(&seq, pattern), vec![(snare, ms(500))]);
  }

  #[test]
  fn test_steps_past_the_end_come_back() {
    let (mut seq, kick, _, pattern) = kit();
    seq.set_pattern_bars(pattern, 2, 4).unwrap();
    seq.set_step(pattern, kick, 20, true).unwrap();

    // shortened: the step is kept, but doesn't play
    seq.set_pattern_bars(pattern, 1, 4).unwrap();
    assert!(hit_times(&seq, pattern).is_empty());
    assert!(seq.set_step(pattern, kick, 20, true).is_err());

    seq.set_pattern_bars(pattern, 2, 4).unwrap();
    assert_eq!(hit_times(&seq, pattern), vec![(kick, ms(2_500))]);
  }

  #[test]
  fn test_removing_a_row_takes_its_steps() {
    let (mut seq, kick, snare, pattern) = kit();
    seq.set_step(pattern, kick, 0, true).unwrap();
    seq.set_step(pattern, snare, 4, true).unwrap();
    seq.remove_row(kick).unwrap();

    assert_eq!(seq.rows().len(), 1);
    assert!(!seq.pattern(pattern).unwrap().steps.contains_key(&kick));
    assert!(seq.set_step(pattern, kick, 0, true).is_err());
  }

  #[test]
  fn test_rename_and_duplicate() {
    let (mut seq, kick, _, pattern) = kit();
    seq.set_step(pattern, kick, 3, true).unwrap();
    seq.rename_pattern(pattern, "  verse ").unwrap();
    assert_eq!(seq.pattern(pattern).unwrap().name.as_deref(), Some("verse"));

    let other = seq.add_pattern(1, 4).unwrap();
    let copy = seq.duplicate_pattern(pattern).unwrap();
    // straight after the original, with its steps
    let order: Vec<u64> = seq.patterns().iter().map(|p| p.id).collect();
    assert_eq!(order, vec![pattern, copy, other]);
    assert_eq!(seq.pattern(copy).unwrap().name.as_deref(), Some("verse copy"));
    assert!(seq.pattern(copy).unwrap().lit(kick, 3));

    // the copy is its own pattern
    seq.set_step(copy, kick, 3, false).unwrap();
    assert!(seq.pattern(pattern).unwrap().lit(kick, 3));

    seq.rename_pattern(pattern, " ").unwrap();
    assert_eq!(seq.pattern(pattern).unwrap().name, None);
  }

  #[test]
  fn test_ids_arent_reused() {
    let (mut seq, kick, _, pattern) = kit();
    seq.remove_row(kick).unwrap();
    seq.remove_pattern(pattern).unwrap();
    assert_ne!(seq.add_row(KICK, RATE).unwrap(), kick);
    assert_ne!(seq.add_pattern(1, 4).unwrap(), pattern);
  }

  #[test]
  fn test_volume_is_kept_in_range() {
    let (mut seq, kick, _, _) = kit();
    seq.set_row_volume(kick, 9.).unwrap();
    assert_eq!(seq.row(kick).unwrap().volume, ROW_VOLUME_MAX);
    seq.set_row_volume(kick, f32::NAN).unwrap();
    assert_eq!(seq.row(kick).unwrap().volume, ROW_VOLUME);
  }

  #[test]
  fn test_a_missing_sample_is_an_error() {
    let mut seq = Sequencer::default();
    assert!(seq.add_row("assets/nope.wav", RATE).is_err());
    assert!(seq.rows().is_empty());
  }

  #[test]
  fn test_save_and_restore() {
    let (mut seq, kick, snare, pattern) = kit();
    seq.set_step(pattern, kick, 0, true).unwrap();
    seq.set_step(pattern, snare, 4, true).unwrap();
    seq.set_row_volume(snare, 0.5).unwrap();
    seq.rename_pattern(pattern, "beat").unwrap();

    let mut saved = seq.saved();
    // the snare's sample has gone missing
    saved.rows[1].path = "assets/nope.wav".into();

    let mut loaded = Sequencer::default();
    let missing = loaded.restore(saved, RATE);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "assets/nope.wav");
    assert_eq!(loaded.rows().len(), 1);
    // its steps went with it
    assert_eq!(hit_times(&loaded, pattern), vec![(kick, ms(0))]);
    assert_eq!(loaded.pattern(pattern).unwrap().name.as_deref(), Some("beat"));
    // new ids carry on after the loaded ones
    assert!(loaded.add_pattern(1, 4).unwrap() > pattern);
    assert!(loaded.add_row(KICK, RATE).unwrap() > snare);
  }

  #[test]
  fn test_state_names_match_the_frontend() {
    let (mut seq, kick, _, pattern) = kit();
    seq.set_step(pattern, kick, 2, true).unwrap();
    let json = serde_json::to_value(seq.state()).unwrap();
    assert_eq!(json["rows"][0]["path"], KICK);
    assert_eq!(json["rows"][0]["muted"], false);
    assert_eq!(json["patterns"][0]["length"], 16);
    assert_eq!(json["patterns"][0]["steps"][kick.to_string()], serde_json::json!([2]));
  }
}
