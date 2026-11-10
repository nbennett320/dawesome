// Edit › Undo and Redo (see history.rs)
//
// undoing can change anything in the project, so both commands return all of
// it as it is afterwards, and the frontend shows that
use serde::Serialize;

use crate::daw;
use crate::history;
use crate::mixer_commands::mixer_settings;
use crate::project::{LoadedClip, LoadedPatternClip};

// the project as it is now, in the shapes the frontend already takes from a
// loaded project
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
  pub tempo: f32,
  pub clips: Vec<LoadedClip>,
  pub pattern_clips: Vec<LoadedPatternClip>,
  pub mixer: daw::MixerSettings,
  pub sequencer: daw::SequencerState,
}

pub fn project_view(state: &daw::InnerState) -> ProjectView {
  let (tempo, clips, pattern_clips) = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let interval = audiograph.beat_interval();
    let clips = audiograph.nodes.iter().map(|node| LoadedClip::of(node, interval)).collect();
    let pattern_clips = audiograph.pattern_nodes.iter().map(|node| LoadedPatternClip::of(node, interval)).collect();

    (audiograph.tempo(), clips, pattern_clips)
  };

  ProjectView {
    tempo,
    clips,
    pattern_clips,
    mixer: mixer_settings(state),
    sequencer: state.playlist.sequencer.lock().unwrap().state(),
  }
}

// what undoing or redoing did
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Travelled {
  // false when there was nothing to undo (or redo), or it can't be done just
  // now (while recording): nothing changed
  pub changed: bool,
  // the clips it changed or put back, which the playlist selects. None when
  // it wasn't about clips, and the selection stays
  pub selected: Option<Vec<u64>>,
  pub can_undo: bool,
  pub can_redo: bool,
  pub project: ProjectView,
}

fn travelled(state: &daw::InnerState, applied: Option<history::Applied>) -> Travelled {
  let (can_undo, can_redo) = {
    let history = state.history.lock().unwrap();
    (history.can_undo(), history.can_redo())
  };

  Travelled {
    changed: applied.is_some(),
    selected: applied.and_then(|applied| applied.clips),
    can_undo,
    can_redo,
    project: project_view(state),
  }
}

#[tauri::command]
pub fn undo(state: daw::TState<'_>) -> Result<Travelled, String> {
  let applied = history::undo(state.inner())?;
  Ok(travelled(&state, applied))
}

#[tauri::command]
pub fn redo(state: daw::TState<'_>) -> Result<Travelled, String> {
  let applied = history::redo(state.inner())?;
  Ok(travelled(&state, applied))
}
