// copying, cutting, pasting and repeating clips in the playlist
//
// the clipboard is the engine's own, and holds copies of the clips
// themselves: a pasted clip plays the same sample or pattern as the clip it
// was copied from, with its own trim and settings, and no file is written.
// it has nothing to do with the system's clipboard
use std::time::Duration;
use serde::Serialize;

use crate::commands::clip::{changed_clips, ChangedClips};
use crate::daw;
use crate::commands::clip::remove_clips;
use crate::history::{self, ClipSnap};

// a copied clip
#[derive(Clone)]
struct Copied {
  clip: ClipSnap,
  // how long after the earliest copied clip it starts: in time, and in
  // beats, as they were when it was copied
  after: Duration,
  beats_after: f64,
}

// the clips last copied or cut
#[derive(Default)]
pub struct Clipboard {
  clips: Vec<Copied>,
  // the tempo they were copied at. pasted at another, they keep their
  // beats rather than their times
  tempo: f32,
}

impl Clipboard {
  pub fn len(&self) -> usize {
    self.clips.len()
  }

  pub fn clear(&mut self) {
    self.clips.clear();
  }
}

fn start_of(clip: &ClipSnap) -> Duration {
  match clip {
    ClipSnap::Sample(node) => node.start_offset,
    ClipSnap::Pattern(node) => node.start_offset,
  }
}

fn track_of(clip: &ClipSnap) -> u32 {
  match clip {
    ClipSnap::Sample(node) => node.track_number,
    ClipSnap::Pattern(node) => node.track_number,
  }
}

// a clip of either kind, as it is now
fn snap(audiograph: &daw::AudioGraph<'static>, id: u64) -> Option<ClipSnap> {
  audiograph.get_node_by_id(id).cloned().map(ClipSnap::Sample)
    .or_else(|| audiograph.get_pattern_node_by_id(id).cloned().map(ClipSnap::Pattern))
}

// where a clip ends: a pattern clip with its pattern, or with the part of
// it that it plays
fn end_of(audiograph: &daw::AudioGraph<'static>, clip: &ClipSnap) -> Duration {
  match clip {
    ClipSnap::Sample(node) => node.start_offset + node.duration(),
    ClipSnap::Pattern(node) => {
      let length = audiograph.sequencer.lock().unwrap()
        .pattern(node.pattern_id)
        .map_or(0, |pattern| pattern.length);
      node.start_offset + node.duration(length, audiograph.beat_interval())
    }
  }
}

// the playlist's last beat, as a time: a clip can start there, and no later
fn playlist_end(state: &daw::InnerState, audiograph: &daw::AudioGraph<'static>) -> Duration {
  let beats = state.playlist.ui.lock().unwrap().max_beats_displayed;
  audiograph.beat_interval().mul_f64(beats as f64)
}

const PAST_THE_END: &str = "would start past the end of the playlist";

// put copies of clips into the playlist, each with a new id, on the track
// and at the time given. a clip of a pattern that's gone is left out. if
// any would start past the playlist's end, none is added. returns the new
// clips' ids, and how many were left out
fn place(
  state: &daw::InnerState,
  audiograph: &mut daw::AudioGraph<'static>,
  copies: Vec<(ClipSnap, Duration, u32)>,
  what: &str,
) -> Result<(Vec<u64>, usize), String> {
  // clips of patterns that are gone are left out, wherever they'd land
  let count = copies.len();
  let copies: Vec<_> = {
    let sequencer = audiograph.sequencer.lock().unwrap();
    copies.into_iter()
      .filter(|(clip, _, _)| match clip {
        ClipSnap::Sample(_) => true,
        ClipSnap::Pattern(node) => sequencer.pattern(node.pattern_id).is_some(),
      })
      .collect()
  };
  let left_out = count - copies.len();

  // (a clip that lands on the last beat give or take the rounding of its
  // place is on it)
  let end = playlist_end(state, audiograph) + Duration::from_micros(1);
  if copies.iter().any(|(_, start, _)| *start > end) {
    return Err(format!("Nothing was {}: a clip {}.", what, PAST_THE_END));
  }

  let mut made = Vec::new();
  for (clip, start, track) in copies {
    match clip {
      ClipSnap::Sample(mut node) => {
        node.start_offset = start;
        node.track_number = track;
        made.push(audiograph.insert_node(node));
      }
      ClipSnap::Pattern(mut node) => {
        node.start_offset = start;
        node.track_number = track;
        made.push(audiograph.insert_pattern_node(node));
      }
    }
  }
  Ok((made, left_out))
}

// copy clips to the clipboard, replacing what was there. with no clips
// (nothing was selected), the clipboard is left as it is. returns how many
// clips the clipboard holds
#[tauri::command]
pub fn copy_clips(state: daw::TState<'_>, ids: Vec<u64>) -> usize {
  copy(&state, &ids)
}

pub(crate) fn copy(state: &daw::InnerState, ids: &[u64]) -> usize {
  let audiograph = state.playlist.audiograph.lock().unwrap();
  // (an id given twice is copied once)
  let mut ids = ids.to_vec();
  ids.sort_unstable();
  ids.dedup();
  let mut clips: Vec<ClipSnap> = ids.iter().filter_map(|&id| snap(&audiograph, id)).collect();
  let mut clipboard = state.clipboard.lock().unwrap();
  if clips.is_empty() { return clipboard.len(); }

  // in the order they start, the earliest first
  clips.sort_by_key(start_of);
  let first = start_of(&clips[0]);
  let interval = audiograph.beat_interval().as_secs_f64();
  clipboard.tempo = audiograph.tempo();
  clipboard.clips = clips.into_iter().map(|clip| {
    let after = start_of(&clip) - first;
    Copied { beats_after: if interval > 0. { after.as_secs_f64() / interval } else { 0. }, after, clip }
  }).collect();
  clipboard.len()
}

// copy clips to the clipboard and take them out of the playlist, leaving
// the gap. one step to undo. returns how many clips the clipboard holds
#[tauri::command]
pub fn cut_clips(state: daw::TState<'_>, ids: Vec<u64>) -> Result<usize, String> {
  cut(&state, &ids)
}

pub(crate) fn cut(state: &daw::InnerState, ids: &[u64]) -> Result<usize, String> {
  if ids.is_empty() { return Ok(state.clipboard.lock().unwrap().len()); }
  // (checked first: nothing is copied unless the clips can be removed)
  {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.placement(id).is_none()) {
      return Err(format!("no node with id {}", missing));
    }
  }

  let count = copy(state, ids);
  remove_clips(state, ids)?;
  Ok(count)
}

// what a paste or a repeat made
#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Placed {
  #[serde(flatten)]
  pub clips: ChangedClips,
  // clips of patterns that are gone, which were left out
  pub left_out: usize,
}

// paste the clipboard's clips, the earliest starting at `beat` (which the
// frontend has already snapped) and the rest as long after it as they were.
// each goes on the track it was copied from, or, when `track` is given, the
// topmost copied clip goes on that track and the others as many tracks
// below it as they were. a paste can overlap what's there. one step to undo
//
// if any clip would start past the playlist's end, nothing is pasted
#[tauri::command]
pub fn paste_clips(state: daw::TState<'_>, beat: f64, track: Option<u32>) -> Result<Placed, String> {
  paste(&state, beat, track)
}

pub(crate) fn paste(state: &daw::InnerState, beat: f64, track: Option<u32>) -> Result<Placed, String> {
  if !beat.is_finite() || beat < 0. {
    return Err(format!("{} isn't a beat of the playlist", beat));
  }
  let (copied, tempo) = {
    let clipboard = state.clipboard.lock().unwrap();
    (clipboard.clips.clone(), clipboard.tempo)
  };
  if copied.is_empty() { return Ok(Placed::default()); }

  let top = copied.iter().map(|copied| track_of(&copied.clip)).min().unwrap_or(0);
  let (made, left_out) = history::clip_edit(state, &[], None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let interval = audiograph.beat_interval();
    let at = interval.mul_f64(beat);
    // at the tempo they were copied at, the clips are exactly as far apart
    // as they were (so the halves of a cut clip still meet). at another,
    // they're as many beats apart
    let same_tempo = audiograph.tempo() == tempo;

    let copies = copied.into_iter().map(|copied| {
      let after = if same_tempo { copied.after } else { interval.mul_f64(copied.beats_after) };
      let on = match track {
        Some(track) => track + (track_of(&copied.clip) - top),
        None => track_of(&copied.clip),
      };
      (copied.clip, at + after, on)
    }).collect();
    place(state, &mut audiograph, copies, "pasted")
  }, |(made, _)| made.clone())?;

  Ok(Placed { clips: changed_clips(state, &made), left_out })
}

// duplicate clips so that the duplicates start exactly where the clips'
// span ends: from the earliest start among them to the latest end. each
// duplicate is on its original's track, and they keep the gaps between
// them. the clipboard isn't changed. one step to undo
//
// if any duplicate would start past the playlist's end, nothing is repeated
#[tauri::command]
pub fn repeat_clips(state: daw::TState<'_>, ids: Vec<u64>) -> Result<Placed, String> {
  repeat(&state, &ids)
}

pub(crate) fn repeat(state: &daw::InnerState, ids: &[u64]) -> Result<Placed, String> {
  if ids.is_empty() { return Ok(Placed::default()); }
  let mut ids = ids.to_vec();
  ids.sort_unstable();
  ids.dedup();

  let (made, left_out) = history::clip_edit(state, &[], None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let mut clips = Vec::new();
    for &id in &ids {
      clips.push(snap(&audiograph, id).ok_or_else(|| format!("no node with id {}", id))?);
    }

    let start = clips.iter().map(start_of).min().unwrap_or_default();
    let end = clips.iter().map(|clip| end_of(&audiograph, clip)).max().unwrap_or_default();
    let span = end.saturating_sub(start);
    let copies = clips.into_iter().map(|clip| {
      let (at, track) = (start_of(&clip) + span, track_of(&clip));
      (clip, at, track)
    }).collect();
    place(state, &mut audiograph, copies, "repeated")
  }, |(made, _)| made.clone())?;

  Ok(Placed { clips: changed_clips(state, &made), left_out })
}

#[cfg(test)]
mod tests;
