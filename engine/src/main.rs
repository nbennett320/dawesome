mod app;
mod daw;
mod util;
// mod lang;
mod event_dispatch;
mod project;
mod project_files;
mod recording;
mod recent;
mod theme;
mod preferences;
mod appearance_commands;
mod history;
mod history_commands;
mod mixer_commands;
mod audio_commands;
mod sequencer_commands;
mod clip_commands;
mod clipboard;
mod bounce;
mod join;
mod mixdown;
mod mixdown_file;
mod mixdown_commands;
mod bind_functions;
mod plugin;
mod plugin_catalog;
mod main_thread;
mod effects;
mod effect_commands;

use std::env;
use std::sync::{Arc, Mutex};
use std::io::{self, Write};
use daw::InnerState;
use tauri;
use mlua;

#[macro_use(lazy_static)]
extern crate lazy_static;

lazy_static! {
  pub static ref STATE: Arc<daw::InnerState> = Arc::new(daw::InnerState::default());
  // pub static ref STATE: Arc<Mutex<daw::InnerState>> = Arc::new(Mutex::from(daw::InnerState::default()));
}

fn debug(){
  println!("debug mode:");
  use mlua::prelude::*;
  let lua = Lua::new();
  let globals = lua.globals();
  // let state = Arc::new(Mutex::from(daw::InnerState::default()));

  let mut input = String::new();

  loop {
    print!("> ");
    io::stdout().flush().ok();
    io::stdin().read_line(&mut input).ok();

    if input == "exit\n" {
      break;
    }

    // println!("input: \"{}\"", input);

    bind_functions::bind_functions(&lua);

    lua.load(&input).exec().unwrap();

    input = "".to_string();
  }
}

fn init_tauri() {
  use event_dispatch::*;
  use project::{save_project, load_project, new_project, project_file_for};
  use recent::{add_recent_project, remove_recent_project};
  use mixer_commands::*;
  use audio_commands::*;
  use sequencer_commands::*;
  use clip_commands::*;
  use clipboard::*;
  use bounce::*;
  use join::*;
  use mixdown_commands::*;
  use appearance_commands::{themes, preferences, set_preferences, set_window_background};
  use history_commands::{undo, redo};
  use effect_commands::*;

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
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .setup(move |app| {
      // (setup runs on the main thread, which plugins are called on)
      main_thread::init(app.handle().clone());
      effects::keep_idling(racks.clone());
      app::window::setup(app)
    })
    .manage(state)
    .manage(recent_projects)
    .manage(preferences::AppPreferences::load())
    .manage(Arc::new(mixdown_commands::Mixdowns::default()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      t_toggle_playlist,
      t_play_playlist,
      t_pause_playlist,
      t_stop_playlist,
      t_seek_playlist,
      get_playlist_position,
      get_playlist_start_time,
      get_playlist_transport,
      get_playlist_tempo,
      set_playlist_tempo,
      t_toggle_metronome_enabled,
      get_metronome_enabled,
      t_toggle_count_in,
      get_count_in,
      get_playlist_runtime_formatted,
      get_playlist_beat_count,
      get_playlist_time_signature,
      set_playlist_time_signature,
      get_sidebar_samples,
      enumerate_directory,
      t_preview_sample,
      get_audio_drivers,
      get_audio_settings,
      set_audio_settings,
      t_add_audiograph_node,
      move_audiograph_node,
      move_audiograph_nodes,
      remove_audiograph_node,
      remove_audiograph_nodes,
      themes,
      preferences,
      set_preferences,
      set_window_background,
      undo,
      redo,
      get_node_data,
      get_playlist_sample_offset,
      get_playlist_data,
      t_toggle_loop_enabled,
      get_loop_enabled,
      toggle_snap_enabled,
      get_snap_enabled,
      get_snap_setting,
      set_snap_subdivision,
      get_sample_preview,
      get_clip,
      set_clip_settings,
      t_preview_node,
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
      set_master_volume,
      reset_mixer,
      get_mixer_levels,
      get_playlist_max_length,
      init_playlist_workspace,
      t_start_recording,
      t_stop_recording,
      collect_recording,
      t_list_recordings,
      t_rename_recording,
      t_delete_recording,
      t_move_recording,
      t_move_recording_folder,
      t_create_recording_folder,
      t_rename_recording_folder,
      t_delete_recording_folder,
      get_record_setup,
      set_track_armed,
      listen_to_input,
      get_input_levels,
      get_sequencer,
      add_sequencer_row,
      remove_sequencer_row,
      set_sequencer_row_volume,
      set_sequencer_row_muted,
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
      t_play_pattern,
      t_toggle_pattern,
      add_pattern_clip,
      explode_pattern_clips,
      set_clip_windows,
      split_clips,
      stretch_clips,
      copy_clips,
      cut_clips,
      paste_clips,
      repeat_clips,
      clipboard_count,
      deep_copy_clips,
      join_clips,
      mixdown_defaults,
      start_mixdown,
      mixdown_status,
      cancel_mixdown,
      reveal_file,
      get_plugins,
      scan_plugins,
      get_effects,
      add_effect,
      remove_effect,
      move_effect,
      set_effect_bypassed
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

/**
 * args: 
 *   -d | --debug => debug mode
 *   --scan-plugin <path> => scan one plugin bundle and print what's in it
 */
fn main() {
  let args: Vec<String> = env::args().skip(1).collect();

  // `--scan-plugin <path>`: this is a child process scanning one plugin
  // bundle (see plugin::scan), so nothing else starts
  if let Some(code) = plugin::scan::scanner_main(&args) {
    std::process::exit(code)
  }

  if args.is_empty() {
    init_tauri();
  }

  for arg in args {
    match &arg[..] {
      "-d" | "--debug" => debug(),
      _ => {
        if arg.starts_with('-') {
          println!("Unkown argument: {}", arg);
        }
      }
    }
  }
}
