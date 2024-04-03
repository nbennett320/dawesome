mod app;
mod daw;
mod util;
// mod lang;
mod event_dispatch;

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

    println!("input: \"{}\"", input);

    event_dispatch::bind_functions(&lua);

    lua.load(&input).exec().unwrap();

    input = "".to_string();
  }
}

fn init_tauri() {
  use event_dispatch::*;

  tauri::Builder::default()
    .setup(app::window::setup)
    .menu(app::window::build_menu())
    .manage(Arc::new(daw::InnerState::default()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      t_toggle_playlist,
      get_playlist_start_time,
      get_playlist_tempo,
      set_playlist_tempo,
      toggle_metronome_enabled,
      get_metronome_enabled,
      get_playlist_runtime_formatted,
      get_playlist_beat_count,
      get_playlist_time_signature,
      set_playlist_time_signature,
      get_sidebar_samples,
      enumerate_directory,
      t_preview_sample,
      get_audio_drivers,
      t_add_audiograph_node,
      move_audiograph_node,
      remove_audiograph_node,
      get_node_data,
      get_playlist_sample_offset,
      get_playlist_data,
      toggle_loop_enabled,
      get_loop_enabled,
      toggle_snap_enabled,
      get_snap_enabled,
      get_playlist_max_length,
      init_playlist_workspace,
      toggle_record_input
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
