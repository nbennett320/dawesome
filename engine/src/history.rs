// undo and redo
//
// every change to the project (clips, the mixer, the sequencer, the tempo)
// is kept as a step: what it changed, as it was before and as it was after.
// undoing a step puts back the "before", redoing it the "after". steps hold
// whole snapshots of what they touched, rather than how to reverse each
// command, so a step can't drift from what it describes
//
// one gesture is one step: a fader dragged from one place to another sends
// many changes, each tagged with the gesture's id, and they're folded into
// the same step (its first "before", its last "after")
//
// the history isn't saved, and starts again with each project. files on disk
// are never touched by it: undoing a recording, or a bounced clip, leaves
// the file where it is
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use crate::daw;

// how many steps are kept
pub const LIMIT: usize = 100;

// a clip as it was, kept so it can be put back with the same id
#[derive(Clone)]
pub enum ClipSnap {
  Sample(daw::AudioNode),
  Pattern(daw::PatternNode),
}

impl ClipSnap {
  fn start_offset(&self) -> Duration {
    match self {
      ClipSnap::Sample(node) => node.start_offset,
      ClipSnap::Pattern(node) => node.start_offset,
    }
  }

  fn set_start_offset(&mut self, offset: Duration) {
    match self {
      ClipSnap::Sample(node) => node.start_offset = offset,
      ClipSnap::Pattern(node) => node.start_offset = offset,
    }
  }
}

// a clip, and the beat it starts on. the beat is what's kept: the tempo may
// have changed by the time the clip is put back
#[derive(Clone)]
pub struct Placed {
  clip: ClipSnap,
  beat: f64,
}

impl Placed {
  // whether two snapshots of a clip are of the same clip, unchanged
  fn same(&self, other: &Placed) -> bool {
    if (self.beat - other.beat).abs() > 1e-9 { return false; }

    match (&self.clip, &other.clip) {
      (ClipSnap::Sample(a), ClipSnap::Sample(b)) => a.same_clip(b),
      (ClipSnap::Pattern(a), ClipSnap::Pattern(b)) =>
        a.pattern_id == b.pattern_id && a.track_number == b.track_number && a.window == b.window,
      _ => false,
    }
  }
}

// some clips, each as it was (None: it wasn't there)
pub type Clips = Vec<(u64, Option<Placed>)>;

// the clips with these ids, as they are now
pub fn clips(state: &daw::InnerState, ids: &[u64]) -> Clips {
  let audiograph = state.playlist.audiograph.lock().unwrap();
  clips_in(&audiograph, ids)
}

pub fn clips_in(audiograph: &daw::AudioGraph<'static>, ids: &[u64]) -> Clips {
  let interval = audiograph.beat_interval();
  let beat = |offset: Duration| if interval.is_zero() { 0. } else {
    offset.as_secs_f64() / interval.as_secs_f64()
  };

  ids.iter().map(|&id| {
    let clip = audiograph.get_node_by_id(id).cloned().map(ClipSnap::Sample)
      .or_else(|| audiograph.get_pattern_node_by_id(id).cloned().map(ClipSnap::Pattern));
    (id, clip.map(|clip| Placed { beat: beat(clip.start_offset()), clip }))
  }).collect()
}

// what a step changed. each part is (before, after)
#[derive(Default)]
pub struct Step {
  // the gesture it belongs to, if any: later changes of the same gesture
  // are folded into it
  gesture: Option<u64>,
  clips: BTreeMap<u64, (Option<Placed>, Option<Placed>)>,
  mixer: Option<(daw::MixerSettings, daw::MixerSettings)>,
  sequencer: Option<(daw::Sequencer, daw::Sequencer)>,
  tempo: Option<(f32, f32)>,
}

impl Step {
  pub fn new() -> Self {
    Step::default()
  }

  // as part of a gesture (see the top of the file)
  pub fn in_gesture(mut self, gesture: Option<u64>) -> Self {
    self.gesture = gesture;
    self
  }

  // clips that changed, came (they're not in `before`) or went (they're not
  // in `after`)
  pub fn with_clips(mut self, before: Clips, after: Clips) -> Self {
    let mut after: BTreeMap<u64, Option<Placed>> = after.into_iter().collect();
    for (id, was) in before {
      let is = after.remove(&id).flatten();
      self.clips.insert(id, (was, is));
    }
    for (id, is) in after {
      self.clips.insert(id, (None, is));
    }
    self
  }

  pub fn with_mixer(mut self, before: daw::MixerSettings, after: daw::MixerSettings) -> Self {
    self.mixer = Some((before, after));
    self
  }

  pub fn with_sequencer(mut self, before: daw::Sequencer, after: daw::Sequencer) -> Self {
    self.sequencer = Some((before, after));
    self
  }

  pub fn with_tempo(mut self, before: f32, after: f32) -> Self {
    self.tempo = Some((before, after));
    self
  }

  // drop the parts that didn't change anything
  fn tidy(&mut self) {
    self.clips.retain(|_, (before, after)| match (before, after) {
      (None, None) => false,
      (Some(before), Some(after)) => !before.same(after),
      _ => true,
    });
    if matches!(&self.mixer, Some((before, after)) if before == after) { self.mixer = None; }
    if matches!(&self.sequencer, Some((before, after)) if before.same_as(after)) { self.sequencer = None; }
    if matches!(self.tempo, Some((before, after)) if before == after) { self.tempo = None; }
  }

  fn is_empty(&self) -> bool {
    self.clips.is_empty() && self.mixer.is_none() && self.sequencer.is_none() && self.tempo.is_none()
  }

  // fold a later change of the same gesture into this step: what was before
  // stays, what's after is the later change's
  fn absorb(&mut self, later: Step) {
    for (id, (before, after)) in later.clips {
      match self.clips.get_mut(&id) {
        Some(mine) => mine.1 = after,
        None => { self.clips.insert(id, (before, after)); }
      }
    }
    if let Some((before, after)) = later.mixer {
      self.mixer = Some((self.mixer.take().map_or(before, |mine| mine.0), after));
    }
    if let Some((before, after)) = later.sequencer {
      self.sequencer = Some((self.sequencer.take().map_or(before, |mine| mine.0), after));
    }
    if let Some((before, after)) = later.tempo {
      self.tempo = Some((self.tempo.take().map_or(before, |mine| mine.0), after));
    }
  }

  // the clips this step changes, for the frontend to select
  fn clip_ids(&self) -> Vec<u64> {
    self.clips.keys().copied().collect()
  }
}

#[derive(Default)]
pub struct History {
  undo: Vec<Step>,
  redo: Vec<Step>,
  // goes up with every change recorded, and when the history is cleared
  changes: u64,
}

impl History {
  pub fn clear(&mut self) {
    self.undo.clear();
    self.redo.clear();
    self.changes += 1;
  }

  pub fn can_undo(&self) -> bool {
    !self.undo.is_empty()
  }

  pub fn can_redo(&self) -> bool {
    !self.redo.is_empty()
  }

  pub fn len(&self) -> usize {
    self.undo.len()
  }

  // a change was made: it can be undone, and what was undone before it
  // can't be redone any more
  pub fn push(&mut self, mut step: Step) {
    step.tidy();
    if step.is_empty() { return; }

    self.changes += 1;
    self.redo.clear();
    match self.undo.last_mut() {
      Some(top) if step.gesture.is_some() && top.gesture == step.gesture => {
        top.absorb(step);
        // a gesture that ended where it began changed nothing
        top.tidy();
        if top.is_empty() { self.undo.pop(); }
      }
      _ => {
        self.undo.push(step);
        if self.undo.len() > LIMIT {
          let extra = self.undo.len() - LIMIT;
          self.undo.drain(..extra);
        }
      }
    }
  }

  // sample files moved (takes brought into the project's folder, or renamed
  // in the recordings browser): what's kept here follows them, so a clip or
  // sequencer row put back later points at where its file is now
  // - to: a path's new place, if it has one
  pub fn repoint(&mut self, to: &dyn Fn(&str) -> Option<String>) {
    let follow = |placed: &mut Option<Placed>| {
      if let Some(Placed { clip: ClipSnap::Sample(node), .. }) = placed {
        if let Some(path) = to(node.sample_path()) {
          node.set_sample_path(path);
        }
      }
    };

    for step in self.undo.iter_mut().chain(self.redo.iter_mut()) {
      for (before, after) in step.clips.values_mut() {
        follow(before);
        follow(after);
      }
      if let Some((before, after)) = &mut step.sequencer {
        before.repoint_rows(to);
        after.repoint_rows(to);
      }
    }
  }

  // the same, for files that moved from one path to another
  // - moved: (from, to) paths
  pub fn move_paths(&mut self, moved: &[(String, String)]) {
    if moved.is_empty() { return; }
    self.repoint(&|path| moved.iter().find(|(from, _)| from == path).map(|(_, to)| to.clone()));
  }
}

// which way a step is applied
#[derive(Clone, Copy, PartialEq)]
enum Way { Undo, Redo }

// what undoing or redoing changed
#[derive(Debug, PartialEq)]
pub struct Applied {
  // the clips it changed or put back (not the ones it took away), for the
  // frontend to select. None when the step wasn't about clips (a fader, the
  // tempo), which leaves the selection as it is
  pub clips: Option<Vec<u64>>,
}

// record a change that's just been made
pub fn record(state: &daw::InnerState, step: Step) {
  state.history.lock().unwrap().push(step);
}

// undo the last change. None when there's nothing to undo, or it can't be
// undone just now (while recording)
pub fn undo(state: &Arc<daw::InnerState>) -> Result<Option<Applied>, String> {
  travel(state, Way::Undo)
}

// redo the last change undone
pub fn redo(state: &Arc<daw::InnerState>) -> Result<Option<Applied>, String> {
  travel(state, Way::Redo)
}

fn travel(state: &Arc<daw::InnerState>, way: Way) -> Result<Option<Applied>, String> {
  // a take is going onto the playlist as it is now
  if crate::recording::is_busy(state) { return Ok(None); }

  let (step, changes) = {
    let mut history = state.history.lock().unwrap();
    let step = match way {
      Way::Undo => history.undo.pop(),
      Way::Redo => history.redo.pop(),
    };
    (step, history.changes)
  };
  let step = match step {
    Some(step) => step,
    None => return Ok(None),
  };

  let applied = apply(state, &step, way);

  let mut history = state.history.lock().unwrap();
  // a change recorded meanwhile (a sample that was still loading when it
  // was dropped) is the newest thing done: it has ended redo, and this step
  // can't be gone back to
  if history.changes == changes {
    match way {
      Way::Undo => history.redo.push(step),
      Way::Redo => history.undo.push(step),
    }
  }
  Ok(Some(applied))
}

// put the project as a step says it was before it (undo) or after it (redo).
// the tempo goes first, as clips are placed by their beats; then the
// sequencer, whose patterns the pattern clips play
fn apply(state: &Arc<daw::InnerState>, step: &Step, way: Way) -> Applied {
  let pick = |before: bool| (way == Way::Undo) == before;

  if let Some((before, after)) = step.tempo {
    let tempo = if pick(true) { before } else { after };
    if let Err(e) = daw::set_playlist_tempo(state, tempo) {
      println!("could not put the tempo back to {}: {}", tempo, e);
    }
  }

  let mut audiograph = state.playlist.audiograph.lock().unwrap();

  if let Some((before, after)) = &step.sequencer {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    sequencer.restore_from(if pick(true) { before } else { after });
  }

  let interval = audiograph.beat_interval();
  let mut clips = Vec::new();
  for (&id, (before, after)) in &step.clips {
    let _ = audiograph.remove_node(id);

    let placed = if pick(true) { before } else { after };
    if let Some(placed) = placed {
      let mut clip = placed.clip.clone();
      clip.set_start_offset(interval.mul_f64(placed.beat.max(0.)));
      match clip {
        ClipSnap::Sample(node) => audiograph.add_node(node),
        ClipSnap::Pattern(node) => audiograph.add_pattern_node_with_id(node),
      }
      clips.push(id);
    }
  }
  drop(audiograph);

  if let Some((before, after)) = &step.mixer {
    state.playlist.mixer.apply(if pick(true) { before } else { after });
  }

  Applied { clips: if step.clips.is_empty() { None } else { Some(clips) } }
}

// run something that changes clips, and record it as a step
// - ids: the clips it may change or remove
// - made: given what it returned, the ids of clips it added
pub fn clip_edit<T>(
  state: &daw::InnerState,
  ids: &[u64],
  gesture: Option<u64>,
  edit: impl FnOnce() -> Result<T, String>,
  made: impl FnOnce(&T) -> Vec<u64>,
) -> Result<T, String> {
  let before = clips(state, ids);
  let result = edit()?;

  let mut all: Vec<u64> = ids.to_vec();
  all.extend(made(&result));
  let after = clips(state, &all);
  record(state, Step::new().in_gesture(gesture).with_clips(before, after));

  Ok(result)
}

// the parts of the project a Sequencer edit can change, as they are now:
// the sequencer, the mixer (exploding names tracks) and some clips
pub struct Picture {
  clips: Clips,
  sequencer: daw::Sequencer,
  mixer: daw::MixerSettings,
}

// - ids: the clips the edit may change, add or remove
pub fn picture(state: &daw::InnerState, ids: &[u64]) -> Picture {
  Picture {
    clips: clips(state, ids),
    sequencer: state.playlist.sequencer.lock().unwrap().clone(),
    mixer: state.playlist.mixer.changed(),
  }
}

// record what changed between two pictures as one step
pub fn record_between(state: &daw::InnerState, gesture: Option<u64>, before: Picture, after: Picture) {
  record(state, Step::new()
    .in_gesture(gesture)
    .with_clips(before.clips, after.clips)
    .with_sequencer(before.sequencer, after.sequencer)
    .with_mixer(before.mixer, after.mixer));
}

// run something that changes the sequencer, and record it as a step
// - ids: the clips it may remove with it (a deleted pattern's)
pub fn sequencer_edit<T>(
  state: &daw::InnerState,
  ids: &[u64],
  gesture: Option<u64>,
  edit: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
  let before = picture(state, ids);
  let result = edit()?;
  record_between(state, gesture, before, picture(state, ids));
  Ok(result)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::event_dispatch::{add_clip, change_clip_settings, move_clip, remove_clips};
  use crate::project;

  fn state() -> Arc<daw::InnerState> {
    Arc::new(daw::InnerState::default())
  }

  // a clip of the metronome's tick on a beat
  fn clip(state: &daw::InnerState, track: u32, beat: f32) -> u64 {
    add_clip(state, daw::METRONOME_TICK_PATH.to_string(), track, Some(beat)).unwrap().id
  }

  // the project as it would be saved: what undo and redo have to get back to
  fn saved(state: &daw::InnerState) -> project::Project {
    project::capture(state, None)
  }

  fn beat_of(state: &daw::InnerState, id: u64) -> Option<f32> {
    crate::event_dispatch::node_beat(state, id).ok()
  }

  fn steps(state: &daw::InnerState) -> usize {
    state.history.lock().unwrap().len()
  }

  // undo gets the project back to how it was, and redo to how it became
  fn there_and_back(state: &Arc<daw::InnerState>, before: &project::Project, after: &project::Project) {
    assert!(undo(state).unwrap().is_some());
    assert_eq!(&saved(state), before);
    assert!(redo(state).unwrap().is_some());
    assert_eq!(&saved(state), after);
  }

  #[test]
  fn test_adding_a_clip() {
    let state = state();
    let empty = saved(&state);
    let id = clip(&state, 1, 4.);
    let added = saved(&state);

    assert_eq!(undo(&state).unwrap(), Some(Applied { clips: Some(vec![]) }));
    assert_eq!(saved(&state), empty);
    assert_eq!(beat_of(&state, id), None);

    // redone, it's the same clip: its id, and it's the one to select
    assert_eq!(redo(&state).unwrap(), Some(Applied { clips: Some(vec![id]) }));
    assert_eq!(saved(&state), added);
    assert_eq!(beat_of(&state, id), Some(4.));
  }

  #[test]
  fn test_moving_a_clip() {
    let state = state();
    let id = clip(&state, 0, 2.);
    let before = saved(&state);
    move_clip(&state, id, 3, Some(9.)).unwrap();
    let after = saved(&state);
    assert_ne!(before, after);

    assert_eq!(undo(&state).unwrap(), Some(Applied { clips: Some(vec![id]) }));
    assert_eq!(beat_of(&state, id), Some(2.));
    assert_eq!(saved(&state), before);
    assert_eq!(redo(&state).unwrap(), Some(Applied { clips: Some(vec![id]) }));
    assert_eq!(saved(&state), after);
  }

  #[test]
  fn test_moving_a_selection_is_one_step() {
    let state = state();
    let (a, b) = (clip(&state, 0, 2.), clip(&state, 1, 5.));
    let before = saved(&state);
    let was = steps(&state);

    // the command the playlist uses for a dragged selection
    let moved = crate::event_dispatch::move_clips(&state, &[a, b], a, 10., 2);
    assert!(moved.is_ok());
    assert_eq!(steps(&state), was + 1);
    let after = saved(&state);

    there_and_back(&state, &before, &after);
    assert_eq!((beat_of(&state, a), beat_of(&state, b)), (Some(10.), Some(13.)));
  }

  #[test]
  fn test_deleting_a_selection_is_one_step() {
    let state = state();
    let ids: Vec<u64> = (0..5).map(|n| clip(&state, n % 3, n as f32 * 2.)).collect();
    change_clip_settings(&state, ids[2], daw::ClipSettings { pitch: 7., ..Default::default() }, None).unwrap();
    let before = saved(&state);
    let was = steps(&state);

    remove_clips(&state, &ids).unwrap();
    assert_eq!(steps(&state), was + 1);
    let after = saved(&state);
    assert!(after.clips.is_empty());

    // one undo brings all five back, as they were, with their ids
    let applied = undo(&state).unwrap().unwrap();
    assert_eq!(applied.clips, Some(ids.clone()));
    assert_eq!(saved(&state), before);
    for &id in &ids {
      assert!(beat_of(&state, id).is_some());
    }
    assert!(redo(&state).unwrap().is_some());
    assert_eq!(saved(&state), after);
  }

  #[test]
  fn test_deleting_is_all_or_nothing() {
    let state = state();
    let id = clip(&state, 0, 1.);
    let before = saved(&state);
    let was = steps(&state);

    assert!(remove_clips(&state, &[id, 9_999]).is_err());
    assert_eq!(saved(&state), before);
    // nothing to undo for it
    assert_eq!(steps(&state), was);
  }

  #[test]
  fn test_a_gesture_is_one_step() {
    let state = state();
    let id = clip(&state, 0, 0.);
    let before = saved(&state);
    let was = steps(&state);

    // a knob dragged from 0 to -19dB sends many changes
    for n in 0..20 {
      let settings = daw::ClipSettings { gain_db: -(n as f32), ..Default::default() };
      change_clip_settings(&state, id, settings, Some(7)).unwrap();
    }
    assert_eq!(steps(&state), was + 1);
    let after = saved(&state);

    there_and_back(&state, &before, &after);

    // another gesture is another step, and so is a change with none
    change_clip_settings(&state, id, daw::ClipSettings { pan: 0.5, ..Default::default() }, Some(8)).unwrap();
    change_clip_settings(&state, id, daw::ClipSettings { pan: 0.7, ..Default::default() }, None).unwrap();
    change_clip_settings(&state, id, daw::ClipSettings { pan: 0.9, ..Default::default() }, None).unwrap();
    assert_eq!(steps(&state), was + 4);
  }

  #[test]
  fn test_a_change_that_changes_nothing_is_no_step() {
    let state = state();
    let id = clip(&state, 0, 4.);
    let was = steps(&state);

    // dropped where it was
    move_clip(&state, id, 0, Some(4.)).unwrap();
    change_clip_settings(&state, id, daw::ClipSettings::default(), None).unwrap();
    assert_eq!(steps(&state), was);

    // a gesture that ends where it began leaves nothing to undo
    change_clip_settings(&state, id, daw::ClipSettings { pan: 0.5, ..Default::default() }, Some(3)).unwrap();
    assert_eq!(steps(&state), was + 1);
    change_clip_settings(&state, id, daw::ClipSettings::default(), Some(3)).unwrap();
    assert_eq!(steps(&state), was);
  }

  #[test]
  fn test_the_last_hundred_steps_are_kept() {
    let state = state();
    let id = clip(&state, 0, 0.);
    for n in 1..=150 {
      move_clip(&state, id, 0, Some(n as f32)).unwrap();
    }
    assert_eq!(steps(&state), LIMIT);

    let mut undone = 0;
    while undo(&state).unwrap().is_some() { undone += 1; }
    assert_eq!(undone, 100);
    // the oldest 51 steps (the add, and 50 moves) are gone: it's on beat 50
    assert_eq!(beat_of(&state, id), Some(50.));
  }

  #[test]
  fn test_a_new_change_ends_redo() {
    let state = state();
    let id = clip(&state, 0, 0.);
    for beat in [1., 2., 3.] {
      move_clip(&state, id, 0, Some(beat)).unwrap();
    }
    for _ in 0..3 { undo(&state).unwrap(); }
    assert!(state.history.lock().unwrap().can_redo());

    move_clip(&state, id, 2, Some(8.)).unwrap();
    assert!(!state.history.lock().unwrap().can_redo());
    assert_eq!(redo(&state).unwrap(), None);
    assert_eq!(beat_of(&state, id), Some(8.));
  }

  #[test]
  fn test_nothing_to_undo() {
    let state = state();
    assert_eq!(undo(&state).unwrap(), None);
    assert_eq!(redo(&state).unwrap(), None);
  }

  #[test]
  fn test_the_history_starts_again_with_a_project() {
    let state = state();
    let id = clip(&state, 0, 0.);
    move_clip(&state, id, 0, Some(3.)).unwrap();
    assert!(state.history.lock().unwrap().can_undo());

    // a new project, or one opened (both go through apply)
    project::apply(&state, &project::Project::default(), None);
    assert!(!state.history.lock().unwrap().can_undo());
    assert!(!state.history.lock().unwrap().can_redo());
    assert_eq!(undo(&state).unwrap(), None);
  }

  #[test]
  fn test_saving_keeps_the_history() {
    let dir = std::env::temp_dir().join(format!("dawesome-history-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let state = state();
    let id = clip(&state, 0, 0.);
    move_clip(&state, id, 0, Some(3.)).unwrap();
    let was = steps(&state);

    project::save_to(&state, &dir.join("song.dawesome")).unwrap();
    assert_eq!(steps(&state), was);
    assert!(undo(&state).unwrap().is_some());
    assert_eq!(beat_of(&state, id), Some(0.));
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_change_made_during_an_undo_ends_redo() {
    let state = state();
    let id = clip(&state, 0, 0.);
    move_clip(&state, id, 0, Some(3.)).unwrap();

    // as travel does, with a change recorded while the step is being applied
    let (step, changes) = {
      let mut history = state.history.lock().unwrap();
      (history.undo.pop().unwrap(), history.changes)
    };
    apply(&state, &step, Way::Undo);
    clip(&state, 1, 8.);
    assert_ne!(state.history.lock().unwrap().changes, changes);

    // and through the real thing: nothing to redo after a new change
    undo(&state).unwrap();
    clip(&state, 2, 4.);
    assert_eq!(redo(&state).unwrap(), None);
  }

  #[test]
  fn test_undo_and_redo_leave_files_alone() {
    let dir = std::env::temp_dir().join(format!("dawesome-history-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // a take, as a recording leaves it
    let take = dir.join("take.wav");
    std::fs::copy(daw::METRONOME_TICK_PATH, &take).unwrap();
    let bytes = std::fs::read(&take).unwrap();
    let listing = || {
      let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
      names.sort();
      names
    };
    let files = listing();

    let state = state();
    let id = add_clip(&state, take.to_string_lossy().into_owned(), 0, Some(1.)).unwrap().id;
    change_clip_settings(&state, id, daw::ClipSettings { pitch: 3., ..Default::default() }, None).unwrap();
    move_clip(&state, id, 2, Some(6.)).unwrap();
    remove_clips(&state, &[id]).unwrap();

    // all the way back, and all the way forward again, twice
    for _ in 0..2 {
      while undo(&state).unwrap().is_some() {}
      assert_eq!(listing(), files);
      assert_eq!(std::fs::read(&take).unwrap(), bytes);
      while redo(&state).unwrap().is_some() {}
      assert_eq!(listing(), files);
      assert_eq!(std::fs::read(&take).unwrap(), bytes);
    }
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_clip_comes_back_on_its_beat_after_a_tempo_change() {
    let state = state();
    let id = clip(&state, 0, 8.);
    remove_clips(&state, &[id]).unwrap();

    state.set_tempo(90.);
    undo(&state).unwrap();
    assert!((beat_of(&state, id).unwrap() - 8.).abs() < 1e-5);
  }

  #[test]
  fn test_mixer_changes() {
    use crate::mixer_commands as mixer;
    let state = state();
    // something already set, which every undo has to leave alone
    mixer::change_track_pan(&state, 0, -0.5, None);
    mixer::change_track_name(&state, 3, "Bass");

    let changes: Vec<(&str, Box<dyn Fn(&daw::InnerState)>)> = vec![
      ("volume", Box::new(|s| { mixer::change_track_volume(s, 2, -12., None); })),
      ("pan", Box::new(|s| { mixer::change_track_pan(s, 2, 0.25, None); })),
      ("mute", Box::new(|s| mixer::change_track_mute(s, 1, true))),
      ("solo", Box::new(|s| mixer::change_track_solo(s, 1, true))),
      ("master", Box::new(|s| { mixer::change_master_volume(s, -3., None); })),
      ("name", Box::new(|s| { mixer::change_track_name(s, 2, "Drums"); })),
      ("rename", Box::new(|s| { mixer::change_track_name(s, 3, "Sub"); })),
      ("unname", Box::new(|s| { mixer::change_track_name(s, 3, ""); })),
      // a track that has no clips, past the playlist's last
      ("far track", Box::new(|s| { mixer::change_track_volume(s, 9, -20., None); })),
      ("reset", Box::new(|s| { mixer::reset(s); })),
    ];

    for (what, change) in &changes {
      let had = steps(&state);
      let before = saved(&state);
      change(&state);
      let after = saved(&state);
      assert_ne!(before, after, "{} changed nothing", what);
      assert_eq!(steps(&state), had + 1, "{} is one step", what);

      // a mixer step isn't about clips: the selection is left alone
      assert_eq!(undo(&state).unwrap(), Some(Applied { clips: None }), "{}", what);
      assert_eq!(saved(&state), before, "{} undone", what);
      assert_eq!(redo(&state).unwrap(), Some(Applied { clips: None }), "{}", what);
      assert_eq!(saved(&state), after, "{} redone", what);
    }

    // and all the way back, to how it started
    for _ in &changes { undo(&state).unwrap(); }
    let mix = state.playlist.mixer.changed();
    assert_eq!(mix.tracks.len(), 2);
    assert_eq!(mix.tracks[0].settings.pan, -0.5);
    assert_eq!(mix.tracks[1].name.as_deref(), Some("Bass"));
  }

  #[test]
  fn test_undoing_a_solo_lets_the_other_tracks_sound_again() {
    use rodio::Source;
    let state = state();
    let mixer = &state.playlist.mixer;
    // what comes out of track 0's strip, given a steady signal
    let level = || {
      let source = rodio::source::Zero::<f32>::new(2, 44_100).map(|_: f32| 0.5).take(64);
      let source = rodio::buffer::SamplesBuffer::new(2, 44_100, source.collect::<Vec<f32>>());
      mixer.track_strip(0, source.convert_samples::<f32>()).last().unwrap().abs()
    };
    assert!(level() > 0.);

    crate::mixer_commands::change_track_solo(&state, 1, true);
    assert_eq!(level(), 0.);
    undo(&state).unwrap();
    assert!(level() > 0.);
    redo(&state).unwrap();
    assert_eq!(level(), 0.);
    // undone and redone again, the solo count hasn't drifted
    undo(&state).unwrap();
    undo(&state).unwrap();
    assert!(level() > 0.);
  }

  #[test]
  fn test_a_fader_drag_is_one_step() {
    use crate::mixer_commands as mixer;
    let state = state();
    mixer::change_track_volume(&state, 0, -6., None);
    let before = saved(&state);

    // twenty values on the way from -6 to 0, as one drag sends them
    for i in 1..=20 {
      mixer::change_track_volume(&state, 0, -6. + 0.3 * i as f32, Some(7));
    }
    assert_eq!(steps(&state), 2);
    let after = saved(&state);
    assert_eq!(state.playlist.mixer.track(0).volume_db(), 0.);
    there_and_back(&state, &before, &after);

    // the next drag is its own step, as is one on another control in between
    mixer::change_track_volume(&state, 0, -3., Some(8));
    mixer::change_track_pan(&state, 0, 0.5, Some(8));
    mixer::change_master_volume(&state, -1., Some(9));
    assert_eq!(steps(&state), 4);
    undo(&state).unwrap();
    undo(&state).unwrap();
    assert_eq!(saved(&state), after);

    // a drag that ends where it began leaves nothing to undo
    mixer::change_track_pan(&state, 1, 0.5, Some(10));
    mixer::change_track_pan(&state, 1, 0., Some(10));
    assert_eq!(steps(&state), 2);
  }

  #[test]
  fn test_tempo_changes() {
    use crate::event_dispatch::change_tempo;
    let state = state();
    let id = clip(&state, 0, 8.);
    let ms = |id: u64| {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      audiograph.get_node_by_id(id).unwrap().start_offset.as_secs_f64() * 1000.
    };
    let before = saved(&state);
    assert_eq!(change_tempo(&state, 90., None), Ok(90.));
    let after = saved(&state);
    assert_eq!(steps(&state), 2);

    assert_eq!(undo(&state).unwrap(), Some(Applied { clips: None }));
    assert_eq!(state.tempo(), 120.);
    assert_eq!(saved(&state), before);
    // the clip is on beat 8 at either tempo: 4s in, or 5.33s
    assert!((ms(id) - 4_000.).abs() < 0.001);
    redo(&state).unwrap();
    assert_eq!(saved(&state), after);
    assert!((ms(id) - 8. * 60_000. / 90.).abs() < 0.01);

    // a refused tempo is no step
    assert!(change_tempo(&state, 0., None).is_err());
    assert!(change_tempo(&state, 90., None).is_ok());
    assert_eq!(steps(&state), 2);

    // a clip moved at one tempo goes back to its beat at another
    move_clip(&state, id, 0, Some(2.)).unwrap();
    change_tempo(&state, 150., None).unwrap();
    undo(&state).unwrap();
    undo(&state).unwrap();
    assert_eq!(state.tempo(), 90.);
    assert!((beat_of(&state, id).unwrap() - 8.).abs() < 1e-5);
    undo(&state).unwrap();
    assert_eq!(saved(&state), before);
  }

  #[test]
  fn test_a_visit_to_the_tempo_field_is_one_step() {
    use crate::event_dispatch::change_tempo;
    let state = state();
    let before = saved(&state);
    // 100 typed, then changed to 140, without leaving the field
    change_tempo(&state, 100., Some(3)).unwrap();
    change_tempo(&state, 140., Some(3)).unwrap();
    assert_eq!(steps(&state), 1);
    undo(&state).unwrap();
    assert_eq!(saved(&state), before);
    redo(&state).unwrap();
    assert_eq!(state.tempo(), 140.);

    // the next visit is another step
    change_tempo(&state, 80., Some(4)).unwrap();
    assert_eq!(steps(&state), 2);
    undo(&state).unwrap();
    assert_eq!(state.tempo(), 140.);
  }

  #[test]
  fn test_clips_that_come_back_follow_moved_files() {
    let state = state();
    let id = clip(&state, 0, 0.);
    let path = state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().sample_path().to_string();
    remove_clips(&state, &[id]).unwrap();

    // its file was moved meanwhile (a take renamed in the recordings browser)
    state.history.lock().unwrap().move_paths(&[(path, "/somewhere/else.wav".to_string())]);
    undo(&state).unwrap();
    let audiograph = state.playlist.audiograph.lock().unwrap();
    assert_eq!(audiograph.get_node_by_id(id).unwrap().sample_path(), "/somewhere/else.wav");
  }
}
