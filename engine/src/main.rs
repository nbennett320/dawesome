mod app;
mod commands;
mod daw;
mod engine;
mod recent;
mod theme;
mod preferences;
mod plugin;
mod effect_windows;
#[cfg(test)]
mod perf;

use std::env;
use std::sync::Arc;
use tauri::{self, Manager};

#[macro_use(lazy_static)]
extern crate lazy_static;

fn init_tauri() {
  use commands::transport::*;
  use commands::sample::*;
  use commands::playlist::*;
  use commands::project::{save_project, load_project, new_project, project_file_for, project_has_unsaved_changes};
  use app::window::quit;
  use recent::{add_recent_project, remove_recent_project};
  use commands::mixer::*;
  use commands::audio::*;
  use commands::sequencer::*;
  use commands::clip::*;
  use commands::clipboard::*;
  use commands::mixdown::*;
  use commands::appearance::{themes, preferences, set_preferences, set_window_background};
  use commands::history::{undo, redo};
  use commands::effect::*;
  use effect_windows::*;
  use commands::synth::*;
  use commands::tape::{get_tape, set_tape_machine, set_track_tape};

  // read before the menu is built, which lists them
  let recent_projects = recent::RecentProjects::load();

  // open the audio output now, so the first play or preview doesn't wait
  // for it. if it can't be opened, playing tries again
  std::thread::spawn(|| {
    if let Err(e) = engine::engine() {
      println!("could not open the audio output: {}", e);
    }
  });

  let state = Arc::new(daw::Session::default());

  // the plugins found last time, then a look for new or changed ones in the
  // background
  let plugins = state.playlist.effects.plugins.clone();
  if let Some(file) = daw::plugin_catalog::default_file() {
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
      let handle = app.handle().clone();
      daw::main_thread::init(Box::new(move |job| handle.run_on_main_thread(job).map_err(|e| e.to_string())));
      daw::effects::keep_idling(racks.clone());
      daw::effects::follow_output(&racks);
      // the tape follows the preferences it's set by (Settings → Controls)
      let prefs = app.state::<preferences::AppPreferences>().current();
      daw::tape::set_preferences(prefs.tape_mode, prefs.tape_tap, prefs.tape_method);
      daw::tape::set_inertia(prefs.tape_inertia);
      daw::scrub::set_preference(prefs.scrub);
      effect_windows::init(&racks);
      daw::actions::synth::keep_following(following.clone());
      // print mode's job: re-prints taped tracks when they change
      daw::tape::keep_printing(following.clone());
      // and the scrub proxy's: the song made again for scrubbing when it changes
      daw::scrub::keep_making(following.clone());
      // the empty project the app starts with is unchanged (see
      // project::has_unsaved_changes)
      daw::project::mark_unchanged(&following);
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
    .manage(Arc::new(daw::actions::mixdown::Mixdowns::default()))
    .manage(std::sync::Mutex::new(app::workspaces::WorkspaceViewport::new()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      toggle_playlist,
      play_playlist,
      pause_playlist,
      stop_playlist,
      seek_playlist,
      scrub_begin,
      scrub_to,
      scrub_end,
      wind_begin,
      wind_end,
      get_playlist_transport,
      set_playlist_tempo,
      toggle_metronome_enabled,
      get_metronome_enabled,
      toggle_count_in,
      set_playlist_time_signature,
      get_sidebar_samples,
      enumerate_directory,
      preview_sample,
      stop_previews,
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
      add_track,
      toggle_loop_enabled,
      set_loop_region,
      clear_loop_region,
      set_loop_end,
      clear_loop_end,
      get_loop_settings,
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
      get_tape,
      set_tape_machine,
      set_track_tape,
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
      add_sample_row,
      add_synth_row,
      remove_row,
      move_row,
      rename_row,
      set_row_volume,
      set_row_pan,
      set_row_muted,
      replace_row_sample,
      replace_row_synth,
      preview_row,
      set_row_step_pitch,
      set_row_sample_pitch,
      set_row_gated,
      play_row_note,
      stop_row_note,
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
      trim_pattern,
      play_pattern,
      toggle_pattern,
      seek_pattern,
      add_pattern_clip,
      explode_pattern_clips,
      paint_pattern_clips,
      set_clip_windows,
      split_clips,
      split_clips_at,
      stretch_clips,
      copy_clips,
      cut_clips,
      paste_clips,
      repeat_clips,
      copy_span_clips,
      cut_span_clips,
      clear_span_clips,
      paste_span_clips,
      repeat_span_clips,
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
      get_effect_params,
      set_effect_param,
      set_effect_param_text,
      get_effect_spectrum,
      set_pattern_notes,
      add_synth_on_track,
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
