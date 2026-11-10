// trimming and cutting clips in the playlist (see the windows in
// daw/daw_core/audiograph.rs and clip_reader.rs)
//
// both commands return every clip they changed or made, as it is now, for
// the frontend to show
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::daw;
use crate::event_dispatch::ClipWindow;
use crate::history;
use crate::project::{LoadedClip, LoadedPatternClip};

// clips as they are after a change, of either kind
#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChangedClips {
  pub clips: Vec<LoadedClip>,
  pub pattern_clips: Vec<LoadedPatternClip>,
}

// the clips with these ids, as they are now (ids that name no clip are
// left out)
pub fn changed_clips(state: &daw::InnerState, ids: &[u64]) -> ChangedClips {
  let audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();

  ChangedClips {
    clips: ids.iter()
      .filter_map(|&id| audiograph.get_node_by_id(id))
      .map(|node| LoadedClip::of(node, interval))
      .collect(),
    pattern_clips: ids.iter()
      .filter_map(|&id| audiograph.get_pattern_node_by_id(id))
      .map(|node| LoadedPatternClip::of(node, interval))
      .collect(),
  }
}

// the part of its source a clip is to play: seconds of the sample for a
// sample clip, steps of the pattern for a pattern clip
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct WindowEdit {
  pub id: u64,
  #[serde(flatten)]
  pub window: ClipWindow,
}

// trim clips: set the part of its sample or pattern each one plays. the
// sound stays where it is in the playlist, so a clip whose start is
// trimmed starts later. every window is kept within its source and no
// shorter than a clip can be. all the clips are trimmed, as one step to
// undo, or (when an id names no clip) none are
#[tauri::command]
pub fn set_clip_windows(state: daw::TState<'_>, edits: Vec<WindowEdit>) -> Result<ChangedClips, String> {
  set_windows(&state, &edits)
}

pub(crate) fn set_windows(state: &daw::InnerState, edits: &[WindowEdit]) -> Result<ChangedClips, String> {
  let ids: Vec<u64> = edits.iter().map(|edit| edit.id).collect();

  history::clip_edit(state, &ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.placement(id).is_none()) {
      return Err(format!("no node with id {}", missing));
    }

    for edit in edits {
      if audiograph.get_node_by_id(edit.id).is_some() {
        let (start, end) = edit.window.sample_times();
        audiograph.set_window(edit.id, start, end)?;
      } else {
        audiograph.set_pattern_window(edit.id, edit.window.start, edit.window.end)?;
      }
    }
    Ok(())
  }, |_| vec![])?;

  Ok(changed_clips(state, &ids))
}

// cut clips in two at a beat of the playlist. each clip keeps what's
// before the cut, and a new clip plays what's after it (see
// AudioGraph::split_node). clips the beat isn't inside, or is too near an
// edge of, are left as they are. one step to undo, however many are cut
//
// returns both halves of every clip that was cut
#[tauri::command]
pub fn split_clips(state: daw::TState<'_>, ids: Vec<u64>, beat: f64) -> Result<ChangedClips, String> {
  split(&state, &ids, beat)
}

pub(crate) fn split(state: &daw::InnerState, ids: &[u64], beat: f64) -> Result<ChangedClips, String> {
  if !beat.is_finite() || beat < 0. {
    return Err(format!("{} isn't a beat of the playlist", beat));
  }

  // each clip that was cut, and the half that was made
  let cut = history::clip_edit(state, ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.placement(id).is_none()) {
      return Err(format!("no node with id {}", missing));
    }

    let at: Duration = audiograph.beat_interval().mul_f64(beat);
    let mut cut = Vec::new();
    for &id in ids {
      if let Some(made) = audiograph.split_node(id, at)? {
        cut.push((id, made));
      }
    }
    Ok(cut)
  }, |cut| cut.iter().map(|&(_, made)| made).collect())?;

  let touched: Vec<u64> = cut.iter().flat_map(|&(id, made)| [id, made]).collect();
  Ok(changed_clips(state, &touched))
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

  fn edit(id: u64, start: f64, end: Option<f64>) -> WindowEdit {
    WindowEdit { id, window: ClipWindow { start, end } }
  }

  // a sample clip a few hundred milliseconds long on beat 4, and a clip of
  // a one bar pattern on beat 8: (state, sample clip, pattern clip)
  fn clips() -> (Arc<daw::InnerState>, u64, u64) {
    let state = state();
    let sample = add_clip(&state, KICK.to_string(), 0, Some(4.)).unwrap().id;
    let pattern = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let row = sequencer.add_row(KICK).unwrap();
      let pattern = sequencer.add_pattern(1, 4).unwrap();
      sequencer.set_step(pattern, row, 0, true).unwrap();
      sequencer.set_step(pattern, row, 10, true).unwrap();
      pattern
    };
    let clip = state.playlist.audiograph.lock().unwrap()
      .add_pattern_node::<daw::timing::Note>(pattern, Duration::from_secs(4), 1, None).unwrap();
    state.history.lock().unwrap().clear();
    (state, sample, clip)
  }

  fn sample_length(state: &daw::InnerState, id: u64) -> f64 {
    state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().sample_duration().as_secs_f64()
  }

  #[test]
  fn test_trimming_clips_of_both_kinds_is_one_step() {
    let (state, sample, pattern) = clips();
    let length = sample_length(&state, sample);
    assert!(length > 0.1, "the kick is too short for this test");
    let before = saved(&state);

    let changed = set_windows(&state, &[edit(sample, 0.02, Some(0.08)), edit(pattern, 4., Some(12.))]).unwrap();
    assert_eq!(steps(&state), 1);
    let after = saved(&state);
    assert_ne!(before, after);

    // what comes back is each clip as it is now
    assert_eq!(changed.clips.len(), 1);
    assert_eq!(changed.clips[0].id, sample);
    assert_eq!(changed.clips[0].info.window, Some(ClipWindow { start: 0.02, end: Some(0.08) }));
    assert!((changed.clips[0].info.played_duration - 0.06).abs() < 1e-6);
    // 20ms later than beat 4, at 120bpm
    assert!((changed.clips[0].start_beat - 4.04).abs() < 1e-5);
    assert_eq!(changed.pattern_clips.len(), 1);
    assert_eq!(changed.pattern_clips[0].window, Some(ClipWindow { start: 4., end: Some(12.) }));
    // a step later for each of the four steps hidden: a beat
    assert_eq!(changed.pattern_clips[0].start_beat, 9.);

    assert!(history::undo(&state).unwrap().is_some());
    assert_eq!(saved(&state), before);
    assert_eq!(node_beat(&state, sample), Ok(4.));
    assert_eq!(state.playlist.audiograph.lock().unwrap().get_node_by_id(sample).unwrap().window(), None);
    assert!(history::redo(&state).unwrap().is_some());
    assert_eq!(saved(&state), after);

    // a window that's the whole source again is no window
    let whole = set_windows(&state, &[edit(sample, 0., None), edit(pattern, 0., Some(16.))]).unwrap();
    assert_eq!(whole.clips[0].info.window, None);
    assert_eq!(whole.pattern_clips[0].window, None);
    assert_eq!(saved(&state), before);
  }

  #[test]
  fn test_trimming_is_all_or_nothing() {
    let (state, sample, _) = clips();
    let before = saved(&state);

    assert!(set_windows(&state, &[edit(sample, 0.02, None), edit(9_999, 0., None)]).is_err());
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);

    // a trim that changes nothing is no step
    set_windows(&state, &[edit(sample, 0., None)]).unwrap();
    assert_eq!(steps(&state), 0);
    // numbers that make no sense are the source's ends
    set_windows(&state, &[edit(sample, f64::NAN, Some(f64::INFINITY))]).unwrap();
    set_windows(&state, &[edit(sample, -4., Some(-1.))]).unwrap();
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);
    // and an end before the start leaves the least a clip can be
    set_windows(&state, &[edit(sample, 0.05, Some(0.01))]).unwrap();
    let node = state.playlist.audiograph.lock().unwrap().get_node_by_id(sample).unwrap().clone();
    assert_eq!(node.window().unwrap().start, Duration::from_millis(50));
    assert_eq!(node.duration(), daw::MIN_CLIP);
  }

  #[test]
  fn test_cutting_a_selection_is_one_step() {
    let (state, sample, pattern) = clips();
    // a second sample clip, which the cut isn't inside
    let elsewhere = add_clip(&state, KICK.to_string(), 2, Some(20.)).unwrap().id;
    // (the pattern clip is moved under the cut: onto beat 3)
    state.playlist.audiograph.lock().unwrap().move_node(pattern, Duration::from_millis(1_500), 1).unwrap();
    change_clip_settings(&state, sample, daw::ClipSettings { attack_ms: 20., decay_ms: 30., ..Default::default() }, None).unwrap();
    state.history.lock().unwrap().clear();
    let before = saved(&state);

    // beat 4.1: 50ms into the sample clip, and 4.4 steps into the pattern clip
    let changed = split(&state, &[sample, pattern, elsewhere], 4.1).unwrap();
    assert_eq!(steps(&state), 1);
    let after = saved(&state);

    // both halves of each clip that was cut, and nothing of the one that wasn't
    assert_eq!(changed.clips.len(), 2);
    assert_eq!(changed.pattern_clips.len(), 2);
    let (left, right) = (&changed.clips[0], &changed.clips[1]);
    assert_eq!(left.id, sample);
    assert!(right.id != sample && right.id != pattern && right.id != elsewhere);
    assert_eq!(left.start_beat, 4.);
    assert!((right.start_beat - 4.1).abs() < 1e-5);
    assert_eq!(left.info.window, Some(ClipWindow { start: 0., end: right.info.window.map(|w| w.start) }));
    assert!((right.info.window.unwrap().start - 0.05).abs() < 1e-8);
    assert_eq!(right.info.window.unwrap().end, None);
    assert_eq!((left.path.as_str(), right.track_number), (KICK, 0));
    // the fade in stays with the first half, the fade out goes with the second
    assert_eq!((left.info.settings.attack_ms, left.info.settings.decay_ms), (20., 0.));
    assert_eq!((right.info.settings.attack_ms, right.info.settings.decay_ms), (0., 30.));
    let (first, second) = (&changed.pattern_clips[0], &changed.pattern_clips[1]);
    assert_eq!(first.id, pattern);
    assert_eq!(first.window, Some(ClipWindow { start: 0., end: Some(4.4) }));
    assert_eq!(second.window, Some(ClipWindow { start: 4.4, end: None }));
    assert_eq!((first.start_beat, second.track_number), (3., 1));
    assert!((second.start_beat - 4.1).abs() < 1e-5);

    // one undo puts every clip back as it was, and takes the new ones away
    let undone = history::undo(&state).unwrap().unwrap();
    assert_eq!(undone.clips, Some(vec![sample, pattern]));
    assert_eq!(saved(&state), before);
    {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      assert_eq!((audiograph.nodes.len(), audiograph.pattern_nodes.len()), (2, 1));
    }
    // redone, the halves are the same clips again
    let redone = history::redo(&state).unwrap().unwrap();
    assert_eq!(redone.clips.unwrap().len(), 4);
    assert_eq!(saved(&state), after);
    assert_eq!(changed_clips(&state, &[right.id]).clips[0], *right);
  }

  #[test]
  fn test_a_cut_that_cuts_nothing_is_no_step() {
    let (state, sample, pattern) = clips();
    let before = saved(&state);

    // beat 30 is inside neither clip, and beat 4 is the sample clip's edge
    for beat in [30., 4., 4.001] {
      assert_eq!(split(&state, &[sample, pattern], beat).unwrap(), ChangedClips::default());
    }
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);

    assert!(split(&state, &[sample, 9_999], 4.1).is_err());
    assert!(split(&state, &[sample], f64::NAN).is_err());
    assert!(split(&state, &[sample], -1.).is_err());
    assert_eq!(saved(&state), before);
  }
}
