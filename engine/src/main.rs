mod app;
mod daw;
mod util;
// mod lang;
mod event_dispatch;
mod project;
mod project_files;
mod recording;
mod recent;
mod mixer_commands;
mod audio_commands;
mod sequencer_commands;
mod bind_functions;

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

  // read before the menu is built, which lists them
  let recent_projects = recent::RecentProjects::load();

  // open the audio output now, so the first play or preview doesn't wait
  // for it. if it can't be opened, playing tries again
  std::thread::spawn(|| {
    if let Err(e) = daw::engine() {
      println!("could not open the audio output: {}", e);
    }
  });

  tauri::Builder::default()
    .setup(app::window::setup)
    .menu(app::window::build_menu(&recent_projects))
    .manage(Arc::new(daw::InnerState::default()))
    .manage(recent_projects)
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
      set_pattern_bars,
      set_pattern_step,
      t_play_pattern,
      t_toggle_pattern,
      add_pattern_clip,
      explode_pattern_clips
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

/**
 * args: 
 *   -d | --debug => debug mode
 */
fn main() {
  let args: Vec<String> = env::args().skip(1).collect();

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
