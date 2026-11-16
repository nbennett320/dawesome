// the effects' commands: the plugins installed, and each mixer channel's
// rack of effects (see actions/effect.rs)
use crate::actions::effect::{add, move_to, remove, set_bypassed, set_param, set_param_text, AddedEffect};
use crate::daw;
use crate::effects::Racks;
use crate::native::{EffectParams, EffectSpectrum, ParamValue};
use crate::plugin_catalog::PluginCatalog;

// the plugins installed, after Dawesome's own effects (native.rs)
#[tauri::command]
pub fn get_plugins(state: daw::TState<'_>) -> PluginCatalog {
  let mut catalog = state.playlist.effects.plugins.status();
  catalog.plugins.splice(0..0, crate::native::catalog());
  catalog
}

// look for plugins in the background (every bundle again with
// `everything`); get_plugins says how it's going
#[tauri::command]
pub fn scan_plugins(state: daw::TState<'_>, everything: bool) {
  state.playlist.effects.plugins.scan(everything);
}

#[tauri::command]
pub fn get_effects(state: daw::TState<'_>) -> Racks {
  state.playlist.effects.racks()
}

#[tauri::command]
pub fn add_effect(state: daw::TState<'_>, track: Option<u32>, plugin_id: String, index: Option<usize>) -> Result<AddedEffect, String> {
  add(&state, track, &plugin_id, index)
}

#[tauri::command]
pub fn remove_effect(state: daw::TState<'_>, slot_id: u64) -> Result<Racks, String> {
  remove(&state, slot_id)
}

#[tauri::command]
pub fn move_effect(state: daw::TState<'_>, slot_id: u64, track: Option<u32>, index: usize) -> Result<Racks, String> {
  move_to(&state, slot_id, track, index)
}

#[tauri::command]
pub fn set_effect_bypassed(state: daw::TState<'_>, slot_id: u64, bypassed: bool) -> Result<Racks, String> {
  set_bypassed(&state, slot_id, bypassed)
}

// a built-in effect's parameters and their values (an error for a plugin:
// its parameters are in its own interface)
#[tauri::command]
pub fn get_effect_params(state: daw::TState<'_>, slot_id: u64) -> Result<EffectParams, String> {
  state.playlist.effects.with_native(slot_id, |native| native.params())
}

// a built-in effect's output, as a spectrum for its analyzer. null when it
// should be asked again (see Native::spectrum). off the main thread (async):
// it's polled while the effect's tab shows, and the FFT is work
#[tauri::command(async)]
pub fn get_effect_spectrum(state: daw::TState<'_>, slot_id: u64) -> Result<Option<EffectSpectrum>, String> {
  state.playlist.effects.with_native(slot_id, |native| native.spectrum())
}

// set one of a built-in effect's parameters, by its normalized value (0 to
// 1): what it became. undoable, a knob drag (`gesture`) as one step
#[tauri::command]
pub fn set_effect_param(
  state: daw::TState<'_>,
  slot_id: u64,
  id: u32,
  normalized: f64,
  gesture: Option<u64>,
) -> Result<ParamValue, String> {
  set_param(&state, slot_id, id, normalized, gesture)
}

// set one from what was typed ("2.5k", "-3 dB", "Notch"), as a step of its own
#[tauri::command]
pub fn set_effect_param_text(state: daw::TState<'_>, slot_id: u64, id: u32, text: String) -> Result<ParamValue, String> {
  set_param_text(&state, slot_id, id, &text)
}
