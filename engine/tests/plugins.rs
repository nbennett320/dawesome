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
    ("twenty_instances", twenty_instances),
    ("processor_dropped_before_plugin", processor_dropped_before_plugin),
    ("processor_dropped_after_plugin_on_another_thread", processor_dropped_after_plugin_on_another_thread),
    ("editor_opens_in_a_window", editor_opens_in_a_window),
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
