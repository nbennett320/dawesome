// reading a project saved before version 4, when a pattern was a drum
// pattern (lit steps on sample rows) or a note pattern (notes for synths),
// never both, and a folder of patterns held one kind
//
// the project's JSON is rewritten as it's read, before it's deserialised
// (as drop_stage_one does): the sequencer goes under "patternSequencer" in
// version 4's shape, and its pattern clips get a start and a length. what
// the old app's load did is done here first (ids used twice left out,
// folders mended), so the project opens showing what the old app showed:
//
// - rows: the sample rows, then the synths as synth rows, numbered again from
//   0 in that order. a synth keeps its id as its plugin's slot
// - a drum pattern's lit steps become one-step notes at the row's step
//   pitch, at full velocity; a note pattern's notes go to their synth's row,
//   sorted. the other kind's are dropped, as the old app dropped them
// - an unnamed pattern is named what it was shown as ("Loop 2", "Sequence
//   3"), and can't shrink below its length (its floor)
// - a note folder named like a drum folder beside it gets a number ("Verse
//   2"), as the two trees become one
// - a pattern clip's part of its pattern is frozen as a start and a length
//
// see context/pattern-merge-project/04-api.md, section 6
use std::collections::{BTreeMap, BTreeSet};
use serde::Deserialize;
use serde_json::Value;

use crate::commands::clip::ClipWindow;
use crate::daw;
use super::ProjectPatternClip;

// the first version whose patterns hold notes for every row
const MERGED: u64 = 4;

// what a pattern held, before version 4
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Kind {
  #[default]
  Drum,
  Note,
}

impl Kind {
  // what the old app showed an unnamed pattern numbered `n` as
  fn unnamed(self, n: u64) -> String {
    match self {
      Kind::Drum => format!("Loop {}", n),
      Kind::Note => format!("Sequence {}", n),
    }
  }
}

// the sequencer as the old app read it. what it refused, this refuses: a
// row or pattern without a numeric id, a step that isn't a whole number from
// 0, a kind that isn't "drum" or "note"
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct OldSequencer {
  rows: Vec<OldRow>,
  synths: Vec<OldSynth>,
  patterns: Vec<OldPattern>,
  folders: Vec<OldFolder>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldRow {
  id: u64,
  path: String,
  #[serde(default)]
  label: Option<String>,
  #[serde(default = "full_volume")]
  volume: f32,
  #[serde(default)]
  pan: f32,
  #[serde(default)]
  muted: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldSynth {
  id: u64,
  plugin_id: String,
  name: String,
  #[serde(default)]
  vendor: String,
  #[serde(default)]
  path: String,
  #[serde(default = "full_volume")]
  volume: f32,
  #[serde(default)]
  muted: bool,
  #[serde(default)]
  state: String,
  #[serde(default)]
  window: Option<crate::effects::WindowPosition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldPattern {
  id: u64,
  name: Option<String>,
  #[serde(default)]
  kind: Kind,
  length: u32,
  steps: BTreeMap<u64, BTreeSet<u32>>,
  #[serde(default)]
  folder: Option<u64>,
  #[serde(default)]
  notes: BTreeMap<u64, Vec<daw::PatternNote>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldFolder {
  id: u64,
  name: String,
  #[serde(default)]
  kind: Kind,
  #[serde(default)]
  parent: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldPatternClip {
  pattern_id: u64,
  track_number: u32,
  start_beat: f64,
  #[serde(default)]
  window: Option<ClipWindow>,
}

fn full_volume() -> f32 { 1. }

fn invalid(e: serde_json::Error) -> String {
  format!("not a valid project file: {}", e)
}

// rewrite a project saved before version 4 as version 4 (see the top of the
// file). one that's version 4 already, or has no sequencer or pattern clips,
// is left as it is
pub(super) fn merge_pattern_kinds(project: &mut Value) -> Result<(), String> {
  // (a version that isn't a number is refused when it's deserialised)
  let Some(version) = project.get("version").map_or(Some(1), Value::as_u64) else { return Ok(()) };
  let Some(fields) = project.as_object_mut() else { return Ok(()) };
  if version >= MERGED || !(fields.contains_key("sequencer") || fields.contains_key("patternClips")) {
    return Ok(());
  }

  let old: OldSequencer = match fields.remove("sequencer") {
    Some(sequencer) => serde_json::from_value(sequencer).map_err(invalid)?,
    None => OldSequencer::default(),
  };
  let old_clips: Vec<OldPatternClip> = match fields.remove("patternClips") {
    Some(clips) => serde_json::from_value(clips).map_err(invalid)?,
    None => Vec::new(),
  };

  let (rows, ids) = merged_rows(old.rows, old.synths);
  let folders = mended_folders(old.folders);
  let kinds: BTreeMap<u64, Kind> = folders.iter().map(|f| (f.id, f.kind)).collect();
  let patterns: Vec<daw::SavedPattern> = old.patterns.into_iter()
    .map(|pattern| merged_pattern(pattern, &ids, &kinds))
    .collect();
  let clips = frozen_clips(old_clips, &patterns);

  let sequencer = daw::SavedSequencer {
    rows,
    patterns,
    folders: folders.into_iter().map(|f| daw::PatternFolder { id: f.id, name: f.name, parent: f.parent }).collect(),
  };
  fields.insert("patternSequencer".into(), serde_json::to_value(sequencer).map_err(invalid)?);
  fields.insert("patternClips".into(), serde_json::to_value(clips).map_err(invalid)?);
  fields.insert("version".into(), MERGED.into());
  Ok(())
}

// the new row id of each old row, and of each old synth
struct RowIds {
  rows: BTreeMap<u64, u64>,
  synths: BTreeMap<u64, u64>,
}

// the sample rows, then the synths, as rows numbered from 0. an id used
// twice (in rows, or in synths) is left out the second time, as the old
// app left it out
fn merged_rows(old_rows: Vec<OldRow>, old_synths: Vec<OldSynth>) -> (Vec<daw::SavedRow>, RowIds) {
  let mut ids = RowIds { rows: BTreeMap::new(), synths: BTreeMap::new() };
  let mut rows = Vec::new();

  for old in old_rows {
    if ids.rows.contains_key(&old.id) { continue; }
    let id = rows.len() as u64;
    ids.rows.insert(old.id, id);
    rows.push(daw::SavedRow {
      id,
      label: old.label,
      volume: old.volume,
      pan: old.pan,
      muted: old.muted,
      step_pitch: daw::DEFAULT_PITCH,
      instrument: daw::SavedInstrument::Sample(daw::SavedSample {
        path: old.path,
        sample_pitch: daw::DEFAULT_PITCH,
        gated: false,
      }),
    });
  }
  for old in old_synths {
    if ids.synths.contains_key(&old.id) { continue; }
    let id = rows.len() as u64;
    ids.synths.insert(old.id, id);
    rows.push(daw::SavedRow {
      id,
      label: None,
      volume: old.volume,
      pan: 0.,
      muted: old.muted,
      step_pitch: daw::DEFAULT_PITCH,
      instrument: daw::SavedInstrument::Synth(daw::SavedSynth {
        slot: old.id,
        plugin_id: old.plugin_id,
        name: old.name,
        vendor: old.vendor,
        path: old.path,
        state: old.state,
        window: old.window,
      }),
    });
  }
  (rows, ids)
}

// the folders as the old app's load left them, in order: one whose id was
// used already left out, then each in turn (seeing the changes made to the
// ones before) at the top when its parent isn't there, holds the other kind,
// or is inside it (the parents loop). then the note folders that clash with
// drum folders renamed (see renamed_clashes)
fn mended_folders(saved: Vec<OldFolder>) -> Vec<OldFolder> {
  let mut folders: Vec<OldFolder> = Vec::new();
  for folder in saved {
    if !folders.iter().any(|f| f.id == folder.id) { folders.push(folder); }
  }
  for at in 0..folders.len() {
    let (id, kind) = (folders[at].id, folders[at].kind);
    let lost = folders[at].parent.map_or(false, |parent| {
      folders.iter().find(|f| f.id == parent).map_or(true, |p| p.kind != kind) || folder_within(&folders, parent, id)
    });
    if lost { folders[at].parent = None; }
  }
  renamed_clashes(&mut folders);
  folders
}

// whether folder `id` is `ancestor`, or inside it, as the old app worked it
// out: parents that loop stop the search
fn folder_within(folders: &[OldFolder], id: u64, ancestor: u64) -> bool {
  let mut at = Some(id);
  for _ in 0..=folders.len() {
    match at {
      Some(found) if found == ancestor => return true,
      Some(found) => at = folders.iter().find(|f| f.id == found).and_then(|f| f.parent),
      None => return false,
    }
  }
  false
}

// each note folder named like a drum folder in the same parent (case aside)
// gets the lowest number from 2 that no folder there is named with, counting
// the folders renamed before it: "Verse 2" (D5)
fn renamed_clashes(folders: &mut [OldFolder]) {
  let named = |folders: &[OldFolder], parent: Option<u64>, name: &str, kind: Option<Kind>| folders.iter()
    .any(|f| f.parent == parent && kind.map_or(true, |k| f.kind == k) && f.name.to_lowercase() == name.to_lowercase());
  for at in 0..folders.len() {
    let (name, parent) = (folders[at].name.clone(), folders[at].parent);
    if folders[at].kind != Kind::Note || !named(folders, parent, &name, Some(Kind::Drum)) { continue; }
    let renamed = (2..).map(|n| format!("{} {}", name, n))
      .find(|candidate| !named(folders, parent, candidate, None))
      .expect("a free name");
    folders[at].name = renamed;
  }
}

// a pattern as version 4 has it (see the top of the file)
// - kinds: each folder's kind
fn merged_pattern(old: OldPattern, ids: &RowIds, kinds: &BTreeMap<u64, Kind>) -> daw::SavedPattern {
  let notes: BTreeMap<u64, Vec<daw::PatternNote>> = match old.kind {
    Kind::Drum => old.steps.into_iter()
      .filter_map(|(row, steps)| Some((*ids.rows.get(&row)?, steps.into_iter().map(step_note).collect())))
      .collect(),
    Kind::Note => old.notes.into_iter()
      .filter_map(|(synth, mut notes)| {
        daw::sort_notes(&mut notes);
        Some((*ids.synths.get(&synth)?, notes))
      })
      .collect(),
  };
  daw::SavedPattern {
    id: old.id,
    name: Some(old.name.unwrap_or_else(|| old.kind.unnamed(old.id + 1))),
    length: old.length,
    floor: old.length,
    notes: notes.into_iter().filter(|(_, notes)| !notes.is_empty()).collect(),
    // at the top, as the old app showed it, when its folder isn't there or
    // holds the other kind
    folder: old.folder.filter(|folder| kinds.get(folder) == Some(&old.kind)),
  }
}

// a lit step as a note: one step long at the row's step pitch, as loud as a
// hit was (D3)
fn step_note(step: u32) -> daw::PatternNote {
  daw::PatternNote { pitch: daw::DEFAULT_PITCH, start: step as f64, length: 1., velocity: 1. }
}

// the pattern clips, each with the part of its pattern it played frozen as
// a start and a length, the old app's way: a start that makes no sense is
// the pattern's start, and an end that's left out, makes no sense, or is past
// the pattern's end is its end. a clip of a pattern that isn't there is
// left out, as the old app left it out
fn frozen_clips(old: Vec<OldPatternClip>, patterns: &[daw::SavedPattern]) -> Vec<ProjectPatternClip> {
  old.into_iter().filter_map(|clip| {
    let length = patterns.iter().find(|p| p.id == clip.pattern_id)?.length as f64;
    let start = clip.window.map(|w| w.start).filter(|start| start.is_finite() && *start > 0.).unwrap_or(0.);
    let end = clip.window.and_then(|w| w.end).filter(|end| end.is_finite() && *end > start);
    let to = end.unwrap_or(length).min(length).max(start + daw::MIN_PATTERN_CLIP);
    Some(ProjectPatternClip {
      pattern_id: clip.pattern_id,
      track_number: clip.track_number,
      start_beat: clip.start_beat,
      start,
      length: to - start,
    })
  }).collect()
}

#[cfg(test)]
mod tests;
