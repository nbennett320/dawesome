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
  // the folder it's in, in the sidebar's pattern browser (none: the top)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub folder: Option<u64>,
}

// a folder of patterns, in the sidebar's pattern browser. folders only sort
// patterns: they don't change how anything plays
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternFolder {
  pub id: u64,
  pub name: String,
  // the folder it's in (none: the top)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parent: Option<u64>,
}

// a folder's name, trimmed. it can't be blank, or too long to show
fn folder_name(name: &str) -> Result<String, String> {
  let name = name.trim();
  if name.is_empty() {
    return Err("a folder needs a name".into());
  }
  if name.chars().count() > 120 {
    return Err("that name is too long".into());
  }
  Ok(name.to_string())
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
  pub folders: Vec<PatternFolder>,
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
  // left out of projects saved before pattern folders
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub folders: Vec<PatternFolder>,
}

impl SavedSequencer {
  pub fn is_empty(&self) -> bool {
    self.rows.is_empty() && self.patterns.is_empty() && self.folders.is_empty()
  }
}

#[derive(Default, Clone)]
pub struct Sequencer {
  rows: Vec<SequencerRow>,
  patterns: Vec<Pattern>,
  folders: Vec<PatternFolder>,
  next_row_id: u64,
  next_pattern_id: u64,
  next_folder_id: u64,
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

  // sample files that have moved, from where to where (their sounds are
  // already loaded). returns whether any row's did
  pub fn move_row_samples(&mut self, moved: &[(String, String)]) -> bool {
    self.repoint_rows(&|path| moved.iter().find(|(from, _)| from == path).map(|(_, to)| to.clone()))
  }

  // sample files that have moved: `to` gives a path's new place, if it has
  // one. returns whether any row's did
  pub fn repoint_rows(&mut self, to: &dyn Fn(&str) -> Option<String>) -> bool {
    let mut any = false;
    for row in self.rows.iter_mut() {
      if let Some(path) = to(&row.path) {
        row.path = path;
        any = true;
      }
    }
    any
  }

  // whether another sequencer holds the same rows, patterns and folders
  // (for the history, to tell whether an edit changed anything)
  pub fn same_as(&self, other: &Sequencer) -> bool {
    let row = |r: &SequencerRow| (r.id, r.path.clone(), r.volume, r.muted);
    self.rows.iter().map(row).eq(other.rows.iter().map(row))
      && self.patterns == other.patterns
      && self.folders == other.folders
  }

  // go back to how another sequencer was (undo and redo). ids keep counting
  // up from wherever they'd got to, so one from before never names
  // something new
  pub fn restore_from(&mut self, other: &Sequencer) {
    self.rows = other.rows.clone();
    self.patterns = other.patterns.clone();
    self.folders = other.folders.clone();
    self.next_row_id = self.next_row_id.max(other.next_row_id);
    self.next_pattern_id = self.next_pattern_id.max(other.next_pattern_id);
    self.next_folder_id = self.next_folder_id.max(other.next_folder_id);
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
      folders: self.folders.clone(),
    }
  }

  // forget every row, pattern and folder, e.g. before loading a project
  pub fn clear(&mut self) {
    self.rows.clear();
    self.patterns.clear();
    self.folders.clear();
  }

  // add a row playing the sample at `path`, at the bottom. returns its id
  pub fn add_row(&mut self, path: &str) -> Result<u64, String> {
    self.add_loaded_row(path, RowSample::load(path)?)
  }

  // add a row playing a sample that's already been loaded (see RowSample),
  // at the bottom. returns its id
  pub fn add_loaded_row(&mut self, path: &str, sample: RowSample) -> Result<u64, String> {
    let id = self.next_row_id;
    self.insert_row(SavedRow { id, path: path.to_string(), volume: ROW_VOLUME, muted: false }, sample)?;
    Ok(id)
  }

  // add a row with a known id (from a project)
  fn insert_row(&mut self, saved: SavedRow, sample: RowSample) -> Result<(), String> {
    if self.row(saved.id).is_some() {
      return Err(format!("there's already a sequencer row with id {}", saved.id));
    }
    let RowSample { duration, buffer } = sample;

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
      folder: None,
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
    self.hit_steps(pattern_id).into_iter()
      .map(|(row, step)| (row, step_offset(step, beat_interval)))
      .collect()
  }

  // the same, as (row, step)
  pub fn hit_steps(&self, pattern_id: u64) -> Vec<(&SequencerRow, u32)> {
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
        .map(move |&step| (row, step)))
      .collect()
  }

  // the single sample a pattern plays, if it lights steps on just one row
  // (the patterns an explode makes)
  pub fn single_sample(&self, pattern_id: u64) -> Option<&str> {
    let pattern = self.pattern(pattern_id)?;
    let mut used = self.rows.iter()
      .filter(|row| pattern.steps.get(&row.id).map_or(false, |s| s.iter().any(|&st| st < pattern.length)));
    let row = used.next()?;
    if used.next().is_some() { None } else { Some(&row.path) }
  }

  // split a pattern into a pattern per row it uses, each holding just that
  // row's steps, at the same length. a pattern that already holds exactly
  // those steps (made by an earlier split, or by hand) is used rather than
  // making another; new ones go after the pattern, named like "verse (kick)", or "1 (kick)" unnamed.
  // returns (row's sample, pattern id), in row order. the pattern itself is
  // left as it is
  pub fn split_pattern(&mut self, id: u64) -> Result<Vec<(String, u64)>, String> {
    let source = self.pattern(id).cloned()
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    let label = source.name.clone().unwrap_or_else(|| (source.id + 1).to_string());

    let parts: Vec<(u64, String, BTreeSet<u32>)> = self.rows.iter()
      .filter_map(|row| {
        let steps: BTreeSet<u32> = source.steps.get(&row.id)?
          .iter().copied().filter(|&s| s < source.length).collect();
        if steps.is_empty() { None } else { Some((row.id, row.path.clone(), steps)) }
      })
      .collect();
    if parts.is_empty() {
      return Err(format!("pattern {} has no steps to explode", label));
    }

    let mut insert_at = self.patterns.iter().position(|p| p.id == id).expect("found above") + 1;
    let mut split = Vec::new();
    for (row_id, path, steps) in parts {
      let wanted: BTreeMap<u64, BTreeSet<u32>> = [(row_id, steps)].into_iter().collect();
      let existing = self.patterns.iter().find(|p| {
        p.length == source.length
          && p.steps.iter()
            .map(|(r, s)| (*r, s.iter().copied().filter(|&st| st < p.length).collect::<BTreeSet<u32>>()))
            .filter(|(_, s)| !s.is_empty())
            .collect::<BTreeMap<_, _>>() == wanted
      }).map(|p| p.id);

      let pattern_id = match existing {
        Some(found) => found,
        None => {
          let new_id = self.next_pattern_id;
          self.next_pattern_id += 1;
          self.patterns.insert(insert_at, Pattern {
            id: new_id,
            name: Some(format!("{} ({})", label, sample_name(&path))),
            length: source.length,
            steps: wanted,
            // next to the pattern it came from
            folder: source.folder,
          });
          insert_at += 1;
          new_id
        }
      };
      split.push((path, pattern_id));
    }
    Ok(split)
  }

  pub fn folders(&self) -> &[PatternFolder] {
    &self.folders
  }

  pub fn folder(&self, id: u64) -> Option<&PatternFolder> {
    self.folders.iter().find(|f| f.id == id)
  }

  fn folder_mut(&mut self, id: u64) -> Result<&mut PatternFolder, String> {
    self.folders.iter_mut().find(|f| f.id == id)
      .ok_or_else(|| format!("no pattern folder with id {}", id))
  }

  // a folder to put something in: none (the top), or one that's there
  fn check_folder(&self, folder: Option<u64>) -> Result<(), String> {
    match folder {
      Some(id) if self.folder(id).is_none() => Err(format!("no pattern folder with id {}", id)),
      _ => Ok(()),
    }
  }

  // whether `id` is `ancestor`, or inside it. parents that loop (which
  // restore doesn't allow) stop the search
  pub fn folder_within(&self, id: u64, ancestor: u64) -> bool {
    let mut at = Some(id);
    for _ in 0..=self.folders.len() {
      match at {
        Some(found) if found == ancestor => return true,
        Some(found) => at = self.folder(found).and_then(|f| f.parent),
        None => return false,
      }
    }
    false
  }

  // folder names don't repeat in a folder (case aside), as on disk
  // - except: a folder that can keep its name (it's the one being renamed)
  fn check_folder_name(&self, name: &str, parent: Option<u64>, except: Option<u64>) -> Result<(), String> {
    let taken = self.folders.iter()
      .any(|f| f.parent == parent && Some(f.id) != except && f.name.to_lowercase() == name.to_lowercase());
    if taken { Err(format!("there's already a folder called '{}' there", name)) } else { Ok(()) }
  }

  // a new, empty folder in `parent` (none: the top). returns its id
  pub fn add_folder(&mut self, name: &str, parent: Option<u64>) -> Result<u64, String> {
    let name = folder_name(name)?;
    self.check_folder(parent)?;
    self.check_folder_name(&name, parent, None)?;
    let id = self.next_folder_id;
    self.next_folder_id += 1;
    self.folders.push(PatternFolder { id, name, parent });
    Ok(id)
  }

  pub fn rename_folder(&mut self, id: u64, name: &str) -> Result<(), String> {
    let name = folder_name(name)?;
    let parent = self.folder(id).ok_or_else(|| format!("no pattern folder with id {}", id))?.parent;
    self.check_folder_name(&name, parent, Some(id))?;
    self.folder_mut(id)?.name = name;
    Ok(())
  }

  // move a folder (with what's in it) into another, or (none) to the top.
  // a folder can't go inside itself
  pub fn move_folder(&mut self, id: u64, parent: Option<u64>) -> Result<(), String> {
    let name = self.folder(id).ok_or_else(|| format!("no pattern folder with id {}", id))?.name.clone();
    self.check_folder(parent)?;
    if parent.map_or(false, |p| self.folder_within(p, id)) {
      return Err("a folder can't go inside itself".into());
    }
    self.check_folder_name(&name, parent, Some(id))?;
    self.folder_mut(id)?.parent = parent;
    Ok(())
  }

  // put a pattern in a folder, or (none) at the top
  pub fn move_pattern(&mut self, id: u64, folder: Option<u64>) -> Result<(), String> {
    self.check_folder(folder)?;
    self.pattern_mut(id)?.folder = folder;
    Ok(())
  }

  // the patterns in a folder, and in its folders
  pub fn patterns_in_folder(&self, id: u64) -> Vec<u64> {
    self.patterns.iter()
      .filter(|p| p.folder.map_or(false, |f| self.folder_within(f, id)))
      .map(|p| p.id)
      .collect()
  }

  // delete a folder, the folders in it, and every pattern in them. returns
  // the patterns deleted (their clips in the playlist are the caller's)
  pub fn remove_folder(&mut self, id: u64) -> Result<Vec<u64>, String> {
    if self.folder(id).is_none() {
      return Err(format!("no pattern folder with id {}", id));
    }
    let patterns = self.patterns_in_folder(id);
    let gone: Vec<u64> = self.folders.iter().filter(|f| self.folder_within(f.id, id)).map(|f| f.id).collect();
    self.patterns.retain(|p| !patterns.contains(&p.id));
    self.folders.retain(|f| !gone.contains(&f.id));
    Ok(patterns)
  }

  // a project's folders. one with an id used already is left out; one whose
  // parent isn't there, or whose parents loop, goes at the top
  fn restore_folders(&mut self, saved: Vec<PatternFolder>) {
    for folder in saved {
      if self.folder(folder.id).is_some() {
        println!("left out pattern folder {}: its id is used twice", folder.id);
        continue;
      }
      self.next_folder_id = self.next_folder_id.max(folder.id + 1);
      self.folders.push(folder);
    }
    let ids: Vec<u64> = self.folders.iter().map(|f| f.id).collect();
    for id in ids {
      let parent = self.folder(id).and_then(|f| f.parent);
      let lost = parent.map_or(false, |p| self.folder(p).is_none() || self.folder_within(p, id));
      if lost {
        if let Ok(folder) = self.folder_mut(id) { folder.parent = None; }
      }
    }
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
      folders: self.folders.clone(),
    }
  }

  // replace everything with a project's sequencer. rows whose sample can't
  // be loaded are left out (and their steps with them), and returned as
  // (path, reason). patterns that don't make sense (no steps, or an id used
  // twice) are left out
  pub fn restore(&mut self, saved: SavedSequencer) -> Vec<(String, String)> {
    let samples = RowSample::load_rows(&saved);
    self.restore_loaded(saved, samples)
  }

  // restore, with the rows' samples already loaded (see RowSample::load_rows)
  pub fn restore_loaded(
    &mut self,
    saved: SavedSequencer,
    samples: Vec<Result<RowSample, String>>,
  ) -> Vec<(String, String)> {
    self.clear();
    let mut missing = Vec::new();

    for (row, sample) in saved.rows.into_iter().zip(samples) {
      // past missing rows' ids too, so a new row never takes one
      self.next_row_id = self.next_row_id.max(row.id + 1);
      let path = row.path.clone();
      if let Err(e) = sample.and_then(|sample| self.insert_row(row, sample)) {
        missing.push((path, e));
      }
    }
    self.restore_folders(saved.folders);
    for mut pattern in saved.patterns {
      pattern.steps.retain(|row_id, steps| self.rows.iter().any(|r| r.id == *row_id) && !steps.is_empty());
      // a folder that isn't there: at the top
      if pattern.folder.map_or(false, |id| self.folder(id).is_none()) {
        pattern.folder = None;
      }
      let id = pattern.id;
      if let Err(e) = self.insert_pattern(pattern) {
        println!("left out pattern {}: {}", id, e);
      }
    }
    missing
  }
}

// a row's sample, decoded. loading one takes a while, so it's done without
// the sequencer, whose lock playback needs
pub struct RowSample {
  duration: Duration,
  buffer: daw::SampleBuffer,
}

impl RowSample {
  // decoded here, so a file that isn't audio is refused when it's added
  // rather than when it's played
  pub fn load(path: &str) -> Result<Self, String> {
    let (duration, _) = daw::preview_sample(path)?;
    let buffer = daw::SampleBuffer::load(path)
      .map_err(|e| format!("could not read sample '{}': {}", path, e))?;
    Ok(RowSample { duration, buffer })
  }

  // each of a saved sequencer's rows' samples, in order
  pub fn load_rows(saved: &SavedSequencer) -> Vec<Result<Self, String>> {
    saved.rows.iter().map(|row| Self::load(&row.path)).collect()
  }
}

// a sample's name: its file name without the extension
pub fn sample_name(path: &str) -> String {
  std::path::Path::new(path)
    .file_stem()
    .map(|s| s.to_string_lossy().into_owned())
    .unwrap_or_else(|| path.to_string())
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

  fn ms(n: u64) -> Duration { Duration::from_millis(n) }

  // a sequencer with a kick and a snare row, and a one bar pattern
  fn kit() -> (Sequencer, u64, u64, u64) {
    let mut seq = Sequencer::default();
    let kick = seq.add_row(KICK).unwrap();
    let snare = seq.add_row(SNARE).unwrap();
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
    assert_ne!(seq.add_row(KICK).unwrap(), kick);
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
    assert!(seq.add_row("assets/nope.wav").is_err());
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
    let missing = loaded.restore(saved);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "assets/nope.wav");
    assert_eq!(loaded.rows().len(), 1);
    // its steps went with it
    assert_eq!(hit_times(&loaded, pattern), vec![(kick, ms(0))]);
    assert_eq!(loaded.pattern(pattern).unwrap().name.as_deref(), Some("beat"));
    // new ids carry on after the loaded ones
    assert!(loaded.add_pattern(1, 4).unwrap() > pattern);
    assert!(loaded.add_row(KICK).unwrap() > snare);
  }

  #[test]
  fn test_folders() {
    let (mut seq, _, _, pattern) = kit();
    let drums = seq.add_folder("  Drums ", None).unwrap();
    let fills = seq.add_folder("Fills", Some(drums)).unwrap();
    assert_eq!(seq.folder(drums).unwrap().name, "Drums");
    assert_eq!(seq.folder(fills).unwrap().parent, Some(drums));

    // names: not blank, and not twice in a folder (case aside)
    assert!(seq.add_folder("  ", None).is_err());
    assert!(seq.add_folder("drums", None).is_err());
    assert!(seq.add_folder("Drums", Some(drums)).is_ok());
    assert!(seq.add_folder("x", Some(99)).is_err());
    assert!(seq.rename_folder(fills, "FILLS").is_ok());
    let other = seq.add_folder("Other", Some(drums)).unwrap();
    assert!(seq.rename_folder(other, "fills").is_err());

    // patterns go in folders, and back to the top
    seq.move_pattern(pattern, Some(fills)).unwrap();
    assert_eq!(seq.pattern(pattern).unwrap().folder, Some(fills));
    assert!(seq.move_pattern(pattern, Some(99)).is_err());
    assert_eq!(seq.patterns_in_folder(drums), vec![pattern]);

    // a copy goes in the same folder
    let copy = seq.duplicate_pattern(pattern).unwrap();
    assert_eq!(seq.pattern(copy).unwrap().folder, Some(fills));

    // folders move, but not inside themselves
    assert!(seq.move_folder(drums, Some(fills)).is_err());
    assert!(seq.move_folder(drums, Some(drums)).is_err());
    seq.move_folder(fills, None).unwrap();
    assert_eq!(seq.folder(fills).unwrap().parent, None);
    assert!(!seq.folder_within(fills, drums));
  }

  #[test]
  fn test_removing_a_folder_takes_what_is_in_it() {
    let (mut seq, _, _, pattern) = kit();
    let outside = seq.add_pattern(1, 4).unwrap();
    let drums = seq.add_folder("Drums", None).unwrap();
    let fills = seq.add_folder("Fills", Some(drums)).unwrap();
    let keep = seq.add_folder("Keep", None).unwrap();
    seq.move_pattern(pattern, Some(fills)).unwrap();

    assert_eq!(seq.remove_folder(drums).unwrap(), vec![pattern]);
    assert!(seq.folder(drums).is_none() && seq.folder(fills).is_none());
    assert!(seq.folder(keep).is_some());
    assert!(seq.pattern(pattern).is_none());
    assert!(seq.pattern(outside).is_some());
    assert!(seq.remove_folder(drums).is_err());
  }

  #[test]
  fn test_folders_are_saved_and_restored() {
    let (mut seq, _, _, pattern) = kit();
    let drums = seq.add_folder("Drums", None).unwrap();
    let fills = seq.add_folder("Fills", Some(drums)).unwrap();
    seq.move_pattern(pattern, Some(fills)).unwrap();

    let mut loaded = Sequencer::default();
    loaded.restore(seq.saved());
    assert_eq!(loaded.folders(), seq.folders());
    assert_eq!(loaded.pattern(pattern).unwrap().folder, Some(fills));
    // new ids carry on after the loaded ones
    assert!(loaded.add_folder("New", None).unwrap() > fills);
  }

  #[test]
  fn test_a_project_without_folders_still_loads() {
    let json = serde_json::json!({
      "rows": [],
      "patterns": [{ "id": 0, "name": null, "length": 16, "steps": {} }],
    });
    let saved: SavedSequencer = serde_json::from_value(json).unwrap();
    assert!(saved.folders.is_empty());
    let mut loaded = Sequencer::default();
    loaded.restore(saved);
    assert_eq!(loaded.pattern(0).unwrap().folder, None);

    // and one without folders is saved as before
    let json = serde_json::to_value(loaded.saved()).unwrap();
    assert!(json.get("folders").is_none());
    assert!(json["patterns"][0].get("folder").is_none());
  }

  #[test]
  fn test_broken_folders_are_mended_on_load() {
    let folder = |id, parent| PatternFolder { id, name: format!("f{}", id), parent };
    let pattern = Pattern { id: 0, name: None, length: 16, steps: BTreeMap::new(), folder: Some(42) };
    let saved = SavedSequencer {
      rows: vec![],
      patterns: vec![pattern],
      folders: vec![
        folder(1, Some(2)), folder(2, Some(1)), // a loop
        folder(3, Some(99)),                    // a parent that's gone
        folder(3, None),                        // an id used twice
      ],
    };
    let mut loaded = Sequencer::default();
    loaded.restore(saved);

    assert_eq!(loaded.folders().len(), 3);
    // the loop is broken, and each folder still reaches the top
    assert!(loaded.folders().iter().all(|f| !f.parent.map_or(false, |p| loaded.folder_within(p, f.id))));
    assert_eq!(loaded.folder(3).unwrap().parent, None);
    // the pattern's folder wasn't there: it's at the top
    assert_eq!(loaded.pattern(0).unwrap().folder, None);
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
    let folder = seq.add_folder("Drums", None).unwrap();
    seq.move_pattern(pattern, Some(folder)).unwrap();
    let json = serde_json::to_value(seq.state()).unwrap();
    assert_eq!(json["patterns"][0]["folder"], folder);
    assert_eq!(json["folders"][0], serde_json::json!({ "id": folder, "name": "Drums" }));
  }

  #[test]
  fn test_split_a_pattern_by_row() {
    let (mut seq, kick, snare, pattern) = kit();
    for step in [0, 8] { seq.set_step(pattern, kick, step, true).unwrap(); }
    seq.set_step(pattern, snare, 4, true).unwrap();
    seq.rename_pattern(pattern, "verse").unwrap();
    let after = seq.add_pattern(1, 4).unwrap();

    let split = seq.split_pattern(pattern).unwrap();
    assert_eq!(split.len(), 2);
    let (kick_path, kick_pattern) = &split[0];
    let (_, snare_pattern) = &split[1];
    assert_eq!(kick_path, KICK);

    // each holds just its row's steps, named after its sample, after the source
    let order: Vec<u64> = seq.patterns().iter().map(|p| p.id).collect();
    assert_eq!(order, vec![pattern, *kick_pattern, *snare_pattern, after]);
    let k = seq.pattern(*kick_pattern).unwrap();
    assert_eq!(k.name.as_deref(), Some("verse (assets_66-bd-01)"));
    assert_eq!(k.length, 16);
    assert!(k.lit(kick, 0) && k.lit(kick, 8) && !k.lit(snare, 4));
    assert_eq!(seq.single_sample(*snare_pattern), Some(SNARE));
    assert_eq!(seq.single_sample(pattern), None);

    // the source is unchanged, and splitting it again reuses the same patterns
    assert!(seq.pattern(pattern).unwrap().lit(snare, 4));
    assert_eq!(seq.split_pattern(pattern).unwrap(), split);
    assert_eq!(seq.patterns().len(), 4);
  }

  #[test]
  fn test_split_needs_steps() {
    let (mut seq, _, _, pattern) = kit();
    assert!(seq.split_pattern(pattern).is_err());
    assert!(seq.split_pattern(99).is_err());
  }
}
