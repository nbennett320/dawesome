use super::testing::*;
use super::*;

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 1e-5
}

#[test]
fn test_adding_effects_and_the_racks_they_make() {
  let rig = rig();
  let effects = rig.effects();
  assert!(effects.racks().is_empty());

  let a = effects.add(Channel::Track(2), &gain().id, None).unwrap();
  let m = effects.add(Channel::Master, &gain().id, None).unwrap();
  let b = effects.add(Channel::Track(2), &gain().id, Some(0)).unwrap();
  // past the end is the end
  let c = effects.add(Channel::Track(2), &gain().id, Some(99)).unwrap();
  assert_eq!(rig.layout(), vec![(Some(2), vec![b, a, c]), (None, vec![m])]);
  assert_eq!(BTreeSet::from([a, b, c, m]).len(), 4);

  let slot = &effects.racks()[0].slots[1];
  assert_eq!(slot, &EffectSlot {
    id: a,
    plugin_id: gain().id,
    name: "Gain".into(),
    vendor: "Fake Audio".into(),
    bypassed: false,
    missing: false,
    has_editor: false,
  });
  let json = serde_json::to_value(effects.racks()).unwrap();
  assert_eq!(json[1]["track"], serde_json::Value::Null);
  assert_eq!(json[0]["slots"][1]["pluginId"], gain().id);
  assert_eq!(json[0]["slots"][1]["hasEditor"], false);
  assert_eq!(effects.slot_name(a).as_deref(), Some("Gain"));
  assert_eq!(rig.loader.rates.lock().unwrap().clone(), vec![RATE; 4]);
}

#[test]
fn test_adding_what_cant_be_added_changes_nothing() {
  let rig = rig();
  let effects = rig.effects();
  let err = effects.add(Channel::Master, &synth().id, None).unwrap_err();
  assert!(err.contains("instrument"), "{}", err);
  let err = effects.add(Channel::Master, &absent().id, None).unwrap_err();
  assert!(err.starts_with("Absent couldn't be loaded"), "{}", err);
  assert!(effects.add(Channel::Master, "vst3:nothing", None).is_err());
  assert!(effects.racks().is_empty());
  assert!(!effects.has_loaded());
}

#[test]
fn test_ids_are_never_reused() {
  let rig = rig();
  let effects = rig.effects();
  let a = effects.add(Channel::Track(0), &gain().id, None).unwrap();
  effects.remove(a).unwrap();
  let b = effects.add(Channel::Track(0), &gain().id, None).unwrap();
  assert_ne!(a, b);
  assert_eq!(effects.remove(a).unwrap_err(), gone());
}

#[test]
fn test_the_engine_plays_the_chains() {
  let mut rig = rig();
  assert!(close(rig.level(Some(0)), 0.2));

  let a = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  assert!(close(rig.level(Some(0)), 0.1));
  // only its own track
  assert!(close(rig.level(Some(1)), 0.2));

  rig.effects().set_bypassed(a, true).unwrap();
  assert!(close(rig.level(Some(0)), 0.2));
  assert!(rig.effects().racks()[0].slots[0].bypassed);
  rig.effects().set_bypassed(a, false).unwrap();

  let b = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  rig.turn(b, 0.25);
  assert!(close(rig.level(Some(0)), 0.2 * 0.5 * 0.25));

  // to the master: the track has one, and everything goes through it
  rig.effects().move_to(b, Channel::Master, 0).unwrap();
  assert!(close(rig.level(Some(0)), 0.2 * 0.5 * 0.25));
  assert!(close(rig.level(Some(1)), 0.2 * 0.25));
  assert!(close(rig.level(None), 0.2 * 0.25));
  // its settings came with it
  assert!(close(rig.gain_of(b), 0.25));

  // to another track
  rig.effects().move_to(b, Channel::Track(1), 5).unwrap();
  assert!(close(rig.level(Some(0)), 0.1));
  assert!(close(rig.level(Some(1)), 0.05));
  assert!(close(rig.level(None), 0.2));
  assert_eq!(rig.layout(), vec![(Some(0), vec![a]), (Some(1), vec![b])]);

  rig.effects().remove(a).unwrap();
  rig.effects().remove(b).unwrap();
  assert!(close(rig.level(Some(0)), 0.2));
  assert!(close(rig.level(Some(1)), 0.2));
  assert!(rig.effects().racks().is_empty());
  assert_eq!(rig.loader.alive.load(Ordering::SeqCst), 0);
}

#[test]
fn test_the_engine_is_told_which_plugins_have_their_interface_showing() {
  let rig = rig();
  let id = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  let slot = rig.effects().inner.lock().unwrap().racks[&Channel::Track(0)][0].loaded().unwrap().slot.clone();
  assert_eq!(slot.id(), id);

  rig.effects().idle();
  assert!(!slot.showing());
  // (a channel doesn't rest while one of its plugins' interfaces is up)
  rig.loader.editors_open.store(true, Ordering::SeqCst);
  rig.effects().idle();
  assert!(slot.showing());
  rig.loader.editors_open.store(false, Ordering::SeqCst);
  rig.effects().idle();
  assert!(!slot.showing());

  // one that's busy on the main thread (its window being opened, say)
  // counts as showing until the next round
  let hosted = rig.effects().hosted(id).unwrap();
  let busy = hosted.lock().unwrap();
  rig.effects().idle();
  assert!(slot.showing());
  drop(busy);
  rig.effects().idle();
  assert!(!slot.showing());
}

#[test]
fn test_order_within_a_chain() {
  let mut rig = rig();
  let a = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  let b = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  let c = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  rig.effects().move_to(c, Channel::Track(0), 0).unwrap();
  assert_eq!(rig.layout(), vec![(Some(0), vec![c, a, b])]);
  rig.effects().move_to(c, Channel::Track(0), 2).unwrap();
  assert_eq!(rig.layout(), vec![(Some(0), vec![a, b, c])]);
  // every one still plays, once
  assert!(close(rig.level(Some(0)), 0.2 * 0.125));
}

#[test]
fn test_removed_plugins_are_torn_down_and_unload_is_heard() {
  let rig = rig();
  let heard = Arc::new(Mutex::new(Vec::new()));
  let h = heard.clone();
  rig.effects().on_unload(move |id| h.lock().unwrap().push(id));
  let a = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  let b = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  assert!(rig.effects().has_loaded());
  rig.effects().remove(a).unwrap();
  assert_eq!(rig.loader.alive.load(Ordering::SeqCst), 1);
  rig.effects().clear();
  assert_eq!(rig.loader.alive.load(Ordering::SeqCst), 0);
  assert_eq!(*heard.lock().unwrap(), vec![a, b]);
  assert!(!rig.effects().has_loaded());
}

#[test]
fn test_an_unload_callback_can_still_reach_the_plugin() {
  let rig = rig();
  let effects = rig.state.playlist.effects.clone();
  // each id heard, and whether its plugin could be reached then
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (h, racks) = (heard.clone(), Arc::downgrade(&effects));
  effects.on_unload(move |id| {
    let reached = racks.upgrade().map_or(false, |effects| effects.hosted(id).is_some());
    h.lock().unwrap().push((id, reached));
  });

  // removed
  let a = effects.add(Channel::Track(0), &gain().id, None).unwrap();
  effects.remove(a).unwrap();
  // a synth that's left the list
  let s = effects.load_synth(&synth().id).unwrap();
  effects.follow_synths(&[s.clone()], &BTreeSet::new());
  effects.follow_synths(&[], &BTreeSet::new());
  let s = s.id;
  effects.add(Channel::Track(0), &gain().id, None).unwrap();
  // undone away
  let before = effects.snapshot();
  let b = effects.add(Channel::Master, &gain().id, None).unwrap();
  let mut left = effects.snapshot();
  effects.restore(&before, &mut left);
  // cleared
  let t = effects.racks()[0].slots[0].id;
  effects.clear();

  assert_eq!(*heard.lock().unwrap(), vec![(a, true), (s, true), (b, true), (t, true)]);
  // gone once the callbacks have run
  assert!(effects.hosted(a).is_none() && effects.hosted(t).is_none());
  assert!(effects.unloading.lock().unwrap().is_empty());
  assert_eq!(rig.loader.alive.load(Ordering::SeqCst), 0);
}

// an engine whose callback has stalled (its renderer is never run) keeps
// the chains it was sent, so none goes to the next engine until it lets go
#[test]
fn test_chains_wait_for_a_stalled_engine_to_let_go_before_going_to_another() {
  let mut rig = rig();
  // the old engine never takes the chains it's sent (nor empties them)
  let (stalled, stalled_renderer) = daw::Engine::new(RATE);
  let stalled = Arc::new(stalled);
  rig.effects().use_backend(rig.loader.clone(), Arc::new(Fixed(stalled.clone())), Duration::ZERO);
  let a = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  rig.effects().detach();
  assert!(!stalled.wait_for_chains(Duration::ZERO));

  // the new one: the effects pass sound dry, and stay that way through
  // edits, while the old one may still be running them
  rig.effects().use_backend(rig.loader.clone(), Arc::new(Fixed(rig.engine.clone())), Duration::ZERO);
  rig.effects().attach();
  assert!(close(rig.level(Some(0)), 0.2));
  let b = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  assert!(close(rig.level(Some(0)), 0.2));
  assert_eq!(rig.layout(), vec![(Some(0), vec![a, b])]);

  // once it's gone (its stream closed), the next edit sends them
  drop(stalled_renderer);
  rig.effects().move_to(b, Channel::Track(0), 0).unwrap();
  assert!(close(rig.level(Some(0)), 0.2 * 0.25));
}

#[test]
fn test_chains_go_to_a_stalled_engine_once_it_has_caught_up() {
  let mut rig = rig();
  let (stalled, mut stalled_renderer) = daw::Engine::new(RATE);
  rig.effects().use_backend(rig.loader.clone(), Arc::new(Fixed(Arc::new(stalled))), Duration::ZERO);
  rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  rig.effects().detach();
  rig.effects().use_backend(rig.loader.clone(), Arc::new(Fixed(rig.engine.clone())), Duration::ZERO);
  rig.effects().attach();
  assert!(close(rig.level(Some(0)), 0.2));

  // it renders again, letting go of them
  let mut out = vec![0f32; 16];
  stalled_renderer.render(&mut out, Instant::now(), Duration::ZERO);
  rig.effects().attach();
  assert!(close(rig.level(Some(0)), 0.1));
}

// no output could be opened when the effects were added: they're heard as
// soon as one is
#[test]
fn test_effects_reach_an_output_opened_later() {
  struct Later(Mutex<Option<Arc<daw::Engine>>>);

  impl Output for Later {
    fn engine(&self) -> Result<Arc<daw::Engine>, String> {
      self.engine_if_open().ok_or_else(|| "no output yet".into())
    }

    fn engine_if_open(&self) -> Option<Arc<daw::Engine>> {
      self.0.lock().unwrap().clone()
    }
  }

  let mut rig = rig();
  let later = Arc::new(Later(Mutex::new(None)));
  rig.effects().use_backend(rig.loader.clone(), later.clone(), Duration::ZERO);
  rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  assert!(rig.effects().inner.lock().unwrap().engine.is_none());

  // the output opens (as daw::engine() would, telling follow_output)
  *later.0.lock().unwrap() = Some(rig.engine.clone());
  rig.effects().output_opened();
  assert!(close(rig.level(Some(0)), 0.1));
}

#[test]
fn test_a_new_output_rate_reaches_every_plugin() {
  let mut rig = rig();
  let a = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  rig.effects().detach();
  // nothing sent while detached
  assert!(close(rig.level(Some(0)), 0.2));
  let b = rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
  assert!(close(rig.level(Some(0)), 0.2));

  let (engine, renderer) = daw::Engine::new(96_000);
  let engine = Arc::new(engine);
  rig.effects().use_backend(rig.loader.clone(), Arc::new(Fixed(engine.clone())), Duration::ZERO);
  rig.engine = engine;
  rig.renderer = renderer;
  rig.effects().attach();
  assert!(close(rig.level(Some(0)), 0.05));
  assert_eq!(rig.layout(), vec![(Some(0), vec![a, b])]);
  let inner = rig.effects().inner.lock().unwrap();
  for effect in inner.racks.values().flatten() {
    assert_eq!(effect.loaded().unwrap().rate, 96_000);
  }
}

#[test]
fn test_an_instrument_isnt_an_effect_and_one_in_a_saved_rack_is_left_out() {
  let rig = rig();
  let effects = rig.effects();
  let err = effects.add(Channel::Track(1), &synth().id, None).unwrap_err();
  assert!(err.contains("is an instrument"), "{}", err);
  let saved: Vec<SavedRack> = serde_json::from_value(serde_json::json!([
    { "track": 0, "slots": [
      { "pluginId": synth().id, "name": "Synth" },
      { "pluginId": gain().id, "name": "Gain" },
    ] },
  ])).unwrap();
  assert!(effects.load_saved(&saved).is_empty());
  assert_eq!(effects.racks()[0].slots.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Gain"]);
}

#[test]
fn test_window_positions_are_saved_and_follow_their_effects_into_a_loaded_project() {
  let rig = rig();
  let effects = rig.effects();
  let a = effects.add(Channel::Track(0), &gain().id, None).unwrap();
  let b = effects.add(Channel::Track(0), &gain().id, None).unwrap();
  let at = WindowPosition { x: -40.5, y: 900. };
  effects.keep_window_position(b, at);

  // only the effect whose window has been open has one, and the other is
  // saved as before
  let saved = effects.saved();
  assert_eq!(saved[0].slots[0].window, None);
  assert_eq!(saved[0].slots[1].window, Some(at));
  let json = serde_json::to_value(&saved).unwrap();
  assert!(json[0]["slots"][0].get("window").is_none());
  assert_eq!(json[0]["slots"][1]["window"], serde_json::json!({ "x": -40.5, "y": 900. }));

  // loaded again, the effects have new ids: the position is under b's
  effects.load_saved(&saved);
  let ids: Vec<u64> = effects.racks()[0].slots.iter().map(|s| s.id).collect();
  assert!(!ids.contains(&a) && !ids.contains(&b));
  assert_eq!(effects.window_position(ids[0]), None);
  assert_eq!(effects.window_position(ids[1]), Some(at));
  assert_eq!(effects.window_position(b), None);

  // removing it and undoing that keeps its place
  let before = effects.snapshot();
  effects.remove(ids[1]).unwrap();
  let mut left = effects.snapshot();
  effects.restore(&before, &mut left);
  assert_eq!(effects.saved()[0].slots[1].window, Some(at));

  // a project from before windows were saved loads without any
  let old: Vec<SavedRack> = serde_json::from_value(serde_json::json!([
    { "track": 0, "slots": [{ "pluginId": gain().id, "name": "Gain" }] },
  ])).unwrap();
  effects.load_saved(&old);
  assert_eq!(effects.window_position(effects.racks()[0].slots[0].id), None);
}

// a real plugin through the racks: CHOW Tape Model on a track, heard,

// a real reverb on a track: its tail rings out after the sound stops,
// then its channel rests, and sound wakes it. (it needs the plugin
// installed, and takes about fifteen seconds)
//   cargo test --no-default-features a_real_reverb -- --ignored
#[test]
#[ignore]
fn test_a_real_reverb_rings_out_then_its_channel_rests() {
  let bundle = Path::new("/Library/Audio/Plug-Ins/VST3/Acon Digital/VerberateBasic2.vst3");
  if !bundle.exists() {
    println!("Verberate Basic 2 isn't installed");
    return;
  }
  let mut info = plugin::vst3::scan_bundle(bundle).unwrap().remove(0);
  info.path = bundle.to_string_lossy().into_owned();

  let mut rig = rig();
  rig.effects().use_backend(Arc::new(Vst3), Arc::new(Fixed(rig.engine.clone())), WAIT);
  rig.effects().plugins.set(plugin::scan::Catalog { plugins: vec![info.clone()], ..Default::default() });
  // the renderer runs on a thread of its own, about as fast as an
  // output's would, noting each block's loudest sample
  let mut renderer = std::mem::replace(&mut rig.renderer, daw::Engine::new(RATE).1);
  let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
  let peaks: Arc<Mutex<Vec<f32>>> = Arc::default();
  const BLOCK: usize = 480;
  let render = {
    let (running, peaks) = (running.clone(), peaks.clone());
    std::thread::spawn(move || {
      let mut block = vec![0f32; BLOCK * 2];
      while running.load(Ordering::SeqCst) {
        block.fill(0.);
        renderer.render(&mut block, std::time::Instant::now(), Duration::ZERO);
        peaks.lock().unwrap().push(block.iter().fold(0f32, |peak, s| peak.max(s.abs())));
        std::thread::sleep(Duration::from_millis(10));
      }
    })
  };
  let blocks = |seconds: f32| (seconds * RATE as f32 / BLOCK as f32) as usize;
  // a tenth of a second of a sine on track 0
  let burst = |rig: &Rig, run: u64| {
    use rodio::buffer::SamplesBuffer;
    let sine: Vec<f32> = (0..RATE as usize / 10).flat_map(|i| {
      let s = 0.3 * (i as f32 * 440. * std::f32::consts::TAU / RATE as f32).sin();
      [s, s]
    }).collect();
    let voice = rig.engine.voice(SamplesBuffer::new(2, RATE, sine), Some(run), 0, daw::Route::Track(rig.state.playlist.mixer.track_route(0)));
    rig.engine.start_run(run, rig.state.playlist.mixer.master(), vec![voice]).unwrap();
  };

  rig.effects().add(Channel::Track(0), &info.id, None).unwrap();
  rig.effects().idle();
  burst(&rig, 1);

  // the tail is still there well after the burst is over
  std::thread::sleep(Duration::from_millis(600));
  let heard = peaks.lock().unwrap().clone();
  let late = &heard[heard.len().saturating_sub(blocks(0.2))..];
  assert!(late.iter().any(|&peak| peak > 1e-4), "no tail: {:?}", late);
  assert_eq!(rig.engine.resting_chains(), 0);

  // and once it has died away, and two seconds more, the channel rests
  let started = std::time::Instant::now();
  while rig.engine.resting_chains() == 0 && started.elapsed() < Duration::from_secs(30) {
    std::thread::sleep(Duration::from_millis(100));
  }
  let rang = started.elapsed().as_secs_f32() + 0.6;
  assert_eq!(rig.engine.resting_chains(), 1, "still running after {} s", rang);
  println!("Verberate's channel rested {:.1} s after the burst", rang);
  assert!(rang > 2., "rested before the tail could have rung out ({} s)", rang);

  // sound wakes it, reverb and all
  peaks.lock().unwrap().clear();
  burst(&rig, 2);
  std::thread::sleep(Duration::from_millis(600));
  assert_eq!(rig.engine.resting_chains(), 0);
  let heard = peaks.lock().unwrap().clone();
  let late = &heard[heard.len().saturating_sub(blocks(0.2))..];
  assert!(late.iter().any(|&peak| peak > 1e-4), "no tail after waking: {:?}", late);

  running.store(false, Ordering::SeqCst);
  render.join().unwrap();
}

// bypassed, moved to the master and given a second instance for a
// mixdown. (on the test harness's thread, not the main one, which is
// enough for this plugin's audio but not its saved state: see
// tests/plugins.rs, which checks that on the main thread)
//   cargo test --no-default-features chow_tape_model -- --ignored
#[test]
#[ignore]
fn test_chow_tape_model_through_the_racks() {
  let bundle = Path::new("/Library/Audio/Plug-Ins/VST3/CHOWTapeModel.vst3");
  if !bundle.exists() {
    println!("CHOW Tape Model isn't installed");
    return;
  }
  let mut info = plugin::vst3::scan_bundle(bundle).unwrap().remove(0);
  info.path = bundle.to_string_lossy().into_owned();

  let mut rig = rig();
  rig.effects().use_backend(Arc::new(Vst3), Arc::new(Fixed(rig.engine.clone())), WAIT);
  rig.effects().plugins.set(plugin::scan::Catalog { plugins: vec![info.clone()], ..Default::default() });
  // the renderer runs on a thread of its own, as an output's would, so
  // the engine takes the chains it's sent
  let renderer = Arc::new(Mutex::new(std::mem::replace(&mut rig.renderer, daw::Engine::new(RATE).1)));
  let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
  let output: Arc<Mutex<Vec<f32>>> = Arc::default();
  let render = {
    let (renderer, running, output) = (renderer.clone(), running.clone(), output.clone());
    std::thread::spawn(move || {
      let mut block = vec![0f32; 512 * 2];
      while running.load(Ordering::SeqCst) {
        block.fill(0.);
        renderer.lock().unwrap().render(&mut block, std::time::Instant::now(), Duration::ZERO);
        output.lock().unwrap().extend_from_slice(&block);
        std::thread::sleep(Duration::from_millis(10));
      }
    })
  };
  // a second of a sine on a track, as heard
  let mut run = 0;
  let mut hear = |rig: &Rig, track: u32| -> Vec<f32> {
    use rodio::buffer::SamplesBuffer;
    run += 1;
    let sine: Vec<f32> = (0..RATE as usize).flat_map(|i| {
      let s = 0.3 * (i as f32 * 440. * std::f32::consts::TAU / RATE as f32).sin();
      [s, s]
    }).collect();
    let voice = rig.engine.voice(SamplesBuffer::new(2, RATE, sine), Some(run), 0, daw::Route::Track(rig.state.playlist.mixer.track_route(track)));
    output.lock().unwrap().clear();
    rig.engine.start_run(run, rig.state.playlist.mixer.master(), vec![voice]).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    let heard = output.lock().unwrap().clone();
    heard[heard.len() / 2..].to_vec()
  };
  let rms = |s: &[f32]| (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt();
  let dry = rms(&hear(&rig, 0));

  let id = rig.effects().add(Channel::Track(0), &info.id, None).unwrap();
  assert!(!rig.effects().racks()[0].slots[0].missing);
  let wet = hear(&rig, 0);
  assert!(wet.iter().all(|s| s.is_finite()));
  println!("dry rms {:.3}, through CHOW {:.3}", dry, rms(&wet));
  assert!((rms(&wet) - dry).abs() > 1e-3, "CHOW changed nothing");

  rig.effects().set_bypassed(id, true).unwrap();
  assert!((rms(&hear(&rig, 0)) - dry).abs() < 1e-2);
  rig.effects().set_bypassed(id, false).unwrap();

  rig.effects().move_to(id, Channel::Master, 0).unwrap();
  assert!((rms(&hear(&rig, 1)) - dry).abs() > 1e-3);
  rig.effects().idle();

  let mixdown = rig.effects().mixdown_chains(96_000, &BTreeSet::new()).unwrap();
  assert_eq!(mixdown.chains.len(), 1);
  drop(mixdown);

  rig.effects().remove(id).unwrap();
  assert!((rms(&hear(&rig, 1)) - dry).abs() < 1e-2);
  running.store(false, Ordering::SeqCst);
  render.join().unwrap();
}
