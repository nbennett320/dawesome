// checks against real VST3 plugins installed on this machine (CI has none,
// so they only run when asked):
//
//   cargo test --no-default-features --test plugins -- --ignored [name...]
//
// this test has no harness: its main runs every check on the process's main
// thread, which is where plugins (JUCE's especially) expect their non-audio
// calls. `--off-main` runs each check on another thread instead, to see what
// breaks. a check whose plugin isn't installed is skipped
//
// the plugin module is compiled into this test directly (the app is a
// binary, with no library to link against). its unit tests' imports go unused
// here, since a test without a harness has no #[test]s
#![allow(unused_imports)]

#[path = "../src/plugin/mod.rs"]
mod plugin;

use plugin::scan::{self, Catalog};
use plugin::{Event, Plugin, PluginInfo, PluginKind, Transport};
use std::path::{Path, PathBuf};
use std::time::Instant;

enum Outcome {
  Pass(String),
  Skip(String),
}

type Check = fn() -> Result<Outcome, String>;

const RATE: u32 = 44_100;
const BLOCK: usize = 512;

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if !args.iter().any(|a| a == "--ignored" || a == "--include-ignored") {
    println!("plugin checks skipped: they need installed plugins (run with -- --ignored)");
    return;
  }
  let off_main = args.iter().any(|a| a == "--off-main");
  let filters: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();

  let checks: Vec<(&str, Check)> = vec![
    ("scan_every_bundle_in_a_child", scan_every_bundle_in_a_child),
    ("scan_every_bundle_in_process", scan_every_bundle_in_process),
    ("catalog_refresh_with_child_scans", catalog_refresh_with_child_scans),
    ("chow_processes_a_sine", chow_processes_a_sine),
    ("chow_params_round_trip", chow_params_round_trip),
    ("chow_state_round_trips_to_a_new_instance", chow_state_round_trips_to_a_new_instance),
    ("chow_at_48k", chow_at_48k),
    ("pancake_loads_and_processes", pancake_loads_and_processes),
    ("vocal_doubler_loads_and_processes", vocal_doubler_loads_and_processes),
    ("every_effect_loads_and_processes", every_effect_loads_and_processes),
    ("nyasynth_plays_and_decays", nyasynth_plays_and_decays),
    ("every_instrument_loads_and_plays", every_instrument_loads_and_plays),
    ("decent_sampler_copy_plays_once_settled", decent_sampler_copy_plays_once_settled),
    ("decent_sampler_synth_copies_on_two_tracks", decent_sampler_synth_copies_on_two_tracks),
    ("twenty_instances", twenty_instances),
    ("processor_dropped_before_plugin", processor_dropped_before_plugin),
    ("processor_dropped_after_plugin_on_another_thread", processor_dropped_after_plugin_on_another_thread),
    ("editor_opens_in_a_window", editor_opens_in_a_window),
    ("editors_float_in_their_own_windows", editors_float_in_their_own_windows),
    ("pancake_docks_over_the_page", pancake_docks_over_the_page),
    ("plugins_dock_side_by_side", plugins_dock_side_by_side),
    ("editors_reopen_in_windows_and_docks", editors_reopen_in_windows_and_docks),
  ];

  let mut failed = 0;
  for (name, check) in checks {
    if !filters.is_empty() && !filters.iter().any(|f| name.contains(f.as_str())) {
      continue;
    }
    let started = Instant::now();
    let result = if off_main {
      std::thread::spawn(move || run(check)).join().unwrap_or_else(|_| Err("panicked".into()))
    } else {
      run(check)
    };
    let took = started.elapsed().as_secs_f32();
    match result {
      Ok(Outcome::Pass(detail)) => println!("ok      {} ({:.1}s) {}", name, took, detail),
      Ok(Outcome::Skip(why)) => println!("skipped {}: {}", name, why),
      Err(e) => {
        failed += 1;
        println!("FAILED  {} ({:.1}s): {}", name, took, e);
      }
    }
  }
  if failed > 0 {
    println!("{} plugin checks failed", failed);
    std::process::exit(1);
  }
}

fn run(check: Check) -> Result<Outcome, String> {
  std::panic::catch_unwind(check).unwrap_or_else(|e| {
    let message = e
      .downcast_ref::<String>()
      .cloned()
      .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
      .unwrap_or_default();
    Err(format!("panicked: {}", message))
  })
}

fn ensure(ok: bool, why: impl Into<String>) -> Result<(), String> {
  if ok {
    Ok(())
  } else {
    Err(why.into())
  }
}

// every installed bundle, scanned in a child process (once). a bundle that
// crashes or hangs there is never loaded into this process
fn child_scans() -> &'static Vec<(PathBuf, Result<Vec<PluginInfo>, String>)> {
  static SCANS: std::sync::OnceLock<Vec<(PathBuf, Result<Vec<PluginInfo>, String>)>> = std::sync::OnceLock::new();
  SCANS.get_or_init(|| {
    let exe = Path::new(env!("CARGO_BIN_EXE_app"));
    scan::find_bundles().into_iter().map(|b| {
      let result = scan::scan_in_child(exe, &b, scan::SCAN_TIMEOUT);
      (b, result)
    }).collect()
  })
}

// the bundles that scanned cleanly in a child, with their plugins
fn safe_bundles() -> Vec<(PathBuf, Vec<PluginInfo>)> {
  child_scans().iter().filter_map(|(b, r)| r.as_ref().ok().map(|p| (b.clone(), p.clone()))).collect()
}

// the first plugin in an installed bundle, by the bundle's file name. Err
// when it's installed but didn't scan cleanly
fn plugin_in(name: &str) -> Result<Option<(PathBuf, PluginInfo)>, String> {
  let named = |p: &PathBuf| p.file_name().map_or(false, |n| n == name);
  let Some((path, result)) = child_scans().iter().find(|(p, _)| named(p)) else { return Ok(None) };
  let plugins = result.as_ref().map_err(|e| format!("{} didn't scan in a child: {}", name, e))?;
  let info = plugins.first().cloned().ok_or(format!("{} has no plugins", name))?;
  Ok(Some((path.clone(), info)))
}

fn load(name: &str, rate: u32) -> Result<Option<Plugin>, String> {
  match plugin_in(name)? {
    Some((path, info)) => Plugin::load(&path, &info.id, rate).map(Some),
    None => Ok(None),
  }
}

fn not_installed(name: &str) -> Result<Outcome, String> {
  Ok(Outcome::Skip(format!("{} isn't installed", name)))
}

fn sine(frames: usize, start: usize, rate: u32) -> Vec<f32> {
  (0..frames).map(|i| 0.5 * (2. * std::f32::consts::PI * 440. * (start + i) as f32 / rate as f32).sin()).collect()
}

// run blocks of a sine (or silence) through a processor, giving its output
fn run_blocks(
  processor: &mut dyn plugin::Processor,
  blocks: usize,
  rate: u32,
  silent: bool,
  events: &[Event],
) -> (Vec<f32>, Vec<f32>) {
  let mut out = (vec![], vec![]);
  for b in 0..blocks {
    let input = if silent { vec![0.; BLOCK] } else { sine(BLOCK, b * BLOCK, rate) };
    let (mut left, mut right) = (vec![0f32; BLOCK], vec![0f32; BLOCK]);
    let transport = Transport { playing: true, position: (b * BLOCK) as i64, sample_rate: rate, ..Transport::default() };
    let events = if b == 0 { events } else { &[] };
    processor.process([&input, &input], [&mut left, &mut right], events, &transport);
    out.0.extend(left);
    out.1.extend(right);
  }
  out
}

// run the main thread's run loop for a while, as the app's window would:
// plugins post work to it
#[cfg(target_os = "macos")]
fn pump(seconds: f64) {
  use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
  let until = Instant::now() + std::time::Duration::from_secs_f64(seconds);
  while Instant::now() < until {
    // safe: reading a CoreFoundation constant
    let mode = unsafe { kCFRunLoopDefaultMode };
    CFRunLoop::run_in_mode(mode, std::time::Duration::from_millis(5), false);
  }
}

#[cfg(not(target_os = "macos"))]
fn pump(seconds: f64) {
  std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
}

fn rms(samples: &[f32]) -> f32 {
  (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

// finite, not silent, and not just the input
fn check_effect_output(name: &str, out: &(Vec<f32>, Vec<f32>), rate: u32) -> Result<String, String> {
  let all: Vec<f32> = out.0.iter().chain(&out.1).copied().collect();
  ensure(all.iter().all(|s| s.is_finite()), format!("{} gave non-finite samples", name))?;
  let level = rms(&all);
  ensure(level > 1e-4, format!("{} was silent", name))?;
  let input = sine(out.0.len(), 0, rate);
  let diff = out.0.iter().zip(&input).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
  Ok(format!("rms {:.3}, max difference from the input {:.3}", level, diff))
}

fn scan_every_bundle_in_a_child() -> Result<Outcome, String> {
  let started = Instant::now();
  let scans = child_scans();
  ensure(!scans.is_empty(), "no bundles installed")?;
  let mut lines = vec![format!("{:.1}s for all", started.elapsed().as_secs_f32())];
  for (path, result) in scans {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    match result {
      Ok(plugins) => {
        for p in plugins {
          lines.push(format!("\n          {}: {} by {} [{:?}, {}] {:?}", name, p.name, p.vendor, p.kind, p.category, p.subcategories));
        }
      }
      Err(e) => lines.push(format!("\n          {}: failed: {}", name, e)),
    }
  }
  Ok(Outcome::Pass(lines.concat()))
}

// scanning here finds what the child found (for the bundles that are safe
// to load here)
fn scan_every_bundle_in_process() -> Result<Outcome, String> {
  let mut scanned = 0;
  for (path, from_child) in safe_bundles() {
    let here = scan::scan_bundle(&path)?;
    ensure(here == from_child, format!("{}: scanning here found something else", path.display()))?;
    scanned += 1;
  }
  ensure(scanned > 0, "nothing was safe to scan")?;
  Ok(Outcome::Pass(format!("{} bundles agree", scanned)))
}

fn catalog_refresh_with_child_scans() -> Result<Outcome, String> {
  let exe = Path::new(env!("CARGO_BIN_EXE_app"));
  let file = std::env::temp_dir().join(format!("dawesome-catalog-{}.json", std::process::id()));
  let mut catalog = Catalog::load(&file);
  let mut last = (0, 0);
  catalog.refresh(exe, false, &mut |done, total| last = (done, total));
  ensure(last.0 == last.1 && last.1 > 0, format!("progress ended at {:?}", last))?;
  catalog.save(&file)?;
  let mut again = Catalog::load(&file);
  ensure(again == catalog, "the saved catalog reads back differently")?;
  let mut second = (9, 9);
  again.refresh(exe, false, &mut |done, total| second = (done, total));
  ensure(second == (0, 0), format!("a second refresh scanned {:?}", second))?;
  let _ = std::fs::remove_file(&file);
  Ok(Outcome::Pass(format!(
    "{} plugins, {} failed {:?}",
    catalog.plugins.len(),
    catalog.failed.len(),
    catalog.failed.iter().map(|f| format!("{}: {}", f.path, f.reason)).collect::<Vec<_>>()
  )))
}

fn chow_processes_a_sine() -> Result<Outcome, String> {
  let Some(mut chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  ensure(chow.info().kind == PluginKind::Effect, "CHOW isn't an effect")?;
  let mut processor = chow.take_processor().ok_or("no processor")?;
  ensure(chow.take_processor().is_none(), "the processor was given twice")?;
  let out = run_blocks(processor.as_mut(), 16, RATE, false, &[]);
  let detail = check_effect_output("CHOW", &out, RATE)?;
  let input = sine(out.0.len(), 0, RATE);
  ensure(out.0.iter().zip(&input).any(|(a, b)| (a - b).abs() > 1e-3), "CHOW's output is its input")?;
  Ok(Outcome::Pass(format!("{}; latency {}", detail, chow.latency())))
}

// a continuous parameter that's meant to be changed
fn changeable(plugin: &Plugin) -> Option<plugin::ParamInfo> {
  plugin
    .params()
    .into_iter()
    .find(|p| p.automatable && !p.read_only && !p.bypass && !p.hidden && p.step_count == 0)
}

fn chow_params_round_trip() -> Result<Outcome, String> {
  let Some(mut chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  let params = chow.params();
  ensure(!params.is_empty(), "no parameters")?;
  let mut processor = chow.take_processor().ok_or("no processor")?;
  let before = run_blocks(processor.as_mut(), 8, RATE, false, &[]);

  // the input gain, if it has one, changes the sound for sure
  let param = params
    .iter()
    .find(|p| p.name.to_lowercase().contains("input") && !p.read_only && p.step_count == 0)
    .cloned()
    .or_else(|| changeable(&chow))
    .ok_or("no changeable parameter")?;
  let value = if param.default > 0.5 { 0.1 } else { 0.9 };
  let text_before = chow.param_text(param.id, chow.param(param.id));
  chow.set_param(param.id, value);
  let got = chow.param(param.id);
  ensure((got - value).abs() < 0.01, format!("{} reads back {} after setting {}", param.name, got, value))?;
  let text_after = chow.param_text(param.id, got);

  let after = run_blocks(processor.as_mut(), 8, RATE, false, &[]);
  chow.idle();
  let difference = before.0.iter().zip(&after.0).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
  ensure(difference > 1e-3 || text_before != text_after, "neither the sound nor the value changed")?;
  Ok(Outcome::Pass(format!(
    "{} params; {} \"{}\" -> \"{}\", output changed by up to {:.3}",
    params.len(),
    param.name,
    text_before,
    text_after,
    difference
  )))
}

fn chow_state_round_trips_to_a_new_instance() -> Result<Outcome, String> {
  let Some(mut chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  let mut processor = chow.take_processor().ok_or("no processor")?;
  let mut changed = 0;
  // (not its preset parameter, which loads a preset over the others)
  let params = chow.params();
  let ordinary = params.iter().filter(|p| {
    let name = p.name.to_lowercase();
    !p.read_only && !p.bypass && p.step_count == 0 && !name.contains("preset") && !name.contains("program")
  });
  for p in ordinary.take(4) {
    let value = if p.default > 0.5 { 0.2 + 0.05 * changed as f64 } else { 0.8 - 0.05 * changed as f64 };
    chow.set_param(p.id, value);
    changed += 1;
  }
  // the component (whose state is saved) hears of changes in its next
  // block, and CHOW files them into its state on the main run loop
  run_blocks(processor.as_mut(), 1, RATE, false, &[]);
  chow.idle();
  pump(0.3);
  let state = chow.state()?;
  let values: Vec<(u32, f64)> = params.iter().map(|p| (p.id, chow.param(p.id))).collect();

  let (path, info) = plugin_in("CHOWTapeModel.vst3")?.unwrap();
  let mut copy = Plugin::load(&path, &info.id, RATE)?;
  ensure(copy.state()? != state, "the saved state is the same as a new instance's")?;
  copy.set_state(&state)?;
  let mut differ = vec![];
  for (id, value) in &values {
    let got = copy.param(*id);
    if (got - value).abs() > 1e-4 {
      differ.push(format!("{}: {} vs {}", id, value, got));
    }
  }
  ensure(differ.is_empty(), format!("parameters differ: {:?}", differ))?;
  ensure(copy.set_state(b"junk").is_err(), "junk state was taken")?;
  Ok(Outcome::Pass(format!("{} bytes, {} params changed, {} compared", state.len(), changed, values.len())))
}

fn chow_at_48k() -> Result<Outcome, String> {
  let Some(mut chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  let mut processor = chow.take_processor().ok_or("no processor")?;
  run_blocks(processor.as_mut(), 4, RATE, false, &[]);
  chow.set_sample_rate(48_000)?;
  let out = run_blocks(processor.as_mut(), 16, 48_000, false, &[]);
  Ok(Outcome::Pass(check_effect_output("CHOW at 48k", &out, 48_000)?))
}

fn effect_check(bundle_name: &str) -> Result<Outcome, String> {
  let Some(mut plugin) = load(bundle_name, RATE)? else { return not_installed(bundle_name) };
  let mut processor = plugin.take_processor().ok_or("no processor")?;
  let out = run_blocks(processor.as_mut(), 32, RATE, false, &[]);
  plugin.idle();
  Ok(Outcome::Pass(format!(
    "{} ({} params, latency {}): {}",
    plugin.info().name,
    plugin.params().len(),
    plugin.latency(),
    check_effect_output(bundle_name, &out, RATE)?
  )))
}

fn pancake_loads_and_processes() -> Result<Outcome, String> {
  effect_check("PanCake 2.vst3")
}

fn vocal_doubler_loads_and_processes() -> Result<Outcome, String> {
  effect_check("Vocal Doubler.vst3")
}

// every installed effect: loads, processes a sine, unloads
fn every_effect_loads_and_processes() -> Result<Outcome, String> {
  let mut lines = vec![];
  for (path, infos) in safe_bundles() {
    for info in infos.into_iter().filter(|i| i.kind == PluginKind::Effect) {
      let line = match Plugin::load(&path, &info.id, RATE) {
        Ok(mut plugin) => {
          let mut processor = plugin.take_processor().unwrap();
          let out = run_blocks(processor.as_mut(), 32, RATE, false, &[]);
          plugin.idle();
          match check_effect_output(&info.name, &out, RATE) {
            Ok(detail) => format!("ok: {}", detail),
            Err(e) => format!("PROBLEM: {}", e),
          }
        }
        Err(e) => format!("didn't load: {}", e),
      };
      lines.push(format!("\n          {} ({}): {}", info.name, path.file_name().unwrap().to_string_lossy(), line));
    }
  }
  Ok(Outcome::Pass(lines.concat()))
}

// plays a note, then lets it go: (level while held, level at the end)
fn play_note(plugin: &mut Plugin) -> Result<(f32, f32), String> {
  let mut processor = plugin.take_processor().ok_or("no processor")?;
  // give it a moment (and the main run loop) to load its sounds, as it
  // would have in the app
  for _ in 0..100 {
    run_blocks(processor.as_mut(), 1, RATE, true, &[]);
    plugin.idle();
    pump(0.02);
  }
  let on = [Event::NoteOn { offset: 0, pitch: 60, velocity: 0.9 }];
  let held = run_blocks(processor.as_mut(), 40, RATE, true, &on);
  let off = [Event::NoteOff { offset: 0, pitch: 60 }];
  let released = run_blocks(processor.as_mut(), 200, RATE, true, &off);
  let all: Vec<f32> = held.0.iter().chain(&held.1).chain(&released.0).chain(&released.1).copied().collect();
  ensure(all.iter().all(|s| s.is_finite()), "non-finite samples")?;
  plugin.idle();
  let tail = released.0.len() - BLOCK * 8;
  Ok((rms(&held.0[BLOCK * 8..]), rms(&released.0[tail..])))
}

fn nyasynth_plays_and_decays() -> Result<Outcome, String> {
  // it's installed but may not be loadable here (see the report it gives)
  let plugin = match plugin_in("nyasynth.vst3") {
    Ok(Some(plugin)) => plugin,
    Ok(None) => return not_installed("nyasynth"),
    Err(e) => return Ok(Outcome::Skip(e)),
  };
  let mut synth = Plugin::load(&plugin.0, &plugin.1.id, RATE)?;
  ensure(synth.info().kind == PluginKind::Instrument, "nyasynth isn't an instrument")?;
  let (held, end) = play_note(&mut synth)?;
  ensure(held > 1e-3, format!("the held note was silent (rms {})", held))?;
  ensure(end < held * 0.5, format!("the note didn't decay: {} held, {} after release", held, end))?;
  Ok(Outcome::Pass(format!("rms {:.4} held, {:.5} two seconds after release", held, end)))
}

fn every_instrument_loads_and_plays() -> Result<Outcome, String> {
  let mut lines = vec![];
  for (path, infos) in safe_bundles() {
    for info in infos.into_iter().filter(|i| i.kind == PluginKind::Instrument) {
      let line = match Plugin::load(&path, &info.id, RATE) {
        Ok(mut plugin) => match play_note(&mut plugin) {
          Ok((held, end)) => format!("rms {:.4} held, {:.5} after release", held, end),
          Err(e) => format!("PROBLEM: {}", e),
        },
        Err(e) => format!("didn't load: {}", e),
      };
      lines.push(format!("\n          {} ({}): {}", info.name, path.file_name().unwrap().to_string_lossy(), line));
    }
  }
  Ok(Outcome::Pass(lines.concat()))
}

// a second instance with the first one's settings, as a mixdown makes one:
// silent at first (it loads its sounds on the main run loop a second or so
// in), heard once it's been played silence while the run loop turns for
// the mixdown's settling time, and safe to tear down while the first lives
// on (DecentSampler used to crash a moment after: see module.rs)
fn decent_sampler_copy_plays_once_settled() -> Result<Outcome, String> {
  let Some((path, info)) = plugin_in("DecentSampler.vst3")? else { return not_installed("DecentSampler") };
  let mut live = Plugin::load(&path, &info.id, RATE)?;
  let mut live_processor = live.take_processor().ok_or("no processor")?;
  let settle = |plugin: &mut Plugin, processor: &mut dyn plugin::Processor| {
    let started = Instant::now();
    while started.elapsed().as_secs_f64() < 2. {
      run_blocks(processor, 1, RATE, true, &[]);
      plugin.idle();
      pump(0.01);
    }
  };
  settle(&mut live, live_processor.as_mut());
  let state = live.state()?;
  let note = [Event::NoteOn { offset: 0, pitch: 60, velocity: 0.9 }];

  let mut levels = vec![];
  for settled in [false, true] {
    let mut copy = Plugin::load_offline(&path, &info.id, RATE)?;
    copy.set_state(&state)?;
    let mut processor = copy.take_processor().ok_or("no processor")?;
    if settled { settle(&mut copy, processor.as_mut()); }
    levels.push(rms(&run_blocks(processor.as_mut(), 40, RATE, true, &note).0));
    drop(processor);
    drop(copy);
    pump(2.);
  }
  ensure(levels[0] == 0., format!("the copy played before it had loaded its sounds ({})", levels[0]))?;
  ensure(levels[1] > 1e-3, format!("the settled copy was silent ({})", levels[1]))?;
  // (it doesn't crash every time)
  for _ in 0..4 {
    let mut copy = Plugin::load(&path, &info.id, RATE)?;
    copy.set_state(&state)?;
    pump(1.);
    drop(copy);
    pump(1.5);
  }
  let live_level = rms(&run_blocks(live_processor.as_mut(), 40, RATE, true, &note).0);
  ensure(live_level > 1e-3, "the first instance went quiet")?;
  Ok(Outcome::Pass(format!("rms {:.4} unsettled, {:.4} settled, {:.4} the first", levels[0], levels[1], live_level)))
}

// a synth played by two tracks, as effects/synths.rs keeps it: its master
// copy, and a copy for each track given the master's settings, live and then
// as a mixdown's offline copies. each copy sounds once it's settled, plays
// only the notes it's sent, sounds again after the master's settings are
// passed on to it again, and is safe to tear down while the others live on
fn decent_sampler_synth_copies_on_two_tracks() -> Result<Outcome, String> {
  let Some((path, info)) = plugin_in("DecentSampler.vst3")? else { return not_installed("DecentSampler") };
  // played silence while the run loop turns, as the app does
  let settle = |copies: &mut [(Plugin, Box<dyn plugin::Processor>)], seconds: f64| {
    let started = Instant::now();
    while started.elapsed().as_secs_f64() < seconds {
      for (plugin, processor) in copies.iter_mut() {
        run_blocks(processor.as_mut(), 1, RATE, true, &[]);
        plugin.idle();
      }
      pump(0.01);
    }
  };
  let take = |mut plugin: Plugin| -> Result<(Plugin, Box<dyn plugin::Processor>), String> {
    let processor = plugin.take_processor().ok_or("no processor")?;
    Ok((plugin, processor))
  };
  let mut master = vec![take(Plugin::load(&path, &info.id, RATE)?)?];
  settle(&mut master, 2.);
  let state = master[0].0.state()?;
  let note = |pitch| [Event::NoteOn { offset: 0, pitch, velocity: 0.9 }];

  let mut report = vec![];
  for offline in [false, true] {
    let mut tracks = vec![];
    for _ in 0..2 {
      let mut copy = if offline { Plugin::load_offline(&path, &info.id, RATE)? } else { Plugin::load(&path, &info.id, RATE)? };
      copy.set_state(&state)?;
      tracks.push(take(copy)?);
    }
    settle(&mut tracks, 2.);
    // each hears only what it's sent: the second nothing, then its own note
    let first = rms(&run_blocks(tracks[0].1.as_mut(), 40, RATE, true, &note(60)).0);
    let quiet = rms(&run_blocks(tracks[1].1.as_mut(), 40, RATE, true, &[]).0);
    let second = rms(&run_blocks(tracks[1].1.as_mut(), 40, RATE, true, &note(64)).0);
    let kind = if offline { "offline" } else { "live" };
    ensure(first > 1e-3, format!("the first {} copy was silent ({})", kind, first))?;
    ensure(quiet < 1e-4, format!("the second {} copy sounded without a note ({})", kind, quiet))?;
    ensure(second > 1e-3, format!("the second {} copy was silent ({})", kind, second))?;

    // the master's settings passed on again (a preset or library changed)
    let (plugin, _) = &mut tracks[0];
    plugin.set_state(&state)?;
    settle(&mut tracks, 2.);
    let again = rms(&run_blocks(tracks[0].1.as_mut(), 40, RATE, true, &note(62)).0);
    ensure(again > 1e-3, format!("the first {} copy was silent once given the settings again ({})", kind, again))?;
    report.push(format!("{} {:.4} {:.4} {:.4}", kind, first, second, again));

    // one torn down while the master and the other live on
    let gone = tracks.remove(0);
    drop(gone);
    pump(2.);
    let left = rms(&run_blocks(tracks[0].1.as_mut(), 40, RATE, true, &note(67)).0);
    ensure(left > 1e-3, format!("the other {} copy went quiet when one was torn down", kind))?;
    drop(tracks);
    pump(2.);
  }
  let level = rms(&run_blocks(master[0].1.as_mut(), 40, RATE, true, &note(60)).0);
  ensure(level > 1e-3, "the master copy went quiet")?;
  Ok(Outcome::Pass(format!("rms {} (first, second, again), master {:.4}", report.join("; "), level)))
}

fn twenty_instances() -> Result<Outcome, String> {
  let Some((path, info)) = plugin_in("CHOWTapeModel.vst3")? else { return not_installed("CHOW Tape Model") };
  // all alive at once, then all gone, then one at a time (the module is
  // closed and opened again each time)
  let mut plugins = vec![];
  for _ in 0..20 {
    let mut p = Plugin::load(&path, &info.id, RATE)?;
    let mut processor = p.take_processor().unwrap();
    run_blocks(processor.as_mut(), 1, RATE, false, &[]);
    plugins.push((p, processor));
  }
  drop(plugins);
  for _ in 0..20 {
    let mut p = Plugin::load(&path, &info.id, RATE)?;
    let mut processor = p.take_processor().unwrap();
    run_blocks(processor.as_mut(), 1, RATE, false, &[]);
  }
  Ok(Outcome::Pass("20 at once, then 20 one after another".into()))
}

fn processor_dropped_before_plugin() -> Result<Outcome, String> {
  let Some(mut chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  let mut processor = chow.take_processor().unwrap();
  run_blocks(processor.as_mut(), 2, RATE, false, &[]);
  drop(processor);
  chow.set_param(chow.params()[0].id, 0.5);
  chow.idle();
  chow.state()?;
  drop(chow);
  Ok(Outcome::Pass(String::new()))
}

fn processor_dropped_after_plugin_on_another_thread() -> Result<Outcome, String> {
  let Some(mut chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  let mut processor = chow.take_processor().unwrap();
  run_blocks(processor.as_mut(), 2, RATE, false, &[]);
  drop(chow);
  // the plugin's gone, so the processor passes its input through
  let out = run_blocks(processor.as_mut(), 2, RATE, false, &[]);
  let input = sine(out.0.len(), 0, RATE);
  ensure(out.0 == input, "a processor without its plugin didn't pass the input through")?;
  std::thread::spawn(move || drop(processor)).join().map_err(|_| "dropping it panicked")?;
  Ok(Outcome::Pass("passes through once its plugin is gone".into()))
}

#[cfg(target_os = "macos")]
fn editor_opens_in_a_window() -> Result<Outcome, String> {
  use objc2::{MainThreadMarker, MainThreadOnly};
  use objc2_app_kit::{NSApplication, NSBackingStoreType, NSWindow, NSWindowStyleMask};
  use objc2_foundation::{NSPoint, NSRect, NSSize};

  let Some(mtm) = MainThreadMarker::new() else {
    return Ok(Outcome::Skip("needs the main thread".into()));
  };
  let _app = NSApplication::sharedApplication(mtm);
  let mut lines = vec![];
  for name in ["CHOWTapeModel.vst3", "PanCake 2.vst3", "nyasynth.vst3"] {
    let mut plugin = match plugin_in(name) {
      Ok(Some((path, info))) => Plugin::load(&path, &info.id, RATE)?,
      Ok(None) => continue,
      Err(e) => {
        lines.push(format!("\n          skipped {}", e));
        continue;
      }
    };
    ensure(plugin.has_editor(), format!("{} has no editor", name))?;
    let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(800., 600.));
    // safe: made on the main thread, and kept until it's closed below
    let window = unsafe {
      NSWindow::initWithContentRect_styleMask_backing_defer(
        NSWindow::alloc(mtm),
        rect,
        NSWindowStyleMask::Titled,
        NSBackingStoreType::Buffered,
        false,
      )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    let view = window.contentView().ok_or("the window has no content view")?;
    let parent = objc2::rc::Retained::as_ptr(&view) as *mut std::ffi::c_void;

    let resized = std::sync::Arc::new(std::sync::Mutex::new(None));
    let seen = resized.clone();
    plugin.on_editor_resize(Box::new(move |w, h| *seen.lock().unwrap() = Some((w, h))));
    let size = plugin.open_editor(parent)?;
    ensure(size.0 > 0 && size.1 > 0, format!("{} opened at {:?}", name, size))?;
    ensure(plugin.editor_size() == Some(size), format!("{} reports another size", name))?;
    let subviews = view.subviews().len();
    let can_resize = plugin.editor_can_resize();
    let took = plugin.resize_editor(size.0 + 40, size.1 + 30);
    plugin.close_editor();
    ensure(plugin.editor_size().is_none(), "the editor is still open")?;
    window.close();
    lines.push(format!(
      "\n          {}: {:?}, {} subview(s), resizable {}, asked +40x30 took {:?}, plugin resized to {:?}",
      name,
      size,
      subviews,
      can_resize,
      took,
      resized.lock().unwrap()
    ));
  }
  if lines.is_empty() {
    return not_installed("CHOW, PanCake 2 or nyasynth");
  }
  Ok(Outcome::Pass(lines.concat()))
}

#[cfg(not(target_os = "macos"))]
fn editor_opens_in_a_window() -> Result<Outcome, String> {
  Ok(Outcome::Skip("only checked on macOS".into()))
}

// windows on screen for plugin::window's checks. set PLUGIN_SHOTS to a
// folder to get a screenshot of each stage (from the screen if the terminal
// may record it, otherwise drawn by the window's views)
#[cfg(target_os = "macos")]
mod screen {
  use objc2::rc::Retained;
  use objc2::{MainThreadMarker, MainThreadOnly};
  use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBitmapImageFileType, NSEventMask, NSView, NSWindow,
    NSWindowStyleMask,
  };
  use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSDictionary, NSPoint, NSRect, NSSize, NSString};
  use std::ffi::c_void;

  // the app, in front: floating plugin windows hide while it isn't
  pub fn app(mtm: MainThreadMarker) -> Retained<NSApplication> {
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    static LAUNCHED: std::sync::Once = std::sync::Once::new();
    LAUNCHED.call_once(|| app.finishLaunching());
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    app
  }

  // handle the app's events (and let it draw) for a while
  pub fn run(app: &NSApplication, seconds: f64) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds);
    while std::time::Instant::now() < until {
      let soon = NSDate::dateWithTimeIntervalSinceNow(0.01);
      // safe: reading an AppKit constant
      let mode = unsafe { NSDefaultRunLoopMode };
      while let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(NSEventMask::Any, Some(&soon), mode, true) {
        app.sendEvent(&event);
      }
    }
  }

  // a window like the app's, at the top left of the screen
  pub fn host_window(mtm: MainThreadMarker, title: &str, width: f64, height: f64) -> Retained<NSWindow> {
    let rect = NSRect::new(NSPoint::new(60., 200.), NSSize::new(width, height));
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable;
    // safe: made on the main thread, and not released when closed (the
    // caller holds it)
    let window = unsafe {
      let window =
        NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm), rect, style, NSBackingStoreType::Buffered, false);
      window.setReleasedWhenClosed(false);
      window
    };
    window.setTitle(&NSString::from_str(title));
    window.makeKeyAndOrderFront(None);
    window
  }

  // like host_window, but off the screen, so the user doesn't see it (and
  // borderless, so AppKit doesn't move it on, or shrink it to fit)
  pub fn hidden_host_window(mtm: MainThreadMarker, width: f64, height: f64) -> Retained<NSWindow> {
    let rect = NSRect::new(NSPoint::new(-20000., -20000.), NSSize::new(width, height));
    // safe: as host_window
    let window = unsafe {
      let window = NSWindow::initWithContentRect_styleMask_backing_defer(
        NSWindow::alloc(mtm),
        rect,
        NSWindowStyleMask::Borderless,
        NSBackingStoreType::Buffered,
        false,
      );
      window.setReleasedWhenClosed(false);
      window
    };
    window.orderFront(None);
    window
  }

  // the window as its views draw it (title bar and all), and the colour at
  // (x, y) points from the content view's top left in it, as 0-255 samples
  pub fn drawn_pixels(window: &NSWindow) -> Option<impl Fn(f64, f64) -> Vec<usize>> {
    // safe: the content view's superview is live while the window is
    let frame = unsafe { window.contentView()?.superview() }?;
    let bitmap = frame.bitmapImageRepForCachingDisplayInRect(frame.bounds())?;
    frame.cacheDisplayInRect_toBitmapImageRep(frame.bounds(), &bitmap);
    let bounds = frame.bounds().size;
    let content = window.contentRectForFrameRect(window.frame()).size;
    let scale = bitmap.pixelsWide() as f64 / bounds.width;
    let title = bounds.height - content.height;
    Some(move |x: f64, y: f64| {
      let mut samples = [0usize; 8];
      let (px, py) = ((x * scale) as isize, ((y + title) * scale) as isize);
      // safe: room for every sample of a pixel, at a point inside the image
      unsafe { bitmap.getPixel_atX_y(std::ptr::NonNull::from(&mut samples).cast(), px, py) };
      samples[..bitmap.samplesPerPixel() as usize].to_vec()
    })
  }

  pub fn ptr<T>(object: &T) -> *mut c_void {
    object as *const T as *mut c_void
  }

  // safe to call with what EditorWindow::ns_window gives while it's open
  pub fn window_at(ptr: *mut c_void) -> &'static NSWindow {
    // safe: see above
    unsafe { &*(ptr as *const NSWindow) }
  }

  pub fn content_size(window: &NSWindow) -> (u32, u32) {
    let size = window.contentRectForFrameRect(window.frame()).size;
    (size.width.round() as u32, size.height.round() as u32)
  }

  pub fn frame_of(view: &NSView) -> (f64, f64, f64, f64) {
    let f = view.frame();
    (f.origin.x, f.origin.y, f.size.width, f.size.height)
  }

  // a screenshot of a window, if PLUGIN_SHOTS names a folder: where it went
  pub fn shot(window: &NSWindow, name: &str) -> String {
    let Ok(folder) = std::env::var("PLUGIN_SHOTS") else { return String::new() };
    let file = std::path::Path::new(&folder).join(format!("{}.png", name));
    let id = format!("-l{}", window.windowNumber());
    let captured = std::process::Command::new("screencapture")
      .args(["-x", "-o", &id])
      .arg(&file)
      .stderr(std::process::Stdio::null())
      .status()
      .map_or(false, |s| s.success() && file.exists());
    if captured {
      return format!(" [{}]", file.display());
    }
    // the window's views drawn into an image: the title bar and all (from
    // the content view's superview, which is safe to read: it's live)
    let drawn = window.contentView().and_then(|c| unsafe { c.superview() }).and_then(|frame| {
      let bitmap = frame.bitmapImageRepForCachingDisplayInRect(frame.bounds())?;
      frame.cacheDisplayInRect_toBitmapImageRep(frame.bounds(), &bitmap);
      // safe: an empty dictionary is a valid set of properties
      let png = unsafe { bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
      png.writeToFile_atomically(&NSString::from_str(&file.to_string_lossy()), true).then_some(())
    });
    match drawn {
      Some(()) => format!(" [drawn: {}]", file.display()),
      None => format!(" [no screenshot of {}]", name),
    }
  }
}

// CHOW and PanCake 2 in floating windows at once: sized to their editors,
// resized by the user (CHOW), kept on screen, closed by their close buttons
// and by the app
#[cfg(target_os = "macos")]
fn editors_float_in_their_own_windows() -> Result<Outcome, String> {
  use objc2::{msg_send, MainThreadMarker};
  use objc2_foundation::NSSize;
  use plugin::window;
  use screen::{content_size, ptr, run, shot, window_at};
  use std::cell::{Cell, RefCell};
  use std::rc::Rc;

  let Some(mtm) = MainThreadMarker::new() else { return Ok(Outcome::Skip("needs the main thread".into())) };
  let Some(chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  let Some(pancake) = load("PanCake 2.vst3", RATE)? else { return not_installed("PanCake 2") };
  let app = screen::app(mtm);
  let host = screen::host_window(mtm, "dawesome (host)", 900., 700.);
  let chow = Rc::new(RefCell::new(chow));
  let pancake = Rc::new(RefCell::new(pancake));
  // how the app reaches a plugin from its window
  let reach = |plugin: &Rc<RefCell<Plugin>>| -> window::WithPlugin {
    let plugin = plugin.clone();
    Box::new(move |f| {
      if let Ok(mut p) = plugin.try_borrow_mut() {
        f(&mut p)
      }
    })
  };
  let mut lines = vec![];

  let chow_closed = Rc::new(Cell::new(false));
  let seen = chow_closed.clone();
  let chow_window = window::open_window(
    &mut chow.borrow_mut(),
    "CHOW Tape Model",
    ptr(&*host),
    None,
    reach(&chow),
    Box::new(move || seen.set(true)),
  )?;
  run(&app, 1.5);
  let panel = window_at(chow_window.ns_window());
  let size = chow.borrow().editor_size().ok_or("CHOW's editor isn't open")?;
  ensure(panel.isVisible(), "CHOW's window isn't visible")?;
  ensure(content_size(panel) == size, format!("CHOW's window is {:?}, its editor {:?}", content_size(panel), size))?;
  ensure(!host.isKeyWindow() || !panel.isKeyWindow(), "both windows are key")?;
  let host_frame = host.frame();
  let (x, y) = chow_window.position();
  let centre = (x + panel.frame().size.width / 2., y - panel.frame().size.height / 2.);
  let host_centre = (host_frame.origin.x + host_frame.size.width / 2., host_frame.origin.y + host_frame.size.height / 2.);
  ensure(
    (centre.0 - host_centre.0).abs() < 2. && (centre.1 - host_centre.1).abs() < 2.,
    format!("CHOW's window isn't centred over the host: {:?} vs {:?}", centre, host_centre),
  )?;
  lines.push(format!("\n          CHOW opened at {:?}, top-left {:?}{}", size, (x, y), shot(panel, "1-chow-window")));

  // the user drags its corner: the delegate is asked, as AppKit asks it
  let resizable = chow.borrow().editor_can_resize();
  let delegate = panel.delegate().ok_or("CHOW's window has no delegate")?;
  let wanted = panel.frameRectForContentRect(objc2_foundation::NSRect::new(
    objc2_foundation::NSPoint::ZERO,
    NSSize::new(size.0 as f64 + 137., size.1 as f64 + 61.),
  ));
  // safe: the delegate's windowWillResize:toSize:, with its signature
  let snapped: NSSize = unsafe { msg_send![&*delegate, windowWillResize: panel, toSize: wanted.size] };
  let mut frame = panel.frame();
  frame.size = snapped;
  panel.setFrame_display(frame, true);
  run(&app, 1.);
  let resized = chow.borrow().editor_size().ok_or("CHOW's editor closed")?;
  ensure(content_size(panel) == resized, format!("CHOW's window is {:?}, its editor {:?}", content_size(panel), resized))?;
  ensure(!resizable || resized != size, "CHOW is resizable but didn't take a new size")?;
  let content = panel.contentView().ok_or("CHOW's window has no content view")?;
  let editor_view = content.subviews().firstObject().ok_or("CHOW's view is gone")?;
  let filled = (0., 0., resized.0 as f64, resized.1 as f64);
  ensure(
    screen::frame_of(&editor_view) == filled,
    format!("CHOW's view is at {:?}, not {:?}", screen::frame_of(&editor_view), filled),
  )?;
  lines.push(format!(
    "\n          CHOW resizable {}, asked {:?}, took {:?}{}",
    resizable,
    (size.0 + 137, size.1 + 61),
    resized,
    shot(panel, "2-chow-resized")
  ));

  // a second window, asked to open far off screen
  let pancake_window =
    window::open_window(&mut pancake.borrow_mut(), "PanCake 2", ptr(&*host), Some((-9000., 9000.)), reach(&pancake), Box::new(|| {}))?;
  run(&app, 1.5);
  let pancake_panel = window_at(pancake_window.ns_window());
  let pancake_size = pancake.borrow().editor_size().ok_or("PanCake's editor isn't open")?;
  ensure(pancake_panel.isVisible() && panel.isVisible(), "both windows aren't visible")?;
  ensure(content_size(pancake_panel) == pancake_size, format!("PanCake's window is {:?}", content_size(pancake_panel)))?;
  let screen = pancake_panel.screen().ok_or("PanCake's window isn't on a screen")?.visibleFrame();
  let (px, py) = pancake_window.position();
  ensure(
    px >= screen.origin.x && py <= screen.origin.y + screen.size.height + 0.5,
    format!("PanCake's window was left at {:?}, off {:?}", (px, py), screen),
  )?;
  pancake_window.focus();
  chow_window.focus();
  run(&app, 0.5);
  lines.push(format!(
    "\n          PanCake opened at {:?}, moved on screen to {:?}{}{}",
    pancake_size,
    (px, py),
    shot(pancake_panel, "3-pancake-window"),
    shot(panel, "3-chow-beside-pancake")
  ));

  // CHOW's close button: its editor is closed and the caller told
  panel.performClose(None);
  run(&app, 0.3);
  ensure(chow_closed.get(), "on_close didn't run")?;
  ensure(chow.borrow().editor_size().is_none(), "CHOW's editor is still open")?;
  ensure(!panel.isVisible(), "CHOW's window is still visible")?;
  let remembered = chow_window.position();
  chow_window.close(&mut chow.borrow_mut());
  // and it opens again where it was
  let again = window::open_window(&mut chow.borrow_mut(), "CHOW Tape Model", ptr(&*host), Some(remembered), reach(&chow), Box::new(|| {}))?;
  run(&app, 0.5);
  let back = again.position();
  ensure(
    (back.0 - remembered.0).abs() < 1. && (back.1 - remembered.1).abs() < 1.,
    format!("CHOW reopened at {:?}, not {:?}", back, remembered),
  )?;
  again.close(&mut chow.borrow_mut());

  // PanCake closed by the app
  pancake_window.close(&mut pancake.borrow_mut());
  run(&app, 0.3);
  ensure(pancake.borrow().editor_size().is_none(), "PanCake's editor is still open")?;
  host.close();
  lines.push("\n          closed by its button (on_close ran), reopened where it was, and closed by the app".into());
  Ok(Outcome::Pass(lines.concat()))
}

#[cfg(not(target_os = "macos"))]
fn editors_float_in_their_own_windows() -> Result<Outcome, String> {
  Ok(Outcome::Skip("only checked on macOS".into()))
}

// PanCake 2 docked over a stand-in for the webview, clipped to a pane
// smaller than it
#[cfg(target_os = "macos")]
fn pancake_docks_over_the_page() -> Result<Outcome, String> {
  use objc2::rc::Retained;
  use objc2::{MainThreadMarker, MainThreadOnly};
  use objc2_app_kit::NSView;
  use objc2_foundation::{NSPoint, NSRect, NSSize};
  use plugin::window::{self, Rect};
  use screen::{frame_of, ptr, run, shot};
  use std::ffi::c_void;

  let Some(mtm) = MainThreadMarker::new() else { return Ok(Outcome::Skip("needs the main thread".into())) };
  let Some(mut pancake) = load("PanCake 2.vst3", RATE)? else { return not_installed("PanCake 2") };
  let app = screen::app(mtm);
  let host = screen::host_window(mtm, "dawesome (docked)", 900., 700.);
  let content = host.contentView().ok_or("no content view")?;
  // the webview: a view inset in the content view, as wry's may be
  let page = NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::new(20., 10.), NSSize::new(860., 650.)));
  content.addSubview(&page);
  let mut lines = vec![];

  // the pane is 300x400 at (100, 50) on the page, and the plugin is scrolled
  // 20 up and 20 left inside it
  let clip = Rect { x: 100., y: 50., width: 300., height: 400. };
  let dock = window::dock(&mut pancake, ptr(&*page), clip, 80., 30., None)?;
  run(&app, 1.5);
  let size = pancake.editor_size().ok_or("PanCake's editor isn't open")?;
  ensure(dock.size() == size, format!("the dock says {:?}, the editor {:?}", dock.size(), size))?;
  let subviews = content.subviews();
  ensure(subviews.len() == 2, format!("the content view has {} subviews", subviews.len()))?;
  let clip_view: Retained<NSView> = subviews.objectAtIndex(1);
  // the page's (100, 50) from its top is (120, 10 + 650 - 50 - 400) up from
  // the content view's bottom
  ensure(frame_of(&clip_view) == (120., 210., 300., 400.), format!("the clip view is at {:?}", frame_of(&clip_view)))?;
  let holder = clip_view.subviews().objectAtIndex(0);
  let expected = (-20., -20., size.0 as f64, size.1 as f64);
  ensure(frame_of(&holder) == expected, format!("the plugin's view is at {:?}, not {:?}", frame_of(&holder), expected))?;
  ensure(holder.subviews().len() > 0, "the plugin added no view")?;
  // clicks: inside the pane to the plugin, outside it to the page
  let inside = content.hitTest(NSPoint::new(200., 300.)).ok_or("nothing under the pane")?;
  ensure(inside.isDescendantOf(&holder) && *inside != *holder, "a click in the pane doesn't reach the plugin")?;
  let outside = content.hitTest(NSPoint::new(600., 300.)).ok_or("nothing beside the pane")?;
  ensure(*outside == *page, "a click beside the pane doesn't reach the page")?;
  lines.push(format!("\n          docked at {:?}, clipped to {:?}{}", size, clip, shot(&host, "4-pancake-docked")));

  // the page scrolls and the pane grows
  let clip = Rect { x: 40., y: 20., width: 600., height: 640. };
  dock.place(clip, 40., 20.);
  run(&app, 1.);
  // (only as big as the plugin: the page shows around it)
  let shown = (60., 10. + 650. - 20. - size.1 as f64, size.0 as f64, size.1 as f64);
  ensure(frame_of(&clip_view) == shown, format!("the clip view moved to {:?}, not {:?}", frame_of(&clip_view), shown))?;
  ensure(frame_of(&holder).0 == 0. && frame_of(&holder).1 == 0., format!("the plugin's view moved to {:?}", frame_of(&holder)))?;
  lines.push(format!("\n          placed again at {:?}{}", clip, shot(&host, "5-pancake-placed")));

  // the plugin has the keyboard when it's undocked: the page gets it back
  let plugin_view = holder.subviews().objectAtIndex(0);
  let focused = host.makeFirstResponder(Some(&plugin_view));
  dock.undock(&mut pancake);
  run(&app, 0.3);
  ensure(pancake.editor_size().is_none(), "PanCake's editor is still open")?;
  ensure(content.subviews().len() == 1, "the clip view is still there")?;
  // safe: reading a live view's superview
  ensure(unsafe { clip_view.superview() }.is_none(), "the clip view is still in a window")?;
  let responder = host.firstResponder().map(|r| Retained::as_ptr(&r) as *const c_void);
  ensure(!focused || responder == Some(Retained::as_ptr(&page) as *const c_void), "the page didn't get the keyboard back")?;

  // the content view itself, as Tauri's ns_view() may give it
  let dock = window::dock(&mut pancake, ptr(&*content), Rect { x: 0., y: 0., width: 200., height: 200. }, 0., 0., None)?;
  run(&app, 0.5);
  let clip_view: Retained<NSView> = content.subviews().objectAtIndex(1);
  ensure(frame_of(&clip_view) == (0., 500., 200., 200.), format!("in the content view it's at {:?}", frame_of(&clip_view)))?;
  lines.push(format!("\n          docked in the content view at the top left{}", shot(&host, "6-pancake-in-content")));
  dock.undock(&mut pancake);
  host.close();
  lines.push(format!("\n          undocked (it had the keyboard: {})", focused));
  Ok(Outcome::Pass(lines.concat()))
}

#[cfg(not(target_os = "macos"))]
fn pancake_docks_over_the_page() -> Result<Outcome, String> {
  Ok(Outcome::Skip("only checked on macOS".into()))
}

// CHOW (resizable) and PanCake 2 (fixed) docked at once, each in a pane much
// bigger than it, in a window off the screen: CHOW fills its pane, PanCake
// sits centred in its with the page showing around it (not black), and
// clicks around it reach the page
#[cfg(target_os = "macos")]
fn plugins_dock_side_by_side() -> Result<Outcome, String> {
  use objc2::rc::Retained;
  use objc2::{MainThreadMarker, MainThreadOnly};
  use objc2_app_kit::NSView;
  use objc2_foundation::{NSPoint, NSRect, NSSize};
  use plugin::window::{self, Rect};
  use screen::{drawn_pixels, frame_of, ptr, run, shot};

  let Some(mtm) = MainThreadMarker::new() else { return Ok(Outcome::Skip("needs the main thread".into())) };
  let Some(mut chow) = load("CHOWTapeModel.vst3", RATE)? else { return not_installed("CHOW Tape Model") };
  let Some(mut pancake) = load("PanCake 2.vst3", RATE)? else { return not_installed("PanCake 2") };
  let app = screen::app(mtm);
  let host = screen::hidden_host_window(mtm, 1700., 900.);
  let content = host.contentView().ok_or("no content view")?;
  let page = NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::ZERO, NSSize::new(1700., 900.)));
  content.addSubview(&page);
  let mut lines = vec![];

  // two panes of 800x840, side by side
  let left = Rect { x: 0., y: 0., width: 800., height: 840. };
  let right = Rect { x: 820., y: 0., width: 800., height: 840. };
  let chow_dock = window::dock(&mut chow, ptr(&*page), left, 0., 0., Some((800, 840)))?;
  let pancake_dock = window::dock(&mut pancake, ptr(&*page), right, 820., 0., Some((800, 840)))?;
  let chow_size = chow.editor_size().ok_or("CHOW's editor isn't open")?;
  let pancake_size = pancake.editor_size().ok_or("PanCake's editor isn't open")?;
  ensure(chow_dock.size() == chow_size, format!("CHOW's dock says {:?}, its editor {:?}", chow_dock.size(), chow_size))?;
  ensure(pancake_dock.size() == pancake_size, format!("PanCake's dock says {:?}", pancake_dock.size()))?;
  ensure(chow.editor_can_resize() && !pancake.editor_can_resize(), "CHOW should resize and PanCake not")?;
  // CHOW took about the room it had (snapped), PanCake kept its own size
  ensure(
    chow_size.0 <= 800 && chow_size.1 <= 840 && (chow_size.0 > 700 || chow_size.1 > 740),
    format!("CHOW didn't fill its pane: {:?}", chow_size),
  )?;
  ensure(pancake_size.0 < 760 && pancake_size.1 < 800, format!("PanCake is too big to check around: {:?}", pancake_size))?;
  // PanCake centred in its pane, as the page would ask
  let (pw, ph) = (pancake_size.0 as f64, pancake_size.1 as f64);
  let (px, py) = (820. + (800. - pw) / 2., (840. - ph) / 2.);
  pancake_dock.place(right, px, py);
  run(&app, 1.5);
  ensure(content.subviews().len() == 3, format!("the content view has {} subviews", content.subviews().len()))?;
  let pancake_clip: Retained<NSView> = content.subviews().objectAtIndex(2);
  let holder = pancake_clip.subviews().objectAtIndex(0);
  // the clip view covers PanCake and nothing more
  let expected = (820. + (800. - pw) / 2., 900. - (840. - ph) / 2. - ph, pw, ph);
  ensure(frame_of(&pancake_clip) == expected, format!("PanCake's clip view is at {:?}, not {:?}", frame_of(&pancake_clip), expected))?;
  ensure(frame_of(&holder) == (0., 0., pw, ph), format!("PanCake's view is at {:?}", frame_of(&holder)))?;

  // clicks (content view points, y up): beside PanCake in its pane, the page;
  // on PanCake and on CHOW, the plugins
  let up = |y: f64| 900. - y;
  let beside = content.hitTest(NSPoint::new(825., up(10.))).ok_or("nothing beside PanCake")?;
  ensure(*beside == *page, "a click beside PanCake in its pane doesn't reach the page")?;
  let on_pancake = content.hitTest(NSPoint::new(px + pw / 2., up(py + ph / 2.))).ok_or("nothing on PanCake")?;
  ensure(on_pancake.isDescendantOf(&holder) && *on_pancake != *holder, "a click on PanCake doesn't reach it")?;
  let on_chow = content.hitTest(NSPoint::new(chow_size.0 as f64 / 2., up(chow_size.1 as f64 / 2.))).ok_or("nothing on CHOW")?;
  ensure(!on_chow.isDescendantOf(&page) && *on_chow != *page, "a click on CHOW doesn't reach it")?;

  // drawn: around PanCake is what's drawn where no pane is (the window's
  // background through the page), not black
  let pixel = drawn_pixels(&host).ok_or("the window couldn't be drawn")?;
  let background = pixel(1680., 890.);
  let around = [(825., 10.), (px - 5., py + ph / 2.), (px + pw + 5., py + ph / 2.), (px + pw / 2., py + ph + 5.)];
  for (x, y) in around {
    let seen = pixel(x, y);
    ensure(seen == background, format!("beside PanCake at {:?} is {:?}, where the page is {:?}", (x, y), seen, background))?;
  }
  let inside = pixel(px + pw / 2., py + ph / 2.);
  lines.push(format!(
    "\n          CHOW filled {:?} of 800x840, PanCake {:?} centred; around it {:?} as the page, inside {:?}{}",
    chow_size,
    pancake_size,
    background,
    inside,
    shot(&host, "7-two-docked")
  ));

  // the pane shrinks: CHOW takes the new room
  let taken = chow_dock.fill(&mut chow, 420, 380);
  run(&app, 0.5);
  let chow_clip: Retained<NSView> = content.subviews().objectAtIndex(1);
  let chow_holder = chow_clip.subviews().objectAtIndex(0);
  ensure(taken == chow.editor_size().unwrap_or_default(), format!("CHOW took {:?}, its editor says otherwise", taken))?;
  ensure(taken.0 <= 420 && taken.1 <= 380 && taken != chow_size, format!("CHOW didn't shrink: {:?}", taken))?;
  ensure(
    frame_of(&chow_holder) == (0., 0., taken.0 as f64, taken.1 as f64),
    format!("CHOW's view is at {:?}", frame_of(&chow_holder)),
  )?;
  // a fixed plugin keeps its size
  ensure(pancake_dock.fill(&mut pancake, 300, 300) == pancake_size, "PanCake changed size")?;
  lines.push(format!("\n          CHOW shrunk to {:?}{}", taken, shot(&host, "8-chow-shrunk")));

  // one undocked, the other stays
  pancake_dock.undock(&mut pancake);
  ensure(content.subviews().len() == 2 && chow.editor_size().is_some(), "undocking PanCake disturbed CHOW")?;
  chow_dock.undock(&mut chow);
  ensure(content.subviews().len() == 1, "a clip view is still there")?;
  host.close();
  Ok(Outcome::Pass(lines.concat()))
}

#[cfg(not(target_os = "macos"))]
fn plugins_dock_side_by_side() -> Result<Outcome, String> {
  Ok(Outcome::Skip("only checked on macOS".into()))
}


// an editor reopened as the app does it: in a floating window, then docked,
// then in a window again, with the run loop turning in between (DecentSampler
// crashed the app a moment after its second opening). CHOW, Addictive Keys,
// Addictive Drums 2 and DecentSampler, those installed
//
// DecentSampler is skipped while its update check is on: when its editor
// opens, its own timer starts the check, and its loading thread writes the
// version into a text box of the editor directly, while the main thread's
// JUCE message for that box's Value writes it too. no host call is involved,
// and either thread can crash (2026-11-11: about every run here, with the
// check on; none in nine with it off)
#[cfg(target_os = "macos")]
fn editors_reopen_in_windows_and_docks() -> Result<Outcome, String> {
  use objc2::{MainThreadMarker, MainThreadOnly};
  use objc2_app_kit::NSView;
  use objc2_foundation::{NSPoint, NSRect, NSSize};
  use plugin::window::{self, Rect};
  use screen::{ptr, run};
  use std::cell::RefCell;
  use std::rc::Rc;

  let Some(mtm) = MainThreadMarker::new() else { return Ok(Outcome::Skip("needs the main thread".into())) };
  let app = screen::app(mtm);
  let host = screen::hidden_host_window(mtm, 1600., 1000.);
  let content = host.contentView().ok_or("no content view")?;
  let page = NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::ZERO, NSSize::new(1600., 1000.)));
  content.addSubview(&page);
  let mut lines = vec![];
  for name in ["CHOWTapeModel.vst3", "Addictive Keys.vst3", "Addictive Drums 2.vst3", "DecentSampler.vst3"] {
    if name == "DecentSampler.vst3" && decent_sampler_checks_for_updates() {
      lines.push(format!("\n          skipped {}: its update check is on (see above)", name));
      continue;
    }
    let plugin = match load(name, RATE) {
      Ok(Some(p)) => Rc::new(RefCell::new(p)),
      Ok(None) => continue,
      Err(e) => {
        lines.push(format!("\n          skipped {}: {}", name, e));
        continue;
      }
    };
    let mut processor = plugin.borrow_mut().take_processor().ok_or("no processor")?;
    // the app plays it and calls idle while the run loop turns
    let mut turn = |seconds: f64| {
      let started = Instant::now();
      while started.elapsed().as_secs_f64() < seconds {
        run_blocks(processor.as_mut(), 1, RATE, true, &[]);
        plugin.borrow_mut().idle();
        run(&app, 0.01);
      }
    };
    turn(2.);
    let reach = {
      let plugin = plugin.clone();
      move || -> window::WithPlugin {
        let plugin = plugin.clone();
        Box::new(move |f| {
          if let Ok(mut p) = plugin.try_borrow_mut() {
            f(&mut p)
          }
        })
      }
    };
    let open = |title: &str| {
      window::open_window(&mut plugin.borrow_mut(), title, ptr(&*host), None, reach(), Box::new(|| {}))
    };
    let first = open(name)?;
    turn(1.5);
    first.close(&mut plugin.borrow_mut());
    let pane = Rect { x: 0., y: 0., width: 1200., height: 900. };
    let dock = window::dock(&mut plugin.borrow_mut(), ptr(&*page), pane, 0., 0., Some((1200, 900)))?;
    turn(1.5);
    ensure(plugin.borrow().editor_open(), format!("{}'s docked editor closed", name))?;
    dock.undock(&mut plugin.borrow_mut());
    let again = open(name)?;
    turn(5.);
    ensure(plugin.borrow().editor_open(), format!("{}'s editor closed", name))?;
    again.close(&mut plugin.borrow_mut());
    turn(1.);
    lines.push(format!("\n          {}: window, dock, window", name));
  }
  host.close();
  if lines.is_empty() {
    return not_installed("CHOW, Addictive Keys, Addictive Drums 2 or DecentSampler");
  }
  Ok(Outcome::Pass(lines.concat()))
}

#[cfg(not(target_os = "macos"))]
fn editors_reopen_in_windows_and_docks() -> Result<Outcome, String> {
  Ok(Outcome::Skip("only checked on macOS".into()))
}

// whether DecentSampler will check for its updates when its editor opens:
// its own preference, unless it's been turned off (it's on until then)
fn decent_sampler_checks_for_updates() -> bool {
  let Some(home) = std::env::var_os("HOME") else { return true };
  let prefs = Path::new(&home).join("Music/Audio Music Apps/Decidedly/DecentSampler/DecentSampler.xml");
  let text = std::fs::read_to_string(prefs).unwrap_or_default();
  !text.contains(r#"name="dontQueryForUpdate" val="1""#)
}
