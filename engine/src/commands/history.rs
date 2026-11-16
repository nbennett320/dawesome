// Edit › Undo and Redo (see actions/history.rs)
use crate::actions::history::{travelled, Travelled};
use crate::daw;
use crate::history;

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
