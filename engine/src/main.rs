mod app;
mod daw;
mod util;
mod lang;
mod event_dispatch;

use std::env;
use std::sync::{Arc};
use std::io::{self, stdout, Write};
use tauri;

#[macro_use(lazy_static)]
extern crate lazy_static;

fn debug(){
  println!("debug mode:");

  let mut vm = lang::vm::Vm::new();
  let mut line_num = 1;
  let mut input = String::new();

  loop {
    print!("> ");
    io::stdout().flush().ok();

    let mut line = String::new();
    io::stdin().read_line(&mut line).ok();

    if line == "exit\n" {
      break;
    }

    let res = lang::parser::Parser::new(line).compile();
    match res {
      Ok(func) => {
        func.chunk
          .disassemble(format!("repl line {}", line_num).as_str());
        let res = vm.run(func);
        match res {
          Ok(value) => println!("{}", value),
          _ => todo!("Handle runtime error"),
        }
      }
      Err(_) => println!("Compile error"),
    }

    line_num += 1;
  }

  // while input != "exit" {
  //   match io::stdin().read_line(&mut input) {
  //     Ok(_n) => {
  //       match &*input {
  //         "exit\n" => {
  //           break;
  //         }
  //         _ => {
  //           println!("exec: \"{}\"", input.trim());
  //           let res = lang::parser::Parser::new(input).compile();

  //           match res {
  //             Ok(func) => {
  //               func.chunk
  //                 .disassemble(format!("repl line {}", line_num).as_str());
  //               let res = vm.run(func);
  //               match res {
  //                 Ok(value) => println!("{}", value),
  //                 _ => todo!("Handle runtime error"),
  //               }
  //             }
  //             Err(_) => println!("Compile error"),
  //           }

  //           line_num += 1;
  //         }
  //       }
  //     }
  //     Err(error) => println!("error: {error}"),
  //   }

  //   // input = "".to_string();
  // }
}

fn init_tauri() {
  use event_dispatch::*;

  tauri::Builder::default()
    .setup(app::window::setup)
    .menu(app::window::build_menu())
    .manage(Arc::new(daw::InnerState::default()))
    .invoke_handler(tauri::generate_handler![
      get_playlist_playing,
      toggle_playlist,
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
      preview_sample,
      get_audio_drivers,
      add_audiograph_node,
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
