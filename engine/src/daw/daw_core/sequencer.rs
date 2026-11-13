// the pattern sequencer. a pattern holds notes (pitch, start, length,
// velocity) for rows, and a row is an instrument: a sample, or a synth (an
// instrument plugin). the rows are one list, in the order they're shown,
// shared by every pattern
//
// a note's start and length are in steps (16th notes) from the pattern's
// start, and needn't be whole. each row's notes in a pattern are kept sorted
// by start, then pitch (see sort_notes), and notes past the pattern's end are
// kept but not played. a pattern is 1 to PATTERN_BARS_MAX bars long
//
// a step clicked on is a one-step note at its row's step pitch (see
// set_pattern_step). a step clicked, or a note drawn, past a pattern's end
// grows it to fit (see grown_length). a sample row plays a note by reading
// its sample at the note's pitch, against the row's sample pitch, and either
// to the sample's end (one-shot) or to the note's end (gated): see
// sample_voice.rs
//
// a synth row's plugin lives in effects.rs under the row's slot id, and
// follows this list (see commands/synth.rs)
//
// a pattern plays on its own in the Pattern Sequencer and the Piano Roll
// (see daw::run_playlist), and as pattern clips in the playlist (PatternNode
// in audiograph.rs), which stay linked to it: editing a pattern changes
// every clip of it
//
// undo keeps whole copies of the sequencer (see history.rs), so the row list,
// each pattern, and each row's notes in a pattern are shared (Arc): a copy
// shares all of them, and an edit copies only what it changes
//
// each row and pattern has a rev, which changes whenever it does, so the
// frontend can tell what an edit changed. edits change rows and patterns
// only through row_mut and pattern_mut, which give the new rev
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::daw;

// a step is a 16th note
pub const STEPS_PER_BEAT: u32 = 4;
// the longest a pattern can be, in bars
pub const PATTERN_BARS_MAX: u32 = 128;
// the pitch a new row's clicked steps play, and a new sample row's sample
// plays unchanged at (60 is middle C)
pub const DEFAULT_PITCH: u8 = 60;
// how loud a clicked step is
pub const STEP_VELOCITY: f32 = 1.;
// a row's volume when it's added, as a gain (1 leaves the sound as is)
pub const ROW_VOLUME: f32 = 1.;
pub const ROW_VOLUME_MAX: f32 = 2.;
// the longest a row's label can be, in characters
pub const ROW_LABEL_MAX: usize = 40;

// the shortest a note can be, in steps (long enough to be some frames at
// any tempo and rate, so its end never lands on its start)
pub const MIN_NOTE: f64 = 1. / 32.;
// the most notes a row can have in a pattern
pub const MAX_NOTES: usize = 40_000;
// the furthest a note can start, in steps: past the longest pattern (in bars
// of 32 beats), so any note a pattern could play is kept
pub const MAX_NOTE_START: f64 = (PATTERN_BARS_MAX * 32 * STEPS_PER_BEAT) as f64;

// a version of a row or a pattern, so the frontend can tell what changed
// without comparing contents. revs come from one counter for the whole run
// of the app, so a rev is never given twice, not even after an undo or a new
// project. a rev isn't part of what a row or pattern is: two that differ only
// in their revs are equal (so history's same_as and the unsaved check ignore it)
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(transparent)]
pub struct Rev(u64);

impl Rev {
  pub fn next() -> Rev {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Rev(NEXT.fetch_add(1, Ordering::Relaxed))
  }

  // (equality can't see a rev that should have changed: tests compare these)
  #[cfg(test)]
  pub fn get(&self) -> u64 {
    self.0
  }
}

impl PartialEq for Rev {
  fn eq(&self, _: &Rev) -> bool { true }
}

// a row: one instrument, and how a pattern's steps play on it
#[derive(Clone, PartialEq)]
pub struct Row {
  // one id space for both instruments, never reused in a run
  pub id: u64,
  // new whenever the row changes (see Rev). moving it changes only the list
  pub rev: Rev,
  // what the row is called ("Snare"), shown instead of its instrument's name
  pub label: Option<String>,
  // a gain, 0 to ROW_VOLUME_MAX
  pub volume: f32,
  // -1 (left) to 1 (right)
  pub pan: f32,
  pub muted: bool,
  // the pitch a clicked step plays
  pub step_pitch: u8,
  pub instrument: Instrument,
}

#[derive(Clone, PartialEq)]
pub enum Instrument {
  Sample(SampleRow),
  Synth(SynthRow),
}

#[derive(Clone, PartialEq)]
pub struct SampleRow {
  pub path: String,
  // the pitch that plays the sample unchanged
  pub sample_pitch: u8,
  // whether a note stops the sample at the note's end, rather than letting
  // it play out
  pub gated: bool,
  sound: RowSound,
}

// what an explode gives a track of its own (see explode_tracks): a sample,
// by its file (sample clips play one too, and two rows of it are one
// sound), or a synth row, by its id (its slot changes with its plugin)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sound<'a> {
  Sample(&'a str),
  Synth(u64),
}

// a synth row's plugin. effects.rs keeps it under `slot`, which its windows,
// docking and the copies tracks play go by
#[derive(Clone, Debug, PartialEq)]
pub struct SynthRow {
  pub slot: u64,
  pub plugin_id: String,
  // its bundle, for when the plugin isn't in the catalog
  pub path: String,
  // unique among the synth rows (case aside): "DecentSampler 2"
  pub name: String,
  pub vendor: String,
}

// a synth row's plugin as effects.rs loads it and keeps it in line with the
// rows (see Effects::follow_synths): `id` is the row's slot, and the volume,
// pan and mute are the row's
#[derive(Clone, Debug, PartialEq)]
pub struct Synth {
  pub id: u64,
  pub plugin_id: String,
  pub path: String,
  pub name: String,
  pub vendor: String,
  pub volume: f32,
  pub pan: f32,
  pub muted: bool,
}

// a sample row's sample, decoded once, when it's added (playing a note
// only reads it). decoding takes a while, so it's done without the
// sequencer, whose lock playback needs. it's what the row's path holds, so
// it isn't compared
#[derive(Clone)]
pub struct RowSound(Arc<daw::Pcm>);

impl PartialEq for RowSound {
  fn eq(&self, _: &RowSound) -> bool { true }
}

impl RowSound {
  // decoded here, so a file that isn't audio is refused when it's added
  // rather than when it's played
  pub fn load(path: &str) -> Result<Self, String> {
    let pcm = daw::Pcm::load(path)?;
    if pcm.frames() == 0 {
      return Err(format!("sample '{}' contains no audio", path));
    }
    Ok(RowSound(Arc::new(pcm)))
  }

  // each of a saved sequencer's rows' samples, in order (none for a synth
  // row)
  pub fn load_rows(saved: &SavedSequencer) -> Vec<Option<Result<Self, String>>> {
    saved.rows.iter().map(|row| row.instrument.sample_path().map(Self::load)).collect()
  }

  #[cfg(test)]
  pub fn pcm(&self) -> &Arc<daw::Pcm> {
    &self.0
  }
}

impl Row {
  // a row as it's added: unlabelled, at full volume, centered and unmuted
  fn new(id: u64, instrument: Instrument) -> Row {
    Row {
      id, rev: Rev::next(), label: None, volume: ROW_VOLUME, pan: 0., muted: false, step_pitch: DEFAULT_PITCH,
      instrument,
    }
  }

  // what the row is called: its label, or its sample's or synth's name
  pub fn name(&self) -> String {
    self.label.clone().unwrap_or_else(|| match &self.instrument {
      Instrument::Sample(sample) => sample_name(&sample.path),
      Instrument::Synth(synth) => synth.name.clone(),
    })
  }

  pub fn sample(&self) -> Option<&SampleRow> {
    match &self.instrument {
      Instrument::Sample(sample) => Some(sample),
      Instrument::Synth(_) => None,
    }
  }

  fn sample_mut(&mut self) -> Option<&mut SampleRow> {
    match &mut self.instrument {
      Instrument::Sample(sample) => Some(sample),
      Instrument::Synth(_) => None,
    }
  }

  pub fn synth(&self) -> Option<&SynthRow> {
    match &self.instrument {
      Instrument::Synth(synth) => Some(synth),
      Instrument::Sample(_) => None,
    }
  }

  // what the row plays, as an explode tells tracks apart
  pub fn sound(&self) -> Sound<'_> {
    match &self.instrument {
      Instrument::Sample(sample) => Sound::Sample(&sample.path),
      Instrument::Synth(_) => Sound::Synth(self.id),
    }
  }

  // what plays a sample row's notes: its sample, at its volume and pan
  // (none for a synth row)
  pub fn sample_instrument(&self) -> Option<daw::SampleInstrument> {
    let sample = self.sample()?;
    Some(daw::SampleInstrument {
      pcm: sample.sound.0.clone(),
      sample_pitch: sample.sample_pitch,
      gated: sample.gated,
      volume: self.volume,
      pan: self.pan,
    })
  }

  fn saved(&self) -> SavedRow {
    SavedRow {
      id: self.id,
      label: self.label.clone(),
      volume: self.volume,
      pan: self.pan,
      muted: self.muted,
      step_pitch: self.step_pitch,
      instrument: match &self.instrument {
        Instrument::Sample(sample) => SavedInstrument::Sample(SavedSample {
          path: sample.path.clone(),
          sample_pitch: sample.sample_pitch,
          gated: sample.gated,
        }),
        // (the plugin's settings and window are the caller's to fill in:
        // see project.rs)
        Instrument::Synth(synth) => SavedInstrument::Synth(SavedSynth {
          slot: synth.slot,
          plugin_id: synth.plugin_id.clone(),
          name: synth.name.clone(),
          vendor: synth.vendor.clone(),
          path: synth.path.clone(),
          state: String::new(),
          window: None,
        }),
      },
    }
  }

  fn state(&self, plugins: &dyn SynthPlugins) -> RowState {
    RowState {
      id: self.id,
      rev: self.rev,
      label: self.label.clone(),
      volume: self.volume,
      pan: self.pan,
      muted: self.muted,
      step_pitch: self.step_pitch,
      instrument: match &self.instrument {
        Instrument::Sample(sample) => InstrumentState::Sample(SampleState {
          path: sample.path.clone(),
          sample_pitch: sample.sample_pitch,
          gated: sample.gated,
        }),
        Instrument::Synth(synth) => {
          let status = plugins.synth_status(synth.slot);
          InstrumentState::Synth(SynthState {
            slot: synth.slot,
            plugin_id: synth.plugin_id.clone(),
            name: synth.name.clone(),
            vendor: synth.vendor.clone(),
            missing: status.missing,
            has_editor: status.has_editor,
          })
        }
      },
    }
  }
}

// a note in a pattern, for a row. `start` and `length` are in steps from the
// pattern's start
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

// the order a row's notes are kept in: by start, then pitch. stable, so
// notes the same in both keep their order. the frontend's sortNotes is the
// same (src/types/sequencer.ts)
pub fn sort_notes(notes: &mut [PatternNote]) {
  notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)))
}

// notes made sensible (see PatternNote::tidy) and sorted, as many as a row
// can have in a pattern
fn tidy_notes(notes: Vec<PatternNote>) -> Vec<PatternNote> {
  let mut notes: Vec<PatternNote> = notes.into_iter().filter_map(PatternNote::tidy).collect();
  sort_notes(&mut notes);
  notes.truncate(MAX_NOTES);
  notes
}

// a pattern: notes for rows
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
  pub id: u64,
  // new whenever the pattern changes (see Rev)
  pub rev: Rev,
  // an unnamed pattern (renamed blank) is shown by its number: see label
  pub name: Option<String>,
  // how many steps long the pattern is: its bars times the steps in a bar,
  // worked out when its length is set, so changing the time signature later
  // doesn't change patterns
  pub length: u32,
  // the shortest it'll be trimmed to, in steps (1 to its length). set with
  // its length by the bars counter
  pub floor: u32,
  // each row's notes, sorted (see sort_notes), by row id. a row without
  // notes isn't in it
  pub notes: BTreeMap<u64, Arc<Vec<PatternNote>>>,
  // the folder it's in, in the sidebar's pattern browser (none: the top)
  pub folder: Option<u64>,
}

// what an unnamed pattern numbered `n` is called, and new patterns are named
fn sequence_name(n: u64) -> String {
  format!("Sequence {}", n)
}

impl Pattern {
  // what the pattern is called: its name, or "Sequence" and its number
  pub fn label(&self) -> String {
    self.name.clone().unwrap_or_else(|| sequence_name(self.id + 1))
  }

  // a row's notes (none if it has none)
  pub fn notes_of(&self, row: u64) -> &[PatternNote] {
    self.notes.get(&row).map_or(&[], |notes| notes.as_slice())
  }

  // whether a row has a note starting on a step
  #[cfg(test)]
  pub fn lit(&self, row: u64, step: u32) -> bool {
    self.notes_of(row).iter().any(|note| note.start == step as f64)
  }

  // where its last note ends, of any row (notes past its end too), in
  // steps. 0 when it has none. (the notes are sorted by start, not end, so
  // each one is looked at)
  fn last_end(&self) -> f64 {
    self.notes.values().flat_map(|notes| notes.iter()).map(PatternNote::end).fold(0., f64::max)
  }

  // a row's notes that start inside the pattern: the ones it plays. (its
  // notes are sorted, so they're the ones before the first past its end)
  fn heard(&self, row: u64) -> &[PatternNote] {
    let notes = self.notes_of(row);
    &notes[..notes.partition_point(|note| note.start < self.length as f64)]
  }

  // the rows it plays notes of
  fn heard_rows(&self) -> impl Iterator<Item = u64> + '_ {
    self.notes.keys().copied().filter(|&row| !self.heard(row).is_empty())
  }

  // whether it plays any note at all
  pub fn plays_notes(&self) -> bool {
    self.heard_rows().next().is_some()
  }

  // whether all it plays is these notes, of one row
  fn plays_just(&self, row: u64, notes: &[PatternNote]) -> bool {
    self.heard_rows().eq([row]) && self.heard(row) == notes
  }

  // replace a row's notes (already tidy and sorted). none takes the row out
  fn set_notes(&mut self, row: u64, notes: Vec<PatternNote>) {
    if notes.is_empty() {
      self.notes.remove(&row);
    } else {
      self.notes.insert(row, Arc::new(notes));
    }
  }

  // whether another pattern holds the same, comparing only the note lists
  // the two don't share. (every field is named, so a new one can't be missed.
  // the rev isn't part of what it holds)
  fn same_as(&self, other: &Pattern) -> bool {
    let Pattern { id, rev: _, name, length, floor, notes, folder } = self;
    *id == other.id && *name == other.name && *length == other.length && *floor == other.floor
      && *folder == other.folder
      && notes.len() == other.notes.len()
      && notes.iter().zip(&other.notes).all(|((row, mine), (theirs_row, theirs))| row == theirs_row && same(mine, theirs))
  }

  fn saved(&self) -> SavedPattern {
    SavedPattern {
      id: self.id,
      name: self.name.clone(),
      length: self.length,
      floor: self.floor,
      notes: self.notes.iter().map(|(&row, notes)| (row, notes.to_vec())).collect(),
      folder: self.folder,
    }
  }
}

// whether two shared things hold the same: without looking, when they're
// the same one
fn same<T: PartialEq>(a: &Arc<T>, b: &Arc<T>) -> bool {
  Arc::ptr_eq(a, b) || a == b
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

// what's known of a synth's plugin (see effects.rs)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynthStatus {
  pub missing: bool,
  pub has_editor: bool,
}

// where the synth rows' plugins are, for the frontend's view of the
// sequencer
pub trait SynthPlugins {
  fn synth_status(&self, slot: u64) -> SynthStatus;
}

// (without plugins: every synth is missing)
impl SynthPlugins for () {
  fn synth_status(&self, _: u64) -> SynthStatus {
    SynthStatus { missing: true, has_editor: false }
  }
}

// a row as the frontend sees it
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowState {
  pub id: u64,
  // (a synth's missing and has_editor are its plugin's, and change without it)
  pub rev: Rev,
  // (always given: null when it has none)
  pub label: Option<String>,
  pub volume: f32,
  pub pan: f32,
  pub muted: bool,
  pub step_pitch: u8,
  pub instrument: InstrumentState,
}

#[cfg(test)]
impl RowState {
  pub fn sample_path(&self) -> Option<&str> {
    match &self.instrument {
      InstrumentState::Sample(sample) => Some(&sample.path),
      InstrumentState::Synth(_) => None,
    }
  }

  pub fn synth(&self) -> Option<&SynthState> {
    match &self.instrument {
      InstrumentState::Synth(synth) => Some(synth),
      InstrumentState::Sample(_) => None,
    }
  }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum InstrumentState {
  Sample(SampleState),
  Synth(SynthState),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleState {
  pub path: String,
  pub sample_pitch: u8,
  pub gated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthState {
  pub slot: u64,
  pub plugin_id: String,
  pub name: String,
  pub vendor: String,
  // its plugin isn't installed (or couldn't be loaded): kept, silent
  pub missing: bool,
  pub has_editor: bool,
}

// a pattern as the frontend sees it, its notes always given (shared, not
// copied: see Cargo.toml's serde "rc")
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternState {
  pub id: u64,
  pub rev: Rev,
  pub name: Option<String>,
  pub length: u32,
  pub floor: u32,
  pub notes: BTreeMap<u64, Arc<Vec<PatternNote>>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub folder: Option<u64>,
}

impl PatternState {
  fn of(pattern: &Pattern) -> PatternState {
    PatternState {
      id: pattern.id,
      rev: pattern.rev,
      name: pattern.name.clone(),
      length: pattern.length,
      floor: pattern.floor,
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
  #[serde(skip_serializing_if = "Option::is_none")]
  pub parent: Option<u64>,
}

// the whole sequencer, as the frontend sees it. an editing command returns
// it, or just the row or pattern it changed (RowChange, PatternChange), so
// the frontend's copy can't drift from the engine's
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequencerState {
  pub rows: Vec<RowState>,
  pub patterns: Vec<PatternState>,
  pub folders: Vec<FolderState>,
}

// a reply that changed one pattern only
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternChange {
  pub pattern: PatternState,
}

// a reply that changed one row's own settings only
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowChange {
  pub row: RowState,
}

impl SequencerState {
  #[cfg(test)]
  pub fn pattern(&self, id: u64) -> Option<&PatternState> {
    self.patterns.iter().find(|p| p.id == id)
  }
}

// a row as saved in a project
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRow {
  pub id: u64,
  // left out when it has none
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  #[serde(default = "default_row_volume")]
  pub volume: f32,
  // left out when centered
  #[serde(default, skip_serializing_if = "is_centered")]
  pub pan: f32,
  #[serde(default)]
  pub muted: bool,
  #[serde(default = "default_pitch")]
  pub step_pitch: u8,
  pub instrument: SavedInstrument,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SavedInstrument {
  Sample(SavedSample),
  Synth(SavedSynth),
}

impl SavedInstrument {
  pub fn sample_path(&self) -> Option<&str> {
    match self {
      SavedInstrument::Sample(sample) => Some(&sample.path),
      SavedInstrument::Synth(_) => None,
    }
  }

  pub fn synth(&self) -> Option<&SavedSynth> {
    match self {
      SavedInstrument::Synth(synth) => Some(synth),
      SavedInstrument::Sample(_) => None,
    }
  }
}

// a sample row's sample as saved (its path is made relative by project.rs)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSample {
  pub path: String,
  #[serde(default = "default_pitch")]
  pub sample_pitch: u8,
  #[serde(default)]
  pub gated: bool,
}

// a synth row's plugin as saved, with its settings (filled in by project.rs,
// from effects.rs)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSynth {
  // given a new one on load, as slot ids aren't reused in a run (see
  // SavedSequencer::renumber_slots)
  pub slot: u64,
  pub plugin_id: String,
  pub name: String,
  #[serde(default)]
  pub vendor: String,
  #[serde(default)]
  pub path: String,
  // the plugin's settings, base64
  #[serde(default)]
  pub state: String,
  // where its floating window was last, if it's been open
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub window: Option<crate::effects::WindowPosition>,
}

// a pattern as saved in a project
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPattern {
  pub id: u64,
  // (written as null when it has none)
  pub name: Option<String>,
  pub length: u32,
  // (0 or left out: its length)
  #[serde(default)]
  pub floor: u32,
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub notes: BTreeMap<u64, Vec<PatternNote>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub folder: Option<u64>,
}

fn default_row_volume() -> f32 { ROW_VOLUME }

fn default_pitch() -> u8 { DEFAULT_PITCH }

fn is_centered(pan: &f32) -> bool { *pan == 0. }

// the sequencer as saved in a project
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedSequencer {
  pub rows: Vec<SavedRow>,
  pub patterns: Vec<SavedPattern>,
  // left out when there are none
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub folders: Vec<PatternFolder>,
}

impl SavedSequencer {
  pub fn is_empty(&self) -> bool {
    self.rows.is_empty() && self.patterns.is_empty() && self.folders.is_empty()
  }

  // the sample rows' sample paths, to be made relative or found again
  pub fn sample_paths_mut(&mut self) -> impl Iterator<Item = &mut String> {
    self.rows.iter_mut().filter_map(|row| match &mut row.instrument {
      SavedInstrument::Sample(sample) => Some(&mut sample.path),
      SavedInstrument::Synth(_) => None,
    })
  }

  pub fn synths_mut(&mut self) -> impl Iterator<Item = &mut SavedSynth> {
    self.rows.iter_mut().filter_map(|row| match &mut row.instrument {
      SavedInstrument::Synth(synth) => Some(synth),
      SavedInstrument::Sample(_) => None,
    })
  }

  // give each synth row's plugin a new slot id from `next` (slot ids are
  // never reused in a run). notes are kept by row id, so none change
  pub fn renumber_slots(&mut self, mut next: impl FnMut() -> u64) {
    for synth in self.synths_mut() {
      synth.slot = next();
    }
  }
}

// one row's part of a split pattern (see Sequencer::split_pattern): the
// row, what it's called, and the pattern holding just its notes
#[derive(Debug, PartialEq)]
pub struct SplitPart {
  pub row: u64,
  pub name: String,
  pub pattern: u64,
}

#[derive(Default, Clone)]
pub struct Sequencer {
  // in the order they're shown
  rows: Arc<Vec<Row>>,
  patterns: Vec<Arc<Pattern>>,
  folders: Arc<Vec<PatternFolder>>,
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

// the length a pattern grows to so a note ending at `end` fits: to the end
// of the bar (`bar` steps) holding that end. never shorter than it was (even
// past the cap), never grown past PATTERN_BARS_MAX bars. the frontend's
// grownLength is the same (src/types/sequencer.ts)
pub fn grown_length(length: u32, end: f64, bar: u32) -> u32 {
  let cap = PATTERN_BARS_MAX * bar;
  let fit = ((end - 1e-9) / bar as f64).ceil().max(1.) as u32 * bar;
  length.max(fit.min(cap))
}

// the length a pattern trims to (auto-trim, after a clear): to the end of
// the bar (`bar` steps) holding its last note's end (see Pattern::last_end),
// never below its floor or a bar, never longer than it is. the frontend's
// trimmedLength is the same (src/types/sequencer.ts)
pub fn trimmed_length(length: u32, floor: u32, last_end: f64, bar: u32) -> u32 {
  let fit = if last_end > 0. { ((last_end - 1e-9) / bar as f64).ceil() as u32 * bar } else { 0 };
  length.min(floor.max(fit).max(bar))
}

// where the notes in `new` that aren't in `old` end, the furthest of them
// (0 when none was added). notes are compared on all they are, and a note
// in `new` twice but in `old` once counts once as added. the frontend's
// addedEnd is the same
pub fn added_end(old: &[PatternNote], new: &[PatternNote]) -> f64 {
  // (the notes both lists start and end with alike aren't looked at: an
  // edit usually changes a few notes, and a row can hold MAX_NOTES)
  let head = old.iter().zip(new).take_while(|(a, b)| a == b).count();
  let (old, new) = (&old[head..], &new[head..]);
  let tail = old.iter().rev().zip(new.iter().rev()).take_while(|(a, b)| a == b).count();
  let (old, new) = (&old[..old.len() - tail], &new[..new.len() - tail]);

  let key = |note: &PatternNote| (note.pitch, note.start.to_bits(), note.length.to_bits(), note.velocity.to_bits());
  let mut had: HashMap<_, usize> = HashMap::new();
  for note in old { *had.entry(key(note)).or_default() += 1; }
  new.iter().fold(0., |end, note| match had.get_mut(&key(note)) {
    Some(count) if *count > 0 => {
      *count -= 1;
      end
    }
    _ => end.max(note.end()),
  })
}

// a pattern's length in bars: 1 to PATTERN_BARS_MAX
fn check_bars(bars: u32) -> Result<(), String> {
  if (1..=PATTERN_BARS_MAX).contains(&bars) {
    Ok(())
  } else {
    Err(format!("a pattern can be 1 to {} bars long, not {}", PATTERN_BARS_MAX, bars))
  }
}

fn no_row(id: u64) -> String {
  format!("no row with id {}", id)
}

// a MIDI note number: 0 to 127
fn check_pitch(pitch: u8) -> Result<u8, String> {
  if pitch <= 127 { Ok(pitch) } else { Err(format!("there's no note {}", pitch)) }
}

impl Sequencer {
  #[cfg(test)]
  pub fn rows(&self) -> &[Row] {
    &self.rows
  }

  #[cfg(test)]
  pub fn patterns(&self) -> &[Arc<Pattern>] {
    &self.patterns
  }

  // sample files that have moved, from where to where (their sounds are
  // already loaded). returns whether any row's did
  pub fn move_row_samples(&mut self, moved: &[(String, String)]) -> bool {
    self.repoint_rows(&|path| moved.iter().find(|(from, _)| from == path).map(|(_, to)| to.clone()))
  }

  // sample files that have moved: `to` gives a path's new place, if it has
  // one. returns whether any row's did. (the row list is only copied when
  // one did: every undo step's copies are repointed)
  pub fn repoint_rows(&mut self, to: &dyn Fn(&str) -> Option<String>) -> bool {
    let moved: Vec<(u64, String)> = self.rows.iter()
      .filter_map(|row| to(&row.sample()?.path).map(|path| (row.id, path)))
      .collect();
    if moved.is_empty() { return false; }
    for (id, path) in moved {
      if let Some(sample) = self.row_mut(id).ok().and_then(Row::sample_mut) { sample.path = path; }
    }
    true
  }

  // whether any row plays a file at or under `path`
  pub fn plays_under(&self, path: &std::path::Path) -> bool {
    self.rows.iter().filter_map(Row::sample).any(|sample| std::path::Path::new(&sample.path).starts_with(path))
  }

  // whether another sequencer holds the same rows, patterns and folders (for
  // the history, to tell whether an edit changed anything). what the two
  // share isn't looked at
  pub fn same_as(&self, other: &Sequencer) -> bool {
    same(&self.rows, &other.rows)
      && self.patterns.len() == other.patterns.len()
      && self.patterns.iter().zip(&other.patterns).all(|(a, b)| Arc::ptr_eq(a, b) || a.same_as(b))
      && same(&self.folders, &other.folders)
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

  pub fn row(&self, id: u64) -> Option<&Row> {
    self.rows.iter().find(|r| r.id == id)
  }

  // a row to change, with a new rev (see Rev): every change to a row is
  // made through here. (the row list is copied only when the row is there)
  fn row_mut(&mut self, id: u64) -> Result<&mut Row, String> {
    let at = self.rows.iter().position(|r| r.id == id).ok_or_else(|| no_row(id))?;
    let row = &mut Arc::make_mut(&mut self.rows)[at];
    row.rev = Rev::next();
    Ok(row)
  }

  pub fn pattern(&self, id: u64) -> Option<&Pattern> {
    self.patterns.iter().find(|p| p.id == id).map(|p| &**p)
  }

  // a pattern to change, with a new rev (see Rev): every change to a
  // pattern is made through here
  fn pattern_mut(&mut self, id: u64) -> Result<&mut Pattern, String> {
    let shared = self.patterns.iter_mut().find(|p| p.id == id)
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    let pattern = Arc::make_mut(shared);
    pattern.rev = Rev::next();
    Ok(pattern)
  }

  // a row as the frontend sees it, after an edit of its own settings
  pub fn row_change(&self, id: u64, plugins: &dyn SynthPlugins) -> Result<RowChange, String> {
    let row = self.row(id).ok_or_else(|| no_row(id))?;
    Ok(RowChange { row: row.state(plugins) })
  }

  // a pattern as the frontend sees it, after an edit of it
  pub fn pattern_change(&self, id: u64) -> Result<PatternChange, String> {
    let pattern = self.pattern(id).ok_or_else(|| format!("no pattern with id {}", id))?;
    Ok(PatternChange { pattern: PatternState::of(pattern) })
  }

  // each synth row's plugin, for effects.rs to keep in line (see
  // Effects::follow_synths), in row order
  pub fn synths(&self) -> Vec<Synth> {
    self.rows.iter().filter_map(|row| {
      let synth = row.synth()?;
      Some(Synth {
        id: synth.slot,
        plugin_id: synth.plugin_id.clone(),
        path: synth.path.clone(),
        name: synth.name.clone(),
        vendor: synth.vendor.clone(),
        volume: row.volume,
        pan: row.pan,
        muted: row.muted,
      })
    }).collect()
  }

  // each sample row's notes a pattern plays (those that start inside it),
  // with what plays them, in row order. muted rows, and rows without notes,
  // are left out
  pub fn sample_notes<'a>(&'a self, pattern: &'a Pattern) -> impl Iterator<Item = (daw::SampleInstrument, &'a [PatternNote])> + 'a {
    self.rows.iter()
      .filter(|row| !row.muted)
      .map(|row| (row, pattern.heard(row.id)))
      .filter(|(_, notes)| !notes.is_empty())
      .filter_map(|(row, notes)| Some((row.sample_instrument()?, notes)))
  }

  // each synth row's notes in a pattern, with the row's slot (which its
  // plugin's copies go by), in row order. rows without notes are left out
  pub fn synth_notes<'a>(&'a self, pattern: &'a Pattern) -> impl Iterator<Item = (u64, &'a [PatternNote])> + 'a {
    self.rows.iter()
      .filter_map(move |row| Some((row.synth()?.slot, pattern.notes_of(row.id))))
      .filter(|(_, notes)| !notes.is_empty())
  }

  // the whole sequencer as the frontend sees it, with what `plugins` knows
  // of each synth row's plugin
  pub fn state(&self, plugins: &dyn SynthPlugins) -> SequencerState {
    SequencerState {
      rows: self.rows.iter().map(|row| row.state(plugins)).collect(),
      patterns: self.patterns.iter().map(|p| PatternState::of(p)).collect(),
      folders: self.folders.iter()
        .map(|f| FolderState { id: f.id, name: f.name.clone(), parent: f.parent })
        .collect(),
    }
  }

  // forget every row, pattern and folder, e.g. before loading a project
  pub fn clear(&mut self) {
    self.rows = Arc::default();
    self.patterns.clear();
    self.folders = Arc::default();
  }

  // add a sample row playing the sample at `path`, loaded here
  #[cfg(test)]
  pub fn add_row(&mut self, path: &str) -> Result<u64, String> {
    Ok(self.add_sample_row(path, RowSound::load(path)?))
  }

  // add a row playing a sample that's already been loaded (see RowSound),
  // at the bottom. returns its id
  pub fn add_sample_row(&mut self, path: &str, sound: RowSound) -> u64 {
    self.push_row(Instrument::Sample(SampleRow {
      path: path.to_string(),
      sample_pitch: DEFAULT_PITCH,
      gated: false,
      sound,
    }))
  }

  // add a row playing a plugin effects.rs has just loaded (see
  // Effects::load_synth), at the bottom, its name made unique. returns its id
  pub fn add_synth_row(&mut self, synth: Synth) -> u64 {
    let name = self.free_synth_name(&synth.name);
    self.push_row(Instrument::Synth(SynthRow {
      slot: synth.id,
      plugin_id: synth.plugin_id,
      path: synth.path,
      name,
      vendor: synth.vendor,
    }))
  }

  // a new row at the bottom (see Row::new). returns its id
  fn push_row(&mut self, instrument: Instrument) -> u64 {
    let id = self.next_row_id;
    self.next_row_id += 1;
    Arc::make_mut(&mut self.rows).push(Row::new(id, instrument));
    id
  }

  // `name`, or "name 2" and so on if a synth row has it already (case
  // aside)
  pub fn free_synth_name(&self, name: &str) -> String {
    self.free_synth_name_besides(name, None)
  }

  // the same, leaving out a row (whose plugin is being replaced)
  fn free_synth_name_besides(&self, name: &str, except: Option<u64>) -> String {
    let free = |candidate: &str| !self.rows.iter()
      .filter(|row| Some(row.id) != except)
      .filter_map(Row::synth)
      .any(|synth| synth.name.eq_ignore_ascii_case(candidate));
    if free(name) { return name.to_string(); }
    (2..).map(|n| format!("{} {}", name, n)).find(|candidate| free(candidate)).expect("a free name")
  }

  // take a row out, and its notes out of every pattern, each pattern that
  // held them then trimmed to what's left, in bars of `beats_per_bar` beats
  // (see trim_pattern). a synth row's plugin follows: see commands/synth.rs
  pub fn remove_row_and_trim(&mut self, id: u64, beats_per_bar: u16) -> Result<(), String> {
    for pattern in self.take_row(id)? {
      self.trim_pattern(pattern, beats_per_bar)?;
    }
    Ok(())
  }

  // remove_row_and_trim without the trim, for tests
  #[cfg(test)]
  pub fn remove_row(&mut self, id: u64) -> Result<(), String> {
    self.take_row(id).map(drop)
  }

  // take a row out, and its notes out of every pattern. returns the
  // patterns that held them
  fn take_row(&mut self, id: u64) -> Result<Vec<u64>, String> {
    let at = self.rows.iter().position(|r| r.id == id).ok_or_else(|| no_row(id))?;
    Arc::make_mut(&mut self.rows).remove(at);
    let holding: Vec<u64> = self.patterns.iter().filter(|p| p.notes.contains_key(&id)).map(|p| p.id).collect();
    for &pattern in &holding {
      self.pattern_mut(pattern)?.notes.remove(&id);
    }
    Ok(holding)
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

  // the pitch a row's clicked steps play. in each pattern where every one
  // of the row's notes is at the old step pitch, they move to the new one, so
  // a row showing steps still shows them. a pattern where they're at mixed
  // pitches is left as it is
  pub fn set_row_step_pitch(&mut self, id: u64, pitch: u8) -> Result<(), String> {
    let pitch = check_pitch(pitch)?;
    let row = self.row_mut(id)?;
    let old = std::mem::replace(&mut row.step_pitch, pitch);
    if old == pitch { return Ok(()); }

    let at_old: Vec<u64> = self.patterns.iter()
      .filter(|p| p.notes.get(&id).is_some_and(|notes| notes.iter().all(|note| note.pitch == old)))
      .map(|p| p.id)
      .collect();
    for pattern in at_old {
      let pattern = self.pattern_mut(pattern)?;
      let notes = pattern.notes_of(id).iter().map(|&note| PatternNote { pitch, ..note }).collect();
      pattern.set_notes(id, notes);
    }
    Ok(())
  }

  // the pitch that plays a sample row's sample unchanged
  pub fn set_row_sample_pitch(&mut self, id: u64, pitch: u8) -> Result<(), String> {
    let pitch = check_pitch(pitch)?;
    self.sample_row_mut(id)?.sample_pitch = pitch;
    Ok(())
  }

  // whether a sample row's notes stop at their ends (gated), or play the
  // sample out (one-shot)
  pub fn set_row_gated(&mut self, id: u64, gated: bool) -> Result<(), String> {
    self.sample_row_mut(id)?.gated = gated;
    Ok(())
  }

  // a sample row's sample settings to change (see row_mut)
  fn sample_row_mut(&mut self, id: u64) -> Result<&mut SampleRow, String> {
    self.row_mut(id)?.sample_mut().ok_or_else(|| format!("row {} plays a synth, not a sample", id))
  }

  // label a row ("Snare"). a blank label goes back to its instrument's name
  pub fn rename_row(&mut self, id: u64, label: &str) -> Result<(), String> {
    self.row_mut(id)?.label = row_label(label);
    Ok(())
  }

  // move a row to `index` in the list, counted without it (past the end:
  // to the bottom). its notes are kept by its id, so no pattern changes
  pub fn move_row(&mut self, id: u64, index: usize) -> Result<(), String> {
    let from = self.rows.iter().position(|r| r.id == id).ok_or_else(|| no_row(id))?;
    let rows = Arc::make_mut(&mut self.rows);
    let row = rows.remove(from);
    let to = index.min(rows.len());
    rows.insert(to, row);
    Ok(())
  }

  // play another sample (already loaded, see RowSound) on a sample row,
  // keeping everything else about it
  pub fn replace_row_sample(&mut self, id: u64, path: &str, sound: RowSound) -> Result<(), String> {
    let sample = self.sample_row_mut(id)?;
    sample.path = path.to_string();
    sample.sound = sound;
    Ok(())
  }

  // refuses a row that isn't there, or that plays a sample
  pub fn check_synth_row(&self, id: u64) -> Result<(), String> {
    match self.row(id).ok_or_else(|| no_row(id))?.synth() {
      Some(_) => Ok(()),
      None => Err(format!("row {} plays a sample, not a synth", id)),
    }
  }

  // play another plugin, which effects.rs has just loaded in a new slot (see
  // Effects::load_synth), on a synth row, keeping everything else about it.
  // its name is made unique among the other synth rows. the old slot's
  // plugin follows the rows out (see commands/synth.rs)
  pub fn replace_row_synth(&mut self, id: u64, synth: Synth) -> Result<(), String> {
    self.check_synth_row(id)?;
    let name = self.free_synth_name_besides(&synth.name, Some(id));
    self.row_mut(id)?.instrument = Instrument::Synth(SynthRow {
      slot: synth.id,
      plugin_id: synth.plugin_id,
      path: synth.path,
      name,
      vendor: synth.vendor,
    });
    Ok(())
  }

  // a new, empty pattern of `bars` bars, at the end, named "Sequence n" (see
  // free_pattern_name), its floor its length. returns its id
  pub fn add_pattern(&mut self, bars: u32, beats_per_bar: u16) -> Result<u64, String> {
    check_bars(bars)?;
    let id = self.next_pattern_id;
    let length = steps_in_bars(bars, beats_per_bar);
    self.insert_pattern(Pattern {
      id,
      rev: Rev::next(),
      name: Some(self.free_pattern_name()),
      length,
      floor: length,
      notes: BTreeMap::new(),
      folder: None,
    })?;
    Ok(id)
  }

  // a new pattern's name: "Sequence" with the lowest number no pattern is
  // shown with (an unnamed one counts as its label), case aside
  pub fn free_pattern_name(&self) -> String {
    let taken: BTreeSet<String> = self.patterns.iter().map(|p| p.label().to_lowercase()).collect();
    (1..).map(sequence_name).find(|name| !taken.contains(&name.to_lowercase())).expect("a free name")
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
    self.patterns.push(Arc::new(pattern));
    Ok(())
  }

  // a copy of a pattern, named after it, straight after it. returns its id
  pub fn duplicate_pattern(&mut self, id: u64) -> Result<u64, String> {
    let at = self.patterns.iter().position(|p| p.id == id)
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    let new_id = self.next_pattern_id;
    self.next_pattern_id += 1;

    let mut copy = (*self.patterns[at]).clone();
    copy.id = new_id;
    copy.rev = Rev::next();
    copy.name = copy.name.map(|n| format!("{} copy", n));
    self.patterns.insert(at + 1, Arc::new(copy));
    Ok(new_id)
  }

  // a new pattern made from others (see join.rs): any number of steps long,
  // its floor its length, with these notes by row (tidied and sorted, and
  // those of rows that aren't here left out), straight after the pattern
  // `after` and in its folder. returns its id
  pub fn add_pattern_after(
    &mut self,
    after: u64,
    name: String,
    length: u32,
    notes: BTreeMap<u64, Vec<PatternNote>>,
  ) -> Result<u64, String> {
    let at = self.patterns.iter().position(|p| p.id == after)
      .ok_or_else(|| format!("no pattern with id {}", after))?;
    if length == 0 {
      return Err("a pattern needs some steps".into());
    }
    let id = self.next_pattern_id;
    self.next_pattern_id += 1;

    let notes = self.known_rows_notes(notes);
    let folder = self.patterns[at].folder;
    self.patterns.insert(at + 1, Arc::new(Pattern {
      id, rev: Rev::next(), name: Some(name), length, floor: length, notes, folder,
    }));
    Ok(id)
  }

  // notes by row made sensible (see tidy_notes), those of rows that aren't
  // here, and rows left without any, left out
  fn known_rows_notes(&self, notes: BTreeMap<u64, Vec<PatternNote>>) -> BTreeMap<u64, Arc<Vec<PatternNote>>> {
    notes.into_iter()
      .filter(|(row, _)| self.row(*row).is_some())
      .map(|(row, notes)| (row, tidy_notes(notes)))
      .filter(|(_, notes)| !notes.is_empty())
      .map(|(row, notes)| (row, Arc::new(notes)))
      .collect()
  }

  pub fn remove_pattern(&mut self, id: u64) -> Result<(), String> {
    let at = self.patterns.iter().position(|p| p.id == id)
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    self.patterns.remove(at);
    Ok(())
  }

  // name a pattern. a blank name goes back to showing its number
  pub fn rename_pattern(&mut self, id: u64, name: &str) -> Result<(), String> {
    let name = name.trim();
    self.pattern_mut(id)?.name = if name.is_empty() { None } else { Some(name.to_string()) };
    Ok(())
  }

  // make a pattern `bars` long (the bars counter), and that its floor.
  // notes past a shortened pattern's end are kept (and don't play), so
  // making it longer again brings them back
  pub fn set_pattern_bars(&mut self, id: u64, bars: u32, beats_per_bar: u16) -> Result<(), String> {
    check_bars(bars)?;
    let pattern = self.pattern_mut(id)?;
    pattern.length = steps_in_bars(bars, beats_per_bar);
    pattern.floor = pattern.length;
    Ok(())
  }

  // replace a row's notes in a pattern, each made sensible (see
  // PatternNote::tidy), and sorted. one that makes no sense is left out.
  // the pattern's length stays as it is, so it can hold notes past its end
  pub fn set_pattern_notes(&mut self, id: u64, row: u64, notes: Vec<PatternNote>) -> Result<(), String> {
    if notes.len() > MAX_NOTES {
      return Err(format!("a pattern can hold at most {} notes for a row", MAX_NOTES));
    }
    if self.row(row).is_none() {
      return Err(no_row(row));
    }
    self.pattern_mut(id)?.set_notes(row, tidy_notes(notes));
    Ok(())
  }

  // a row's notes edited (the Piano Roll): set_pattern_notes, and the
  // pattern grows to fit the notes the edit added, up to PATTERN_BARS_MAX
  // bars of `beats_per_bar` beats (see added_end and grown_length). only
  // what's added counts, so notes kept past an end the bars counter made
  // shorter don't grow it back each time another note is moved. growing
  // never raises the floor
  pub fn edit_pattern_notes(&mut self, id: u64, row: u64, notes: Vec<PatternNote>, beats_per_bar: u16) -> Result<(), String> {
    let before = self.pattern(id).and_then(|p| p.notes.get(&row).cloned()).unwrap_or_default();
    self.set_pattern_notes(id, row, notes)?;
    let pattern = self.pattern(id).ok_or_else(|| format!("no pattern with id {}", id))?;
    let end = added_end(&before, pattern.notes_of(row));
    if end <= 0. { return Ok(()); }
    let length = grown_length(pattern.length, end, steps_in_bars(1, beats_per_bar));
    if length != pattern.length { self.pattern_mut(id)?.length = length; }
    Ok(())
  }

  // turn one of a row's steps on or off in a pattern (the Pattern
  // Sequencer's keys). a note is on the step when it starts there at the
  // row's step pitch. on adds a one-step note there, unless there's one
  // already; off takes out every note on it, whatever its length or velocity
  //
  // a step past the pattern's end can be set too, up to the longest the
  // pattern can grow to (PATTERN_BARS_MAX bars of `beats_per_bar` beats).
  // turning one on grows the pattern to the end of the step's bar (see
  // grown_length), whether it was on already or not. growing never raises
  // the floor. nothing that's already so is changed
  pub fn set_pattern_step(&mut self, pattern_id: u64, row_id: u64, step: u32, on: bool, beats_per_bar: u16) -> Result<(), String> {
    let bar = steps_in_bars(1, beats_per_bar);
    let pitch = self.row(row_id).ok_or_else(|| no_row(row_id))?.step_pitch;
    let pattern = self.pattern(pattern_id).ok_or_else(|| format!("no pattern with id {}", pattern_id))?;
    if step >= pattern.length.max(PATTERN_BARS_MAX * bar) {
      return Err(format!("pattern {} has {} steps, there's no step {}", pattern_id, pattern.length, step));
    }

    let length = if on { grown_length(pattern.length, f64::from(step + 1), bar) } else { pattern.length };
    let notes = pattern.notes_of(row_id);
    let on_step = |note: &PatternNote| note.start == step as f64 && note.pitch == pitch;
    let toggled = (notes.iter().any(on_step) != on).then(|| if on {
      let mut notes = notes.to_vec();
      notes.push(PatternNote { pitch, start: step as f64, length: 1., velocity: STEP_VELOCITY });
      sort_notes(&mut notes);
      notes
    } else {
      notes.iter().copied().filter(|note| !on_step(note)).collect()
    });
    if toggled.is_none() && length == pattern.length { return Ok(()); }

    let pattern = self.pattern_mut(pattern_id)?;
    pattern.length = length;
    if let Some(notes) = toggled { pattern.set_notes(row_id, notes); }
    Ok(())
  }

  // auto-trim: a pattern made as short as its notes let it, in bars of
  // `beats_per_bar` beats (see trimmed_length). the empty bars a clear left
  // at its end go, down to its floor (where the bars counter put it). with
  // nothing to trim, nothing changes, its rev included
  pub fn trim_pattern(&mut self, id: u64, beats_per_bar: u16) -> Result<(), String> {
    let pattern = self.pattern(id).ok_or_else(|| format!("no pattern with id {}", id))?;
    let length = trimmed_length(pattern.length, pattern.floor, pattern.last_end(), steps_in_bars(1, beats_per_bar));
    if length != pattern.length { self.pattern_mut(id)?.length = length; }
    Ok(())
  }

  // set_pattern_step in 4/4, for tests building patterns
  #[cfg(test)]
  pub fn set_step(&mut self, pattern_id: u64, row_id: u64, step: u32, on: bool) -> Result<(), String> {
    self.set_pattern_step(pattern_id, row_id, step, on, 4)
  }

  // how long a pattern plays for, at the beat length given
  pub fn pattern_len(&self, id: u64, beat_interval: Duration) -> Option<Duration> {
    self.pattern(id).map(|p| step_offset(p.length, beat_interval))
  }

  // what a pattern plays, if it plays notes of just one row (the patterns
  // an explode makes)
  pub fn sole_sound(&self, pattern_id: u64) -> Option<Sound<'_>> {
    let pattern = self.pattern(pattern_id)?;
    let mut rows = pattern.heard_rows();
    let row = rows.next()?;
    if rows.next().is_some() { return None; }
    Some(self.row(row)?.sound())
  }

  // split a pattern into a pattern per row it plays, sample or synth, each
  // holding just that row's notes inside it, at the same length and floor.
  // a pattern that already plays exactly those notes (made by an earlier
  // split, or by hand) is used rather than making another; new ones go
  // after the pattern, named like "verse (kick)". returns a part per row,
  // in row order. the pattern itself is left as it is
  pub fn split_pattern(&mut self, id: u64) -> Result<Vec<SplitPart>, String> {
    let source = self.pattern(id).cloned()
      .ok_or_else(|| format!("no pattern with id {}", id))?;
    let label = source.label();
    let heard: Vec<(u64, String, Vec<PatternNote>)> = self.rows.iter()
      .filter_map(|row| {
        let notes = source.heard(row.id);
        (!notes.is_empty()).then(|| (row.id, row.name(), notes.to_vec()))
      })
      .collect();
    if heard.is_empty() {
      return Err(format!("pattern {} has no notes to explode", label));
    }

    let mut insert_at = self.patterns.iter().position(|p| p.id == id).expect("found above") + 1;
    let mut parts = Vec::new();
    for (row, name, notes) in heard {
      let existing = self.patterns.iter()
        .find(|p| p.length == source.length && p.plays_just(row, &notes))
        .map(|p| p.id);

      let pattern = match existing {
        Some(found) => found,
        None => {
          let new_id = self.next_pattern_id;
          self.next_pattern_id += 1;
          self.patterns.insert(insert_at, Arc::new(Pattern {
            id: new_id,
            rev: Rev::next(),
            name: Some(format!("{} ({})", label, name)),
            length: source.length,
            floor: source.floor,
            notes: BTreeMap::from([(row, Arc::new(notes))]),
            // next to the pattern it came from
            folder: source.folder,
          }));
          insert_at += 1;
          new_id
        }
      };
      parts.push(SplitPart { row, name, pattern });
    }
    Ok(parts)
  }

  #[cfg(test)]
  pub fn folders(&self) -> &[PatternFolder] {
    &self.folders
  }

  pub fn folder(&self, id: u64) -> Option<&PatternFolder> {
    self.folders.iter().find(|f| f.id == id)
  }

  // (the folder list is copied only when the folder is there)
  fn folder_mut(&mut self, id: u64) -> Result<&mut PatternFolder, String> {
    let at = self.folders.iter().position(|f| f.id == id)
      .ok_or_else(|| format!("no pattern folder with id {}", id))?;
    Ok(&mut Arc::make_mut(&mut self.folders)[at])
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
    Arc::make_mut(&mut self.folders).push(PatternFolder { id, name, parent });
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
    if self.pattern(id).is_none() {
      return Err(format!("no pattern with id {}", id));
    }
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
    Arc::make_mut(&mut self.folders).retain(|f| !gone.contains(&f.id));
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
      Arc::make_mut(&mut self.folders).push(folder);
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

  // the sequencer for a project file (the synth rows' plugins' settings and
  // windows are the caller's to fill in: see project.rs)
  pub fn saved(&self) -> SavedSequencer {
    SavedSequencer {
      rows: self.rows.iter().map(Row::saved).collect(),
      patterns: self.patterns.iter().map(|p| p.saved()).collect(),
      folders: self.folders.to_vec(),
    }
  }

  // replace everything with a project's sequencer, loading the samples here
  // (see restore_loaded)
  #[cfg(test)]
  pub fn restore(&mut self, saved: SavedSequencer) -> Vec<(String, String)> {
    let sounds = RowSound::load_rows(&saved);
    self.restore_loaded(saved, sounds)
  }

  // replace everything with a project's sequencer, with its sample rows'
  // samples already loaded (see RowSound::load_rows). a sample row whose
  // sample can't be loaded is left out, with its notes, and returned as
  // (path, reason). synth rows are kept whether their plugins load or not
  // (see effects.rs). patterns that don't make sense (no steps, or an id
  // used twice) are left out
  pub fn restore_loaded(
    &mut self,
    saved: SavedSequencer,
    sounds: Vec<Option<Result<RowSound, String>>>,
  ) -> Vec<(String, String)> {
    self.clear();
    let mut missing = Vec::new();

    for (row, sound) in saved.rows.into_iter().zip(sounds) {
      // past left-out rows' ids too, so a new row never takes one
      self.next_row_id = self.next_row_id.max(row.id + 1);
      let sample_path = row.instrument.sample_path().map(str::to_string);
      if let Err(e) = self.restore_row(row, sound) {
        match sample_path {
          Some(path) => missing.push((path, e)),
          None => println!("left out a synth row: {}", e),
        }
      }
    }
    self.restore_folders(saved.folders);
    for saved in saved.patterns {
      let id = saved.id;
      let pattern = self.restored_pattern(saved);
      if let Err(e) = self.insert_pattern(pattern) {
        println!("left out pattern {}: {}", id, e);
      }
    }
    missing
  }

  // a project's row, with its sample's sound if it's a sample row. refused
  // when its id is taken, or its sample can't be loaded
  fn restore_row(&mut self, saved: SavedRow, sound: Option<Result<RowSound, String>>) -> Result<(), String> {
    if self.row(saved.id).is_some() {
      return Err(format!("there's already a row with id {}", saved.id));
    }
    let instrument = match saved.instrument {
      SavedInstrument::Sample(sample) => Instrument::Sample(SampleRow {
        // (one not loaded beforehand is loaded now)
        sound: sound.unwrap_or_else(|| RowSound::load(&sample.path))?,
        path: sample.path,
        sample_pitch: sample.sample_pitch.min(127),
        gated: sample.gated,
      }),
      SavedInstrument::Synth(synth) => Instrument::Synth(SynthRow {
        name: self.free_synth_name(&synth.name),
        slot: synth.slot,
        plugin_id: synth.plugin_id,
        path: synth.path,
        vendor: synth.vendor,
      }),
    };
    Arc::make_mut(&mut self.rows).push(Row {
      id: saved.id,
      rev: Rev::next(),
      label: saved.label.and_then(|l| row_label(&l)),
      volume: clamp_volume(saved.volume),
      pan: clamp_pan(saved.pan),
      muted: saved.muted,
      step_pitch: saved.step_pitch.min(127),
      instrument,
    });
    Ok(())
  }

  // a project's pattern made sensible: its notes tidied and sorted (those
  // of rows that aren't here left out), its floor within its length (none:
  // its length), and in a folder that's here or at the top
  fn restored_pattern(&self, saved: SavedPattern) -> Pattern {
    Pattern {
      id: saved.id,
      rev: Rev::next(),
      name: saved.name,
      length: saved.length,
      floor: if saved.floor == 0 { saved.length } else { saved.floor.min(saved.length) },
      notes: self.known_rows_notes(saved.notes),
      folder: saved.folder.filter(|&id| self.folder(id).is_some()),
    }
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

fn clamp_pan(pan: f32) -> f32 {
  if pan.is_finite() { pan.clamp(-1., 1.) } else { 0. }
}

// a row's label, trimmed and cut to ROW_LABEL_MAX characters. none when blank
fn row_label(label: &str) -> Option<String> {
  let label: String = label.trim().chars().take(ROW_LABEL_MAX).collect();
  let label = label.trim_end();
  if label.is_empty() { None } else { Some(label.to_string()) }
}

#[cfg(test)]
mod tests;
