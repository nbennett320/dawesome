// editing the Pattern Sequencer (see daw/sequencer.rs), and
// placing patterns in the playlist. commands/sequencer.rs is the frontend's
// side of these
//
// every edit returns what it changed (one row, one pattern, or the whole
// sequencer), so the frontend's copy stays the same as the engine's. edits
// take effect straight away, even while playing
use std::sync::Arc;
use serde::Serialize;

use crate::daw::{audiograph, playback, sequencer, timing};
use crate::engine::mixer;
use crate::daw;
use crate::daw::history;
use crate::daw::actions::playlist::beat_to_offset;
use crate::daw::project::LoadedPatternClip;

use crate::daw::actions::synth::sequencer_state;

// run an edit on the sequencer, as a step that can be undone, then return
// the whole sequencer
// - gesture: the press of the mouse it's part of, if any (a stroke painting
//   steps, a volume knob dragged): one press is one step to undo
pub(crate) fn edit(
  state: &daw::Session,
  gesture: Option<u64>,
  f: impl FnOnce(&mut sequencer::Sequencer) -> Result<(), String>,
) -> Result<sequencer::SequencerState, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  Ok(sequencer_state(state))
}

// the same for an edit of one row's own settings, returning just that row
pub(crate) fn edit_row(
  state: &daw::Session,
  gesture: Option<u64>,
  row_id: u64,
  f: impl FnOnce(&mut sequencer::Sequencer) -> Result<(), String>,
) -> Result<sequencer::RowChange, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  state.playlist.sequencer.lock().unwrap().row_change(row_id, &*state.playlist.effects)
}

// and for an edit of one pattern, returning just that pattern
pub(crate) fn edit_pattern(
  state: &daw::Session,
  gesture: Option<u64>,
  pattern_id: u64,
  f: impl FnOnce(&mut sequencer::Sequencer) -> Result<(), String>,
) -> Result<sequencer::PatternChange, String> {
  history::sequencer_edit(state, &[], gesture, || f(&mut state.playlist.sequencer.lock().unwrap()))?;
  state.playlist.sequencer.lock().unwrap().pattern_change(pattern_id)
}

pub(crate) fn beats_per_bar(state: &daw::Session) -> u16 {
  state.playlist.time_signature.lock().unwrap().numerator
}

// a row that was just added, and the sequencer with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddedRow {
  pub row_id: u64,
  pub sequencer: sequencer::SequencerState,
}

pub(crate) fn add_sample(state: &daw::Session, path: &str) -> Result<AddedRow, String> {
  // decoded before taking the sequencer's lock, which playback needs
  let sound = sequencer::RowSound::load(path)?;
  let row_id = history::sequencer_edit(state, &[], None, || {
    Ok(state.playlist.sequencer.lock().unwrap().add_sample_row(path, sound))
  })?;
  Ok(AddedRow { row_id, sequencer: sequencer_state(state) })
}

pub(crate) fn remove_row_of(state: &daw::Session, row_id: u64) -> Result<sequencer::SequencerState, String> {
  let beats = beats_per_bar(state);
  edit(state, None, |seq| seq.remove_row_and_trim(row_id, beats))
}

pub(crate) fn replace_sample(state: &daw::Session, row_id: u64, path: &str) -> Result<sequencer::RowChange, String> {
  // decoded before taking the sequencer's lock, which playback needs
  let sound = sequencer::RowSound::load(path)?;
  edit_row(state, None, row_id, |seq| seq.replace_row_sample(row_id, path, sound))
}

// a pattern that was just made, and the sequencer with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddedPattern {
  id: u64,
  sequencer: sequencer::SequencerState,
}

pub(crate) fn add_pattern_of(state: &daw::Session, bars: u32) -> Result<AddedPattern, String> {
  let beats = beats_per_bar(state);
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().add_pattern(bars, beats)
  })?;
  Ok(AddedPattern { id, sequencer: sequencer_state(state) })
}

pub(crate) fn duplicate_pattern_of(state: &daw::Session, pattern_id: u64) -> Result<AddedPattern, String> {
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().duplicate_pattern(pattern_id)
  })?;
  Ok(AddedPattern { id, sequencer: sequencer_state(state) })
}

// a pattern that was deleted: the sequencer without it, and the playlist
// clips of it that went with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemovedPattern {
  removed_clips: Vec<u64>,
  sequencer: sequencer::SequencerState,
}

pub(crate) fn remove_pattern_of(state: &daw::Session, pattern_id: u64) -> Result<RemovedPattern, String> {
  // its clips go with it, and come back with it when that's undone
  let clips: Vec<u64> = state.playlist.audiograph.lock().unwrap().pattern_nodes.iter()
    .filter(|node| node.pattern_id == pattern_id)
    .map(|node| node.id)
    .collect();

  let removed_clips = history::sequencer_edit(state, &clips, None, || {
    // the graph is locked before the sequencer, as everywhere
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    sequencer.remove_pattern(pattern_id)?;
    Ok(audiograph.remove_pattern_nodes_of(pattern_id))
  })?;
  Ok(RemovedPattern { removed_clips, sequencer: sequencer_state(state) })
}

// a folder that was just made, and the sequencer with it
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddedFolder {
  id: u64,
  sequencer: sequencer::SequencerState,
}

pub(crate) fn add_pattern_folder_to(state: &daw::Session, name: &str, parent: Option<u64>) -> Result<AddedFolder, String> {
  let id = history::sequencer_edit(state, &[], None, || {
    state.playlist.sequencer.lock().unwrap().add_folder(name, parent)
  })?;
  Ok(AddedFolder { id, sequencer: sequencer_state(state) })
}

pub(crate) fn remove_pattern_folder_of(state: &daw::Session, folder_id: u64) -> Result<sequencer::SequencerState, String> {
  history::sequencer_edit(state, &[], None, || {
    // the graph is locked before the sequencer, as everywhere
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    let inside = sequencer.patterns_in_folder(folder_id);
    let placed = inside.iter().filter(|id| audiograph.pattern_nodes.iter().any(|n| n.pattern_id == **id)).count();
    if placed > 0 {
      return Err(format!(
        "{} of its patterns {} in the playlist",
        placed, if placed == 1 { "is" } else { "are" },
      ));
    }
    sequencer.remove_folder(folder_id).map(|_| ())
  })?;
  Ok(sequencer_state(state))
}

pub(crate) fn set_pattern_step_in(
  state: &daw::Session,
  pattern_id: u64,
  row_id: u64,
  step: u32,
  on: bool,
  gesture: Option<u64>,
  trim: bool,
) -> Result<sequencer::PatternChange, String> {
  let beats = beats_per_bar(state);
  edit_pattern(state, gesture, pattern_id, |seq| {
    seq.set_pattern_step(pattern_id, row_id, step, on, beats)?;
    if trim { seq.trim_pattern(pattern_id, beats)?; }
    Ok(())
  })
}

pub(crate) fn set_pattern_notes_in(
  state: &daw::Session,
  pattern_id: u64,
  row_id: u64,
  notes: Vec<sequencer::PatternNote>,
  gesture: Option<u64>,
  trim: bool,
) -> Result<sequencer::PatternChange, String> {
  let beats = beats_per_bar(state);
  edit_pattern(state, gesture, pattern_id, |seq| {
    seq.edit_pattern_notes(pattern_id, row_id, notes, beats)?;
    if trim { seq.trim_pattern(pattern_id, beats)?; }
    Ok(())
  })
}

pub(crate) fn trim_pattern_in(state: &daw::Session, pattern_id: u64, gesture: Option<u64>) -> Result<sequencer::PatternChange, String> {
  let beats = beats_per_bar(state);
  edit_pattern(state, gesture, pattern_id, |seq| seq.trim_pattern(pattern_id, beats))
}

pub(crate) fn toggle_pattern_in(state: &Arc<daw::Session>, pattern_id: u64, from_step: f64) -> Result<(), String> {
  if state.playlist.playing() {
    playback::pause_playlist(state);
    Ok(())
  } else {
    playback::start_pattern(state, pattern_id, from_step)
  }
}

pub(crate) fn add_pattern_clip_to(
  state: &daw::Session,
  pattern_id: u64,
  track_number: u32,
  drop_beat: Option<f32>,
) -> Result<LoadedPatternClip, String> {
  let start_offset = beat_to_offset(state, drop_beat);
  let subdivision = {
    let ui = state.playlist.ui.lock().unwrap();
    if ui.snap_enabled { Some(ui.snap_subdivision) } else { None }
  };

  // one step to undo
  let id = crate::daw::history::clip_edit(state, &[], None, || state.playlist.audiograph.lock().unwrap()
    .add_pattern_node(pattern_id, start_offset, track_number, subdivision), |&id| vec![id])?;

  let audiograph = state.playlist.audiograph.lock().unwrap();
  let node = audiograph.get_pattern_node_by_id(id).ok_or_else(|| format!("no node with id {}", id))?;
  Ok(LoadedPatternClip::of(node, audiograph.beat_interval()))
}

// pattern clips that were exploded: the ones taken out, the pattern clips
// that replaced them (one per row), the sequencer (with the patterns the
// explode made), and the mixer (whose tracks may have been named)
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Exploded {
  removed: Vec<u64>,
  clips: Vec<LoadedPatternClip>,
  sequencer: sequencer::SequencerState,
  mixer: mixer::MixerSettings,
}

pub(crate) fn explode(state: &daw::Session, ids: &[u64]) -> Result<Exploded, String> {
  // one step to undo, which also takes back the patterns it made and the
  // names it gave tracks. recorded even if it fails part-way, as the clips
  // exploded by then stay exploded
  let before = history::picture(state, ids);
  let mut touched = ids.to_vec();
  let result = explode_each(state, ids, &mut touched);
  history::record_between(state, None, before, history::picture(state, &touched));
  result
}

// - touched: the clips it made are added to these
fn explode_each(state: &daw::Session, ids: &[u64], touched: &mut Vec<u64>) -> Result<Exploded, String> {
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();
  let mut removed = Vec::new();
  let mut clips = Vec::new();

  // one at a time, so a later clip's rows land on the tracks an earlier
  // one's went to
  for &id in ids {
    if !explodable(&audiograph, id) { continue; }
    let made = audiograph.explode_pattern_node(id)?;
    touched.extend(&made);
    removed.push(id);
    for new_id in made {
      let node = audiograph.get_pattern_node_by_id(new_id).expect("a clip that was just made");
      clips.push(LoadedPatternClip::of(node, interval));
    }
  }

  let mixer = state.playlist.mixer.settings(audiograph.track_count());
  drop(audiograph);
  Ok(Exploded { removed, clips, sequencer: sequencer_state(state), mixer })
}

// whether a clip is a pattern clip with notes to explode. (a selection
// often holds an empty clip painted with the Draw tool: refusing it would
// leave the clips exploded before it unseen by the app)
fn explodable(audiograph: &audiograph::AudioGraph<'static>, id: u64) -> bool {
  let Some(clip) = audiograph.get_pattern_node_by_id(id) else { return false };
  audiograph.sequencer.lock().unwrap().pattern(clip.pattern_id).map_or(false, |p| p.plays_notes())
}

// the clips a drag from `from` to `to` (either way round, already snapped)
// paints, of a pattern `length` beats long, as (start, end) in beats: end to
// end from where the drag starts, the last one cut short where it ends. a
// press let go where it went down paints one whole clip, and an end piece
// shorter than `min_length` is left off. the same rule as paintSpans in
// src/editing/paint.ts, which draws them while the mouse is held
pub(crate) fn paint_spans(from: f64, to: f64, length: f64, min_length: f64) -> Vec<(f64, f64)> {
  if !(length > 0.) { return vec![]; }
  let start = from.min(to).max(0.);
  let end = from.max(to);
  if end - start < min_length { return vec![(start, start + length)]; }

  let mut spans = Vec::new();
  let mut i = 0.;
  while start + i * length < end {
    let at = start + i * length;
    let until = (at + length).min(end);
    if until - at >= min_length { spans.push((at, until)); }
    i += 1.;
  }
  spans
}

// how many bars a new pattern painted over a drag from `from` to `to` is:
// enough to hold it, from 1 to `max_bars`. the same rule as paintBars in
// src/editing/paint.ts
pub(crate) fn paint_bars(from: f64, to: f64, beats_per_bar: f64, max_bars: u32) -> u32 {
  if !(beats_per_bar > 0.) { return 1; }
  let bars = (((to - from).abs() - 1e-9) / beats_per_bar).ceil();
  bars.clamp(1., max_bars.max(1) as f64) as u32
}

// pattern clips painted with the Draw tool: the clips made, in order, the
// pattern they play (made by the paint when none was given), the sequencer,
// the mixer, and how many tracks the playlist has now (a paint below the last
// track makes one)
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Painted {
  clips: Vec<LoadedPatternClip>,
  pattern_id: u64,
  sequencer: sequencer::SequencerState,
  mixer: mixer::MixerSettings,
  track_count: u32,
}

pub(crate) fn paint(
  state: &daw::Session,
  pattern_id: Option<u64>,
  track_number: u32,
  start_beat: f64,
  end_beat: f64,
) -> Result<Painted, String> {
  if !start_beat.is_finite() || !end_beat.is_finite() {
    return Err(format!("{} to {} isn't a stretch of the playlist", start_beat, end_beat));
  }
  // checked first, so a pattern that isn't there changes nothing
  if let Some(id) = pattern_id {
    if state.playlist.sequencer.lock().unwrap().pattern(id).is_none() {
      return Err(format!("no pattern with id {}", id));
    }
  }
  let beats_per_bar = beats_per_bar(state);

  // one step to undo: the pattern, if one's made, and every clip
  let before = history::picture(state, &[]);
  let made = (|| -> Result<(u64, Vec<u64>), String> {
    // the graph is locked before the sequencer, as everywhere. (it locks the
    // sequencer itself to place a clip, so the sequencer's lock is let go
    // before then)
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let (pattern, steps) = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let pattern = match pattern_id {
        Some(id) => id,
        None => {
          let bars = paint_bars(start_beat, end_beat, beats_per_bar as f64, sequencer::PATTERN_BARS_MAX);
          sequencer.add_pattern(bars, beats_per_bar)?
        }
      };
      let steps = sequencer.pattern(pattern).map(|p| p.length)
        .ok_or_else(|| format!("no pattern with id {}", pattern))?;
      (pattern, steps)
    };

    let per_beat = sequencer::STEPS_PER_BEAT as f64;
    let length = steps as f64 / per_beat;
    let interval = audiograph.beat_interval();
    let mut clips = Vec::new();
    for (at, until) in paint_spans(start_beat, end_beat, length, audiograph::MIN_PATTERN_CLIP / per_beat) {
      let id = audiograph.add_pattern_node(pattern, interval.mul_f64(at), track_number, None::<timing::Note>)?;
      clips.push(id);
      // (the last, cut short where the drag ends)
      if until < at + length {
        audiograph.set_pattern_window(id, 0., Some((until - at) * per_beat))?;
      }
    }
    Ok((pattern, clips))
  })();
  let ids: Vec<u64> = made.as_ref().map(|(_, clips)| clips.clone()).unwrap_or_default();
  history::record_between(state, None, before, history::picture(state, &ids));
  let (pattern_id, ids) = made?;

  let audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();
  let clips = ids.iter()
    .filter_map(|&id| audiograph.get_pattern_node_by_id(id))
    .map(|node| LoadedPatternClip::of(node, interval))
    .collect();
  let track_count = audiograph.track_count();
  let mixer = state.playlist.mixer.settings(track_count);
  drop(audiograph);
  Ok(Painted { clips, pattern_id, sequencer: sequencer_state(state), mixer, track_count })
}

// (for other modules' tests)
#[cfg(test)]
pub(crate) fn add_pattern_clip_for_tests(state: &daw::Session, pattern_id: u64, track_number: u32, drop_beat: Option<f32>) -> u64 {
  add_pattern_clip_to(state, pattern_id, track_number, drop_beat).unwrap().id
}

#[cfg(test)]
pub(crate) fn remove_pattern_for_tests(state: &daw::Session, pattern_id: u64) {
  // without its clips going with it, as the clipboard's copies don't
  state.playlist.sequencer.lock().unwrap().remove_pattern(pattern_id).unwrap();
  state.playlist.audiograph.lock().unwrap().remove_pattern_nodes_of(pattern_id);
}

#[cfg(test)]
mod tests;
