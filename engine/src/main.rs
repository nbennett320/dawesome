mod app;
mod commands;
mod daw;
mod util;
mod project;
mod project_files;
mod recording;
mod recent;
mod theme;
mod preferences;
mod history;
mod clipboard;
mod bounce;
mod join;
mod mixdown;
mod mixdown_file;
mod plugin;
mod plugin_catalog;
mod main_thread;
mod effects;
mod effect_windows;
#[cfg(test)]
mod test_support;

use std::env;
use std::sync::Arc;
use tauri::{self, Manager};

#[macro_use(lazy_static)]
extern crate lazy_static;

fn init_tauri() {
  use commands::transport::*;
  use commands::sample::*;
  use commands::playlist::*;
  use project::{save_project, load_project, new_project, project_file_for, project_has_unsaved_changes};
  use app::window::quit;
  use recent::{add_recent_project, remove_recent_project};
  use commands::mixer::*;
  use commands::audio::*;
  use commands::sequencer::*;
  use commands::clip::*;
  use clipboard::*;
  use bounce::*;
  use join::*;
  use commands::mixdown::*;
  use commands::appearance::{themes, preferences, set_preferences, set_window_background};
  use commands::history::{undo, redo};
  use commands::effect::*;
  use effect_windows::*;
  use commands::synth::*;

  // read before the menu is built, which lists them
  let recent_projects = recent::RecentProjects::load();

  // open the audio output now, so the first play or preview doesn't wait
  // for it. if it can't be opened, playing tries again
  std::thread::spawn(|| {
    if let Err(e) = daw::engine() {
      println!("could not open the audio output: {}", e);
    }
  });

  let state = Arc::new(daw::InnerState::default());

  // the plugins found last time, then a look for new or changed ones in the
  // background
  let plugins = state.playlist.effects.plugins.clone();
  if let Some(file) = plugin_catalog::default_file() {
    plugins.open(&file);
  }
  plugins.scan(false);

  let racks = state.playlist.effects.clone();
  let following = state.clone();
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .setup(move |app| {
      // (setup runs on the main thread, which plugins are called on)
      main_thread::init(app.handle().clone());
      effects::keep_idling(racks.clone());
      effects::follow_output(&racks);
      effect_windows::init(&racks);
      commands::synth::keep_following(following.clone());
      // the empty project the app starts with is unchanged (see
      // project::has_unsaved_changes)
      project::mark_unchanged(&following);
      app::window::setup(app)
    })
    // closing the main window quits, so it asks about unsaved changes first
    .on_window_event(|window, event| {
      if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" && !app::window::may_quit(window.app_handle()) {
          api.prevent_close();
        }
      }
    })
    .manage(state)
    .manage(recent_projects)
    .manage(preferences::AppPreferences::load())
    .manage(Arc::new(commands::mixdown::Mixdowns::default()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      toggle_playlist,
      play_playlist,
      pause_playlist,
      stop_playlist,
      seek_playlist,
      get_playlist_transport,
      set_playlist_tempo,
      toggle_metronome_enabled,
      get_metronome_enabled,
      toggle_count_in,
      set_playlist_time_signature,
      get_sidebar_samples,
      enumerate_directory,
      preview_sample,
      get_audio_drivers,
      get_audio_settings,
      set_audio_settings,
      add_audiograph_node,
      move_audiograph_node,
      move_audiograph_nodes,
      remove_audiograph_nodes,
      themes,
      preferences,
      set_preferences,
      set_window_background,
      undo,
      redo,
      get_node_data,
      get_playlist_data,
      toggle_loop_enabled,
      get_loop_enabled,
      toggle_snap_enabled,
      get_snap_enabled,
      get_snap_setting,
      set_snap_subdivision,
      get_sample_preview,
      get_clip,
      set_clip_settings,
      preview_node,
      save_project,
      load_project,
      new_project,
      project_file_for,
      add_recent_project,
      remove_recent_project,
      get_mixer,
      set_track_volume,
      set_track_pan,
      set_track_mute,
      set_track_solo,
      set_track_name,
      set_track_color,
      set_master_volume,
      reset_mixer,
      get_mixer_levels,
      init_playlist_workspace,
      start_recording,
      stop_recording,
      collect_recording,
      list_recordings,
      rename_recording,
      delete_recording,
      move_recording,
      move_recording_folder,
      create_recording_folder,
      rename_recording_folder,
      delete_recording_folder,
      get_record_setup,
      set_track_armed,
      listen_to_input,
      get_input_levels,
      get_sequencer,
      add_sequencer_row,
      remove_sequencer_row,
      set_sequencer_row_volume,
      set_sequencer_row_muted,
      set_sequencer_row_pan,
      rename_sequencer_row,
      move_sequencer_row,
      replace_sequencer_row_sample,
      preview_sequencer_row,
      add_pattern,
      duplicate_pattern,
      remove_pattern,
      rename_pattern,
      add_pattern_folder,
      rename_pattern_folder,
      move_pattern_folder,
      move_pattern,
      remove_pattern_folder,
      set_pattern_bars,
      set_pattern_step,
      play_pattern,
      toggle_pattern,
      seek_pattern,
      add_pattern_clip,
      explode_pattern_clips,
      paint_pattern_clips,
      set_clip_windows,
      split_clips,
      stretch_clips,
      copy_clips,
      cut_clips,
      paste_clips,
      repeat_clips,
      deep_copy_clips,
      join_clips,
      mixdown_defaults,
      start_mixdown,
      mixdown_status,
      cancel_mixdown,
      reveal_file,
      open_effect_window,
      close_effect_window,
      dock_effect,
      move_docked_effect,
      resize_docked_effect,
      undock_effect,
      get_plugins,
      scan_plugins,
      get_effects,
      add_effect,
      remove_effect,
      move_effect,
      set_effect_bypassed,
      set_pattern_notes,
      add_synth,
      remove_synth,
      set_synth_volume,
      set_synth_muted,
      add_synth_on_track,
      play_note,
      stop_note,
      project_has_unsaved_changes,
      quit
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    // the system quitting the app (the Dock's Quit, logging out) asks about
    // unsaved changes too
    .run(|app, event| {
      if let tauri::RunEvent::ExitRequested { api, .. } = event {
        if !app::window::may_quit(app) {
          api.prevent_exit();
        }
      }
    });
}

/**
 * args:
 *   --scan-plugin <path> => scan one plugin bundle and print what's in it
 */
fn main() {
  let args: Vec<String> = env::args().skip(1).collect();

  // `--scan-plugin <path>`: this is a child process scanning one plugin
  // bundle (see plugin::scan), so nothing else starts
  if let Some(code) = plugin::scan::scanner_main(&args) {
    std::process::exit(code)
  }

  init_tauri();
}
