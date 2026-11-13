// undo and redo
//
// every change to the project (clips, the tracks, the mixer, the sequencer,
// the tempo, the effects racks) is kept as a step: what it changed, as it was before and as it was after.
// undoing a step puts back the "before", redoing it the "after". steps hold
// whole snapshots of what they touched, rather than how to reverse each
// command, so a step can't drift from what it describes
//
// one gesture is one step: a fader dragged from one place to another sends
// many changes, each tagged with the gesture's id, and they're folded into
// the same step (its first "before", its last "after")
//
// an effects step keeps snapshots of the racks, not of the plugins' settings
// (knob turns in a plugin aren't undoable): effects that are there before and
// after keep their live settings, and one that undo or redo takes away has
// its settings kept in the snapshot being left, so it comes back with them
// (see effects.rs)
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
  effects: Option<(crate::effects::RackSnap, crate::effects::RackSnap)>,
  // the playlist's fewest tracks (see AudioGraph::min_tracks)
  tracks: Option<(u32, u32)>,
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

  pub fn with_effects(mut self, before: crate::effects::RackSnap, after: crate::effects::RackSnap) -> Self {
    self.effects = Some((before, after));
    self
  }

  pub fn with_tracks(mut self, before: u32, after: u32) -> Self {
    self.tracks = Some((before, after));
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
    if matches!(&self.effects, Some((before, after)) if before.same(after)) { self.effects = None; }
    if matches!(self.tracks, Some((before, after)) if before == after) { self.tracks = None; }
  }

  fn is_empty(&self) -> bool {
    self.clips.is_empty() && self.mixer.is_none() && self.sequencer.is_none() && self.tempo.is_none()
      && self.effects.is_none() && self.tracks.is_none()
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
    if let Some((before, after)) = later.effects {
      self.effects = Some((self.effects.take().map_or(before, |mine| mine.0), after));
    }
    if let Some((before, after)) = later.tracks {
      self.tracks = Some((self.tracks.take().map_or(before, |mine| mine.0), after));
    }
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

  #[cfg(test)]
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

  // whether anything that undo or redo could put back plays a file at or
  // under `path`: a clip, or a sequencer row
  pub fn uses(&self, path: &std::path::Path) -> bool {
    let plays = |placed: &Option<Placed>| matches!(
      placed,
      Some(Placed { clip: ClipSnap::Sample(node), .. }) if std::path::Path::new(node.sample_path()).starts_with(path)
    );
    self.undo.iter().chain(self.redo.iter()).any(|step| {
      step.clips.values().any(|(before, after)| plays(before) || plays(after))
        || step.sequencer.as_ref().map_or(false, |(before, after)| before.plays_under(path) || after.plays_under(path))
    })
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
  let mut step = match step {
    Some(step) => step,
    None => return Ok(None),
  };

  let applied = apply(state, &mut step, way);

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
// sequencer, whose patterns the pattern clips play. the effects go last, with
// no other lock held (their plugins are loaded on the main thread)
fn apply(state: &Arc<daw::InnerState>, step: &mut Step, way: Way) -> Applied {
  let pick = |before: bool| (way == Way::Undo) == before;

  if let Some((before, after)) = step.tempo {
    let tempo = if pick(true) { before } else { after };
    if let Err(e) = daw::set_playlist_tempo(state, tempo) {
      println!("could not put the tempo back to {}: {}", tempo, e);
    }
  }

  let mut audiograph = state.playlist.audiograph.lock().unwrap();

  if let Some((before, after)) = step.tracks {
    audiograph.min_tracks = if pick(true) { before } else { after };
  }

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

  if let Some((before, after)) = &mut step.effects {
    // the snapshot being left keeps the settings of effects that go
    let (target, left) = if pick(true) { (&*before, after) } else { (&*after, before) };
    state.playlist.effects.restore(target, left);
  }
  // the synths' plugins follow the list and the clips put back
  crate::commands::synth::follow(state);

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
  // a clip of a note pattern on a track that hasn't its synths yet
  crate::commands::synth::follow(state);

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

// record what changed between two pictures as one step (and bring the
// synths' plugins in line with what it changed)
pub fn record_between(state: &daw::InnerState, gesture: Option<u64>, before: Picture, after: Picture) {
  record(state, Step::new()
    .in_gesture(gesture)
    .with_clips(before.clips, after.clips)
    .with_sequencer(before.sequencer, after.sequencer)
    .with_mixer(before.mixer, after.mixer));
  crate::commands::synth::follow(state);
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
mod tests;
