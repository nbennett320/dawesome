use std::sync::atomic::{Ordering};
use std::sync::{Arc, Mutex};
use std::process;
use num_traits::ToPrimitive;
use mlua::{self, Function, Table};

use crate::daw::{beat_interval_from_tempo, InnerState};
use crate::{app, daw, util, event_dispatch, STATE};

fn list_defined_globals(globals: mlua::Table) {
  for pair in globals.pairs() {
    let (key, _): (String, mlua::Value) = pair.unwrap();
    println!("{}", key);
  }
}

pub fn bind_functions(
  lua: &mlua::Lua,
  // state: &Arc<daw::InnerState>,
) {
  let globals = lua.globals();

  // preview_sample
  let f_preview_sample = 
    lua.create_function(|_, path: String| {
    event_dispatch::preview_sample(path);

    Ok(())
  }).unwrap();

  let f_list_samples = lua.create_function(|_, (): ()| {
    let mut ls = process::Command::new("ls");
    ls.arg("./assets");
    ls.status().expect("ls didnt work");
    println!();
    Ok(())
  }).unwrap();
  
  // add_audiograph_node
  // todo: refactor to use consistent state
  let f_add_audiograph_node = 
    lua.create_function(|_, (path, start_offset_ms, track_number): (String, u64, u32)| {
    let start_offset = std::time::Duration::from_millis(start_offset_ms);

    // add_audiograph_node(state, path, track_number, start_offset, false);

    let id: u64;
    // don't snap
    id = crate::STATE
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .construct_and_add_node(path, start_offset, track_number);

    Ok(id)
  }).unwrap();

  let f_add_audiograph_node = 
    lua.create_function(|_, (path, beat_offset, track_number): (String, u64, u32)| {
    let start_offset = daw::n_beat_duration_from_tempo(crate::STATE.tempo(), beat_offset.to_u32().unwrap());
    let sample_path = String::from("./assets/") + &path;
    println!("adding {} to beat {} on track {}", sample_path, beat_offset, track_number);

    let id: u64;
    // don't snap
    id = crate::STATE
      .playlist
      .audiograph
      .lock()
      .unwrap()
      .construct_and_add_node(sample_path, start_offset, track_number);

    Ok(id)
  }).unwrap();
  
  let f_toggle_playlist = 
    lua.create_function(|_, (): ()| {    
    event_dispatch::toggle_playlist(&crate::STATE);

    Ok(())
  }).unwrap();

  let f_print_state = 
    lua.create_function(|_, (): ()| {
    println!("playlist.playing: {}", crate::STATE.playlist.playing());
    println!("# nodes: {}", crate::STATE.playlist.audiograph.lock().unwrap().nodes.len());
    println!("# metronome_enabled: {:?}", crate::STATE.metronome_enabled.load(Ordering::SeqCst));
    println!("# loop_enabled: {:?}", crate::STATE.playlist.loop_enabled.load(Ordering::SeqCst));
    println!("# total_beats: {:?}", crate::STATE.playlist.total_beats.load(Ordering::SeqCst));

    Ok(())
  }).unwrap();

  let f_print_nodes = 
    lua.create_function(|_, (): ()| {
      
      let nodes = &crate::STATE.playlist.audiograph.lock().unwrap().nodes;
      println!("# nodes: {}", nodes.len());

      for node in nodes {
        println!("-----------");
        println!("start_offset: {}", node.start_offset.as_millis());
        println!("track_number: {}", node.track_number);
      }
      println!("-----------");

    Ok(())
  }).unwrap();

  let f_toggle_metronome_enabled = 
    lua.create_function(|_, (): ()| {
    event_dispatch::toggle_metronome_enabled(&crate::STATE);

    Ok(())
  }).unwrap();

  let f_toggle_loop_enabled = 
    lua.create_function(|_, (): ()| {
    event_dispatch::toggle_loop_enabled(&crate::STATE);

    Ok(())
  }).unwrap();


  let f_help = lua.create_function(|lua_ref, (): ()| {
    list_defined_globals(lua_ref.globals());

    Ok(())
  }).unwrap();

  globals.set("preview_sample", f_preview_sample).unwrap();
  globals.set("ls", f_list_samples).unwrap();
  globals.set("add", f_add_audiograph_node).unwrap();
  // globals.set("add_audiograph_node_on_beat", f_add_audiograph_node).unwrap();
  globals.set("tp", f_toggle_playlist).unwrap();
  globals.set("print_state", f_print_state).unwrap();
  globals.set("print_nodes", f_print_nodes).unwrap();
  globals.set("toggle_metronome_enabled", f_toggle_metronome_enabled).unwrap();
  globals.set("toggle_loop_enabled", f_toggle_loop_enabled).unwrap();
  globals.set("help", f_help).unwrap();
}
