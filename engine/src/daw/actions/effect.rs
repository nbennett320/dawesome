// changing each mixer channel's rack of effects, and their parameters (see
// effects.rs; commands/effect.rs is the frontend's side, and
// context/vst-project/04-api.md the contract with it)
//
// every change returns all of the racks, which the frontend replaces its
// copy with, and is recorded so it can be undone. knob turns inside a
// plugin's own window aren't undoable. instruments aren't effects: they're
// the sequencer's synth rows (see actions/synth.rs)
use serde::Serialize;

use crate::daw::{self, Channel};
use crate::effects::Racks;
use crate::history;
use crate::native::{Native, ParamValue};

fn channel(track: Option<u32>) -> Channel {
  track.map_or(Channel::Master, Channel::Track)
}

// change the racks, and record the change so it can be undone
// - keep: an effect about to go, whose settings undoing it brings back
fn edit<T>(
  state: &daw::InnerState,
  keep: Option<u64>,
  change: impl FnOnce(&crate::effects::Effects) -> Result<T, String>,
) -> Result<T, String> {
  let effects = &state.playlist.effects;
  let mut before = effects.snapshot();
  if let Some(id) = keep {
    if let Some(settings) = effects.state_of(id) {
      before.keep_state(id, settings);
    }
  }
  let result = change(effects)?;
  history::record(state, history::Step::new().with_effects(before, effects.snapshot()));
  Ok(result)
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddedEffect {
  pub racks: Racks,
  pub slot_id: u64,
}

pub fn add(state: &daw::InnerState, track: Option<u32>, plugin_id: &str, index: Option<usize>) -> Result<AddedEffect, String> {
  let slot_id = edit(state, None, |effects| effects.add(channel(track), plugin_id, index))?;
  Ok(AddedEffect { racks: state.playlist.effects.racks(), slot_id })
}

pub fn remove(state: &daw::InnerState, slot_id: u64) -> Result<Racks, String> {
  edit(state, Some(slot_id), |effects| effects.remove(slot_id))?;
  Ok(state.playlist.effects.racks())
}

pub fn move_to(state: &daw::InnerState, slot_id: u64, track: Option<u32>, index: usize) -> Result<Racks, String> {
  edit(state, None, |effects| effects.move_to(slot_id, channel(track), index))?;
  Ok(state.playlist.effects.racks())
}

pub fn set_bypassed(state: &daw::InnerState, slot_id: u64, bypassed: bool) -> Result<Racks, String> {
  edit(state, None, |effects| effects.set_bypassed(slot_id, bypassed))?;
  Ok(state.playlist.effects.racks())
}

// change a built-in effect's parameters, and record it so it can be undone
// - gesture: the knob drag it's part of, if any. one drag is one step
fn param_edit(
  state: &daw::InnerState,
  slot_id: u64,
  gesture: Option<u64>,
  change: impl FnOnce(&Native) -> Result<ParamValue, String>,
) -> Result<ParamValue, String> {
  let (before, value, after) = state.playlist.effects.with_native(slot_id, |native| {
    let before = native.values();
    let value = change(native)?;
    Ok::<_, String>((before, value, native.values()))
  })??;
  history::record(state, history::Step::new().in_gesture(gesture).with_params(slot_id, before, after));
  Ok(value)
}

pub fn set_param(state: &daw::InnerState, slot_id: u64, id: u32, normalized: f64, gesture: Option<u64>) -> Result<ParamValue, String> {
  param_edit(state, slot_id, gesture, |native| native.set(id, normalized))
}

pub fn set_param_text(state: &daw::InnerState, slot_id: u64, id: u32, text: &str) -> Result<ParamValue, String> {
  param_edit(state, slot_id, None, |native| native.set_text(id, text))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::effects::testing::*;
  use std::sync::atomic::Ordering;
  use std::sync::Arc;

  fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
  }

  fn undo(state: &Arc<daw::InnerState>) {
    assert!(history::undo(state).unwrap().is_some(), "nothing to undo");
  }

  fn redo(state: &Arc<daw::InnerState>) {
    assert!(history::redo(state).unwrap().is_some(), "nothing to redo");
  }

  fn steps(state: &daw::InnerState) -> usize {
    state.history.lock().unwrap().len()
  }

  #[test]
  fn test_undoing_an_add_and_redoing_it_keeps_its_settings() {
    let mut rig = rig();
    let state = rig.state.clone();
    let added = add(&state, Some(0), &gain().id, None).unwrap();
    assert_eq!(added.racks, state.playlist.effects.racks());
    let a = added.slot_id;
    rig.turn(a, 0.25);

    undo(&state);
    assert!(rig.layout().is_empty());
    assert_eq!(rig.loader.alive.load(Ordering::SeqCst), 0);
    assert!(close(rig.level(Some(0)), 0.2));

    redo(&state);
    assert_eq!(rig.layout(), vec![(Some(0), vec![a])]);
    assert!(close(rig.gain_of(a), 0.25));
    assert!(close(rig.level(Some(0)), 0.05));
  }

  #[test]
  fn test_undoing_a_remove_brings_it_back_as_it_was() {
    let mut rig = rig();
    let state = rig.state.clone();
    let a = add(&state, None, &gain().id, None).unwrap().slot_id;
    let b = add(&state, None, &gain().id, None).unwrap().slot_id;
    rig.turn(a, 0.3);
    set_bypassed(&state, b, true).unwrap();
    assert!(remove(&state, a).unwrap().iter().all(|rack| rack.slots.iter().all(|s| s.id != a)));
    assert!(close(rig.level(None), 0.2));

    undo(&state);
    assert_eq!(rig.layout(), vec![(None, vec![a, b])]);
    assert!(close(rig.gain_of(a), 0.3));
    assert!(close(rig.level(None), 0.06));

    redo(&state);
    assert_eq!(rig.layout(), vec![(None, vec![b])]);
    undo(&state);
    assert!(close(rig.gain_of(a), 0.3));
  }

  #[test]
  fn test_undoing_a_move_keeps_the_plugin() {
    let mut rig = rig();
    let state = rig.state.clone();
    let a = add(&state, Some(0), &gain().id, None).unwrap().slot_id;
    let b = add(&state, Some(0), &gain().id, None).unwrap().slot_id;
    rig.turn(b, 0.25);
    move_to(&state, b, None, 0).unwrap();
    // (where it already is: not a step)
    move_to(&state, a, Some(0), 3).unwrap();
    assert_eq!(steps(&state), 3);
    assert_eq!(rig.layout(), vec![(Some(0), vec![a]), (None, vec![b])]);

    undo(&state);
    assert_eq!(rig.layout(), vec![(Some(0), vec![a, b])]);
    assert!(close(rig.level(Some(0)), 0.2 * 0.5 * 0.25));
    assert!(close(rig.level(None), 0.2));
    redo(&state);
    assert_eq!(rig.layout(), vec![(Some(0), vec![a]), (None, vec![b])]);
    assert!(close(rig.level(None), 0.05));
    // never loaded again, so never reset
    assert_eq!(rig.loader.loads.load(Ordering::SeqCst), 2);
    assert!(close(rig.gain_of(b), 0.25));
  }

  #[test]
  fn test_undoing_a_bypass() {
    let mut rig = rig();
    let state = rig.state.clone();
    let a = add(&state, Some(2), &gain().id, None).unwrap().slot_id;
    let racks = set_bypassed(&state, a, true).unwrap();
    assert!(racks[0].slots[0].bypassed);
    // the same again changes nothing, so it isn't a step
    set_bypassed(&state, a, true).unwrap();
    assert_eq!(steps(&state), 2);
    assert!(close(rig.level(Some(2)), 0.2));

    undo(&state);
    assert!(!state.playlist.effects.racks()[0].slots[0].bypassed);
    assert!(close(rig.level(Some(2)), 0.1));
    redo(&state);
    assert!(state.playlist.effects.racks()[0].slots[0].bypassed);
    assert!(close(rig.level(Some(2)), 0.2));
  }

  #[test]
  fn test_effects_that_stay_keep_their_live_settings() {
    let rig = rig();
    let state = rig.state.clone();
    let a = add(&state, Some(0), &gain().id, None).unwrap().slot_id;
    add(&state, Some(0), &gain().id, None).unwrap();
    set_bypassed(&state, a, true).unwrap();
    rig.turn(a, 0.125);

    // undoing the bypass, then the second add: the first keeps its knob
    undo(&state);
    assert!(close(rig.gain_of(a), 0.125));
    undo(&state);
    assert!(close(rig.gain_of(a), 0.125));
    redo(&state);
    redo(&state);
    assert!(close(rig.gain_of(a), 0.125));
    assert_eq!(rig.loader.loads.load(Ordering::SeqCst), 3);
  }

  #[test]
  fn test_an_effect_that_cant_be_loaded_again_comes_back_missing() {
    let mut rig = rig();
    let state = rig.state.clone();
    let a = add(&state, Some(0), &gain().id, None).unwrap().slot_id;
    rig.turn(a, 0.75);
    remove(&state, a).unwrap();
    rig.loader.broken.store(true, Ordering::SeqCst);

    undo(&state);
    let slot = state.playlist.effects.racks()[0].slots[0].clone();
    assert_eq!((slot.id, slot.missing), (a, true));
    // sound passes through, and its settings are kept
    assert!(close(rig.level(Some(0)), 0.2));
    assert_eq!(state.playlist.effects.state_of(a), Some(0.75f32.to_le_bytes().to_vec()));

    // once it's back, removing and undoing it loads it with them
    rig.loader.broken.store(false, Ordering::SeqCst);
    redo(&state);
    undo(&state);
    assert!(!state.playlist.effects.racks()[0].slots[0].missing);
    assert!(close(rig.gain_of(a), 0.75));
  }

  #[test]
  fn test_a_failed_add_is_no_step() {
    let rig = rig();
    let state = rig.state.clone();
    assert!(add(&state, Some(0), &absent().id, None).is_err());
    assert!(remove(&state, 99).is_err());
    assert!(move_to(&state, 99, None, 0).is_err());
    assert_eq!(steps(&state), 0);
  }

  #[test]
  fn test_an_instrument_isnt_an_effect() {
    let rig = rig();
    let state = rig.state.clone();
    let err = add(&state, Some(0), &synth().id, None).unwrap_err();
    assert!(err.contains("is an instrument"), "{}", err);
    assert!(rig.layout().is_empty());
    assert_eq!(steps(&state), 0);
  }

  #[test]
  fn test_undo_and_redo_show_the_effects() {

    let rig = rig();
    let state = rig.state.clone();
    add(&state, None, &gain().id, None).unwrap();
    undo(&state);
    assert!(crate::actions::history::project_view(&state).effects.is_empty());
    redo(&state);
    assert_eq!(crate::actions::history::project_view(&state).effects, state.playlist.effects.racks());
    assert_eq!(crate::actions::history::project_view(&state).effects.len(), 1);
  }

  // ---- built-in effects' knobs

  const EQ: &str = "dawesome:eq";
  // the EQ's output gain, and band 1's frequency
  const OUTPUT: u32 = 40;
  const FREQ: u32 = 2;

  fn value(state: &daw::InnerState, slot: u64, id: u32) -> f32 {
    state.playlist.effects.with_native(slot, |n| n.values()[id as usize]).unwrap()
  }

  #[test]
  fn test_a_knob_drag_is_one_step() {
    let rig = rig();
    let state = rig.state.clone();
    let eq = add(&state, Some(0), EQ, None).unwrap().slot_id;
    let before = steps(&state);
    for n in [0.4, 0.3, 0.2, 0.1] {
      set_param(&state, eq, OUTPUT, n, Some(7)).unwrap();
    }
    assert_eq!(steps(&state), before + 1);
    assert_eq!(value(&state, eq, OUTPUT), -19.2);

    undo(&state);
    assert_eq!(value(&state, eq, OUTPUT), 0.);
    redo(&state);
    assert_eq!(value(&state, eq, OUTPUT), -19.2);
  }

  #[test]
  fn test_turns_without_a_drag_are_a_step_each() {
    let rig = rig();
    let state = rig.state.clone();
    let eq = add(&state, Some(0), EQ, None).unwrap().slot_id;
    set_param(&state, eq, OUTPUT, 0.25, None).unwrap();
    set_param(&state, eq, OUTPUT, 0.75, None).unwrap();
    set_param_text(&state, eq, FREQ, "2.5k").unwrap();
    assert_eq!(value(&state, eq, FREQ), 2500.);

    undo(&state);
    assert_eq!((value(&state, eq, OUTPUT), value(&state, eq, FREQ)), (12., 50.));
    undo(&state);
    assert_eq!(value(&state, eq, OUTPUT), -12.);
    undo(&state);
    assert_eq!(value(&state, eq, OUTPUT), 0.);
    // and then the add
    undo(&state);
    assert!(state.playlist.effects.racks().is_empty());
  }

  #[test]
  fn test_a_drag_that_ends_where_it_began_changes_nothing() {
    let rig = rig();
    let state = rig.state.clone();
    let eq = add(&state, Some(0), EQ, None).unwrap().slot_id;
    let before = steps(&state);
    set_param(&state, eq, OUTPUT, 0.9, Some(3)).unwrap();
    set_param(&state, eq, OUTPUT, 0.5, Some(3)).unwrap();
    assert_eq!(steps(&state), before);
    // a set to where it is already is no step either
    set_param(&state, eq, OUTPUT, 0.5, None).unwrap();
    assert_eq!(steps(&state), before);
    // nor one that fails
    assert!(set_param_text(&state, eq, FREQ, "loud").is_err());
    assert_eq!(steps(&state), before);
  }

  #[test]
  fn test_a_turn_is_undone_after_its_effect_moved_and_came_back() {
    let rig = rig();
    let state = rig.state.clone();
    let eq = add(&state, Some(0), EQ, None).unwrap().slot_id;
    set_param(&state, eq, OUTPUT, 0.25, None).unwrap();
    move_to(&state, eq, Some(2), 0).unwrap();
    remove(&state, eq).unwrap();

    // the remove: it's back with its knob as it was
    undo(&state);
    assert_eq!(value(&state, eq, OUTPUT), -12.);
    // the move, then the turn
    undo(&state);
    undo(&state);
    assert_eq!(value(&state, eq, OUTPUT), 0.);
    assert_eq!(state.playlist.effects.racks()[0].track, Some(0));
  }

  #[test]
  fn test_a_turn_is_heard_undone() {
    let mut rig = rig();
    let state = rig.state.clone();
    let eq = add(&state, Some(0), EQ, None).unwrap().slot_id;
    set_param(&state, eq, OUTPUT, 0., Some(1)).unwrap();
    for _ in 0..100 { rig.level(Some(0)); }
    assert!(close(rig.level(Some(0)), 0.2 * 10f32.powf(-24. / 20.)));
    undo(&state);
    for _ in 0..100 { rig.level(Some(0)); }
    assert!(close(rig.level(Some(0)), 0.2));
  }
}
