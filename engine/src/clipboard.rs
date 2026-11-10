// copying, cutting, pasting and repeating clips in the playlist
//
// the clipboard is the engine's own, and holds copies of the clips
// themselves: a pasted clip plays the same sample or pattern as the clip it
// was copied from, with its own trim and settings, and no file is written.
// it has nothing to do with the system's clipboard
use std::time::Duration;
use serde::Serialize;

use crate::clip_commands::{changed_clips, ChangedClips};
use crate::daw;
use crate::event_dispatch::remove_clips;
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

// how many clips the clipboard holds
#[tauri::command]
pub fn clipboard_count(state: daw::TState<'_>) -> usize {
  state.clipboard.lock().unwrap().len()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;
  use crate::event_dispatch::{add_clip, change_clip_settings, node_beat};
  use crate::project;

  const KICK: &str = "assets/assets_66-bd-01.wav";

  fn state() -> Arc<daw::InnerState> {
    Arc::new(daw::InnerState::default())
  }

  fn saved(state: &daw::InnerState) -> project::Project {
    project::capture(state, None)
  }

  fn steps(state: &daw::InnerState) -> usize {
    state.history.lock().unwrap().len()
  }

  fn sample(state: &daw::InnerState, track: u32, beat: f32) -> u64 {
    add_clip(state, KICK.to_string(), track, Some(beat)).unwrap().id
  }

  // a one bar pattern with a kick on its first step, and a clip of it
  fn pattern_clip(state: &daw::InnerState, track: u32, beat: f64) -> (u64, u64) {
    let pattern = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let row = sequencer.add_row(KICK).unwrap();
      let pattern = sequencer.add_pattern(1, 4).unwrap();
      sequencer.set_step(pattern, row, 0, true).unwrap();
      pattern
    };
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let at = audiograph.beat_interval().mul_f64(beat);
    let clip = audiograph.add_pattern_node::<daw::timing::Note>(pattern, at, track, None).unwrap();
    (pattern, clip)
  }

  fn node(state: &daw::InnerState, id: u64) -> daw::AudioNode {
    state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().clone()
  }

  #[test]
  fn test_paste_puts_the_earliest_clip_on_the_beat_and_keeps_the_rest_apart() {
    let state = state();
    // on tracks 0 and 2, three and a half beats apart, and a pattern clip
    let (a, b) = (sample(&state, 0, 2.), sample(&state, 2, 5.5));
    let (pattern, c) = pattern_clip(&state, 1, 4.);
    change_clip_settings(&state, b, daw::ClipSettings { pitch: 3., pan: 0.5, ..Default::default() }, None).unwrap();
    crate::clip_commands::set_windows(&state, &[crate::clip_commands::WindowEdit {
      id: b, window: crate::event_dispatch::ClipWindow { start: 0.02, end: Some(0.09) },
    }]).unwrap();
    let b_beat = node_beat(&state, b).unwrap();
    state.history.lock().unwrap().clear();

    assert_eq!(copy(&state, &[b, a, c]), 3);
    let before = saved(&state);
    let pasted = paste(&state, 32., None).unwrap();
    assert_eq!(steps(&state), 1);
    assert_eq!(pasted.left_out, 0);

    // the earliest on beat 32, on the tracks they were copied from
    let clips = &pasted.clips.clips;
    assert_eq!(clips.len(), 2);
    assert_eq!((clips[0].start_beat, clips[0].track_number), (32., 0));
    assert!((clips[1].start_beat - (32. + b_beat - 2.)).abs() < 1e-5);
    assert_eq!(clips[1].track_number, 2);
    let patterns = &pasted.clips.pattern_clips;
    assert_eq!((patterns[0].start_beat, patterns[0].track_number, patterns[0].pattern_id), (34., 1, pattern));
    // new clips, with the trim and settings of the ones they were copied from
    assert!(![a, b, c].contains(&clips[1].id));
    assert_eq!(clips[1].info.settings, node(&state, b).settings());
    assert_eq!(clips[1].info.window, node(&state, b).window().map(crate::event_dispatch::ClipWindow::of_sample));
    assert_eq!(clips[0].path, KICK);

    // pasted again, a second set lands in the same place
    let again = paste(&state, 32., None).unwrap();
    assert_eq!(again.clips.clips[0].start_beat, 32.);
    assert_ne!(again.clips.clips[0].id, clips[0].id);
    assert_eq!(state.playlist.audiograph.lock().unwrap().nodes.len(), 6);

    // each paste is one step
    assert!(history::undo(&state).unwrap().is_some());
    let undone = history::undo(&state).unwrap().unwrap();
    assert_eq!(undone.clips, Some(vec![]));
    assert_eq!(saved(&state), before);
    let redone = history::redo(&state).unwrap().unwrap();
    assert_eq!(redone.clips.unwrap().len(), 3);
  }

  #[test]
  fn test_paste_on_a_track_moves_every_clip_by_as_many_tracks() {
    let state = state();
    let (a, b) = (sample(&state, 1, 2.), sample(&state, 3, 3.));
    copy(&state, &[a, b]);

    // the topmost copied clip (track 1) on track 4: the other two below it
    let pasted = paste(&state, 16.25, Some(4)).unwrap();
    let clips = &pasted.clips.clips;
    assert_eq!((clips[0].start_beat, clips[0].track_number), (16.25, 4));
    assert_eq!((clips[1].start_beat, clips[1].track_number), (17.25, 6));
    // and above where it was copied from
    let up = paste(&state, 8., Some(0)).unwrap();
    assert_eq!(up.clips.clips.iter().map(|clip| clip.track_number).collect::<Vec<_>>(), vec![0, 2]);
  }

  #[test]
  fn test_a_paste_past_the_end_pastes_nothing() {
    let state = state();
    let (a, b) = (sample(&state, 0, 2.), sample(&state, 1, 10.));
    copy(&state, &[a, b]);
    state.history.lock().unwrap().clear();
    let before = saved(&state);

    // the second clip would start on beat 258, past bar 64
    let refused = paste(&state, 250., None).unwrap_err();
    assert!(refused.contains("past the end of the playlist"), "{}", refused);
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);

    // the last beat itself is still the playlist
    assert!(paste(&state, 248., None).is_ok());
    assert!(paste(&state, f64::NAN, None).is_err());
    assert!(paste(&state, -1., None).is_err());
  }

  #[test]
  fn test_a_pasted_clip_is_its_own_clip_of_the_same_sound() {
    let state = state();
    let original = sample(&state, 0, 2.);
    copy(&state, &[original]);
    let pasted = paste(&state, 8., None).unwrap().clips.clips[0].id;
    let heard = |id: u64| -> Vec<f32> {
      let clip = node(&state, id);
      clip.voice(48_000, clip.start_offset.as_nanos() as i128, 0).collect()
    };
    let sound = heard(original);
    assert!(sound.iter().any(|&sample| sample != 0.));
    assert!(heard(pasted) == sound);

    // trimmed, pitched and turned down, the pasted clip changes nothing
    // about the one it was copied from
    crate::clip_commands::set_windows(&state, &[crate::clip_commands::WindowEdit {
      id: pasted, window: crate::event_dispatch::ClipWindow { start: 0.03, end: Some(0.06) },
    }]).unwrap();
    change_clip_settings(&state, pasted, daw::ClipSettings { pitch: -5., gain_db: -9., ..Default::default() }, None).unwrap();
    assert!(heard(pasted) != sound);
    assert!(heard(original) == sound);
    assert_eq!(node(&state, original).window(), None);
    assert_eq!(node(&state, original).settings(), daw::ClipSettings::default());
    // and the same the other way round
    let trimmed = heard(pasted);
    change_clip_settings(&state, original, daw::ClipSettings { pitch: 7., ..Default::default() }, None).unwrap();
    assert!(heard(pasted) == trimmed);
  }

  #[test]
  fn test_copy_and_paste_write_no_file() {
    let dir = std::env::temp_dir().join(format!("dawesome-clipboard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let take = dir.join("take.wav");
    std::fs::copy(KICK, &take).unwrap();
    let listing = || {
      let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
      names.sort();
      names
    };

    let state = state();
    let id = add_clip(&state, take.to_string_lossy().into_owned(), 0, Some(1.)).unwrap().id;
    copy(&state, &[id]);
    paste(&state, 4., None).unwrap();
    cut(&state, &[id]).unwrap();
    paste(&state, 9., Some(2)).unwrap();
    let first = state.playlist.audiograph.lock().unwrap().nodes[0].id;
    repeat(&state, &[first]).unwrap();
    assert_eq!(listing(), vec!["take.wav".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_cutting_takes_the_clips_out_and_leaves_the_rest() {
    let state = state();
    let (a, b, c) = (sample(&state, 0, 2.), sample(&state, 1, 4.), sample(&state, 0, 9.));
    state.history.lock().unwrap().clear();
    let before = saved(&state);

    assert_eq!(cut(&state, &[a, b]).unwrap(), 2);
    assert_eq!(steps(&state), 1);
    // gone, with nothing else moved
    {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      assert_eq!(audiograph.nodes.len(), 1);
      assert_eq!(audiograph.nodes[0].id, c);
    }
    assert_eq!(node_beat(&state, c), Ok(9.));

    // and they can be pasted
    let pasted = paste(&state, 20., None).unwrap();
    assert_eq!(pasted.clips.clips.iter().map(|clip| clip.start_beat).collect::<Vec<_>>(), vec![20., 22.]);
    history::undo(&state).unwrap();
    // one undo brings both back
    assert!(history::undo(&state).unwrap().is_some());
    assert_eq!(saved(&state), before);

    // with nothing to cut or copy, the clipboard keeps what it had
    assert_eq!(cut(&state, &[]).unwrap(), 2);
    assert_eq!(copy(&state, &[]), 2);
    assert_eq!(copy(&state, &[9_999]), 2);
    // a clip that isn't there: nothing is cut
    let had = steps(&state);
    assert!(cut(&state, &[c, 9_999]).is_err());
    assert_eq!(steps(&state), had);
    assert_eq!(state.clipboard.lock().unwrap().len(), 2);
  }

  #[test]
  fn test_a_clip_given_twice_is_copied_once() {
    let state = state();
    let a = sample(&state, 0, 2.);
    assert_eq!(copy(&state, &[a, a]), 1);
    assert_eq!(repeat(&state, &[a, a]).unwrap().clips.clips.len(), 1);
  }

  #[test]
  fn test_the_last_beat_is_still_the_playlist_at_any_tempo() {
    let state = state();
    let (a, b) = (sample(&state, 0, 2.), sample(&state, 1, 5.));
    for tempo in [97., 133.3, 61.7] {
      crate::event_dispatch::change_tempo(&state, tempo, None).unwrap();
      copy(&state, &[a, b]);
      // the second clip lands on beat 256, the last
      assert!(paste(&state, 253., None).is_ok(), "at {}bpm", tempo);
      assert!(paste(&state, 253.01, None).is_err(), "at {}bpm", tempo);
    }
  }

  #[test]
  fn test_a_clip_of_a_pattern_thats_gone_is_left_out() {
    let state = state();
    let a = sample(&state, 0, 2.);
    let (pattern, c) = pattern_clip(&state, 1, 3.);
    copy(&state, &[a, c]);
    crate::sequencer_commands::remove_pattern_for_tests(&state, pattern);

    // (left out even where it would have landed past the end: the paste
    // isn't refused for a clip that isn't pasted)
    assert_eq!(paste(&state, 255.5, None).unwrap().left_out, 1);
    let pasted = paste(&state, 8., None).unwrap();
    assert_eq!(pasted.left_out, 1);
    assert_eq!(pasted.clips.clips.len(), 1);
    assert!(pasted.clips.pattern_clips.is_empty());
    assert_eq!(pasted.clips.clips[0].start_beat, 8.);
  }

  #[test]
  fn test_repeat_starts_exactly_where_the_span_ends() {
    let state = state();
    // off the grid: from beat 2 to wherever the second clip ends
    state.playlist.ui.lock().unwrap().snap_enabled = false;
    let (a, b) = (sample(&state, 0, 2.), sample(&state, 2, 5.));
    let (_, c) = pattern_clip(&state, 1, 3.);
    state.history.lock().unwrap().clear();
    let before = saved(&state);
    let (first, last) = (node(&state, a), node(&state, b));
    let span = last.start_offset + last.duration() - first.start_offset;

    // the pattern clip ends on beat 7, after the second sample clip: the
    // span is from beat 2 to beat 7
    let pattern_end = Duration::from_millis(3_500);
    assert!(pattern_end > last.start_offset + last.duration());
    let repeated = repeat(&state, &[b, c, a]).unwrap();
    assert_eq!(steps(&state), 1);
    let clips = &repeated.clips.clips;
    let by_track = |track: u32| node(&state, clips.iter().find(|clip| clip.track_number == track).unwrap().id);
    // to the nanosecond, on the same tracks, the same distance apart
    assert_eq!(by_track(0).start_offset, pattern_end);
    assert_eq!(by_track(2).start_offset - by_track(0).start_offset, last.start_offset - first.start_offset);
    assert_eq!((repeated.clips.pattern_clips[0].start_beat, repeated.clips.pattern_clips[0].track_number), (8., 1));
    assert!(span < pattern_end);

    // without the pattern clip, the span ends with the last sample clip,
    // wherever that is
    history::undo(&state).unwrap();
    assert_eq!(saved(&state), before);
    let repeated = repeat(&state, &[a, b]).unwrap();
    let first_copy = node(&state, repeated.clips.clips.iter().map(|clip| clip.id).min().unwrap());
    assert_eq!(first_copy.start_offset, first.start_offset + span);

    // the clipboard is as it was: empty
    assert_eq!(state.clipboard.lock().unwrap().len(), 0);
    assert!(repeat(&state, &[]).unwrap().clips.clips.is_empty());
    assert!(repeat(&state, &[9_999]).is_err());
  }

  #[test]
  fn test_a_repeat_past_the_end_repeats_nothing() {
    let state = state();
    let (a, b) = (sample(&state, 0, 130.), sample(&state, 1, 250.));
    state.history.lock().unwrap().clear();
    let before = saved(&state);

    // from beat 130 to past beat 250: the second duplicate would be past bar 64
    let refused = repeat(&state, &[a, b]).unwrap_err();
    assert!(refused.contains("past the end of the playlist"), "{}", refused);
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);
  }

  #[test]
  fn test_the_halves_of_a_cut_clip_stay_together_when_pasted() {
    let state = state();
    state.playlist.ui.lock().unwrap().snap_enabled = false;
    let id = sample(&state, 0, 2.337);
    let changed = crate::clip_commands::split(&state, &[id], 2.4).unwrap();
    let right = changed.clips[1].id;
    let (left_node, right_node) = (node(&state, id), node(&state, right));
    assert_eq!(left_node.origin(), right_node.origin());

    copy(&state, &[id, right]);
    let pasted = paste(&state, 40.123, None).unwrap();
    let (a, b) = (node(&state, pasted.clips.clips[0].id), node(&state, pasted.clips.clips[1].id));
    // the second still starts on the very nanosecond the first ends on
    assert_eq!(a.origin(), b.origin());
    assert_eq!(a.start_offset + a.duration(), b.start_offset);

    // and repeated
    let repeated = repeat(&state, &[id, right]).unwrap();
    let (c, d) = (node(&state, repeated.clips.clips[0].id), node(&state, repeated.clips.clips[1].id));
    assert_eq!(c.origin(), d.origin());
    assert_eq!(c.start_offset, right_node.start_offset + right_node.duration());
  }

  #[test]
  fn test_pasted_at_another_tempo_clips_keep_their_beats() {
    let state = state();
    let (a, b) = (sample(&state, 0, 2.), sample(&state, 1, 6.));
    copy(&state, &[a, b]);
    crate::event_dispatch::change_tempo(&state, 90., None).unwrap();

    let pasted = paste(&state, 16., None).unwrap();
    let beats: Vec<f32> = pasted.clips.clips.iter().map(|clip| clip.start_beat).collect();
    assert!((beats[0] - 16.).abs() < 1e-5 && (beats[1] - 20.).abs() < 1e-5, "{:?}", beats);
  }

  #[test]
  fn test_a_new_project_empties_the_clipboard() {
    let state = state();
    let a = sample(&state, 0, 2.);
    assert_eq!(copy(&state, &[a]), 1);
    project::apply(&state, &project::Project::default(), None);
    assert_eq!(state.clipboard.lock().unwrap().len(), 0);
    assert_eq!(paste(&state, 4., None).unwrap(), Placed::default());
  }
}
