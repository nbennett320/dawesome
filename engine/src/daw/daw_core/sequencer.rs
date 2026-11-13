// the pattern sequencer. a pattern is a drum pattern or a note pattern,
// never both, and a folder of patterns holds patterns of one kind
//
// a drum pattern is a drum machine's: rows are samples, shared by every drum
// pattern, and each pattern lights some of each row's steps. steps are 16th
// notes, and a drum pattern is 1, 2 or 4 bars long
//
// a note pattern holds notes (pitch, start, length, velocity) for each
// synth, timed in steps that needn't be whole. the synths (instrument
// plugins) are one list shared by every note pattern, as the rows are by
// every drum pattern; their plugins live in effects.rs, which keeps them in
// line with this list. a synth's notes are kept in the order the frontend
// sent them, which it tells them apart by, and notes past the pattern's end
// are kept but not played. a note pattern is 1 to 64 bars long
//
// a pattern plays on its own in the sequencer tabs (see daw::run_playlist),
// and as pattern clips in the playlist (PatternNode in audiograph.rs), which
// stay linked to it: editing a pattern changes every clip of it
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use rodio::Source;
use serde::{Deserialize, Serialize};

use crate::daw;

// a step is a 16th note
pub const STEPS_PER_BEAT: u32 = 4;
// the lengths a drum pattern can be, in bars
pub const BAR_CHOICES: [u32; 3] = [1, 2, 4];
// the longest a note pattern can be, in bars
pub const NOTE_BARS_MAX: u32 = 64;
// a row's volume when it's added, as a gain (1 leaves the sample as is)
pub const ROW_VOLUME: f32 = 1.;
pub const ROW_VOLUME_MAX: f32 = 2.;
// the longest a row's label can be, in characters
pub const ROW_LABEL_MAX: usize = 40;

// a sample in the sequencer
#[derive(Clone)]
pub struct SequencerRow {
  pub id: u64,
  pub path: String,
  // what the row is called ("Snare"), shown instead of its sample's name
  pub label: Option<String>,
  pub volume: f32,
  // -1 (left) to 1 (right)
  pub pan: f32,
  pub muted: bool,
  buffer: daw::SampleBuffer,
  // how long the sample plays for
  duration: Duration,
}

impl SequencerRow {
  // the row's sample at its volume and pan, in stereo, ready to be played
  // once
  pub fn source(&self) -> impl Source<Item = f32> + Send + 'static {
    Panned::new(self.buffer.convert_samples().amplify(self.volume), self.pan)
  }

  // what the row is called: its label, or its sample's name
  pub fn name(&self) -> String {
    self.label.clone().unwrap_or_else(|| sample_name(&self.path))
  }

  pub fn duration(&self) -> Duration {
    self.duration
  }
}

// what a pattern holds: lit steps, or notes
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternKind {
  #[default]
  Drum,
  Note,
}

impl PatternKind {
  // (projects leave the kind out of drum patterns and folders, as they had
  // them before there were note patterns)
  pub fn is_drum(&self) -> bool {
    *self == PatternKind::Drum
  }

  // what its patterns are called until they're named: "Loop 3", "Sequence 3"
  pub fn default_name(&self, n: u64) -> String {
    match self {
      PatternKind::Drum => format!("Loop {}", n),
      PatternKind::Note => format!("Sequence {}", n),
    }
  }
}

// a pattern, as the sequencer keeps it and a project saves it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
  pub id: u64,
  // unnamed patterns (from older projects, or renamed blank) are shown by
  // their number: see label
  pub name: Option<String>,
  #[serde(default, skip_serializing_if = "PatternKind::is_drum")]
  pub kind: PatternKind,
  // how many steps long the pattern is: its bars times the steps in a bar,
  // worked out when its length is set, so changing the time signature later
  // doesn't change patterns
  pub length: u32,
  // a drum pattern's steps lit on each row, by row id
  pub steps: BTreeMap<u64, BTreeSet<u32>>,
  // the folder it's in, in the sidebar's pattern browser (none: the top)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub folder: Option<u64>,
  // a note pattern's notes, by synth id, each synth's in the order they were
  // given. a synth without notes isn't in it
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub notes: BTreeMap<u64, Vec<PatternNote>>,
}

// a note in a pattern, for an instrument. `start` and `length` are in steps
// from the pattern's start
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternNote {
  // a MIDI note number, 0 to 127 (60 is middle C)
  pub pitch: u8,
  pub start: f64,
  pub length: f64,
  // 0 to 1
  pub velocity: f32,
}

// the shortest a note can be, in steps (long enough to be some frames at
// any tempo and rate, so its end never lands on its start)
pub const MIN_NOTE: f64 = 1. / 32.;
// the most notes a pattern holds
pub const MAX_NOTES: usize = 20_000;

// the furthest a note can start, in steps: past the longest note pattern
// (64 bars of 32 beats), so any note a pattern could play is kept
pub const MAX_NOTE_START: f64 = (NOTE_BARS_MAX * 32 * STEPS_PER_BEAT) as f64;

impl PatternNote {
  // the note made sensible: not before the pattern's start, at least
  // MIN_NOTE long, a pitch MIDI has and a velocity from 0 to 1. one past the
  // pattern's end is kept as it is (it doesn't play: see P32). None if it
  // makes no sense at all (a number that isn't one)
  pub fn tidy(self) -> Option<PatternNote> {
    if !self.start.is_finite() || !self.length.is_finite() || !self.velocity.is_finite() { return None; }
    let start = self.start.clamp(0., MAX_NOTE_START);
    Some(PatternNote {
      pitch: self.pitch.min(127),
      start,
      length: self.length.clamp(MIN_NOTE, MAX_NOTE_START),
      velocity: self.velocity.clamp(0., 1.),
    })
  }

  pub fn end(&self) -> f64 {
    self.start + self.length
  }
}

// a folder of patterns, in the sidebar's pattern browser. folders only sort
// patterns: they don't change how anything plays
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternFolder {
  pub id: u64,
  pub name: String,
  // the kind of pattern it holds
  #[serde(default, skip_serializing_if = "PatternKind::is_drum")]
  pub kind: PatternKind,
  // the folder it's in (none: the top)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parent: Option<u64>,
}

// a synth: an instrument plugin that note patterns play, shared by all of
// them. its plugin (and the copies of it the tracks play) is in effects.rs.
// its id is also a slot id, from effects.rs's ids
#[derive(Clone, Debug, PartialEq)]
pub struct Synth {
  pub id: u64,
  pub plugin_id: String,
  // its bundle, for when the plugin isn't in the catalog
  pub path: String,
  // unique in the project
  pub name: String,
  pub vendor: String,
  // a gain, 0 to ROW_VOLUME_MAX, like a row's
  pub volume: f32,
  pub muted: bool,
}

// a synth as the frontend sees it
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthInfo {
  pub id: u64,
  pub plugin_id: String,
  pub name: String,
  pub vendor: String,
  pub volume: f32,
  pub muted: bool,
  // its plugin isn't installed (or couldn't be loaded): kept, silent
  pub missing: bool,
  pub has_editor: bool,
}

// what's known of a synth's plugin (see effects.rs)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynthStatus {
  pub missing: bool,
  pub has_editor: bool,
}

// where the synths' plugins are, for the frontend's view of the sequencer
pub trait SynthPlugins {
  fn synth_status(&self, id: u64) -> SynthStatus;
}

// (without plugins: every synth is missing)
impl SynthPlugins for () {
  fn synth_status(&self, _: u64) -> SynthStatus {
    SynthStatus { missing: true, has_editor: false }
  }
}

// a synth as saved in a project, with its plugin's settings (filled in by
// project.rs, from effects.rs)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSynth {
  pub id: u64,
  pub plugin_id: String,
  pub name: String,
  #[serde(default)]
  pub vendor: String,
  #[serde(default)]
  pub path: String,
  #[serde(default = "default_row_volume")]
  pub volume: f32,
  #[serde(default)]
  pub muted: bool,
  // the plugin's settings, base64
  #[serde(default)]
  pub state: String,
  // where its floating window was last, if it's been open
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub window: Option<crate::effects::WindowPosition>,
}

// a pattern as the frontend sees it: the kind always given, and both maps
// (one of them empty)
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternState {
  pub id: u64,
  pub name: Option<String>,
  pub kind: PatternKind,
  pub length: u32,
  pub steps: BTreeMap<u64, BTreeSet<u32>>,
  pub notes: BTreeMap<u64, Vec<PatternNote>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub folder: Option<u64>,
}

impl PatternState {
  fn of(pattern: &Pattern) -> PatternState {
    PatternState {
      id: pattern.id,
      name: pattern.name.clone(),
      kind: pattern.kind,
      length: pattern.length,
      steps: pattern.steps.clone(),
      notes: pattern.notes.clone(),
      folder: pattern.folder,
    }
  }
}

// a folder as the frontend sees it
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderState {
  pub id: u64,
  pub name: String,
  pub kind: PatternKind,
  #[serde(skip_serializing_if = "Option::is_none")]
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
  // what the pattern is called: its name, or for an unnamed one its kind's
  // name with its number ("Loop 3")
  pub fn label(&self) -> String {
    self.name.clone().unwrap_or_else(|| self.kind.default_name(self.id + 1))
  }

  // whether a row's step is lit
  #[cfg(test)]
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
  pub label: Option<String>,
  pub volume: f32,
  pub pan: f32,
  pub muted: bool,
}

// the whole sequencer, as the frontend sees it. every editing command
// returns it, so the frontend's copy can't drift from the engine's
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequencerState {
  pub rows: Vec<RowInfo>,
  pub synths: Vec<SynthInfo>,
  pub patterns: Vec<PatternState>,
  pub folders: Vec<FolderState>,
}

impl SequencerState {
  #[cfg(test)]
  pub fn pattern(&self, id: u64) -> Option<&PatternState> {
    self.patterns.iter().find(|p| p.id == id)
  }
}

// a row as saved in a project (its path is made relative by project.rs)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRow {
  pub id: u64,
  pub path: String,
  // left out when it has none (and by projects from before labels)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  #[serde(default = "default_row_volume")]
  pub volume: f32,
  // left out when centred (and by projects from before pan)
  #[serde(default, skip_serializing_if = "is_centred")]
  pub pan: f32,
  #[serde(default)]
  pub muted: bool,
}

fn default_row_volume() -> f32 { ROW_VOLUME }

fn is_centred(pan: &f32) -> bool { *pan == 0. }

// the sequencer as saved in a project
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedSequencer {
  pub rows: Vec<SavedRow>,
  // left out of projects without any
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub synths: Vec<SavedSynth>,
  pub patterns: Vec<Pattern>,
  // left out of projects saved before pattern folders
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub folders: Vec<PatternFolder>,
}

impl SavedSequencer {
  pub fn is_empty(&self) -> bool {
    self.rows.is_empty() && self.synths.is_empty() && self.patterns.is_empty() && self.folders.is_empty()
  }

  // give each synth a new id from `next` (a project's synth ids are slot
  // ids, which are never reused in a session), and its notes with it
  pub fn renumber_synths(&mut self, mut next: impl FnMut() -> u64) {
    let ids: BTreeMap<u64, u64> = self.synths.iter_mut().map(|synth| {
      let new = next();
      let old = std::mem::replace(&mut synth.id, new);
      (old, new)
    }).collect();
    for pattern in &mut self.patterns {
      pattern.notes = std::mem::take(&mut pattern.notes).into_iter()
        .filter_map(|(synth, notes)| ids.get(&synth).map(|&new| (new, notes)))
        .collect();
    }
  }
}

#[derive(Default, Clone)]
pub struct Sequencer {
  rows: Vec<SequencerRow>,
  synths: Vec<Synth>,
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

// a pattern's length in bars: one of BAR_CHOICES for a drum pattern, 1 to
// NOTE_BARS_MAX for a note pattern
fn check_bars(kind: PatternKind, bars: u32) -> Result<(), String> {
  match kind {
    PatternKind::Drum if BAR_CHOICES.contains(&bars) => Ok(()),
    PatternKind::Drum => Err(format!("a drum pattern can be {:?} bars long, not {}", BAR_CHOICES, bars)),
    PatternKind::Note if (1..=NOTE_BARS_MAX).contains(&bars) => Ok(()),
    PatternKind::Note => Err(format!("a note pattern can be 1 to {} bars long, not {}", NOTE_BARS_MAX, bars)),
  }
}

// a synth's name made unique among `taken`: "Name", then "Name 2", ...
pub fn unique_synth_name(name: &str, taken: &[&str]) -> String {
  let free = |candidate: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(candidate));
  if free(name) { return name.to_string(); }
  (2..).map(|n| format!("{} {}", name, n)).find(|candidate| free(candidate)).expect("a free name")
}

impl Sequencer {
  pub fn rows(&self) -> &[SequencerRow] {
    &self.rows
  }

  #[cfg(test)]
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

  // whether any row plays a file at or under `path`
  pub fn plays_under(&self, path: &std::path::Path) -> bool {
    self.rows.iter().any(|row| std::path::Path::new(&row.path).starts_with(path))
  }

  // whether another sequencer holds the same rows, synths, patterns and
  // folders (for the history, to tell whether an edit changed anything)
  pub fn same_as(&self, other: &Sequencer) -> bool {
    let row = |r: &SequencerRow| (r.id, r.path.clone(), r.label.clone(), r.volume, r.pan, r.muted);
    self.rows.iter().map(row).eq(other.rows.iter().map(row))
      && self.synths == other.synths
      && self.patterns == other.patterns
      && self.folders == other.folders
  }

  // go back to how another sequencer was (undo and redo). ids keep counting
  // up from wherever they'd got to, so one from before never names
  // something new
  pub fn restore_from(&mut self, other: &Sequencer) {
    self.rows = other.rows.clone();
    self.synths = other.synths.clone();
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

  pub fn synths(&self) -> &[Synth] {
    &self.synths
  }

  pub fn synth(&self, id: u64) -> Option<&Synth> {
    self.synths.iter().find(|s| s.id == id)
  }

  fn synth_mut(&mut self, id: u64) -> Result<&mut Synth, String> {
    self.synths.iter_mut().find(|s| s.id == id)
      .ok_or_else(|| format!("no synth with id {}", id))
  }

  // the whole sequencer as the frontend sees it, with what `plugins` knows
  // of each synth's plugin
  pub fn state(&self, plugins: &dyn SynthPlugins) -> SequencerState {
    SequencerState {
      rows: self.rows.iter().map(|r| RowInfo {
        id: r.id,
        path: r.path.clone(),
        label: r.label.clone(),
        volume: r.volume,
        pan: r.pan,
        muted: r.muted,
      }).collect(),
      synths: self.synths.iter().map(|s| {
        let status = plugins.synth_status(s.id);
        SynthInfo {
          id: s.id,
          plugin_id: s.plugin_id.clone(),
          name: s.name.clone(),
          vendor: s.vendor.clone(),
          volume: s.volume,
          muted: s.muted,
          missing: status.missing,
          has_editor: status.has_editor,
        }
      }).collect(),
      patterns: self.patterns.iter().map(PatternState::of).collect(),
      folders: self.folders.iter().map(|f| FolderState { id: f.id, name: f.name.clone(), kind: f.kind, parent: f.parent }).collect(),
    }
  }

  // forget every row, synth, pattern and folder, e.g. before loading a
  // project
  pub fn clear(&mut self) {
    self.rows.clear();
    self.synths.clear();
    self.patterns.clear();
    self.folders.clear();
  }

  // add a row playing the sample at `path`, at the bottom. returns its id
  #[cfg(test)]
  pub fn add_row(&mut self, path: &str) -> Result<u64, String> {
    self.add_loaded_row(path, RowSample::load(path)?)
  }

  // add a row playing a sample that's already been loaded (see RowSample),
  // at the bottom. returns its id
  pub fn add_loaded_row(&mut self, path: &str, sample: RowSample) -> Result<u64, String> {
    let id = self.next_row_id;
    let saved = SavedRow { id, path: path.to_string(), label: None, volume: ROW_VOLUME, pan: 0., muted: false };
    self.insert_row(saved, sample)?;
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
      label: saved.label.and_then(|l| row_label(&l)),
      volume: clamp_volume(saved.volume),
      pan: clamp_pan(saved.pan),
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

  // a row's pan, kept between -1 (left) and 1 (right)
  pub fn set_row_pan(&mut self, id: u64, pan: f32) -> Result<(), String> {
    self.row_mut(id)?.pan = clamp_pan(pan);
    Ok(())
  }

  // label a row ("Snare"). a blank label goes back to its sample's name
  pub fn rename_row(&mut self, id: u64, label: &str) -> Result<(), String> {
    self.row_mut(id)?.label = row_label(label);
    Ok(())
  }

  // move a row to `index` in the list (past the end: to the bottom). its
  // steps are kept by its id, so no pattern changes
  pub fn move_row(&mut self, id: u64, index: usize) -> Result<(), String> {
    let from = self.rows.iter().position(|r| r.id == id)
      .ok_or_else(|| format!("no sequencer row with id {}", id))?;
    let row = self.rows.remove(from);
    let to = index.min(self.rows.len());
    self.rows.insert(to, row);
    Ok(())
  }

  // play another sample (already loaded, see RowSample) on a row, keeping
  // its steps in every pattern, its label, volume, pan and mute
  pub fn replace_row_sample(&mut self, id: u64, path: &str, sample: RowSample) -> Result<(), String> {
    let row = self.row_mut(id)?;
    let RowSample { duration, buffer } = sample;
    row.path = path.to_string();
    row.buffer = buffer;
    row.duration = duration;
    Ok(())
  }

  // add a synth at the end of the list. its id comes from its plugin's slot
  // (see effects.rs), and its name is made unique
  pub fn add_synth(&mut self, mut synth: Synth) -> Result<(), String> {
    if self.synth(synth.id).is_some() {
      return Err(format!("there's already a synth with id {}", synth.id));
    }
    synth.name = self.free_synth_name(&synth.name);
    synth.volume = clamp_volume(synth.volume);
    self.synths.push(synth);
    Ok(())
  }

  // `name`, or "name 2" and so on if a synth has it already
  pub fn free_synth_name(&self, name: &str) -> String {
    let taken: Vec<&str> = self.synths.iter().map(|s| s.name.as_str()).collect();
    unique_synth_name(name, &taken)
  }

  // take a synth out, and its notes out of every pattern
  pub fn remove_synth(&mut self, id: u64) -> Result<(), String> {
    let idx = self.synths.iter().position(|s| s.id == id)
      .ok_or_else(|| format!("no synth with id {}", id))?;
    self.synths.remove(idx);
    for pattern in &mut self.patterns {
      pattern.notes.remove(&id);
    }
    Ok(())
  }

  // a synth's volume as a gain, kept between 0 and ROW_VOLUME_MAX
  pub fn set_synth_volume(&mut self, id: u64, volume: f32) -> Result<(), String> {
    self.synth_mut(id)?.volume = clamp_volume(volume);
    Ok(())
  }

  pub fn set_synth_muted(&mut self, id: u64, muted: bool) -> Result<(), String> {
    self.synth_mut(id)?.muted = muted;
    Ok(())
  }

  // a new, empty pattern of `bars` bars, at the end, named "Loop n" or
  // "Sequence n" (see free_pattern_name). returns its id
  pub fn add_pattern(&mut self, bars: u32, beats_per_bar: u16, kind: PatternKind) -> Result<u64, String> {
    check_bars(kind, bars)?;
    let id = self.next_pattern_id;
    let name = self.free_pattern_name(kind);
    self.insert_pattern(Pattern {
      id,
      name: Some(name),
      kind,
      length: steps_in_bars(bars, beats_per_bar),
      steps: BTreeMap::new(),
      folder: None,
      notes: BTreeMap::new(),
    })?;
    Ok(id)
  }

  // a new pattern's name: its kind's name with the lowest number no pattern
  // of that kind is shown with (an unnamed one counts as its label)
  pub fn free_pattern_name(&self, kind: PatternKind) -> String {
    let taken: BTreeSet<String> = self.patterns.iter()
      .filter(|p| p.kind == kind)
      .map(|p| p.label().to_lowercase())
      .collect();
    (1..).map(|n| kind.default_name(n)).find(|name| !taken.contains(&name.to_lowercase())).expect("a free name")
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

  // a new pattern made from others (see join.rs): any number of steps
  // long, with these steps lit and these notes, straight after the pattern
  // `after` and in its folder (so of its kind). returns its id
  pub fn add_pattern_after(
    &mut self,
    after: u64,
    name: String,
    length: u32,
    steps: BTreeMap<u64, BTreeSet<u32>>,
    notes: BTreeMap<u64, Vec<PatternNote>>,
  ) -> Result<u64, String> {
    let idx = self.patterns.iter().position(|p| p.id == after)
      .ok_or_else(|| format!("no pattern with id {}", after))?;
    if length == 0 {
      return Err("a pattern needs some steps".into());
    }
    let id = self.next_pattern_id;
    self.next_pattern_id += 1;

    let Pattern { folder, kind, .. } = self.patterns[idx];
    let notes = notes.into_iter()
      .filter(|(synth, _)| self.synth(*synth).is_some())
      .map(|(synth, notes)| (synth, tidy_notes(notes)))
      .filter(|(_, notes)| !notes.is_empty())
      .collect();
    self.patterns.insert(idx + 1, Pattern { id, name: Some(name), kind, length, steps, folder, notes });
    Ok(id)
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

  // make a pattern `bars` long. steps and notes past a shortened pattern's
  // end are kept (and don't play), so making it longer again brings them back
  pub fn set_pattern_bars(&mut self, id: u64, bars: u32, beats_per_bar: u16) -> Result<(), String> {
    let pattern = self.pattern_mut(id)?;
    check_bars(pattern.kind, bars)?;
    pattern.length = steps_in_bars(bars, beats_per_bar);
    Ok(())
  }

  // replace a synth's notes in a note pattern, each made sensible (see
  // PatternNote::tidy), in the order given. one that makes no sense is left
  // out
  pub fn set_pattern_notes(&mut self, id: u64, synth: u64, notes: Vec<PatternNote>) -> Result<(), String> {
    if notes.len() > MAX_NOTES {
      return Err(format!("a pattern can hold at most {} notes for a synth", MAX_NOTES));
    }
    if self.synth(synth).is_none() {
      return Err(format!("no synth with id {}", synth));
    }
    let pattern = self.pattern_mut(id)?;
    if pattern.kind != PatternKind::Note {
      return Err("a drum pattern holds steps, not notes".into());
    }
    let notes = tidy_notes(notes);
    if notes.is_empty() { pattern.notes.remove(&synth); } else { pattern.notes.insert(synth, notes); }
    Ok(())
  }

  // light or clear one step of a row in a pattern
  pub fn set_step(&mut self, pattern_id: u64, row_id: u64, step: u32, on: bool) -> Result<(), String> {
    if self.row(row_id).is_none() {
      return Err(format!("no sequencer row with id {}", row_id));
    }
    let pattern = self.pattern_mut(pattern_id)?;
    if pattern.kind != PatternKind::Drum {
      return Err("a note pattern holds notes, not steps".into());
    }
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
  // making another; new ones go after the pattern, named like "verse (kick)".
  // returns (row's sample, row's name, pattern id), in row order. the
  // pattern itself is left as it is
  pub fn split_pattern(&mut self, id: u64) -> Result<Vec<(String, String, u64)>, String> {
    let source = self.pattern(id).cloned()
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    let label = source.label();
    // its notes have no sample to go with: exploding would lose them
    if source.kind != PatternKind::Drum {
      return Err(format!("pattern {} is a note pattern, and only a drum pattern can be exploded", label));
    }

    let parts: Vec<(u64, String, String, BTreeSet<u32>)> = self.rows.iter()
      .filter_map(|row| {
        let steps: BTreeSet<u32> = source.steps.get(&row.id)?
          .iter().copied().filter(|&s| s < source.length).collect();
        if steps.is_empty() { None } else { Some((row.id, row.path.clone(), row.name(), steps)) }
      })
      .collect();
    if parts.is_empty() {
      return Err(format!("pattern {} has no steps to explode", label));
    }

    let mut insert_at = self.patterns.iter().position(|p| p.id == id).expect("found above") + 1;
    let mut split = Vec::new();
    for (row_id, path, name, steps) in parts {
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
            name: Some(format!("{} ({})", label, name)),
            kind: PatternKind::Drum,
            length: source.length,
            steps: wanted,
            // next to the pattern it came from
            folder: source.folder,
            notes: BTreeMap::new(),
          });
          insert_at += 1;
          new_id
        }
      };
      split.push((path, name, pattern_id));
    }
    Ok(split)
  }

  #[cfg(test)]
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

  // a folder to put something of `kind` in: none (the top), or one of that
  // kind that's there
  fn check_folder(&self, folder: Option<u64>, kind: PatternKind) -> Result<(), String> {
    match folder.map(|id| (id, self.folder(id))) {
      Some((id, None)) => Err(format!("no pattern folder with id {}", id)),
      Some((_, Some(f))) if f.kind != kind => Err(match kind {
        PatternKind::Drum => "a drum pattern can't go in a folder of note patterns".into(),
        PatternKind::Note => "a note pattern can't go in a folder of drum patterns".into(),
      }),
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

  // a new, empty folder of `kind` in `parent` (none: the top). returns its
  // id
  pub fn add_folder(&mut self, name: &str, parent: Option<u64>, kind: PatternKind) -> Result<u64, String> {
    let name = folder_name(name)?;
    self.check_folder(parent, kind)?;
    self.check_folder_name(&name, parent, None)?;
    let id = self.next_folder_id;
    self.next_folder_id += 1;
    self.folders.push(PatternFolder { id, name, kind, parent });
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
    let folder = self.folder(id).ok_or_else(|| format!("no pattern folder with id {}", id))?;
    let (name, kind) = (folder.name.clone(), folder.kind);
    self.check_folder(parent, kind)?;
    if parent.map_or(false, |p| self.folder_within(p, id)) {
      return Err("a folder can't go inside itself".into());
    }
    self.check_folder_name(&name, parent, Some(id))?;
    self.folder_mut(id)?.parent = parent;
    Ok(())
  }

  // put a pattern in a folder, or (none) at the top
  pub fn move_pattern(&mut self, id: u64, folder: Option<u64>) -> Result<(), String> {
    let kind = self.pattern(id).ok_or_else(|| format!("no pattern with id {}", id))?.kind;
    self.check_folder(folder, kind)?;
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
      let (parent, kind) = self.folder(id).map_or((None, PatternKind::Drum), |f| (f.parent, f.kind));
      let lost = parent.map_or(false, |p| {
        self.folder(p).map_or(true, |p| p.kind != kind) || self.folder_within(p, id)
      });
      if lost {
        if let Ok(folder) = self.folder_mut(id) { folder.parent = None; }
      }
    }
  }

  // the sequencer for a project file (the synths' settings and windows are
  // the caller's to fill in: see project.rs)
  pub fn saved(&self) -> SavedSequencer {
    SavedSequencer {
      rows: self.rows.iter().map(|r| SavedRow {
        id: r.id,
        path: r.path.clone(),
        label: r.label.clone(),
        volume: r.volume,
        pan: r.pan,
        muted: r.muted,
      }).collect(),
      synths: self.synths.iter().map(|s| SavedSynth {
        id: s.id,
        plugin_id: s.plugin_id.clone(),
        name: s.name.clone(),
        vendor: s.vendor.clone(),
        path: s.path.clone(),
        volume: s.volume,
        muted: s.muted,
        state: String::new(),
        window: None,
      }).collect(),
      patterns: self.patterns.clone(),
      folders: self.folders.clone(),
    }
  }

  // replace everything with a project's sequencer. rows whose sample can't
  // be loaded are left out (and their steps with them), and returned as
  // (path, reason). synths are kept whether their plugins load or not (see
  // effects.rs). patterns that don't make sense (no steps, or an id used
  // twice) are left out
  #[cfg(test)]
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
    for synth in saved.synths {
      let synth = Synth {
        id: synth.id,
        plugin_id: synth.plugin_id,
        path: synth.path,
        name: synth.name,
        vendor: synth.vendor,
        volume: synth.volume,
        muted: synth.muted,
      };
      let id = synth.id;
      if let Err(e) = self.add_synth(synth) {
        println!("left out synth {}: {}", id, e);
      }
    }
    self.restore_folders(saved.folders);
    for mut pattern in saved.patterns {
      // a pattern holds steps or notes, as its kind says
      match pattern.kind {
        PatternKind::Drum => pattern.notes.clear(),
        PatternKind::Note => pattern.steps.clear(),
      }
      pattern.steps.retain(|row_id, steps| self.rows.iter().any(|r| r.id == *row_id) && !steps.is_empty());
      // a folder that isn't there, or holds the other kind: at the top
      if pattern.folder.map_or(false, |id| self.folder(id).map_or(true, |f| f.kind != pattern.kind)) {
        pattern.folder = None;
      }
      pattern.notes = std::mem::take(&mut pattern.notes).into_iter()
        .filter(|(synth, _)| self.synth(*synth).is_some())
        .map(|(synth, notes)| (synth, tidy_notes(notes)))
        .filter(|(_, notes)| !notes.is_empty())
        .collect();
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

// notes made sensible (see PatternNote::tidy), in their order, as many as a
// synth can have in a pattern
fn tidy_notes(notes: Vec<PatternNote>) -> Vec<PatternNote> {
  notes.into_iter().filter_map(PatternNote::tidy).take(MAX_NOTES).collect()
}

fn clamp_volume(volume: f32) -> f32 {
  if volume.is_finite() { volume.clamp(0., ROW_VOLUME_MAX) } else { ROW_VOLUME }
}

fn clamp_pan(pan: f32) -> f32 {
  if pan.is_finite() { pan.clamp(-1., 1.) } else { 0. }
}

// a row's label, trimmed and cut to ROW_LABEL_MAX characters. none when blank
fn row_label(label: &str) -> Option<String> {
  let label: String = label.trim().chars().take(ROW_LABEL_MAX).collect();
  let label = label.trim_end();
  if label.is_empty() { None } else { Some(label.to_string()) }
}

// a source panned, in stereo: a mono one is put on both sides first, and
// channels past the first two are dropped. the same pan law as a clip's
// (see clip_reader.rs): the far side fades out, the near side stays as is
pub struct Panned<S> {
  inner: S,
  channels: u16,
  gains: [f32; 2],
  // the right half of the frame whose left was just given
  right: Option<f32>,
}

impl<S: Source<Item = f32>> Panned<S> {
  pub fn new(inner: S, pan: f32) -> Self {
    let pan = clamp_pan(pan);
    let channels = inner.channels().max(1);
    Panned { inner, channels, gains: [(1. - pan).min(1.), (1. + pan).min(1.)], right: None }
  }
}

impl<S: Source<Item = f32>> Iterator for Panned<S> {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    if let Some(right) = self.right.take() {
      return Some(right);
    }
    let left = self.inner.next()?;
    let right = if self.channels >= 2 { self.inner.next().unwrap_or(left) } else { left };
    for _ in 2..self.channels {
      self.inner.next();
    }
    self.right = Some(right * self.gains[1]);
    Some(left * self.gains[0])
  }
}

impl<S: Source<Item = f32>> Source for Panned<S> {
  // (the inner source's channels are read once, so its format is taken to
  // stay the same throughout)
  fn current_frame_len(&self) -> Option<usize> { None }
  fn channels(&self) -> u16 { 2 }
  fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
  fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

#[cfg(test)]
mod tests;
