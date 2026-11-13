// the mixer's effects: every channel (each track, and the master) has a rack,
// its effects in the order the sound goes through them. racks hold effects
// only
//
// and the synth rows' plugins (see synths.rs): instruments patterns play,
// kept here as the copies of each that the channels' chains start with
//
// an effect is a loaded plugin (its main-thread half lives here, its audio
// half in a Slot in the engine's chain for that channel) or, if its plugin
// couldn't be loaded, a placeholder that keeps its saved settings so saving
// the project doesn't lose them. sound passes straight through a missing
// effect: it has no slot in the chain
//
// every change is sent on to the live engine's chains (see sync). a slot is
// in at most one engine chain at a time (the Slot rules in engine.rs): one
// leaving a chain is taken out, and the engine is waited for, before it goes
// anywhere else or its plugin is torn down
//
// plugins are only called on the main thread (see main_thread.rs). each
// method that touches one runs its work there, so it can be called from
// anywhere that holds none of the project's locks
//
// undo keeps snapshots of the racks (RackSnap). putting one back matches
// effects by their ids: those still here keep their plugin and its live
// settings, and only ones that have to come back are loaded again, with the
// settings they had when they went
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::daw::{self, Channel, Slot};
use crate::main_thread::{self, DropOnMain};
use crate::plugin::{self, PluginKind, Processor};

mod synths;

use crate::plugin_catalog::Plugins;

// how long to wait for the engine to take a chain before carrying on (with
// no output running it never does; a plugin is still safe to tear down then,
// as its processor stops calling it first). a slot isn't sent to another
// engine without it, though: see held_by
const WAIT: Duration = Duration::from_millis(500);
// how often loaded plugins are given time on the main thread
const IDLE_EVERY: Duration = Duration::from_millis(50);
// the rate plugins are prepared for until an output is open
const DEFAULT_RATE: u32 = 44_100;
// how long a plugin is given to set itself up after it's loaded. plugins
// often finish that on the main run loop a little later, and VST3 has no
// way to say when they're done: DecentSampler loads its sounds about a
// second in (its settings say it's empty until then), and Addictive Drums 2
// only once it's been played a few blocks and the run loop has turned. a
// mixdown takes a live plugin's settings only once it's been loaded this
// long, and plays its copies for this long before rendering with them
const SETTLE: Duration = Duration::from_secs(2);

// the main-thread half of a plugin, as the racks use it. plugin::Plugin is
// the real one; the tests use a fake
pub trait Hosted: Send {
  fn take_processor(&mut self) -> Option<Box<dyn Processor>>;
  fn state(&self) -> Result<Vec<u8>, String>;
  fn set_state(&mut self, blob: &[u8]) -> Result<(), String>;
  fn set_sample_rate(&mut self, rate: u32) -> Result<(), String>;
  fn has_editor(&self) -> bool;
  fn idle(&mut self);
  // whether its interface is showing
  fn editor_open(&self) -> bool;
  // the real plugin, for its window
  fn plugin(&mut self) -> Option<&mut plugin::Plugin>;
}

impl Hosted for plugin::Plugin {
  fn take_processor(&mut self) -> Option<Box<dyn Processor>> {
    plugin::Plugin::take_processor(self)
  }

  fn state(&self) -> Result<Vec<u8>, String> {
    plugin::Plugin::state(self)
  }

  fn set_state(&mut self, blob: &[u8]) -> Result<(), String> {
    plugin::Plugin::set_state(self, blob)
  }

  fn set_sample_rate(&mut self, rate: u32) -> Result<(), String> {
    plugin::Plugin::set_sample_rate(self, rate)
  }

  fn has_editor(&self) -> bool {
    plugin::Plugin::has_editor(self)
  }

  fn idle(&mut self) {
    plugin::Plugin::idle(self)
  }

  fn editor_open(&self) -> bool {
    plugin::Plugin::editor_open(self)
  }

  fn plugin(&mut self) -> Option<&mut plugin::Plugin> {
    Some(self)
  }
}

// how plugins are loaded (on the main thread)
pub trait Loader: Send + Sync {
  fn load(&self, path: &Path, id: &str, rate: u32) -> Result<Box<dyn Hosted>, String>;

  // a copy to render a mixdown with, which is told it's rendering offline
  fn load_offline(&self, path: &Path, id: &str, rate: u32) -> Result<Box<dyn Hosted>, String> {
    self.load(path, id, rate)
  }
}

struct Vst3;

impl Loader for Vst3 {
  fn load(&self, path: &Path, id: &str, rate: u32) -> Result<Box<dyn Hosted>, String> {
    plugin::Plugin::load(path, id, rate).map(|p| Box::new(p) as Box<dyn Hosted>)
  }

  fn load_offline(&self, path: &Path, id: &str, rate: u32) -> Result<Box<dyn Hosted>, String> {
    plugin::Plugin::load_offline(path, id, rate).map(|p| Box::new(p) as Box<dyn Hosted>)
  }
}

// the engine the chains are played by
pub trait Output: Send + Sync {
  // opening the output if it isn't
  fn engine(&self) -> Result<Arc<daw::Engine>, String>;
  fn engine_if_open(&self) -> Option<Arc<daw::Engine>>;
}

struct LiveOutput;

impl Output for LiveOutput {
  fn engine(&self) -> Result<Arc<daw::Engine>, String> {
    daw::engine()
  }

  fn engine_if_open(&self) -> Option<Arc<daw::Engine>> {
    daw::engine_if_open()
  }
}

// one effect in a channel's chain, as the frontend shows it
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EffectSlot {
  // unique in the session, never reused
  pub id: u64,
  pub plugin_id: String,
  pub name: String,
  pub vendor: String,
  pub bypassed: bool,
  // its plugin couldn't be loaded: sound passes straight through, and its
  // saved settings are kept
  pub missing: bool,
  // it has an interface of its own to show
  pub has_editor: bool,
}

// a channel's chain, in the order the sound goes through it
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRack {
  // none is the master
  pub track: Option<u32>,
  pub slots: Vec<EffectSlot>,
}

// every channel that has effects
pub type Racks = Vec<ChannelRack>;

// an effect, as saved in a project
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedEffect {
  pub plugin_id: String,
  pub name: String,
  #[serde(default)]
  pub vendor: String,
  // its bundle, for when the plugin isn't in the catalog
  #[serde(default)]
  pub path: String,
  #[serde(default)]
  pub bypassed: bool,
  // the plugin's settings, base64
  #[serde(default)]
  pub state: String,
  // where its floating window was last, if it's been open (left out if not,
  // as projects had it before)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub window: Option<WindowPosition>,
}

// a floating window's top-left corner, as plugin::window gives it: macOS
// screen points with y going up
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct WindowPosition {
  pub x: f64,
  pub y: f64,
}

// a channel's effects, as saved in a project
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedRack {
  // none (null) is the master
  #[serde(default)]
  pub track: Option<u32>,
  pub slots: Vec<SavedEffect>,
}

// an effect a project has whose plugin couldn't be loaded. it's in its rack,
// marked missing (or a synth row's, in its row)
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MissingEffect {
  // none is the master (and a synth)
  pub track: Option<u32>,
  pub name: String,
  pub vendor: String,
  pub path: String,
  pub reason: String,
}

fn channel_of(track: Option<u32>) -> Channel {
  track.map_or(Channel::Master, Channel::Track)
}

fn track_of(channel: Channel) -> Option<u32> {
  match channel {
    Channel::Track(n) => Some(n),
    Channel::Master => None,
  }
}

// what an effect is, whether or not its plugin is loaded
#[derive(Clone, Debug, PartialEq)]
struct Desc {
  id: u64,
  plugin_id: String,
  path: String,
  name: String,
  vendor: String,
  bypassed: bool,
}

struct Loaded {
  plugin: Arc<Mutex<Box<dyn Hosted>>>,
  slot: Arc<Slot>,
  has_editor: bool,
  // the rate the plugin is prepared for
  rate: u32,
  // when it was loaded (see SETTLE)
  since: Instant,
}

enum Instance {
  Loaded(Loaded),
  // the settings it was saved with, and why it isn't loaded
  Missing { state: Vec<u8>, reason: String },
}

impl Instance {
  fn loaded(&self) -> Option<&Loaded> {
    match self {
      Instance::Loaded(loaded) => Some(loaded),
      Instance::Missing { .. } => None,
    }
  }
}

struct Effect {
  desc: Desc,
  instance: Instance,
}

impl Effect {
  fn loaded(&self) -> Option<&Loaded> {
    self.instance.loaded()
  }

  // its plugin, to be torn down (see unload)
  fn into_loaded(self) -> Option<(u64, Loaded)> {
    match self.instance {
      Instance::Loaded(loaded) => Some((self.desc.id, loaded)),
      Instance::Missing { .. } => None,
    }
  }

  fn wire(&self) -> EffectSlot {
    EffectSlot {
      id: self.desc.id,
      plugin_id: self.desc.plugin_id.clone(),
      name: self.desc.name.clone(),
      vendor: self.desc.vendor.clone(),
      bypassed: self.desc.bypassed,
      missing: self.loaded().is_none(),
      has_editor: self.loaded().map_or(false, |l| l.has_editor),
    }
  }

  fn set_bypassed(&mut self, bypassed: bool) {
    self.desc.bypassed = bypassed;
    if let Some(loaded) = self.loaded() {
      loaded.slot.set_bypassed(bypassed);
    }
  }
}

// an effect in a snapshot of the racks
#[derive(Clone, Debug)]
struct SnapSlot {
  desc: Desc,
  // why it was missing, if it was
  missing: Option<String>,
  // its settings, if they were taken: always for a missing effect, and for
  // a loaded one once it's gone (so it comes back as it went)
  state: Option<Vec<u8>>,
}

// the racks as they were, for undo
#[derive(Clone, Debug, Default)]
pub struct RackSnap {
  channels: Vec<(Channel, Vec<SnapSlot>)>,
}

impl RackSnap {
  // whether two snapshots have the same effects in the same places, bypassed
  // the same (their settings aren't compared: knob turns aren't undone)
  pub fn same(&self, other: &RackSnap) -> bool {
    let shape = |snap: &RackSnap| -> Vec<(Channel, Vec<(Desc, bool)>)> {
      snap.channels.iter()
        .map(|(c, slots)| (*c, slots.iter().map(|s| (s.desc.clone(), s.missing.is_some())).collect()))
        .collect()
    };
    shape(self) == shape(other)
  }

  // keep an effect's settings as they are now, so it comes back with them
  pub fn keep_state(&mut self, id: u64, state: Vec<u8>) {
    for (_, slots) in &mut self.channels {
      for slot in slots.iter_mut().filter(|s| s.desc.id == id) {
        slot.state = Some(state.clone());
      }
    }
  }

  fn ids(&self) -> HashSet<u64> {
    self.channels.iter().flat_map(|(_, slots)| slots.iter().map(|s| s.desc.id)).collect()
  }
}

// second instances of the live effects, for a mixdown's own engine
pub struct MixdownEffects {
  pub chains: Vec<(Channel, Vec<Arc<Slot>>)>,
  // the synths' copies in them, by (synth, track)
  pub copies: daw::CopyIds,
  // torn down on the main thread when the mixdown's done with them
  pub plugins: DropOnMain<Vec<Box<dyn Hosted>>>,
  // how long they're played before the render, to set themselves up (see
  // SETTLE). none without any
  pub settle: Duration,
}

#[derive(Clone)]
struct Backend {
  loader: Arc<dyn Loader>,
  output: Arc<dyn Output>,
  wait: Duration,
  settle: Duration,
}

struct Inner {
  racks: BTreeMap<Channel, Vec<Effect>>,
  // the engine the chains have been sent to, and what each of its chains
  // holds (slot ids), as far as we've sent
  engine: Option<Arc<daw::Engine>>,
  sent: BTreeMap<Channel, Vec<u64>>,
  // the output is being replaced: nothing is sent until it's back
  paused: bool,
  // an engine the chains were taken off that hasn't been seen to let go of
  // them (its callback stalled). nothing is sent to another engine until it
  // has, or it's gone: a slot is never in two engines' chains
  held_by: Option<Arc<daw::Engine>>,
  // the rate plugins are loaded at while no output is open
  rate: u32,
  // the synths, by id (see synths.rs), and their order in the list
  synths: BTreeMap<u64, synths::LiveSynth>,
  synth_order: Vec<u64>,
  // the synth rows' plugins and the copies wanted when they were last
  // followed
  followed: Option<synths::Followed>,
}

pub struct Effects {
  inner: Mutex<Inner>,
  next_id: AtomicU64,
  // plugins loaded, for the idle ticker
  loaded: AtomicUsize,
  // for the tracks' routes into the engine
  mixer: Arc<daw::Mixer>,
  // the plugins installed, which effects are added from
  pub plugins: Arc<Plugins>,
  backend: Mutex<Backend>,
  on_unload: Mutex<Option<Arc<dyn Fn(u64) + Send + Sync>>>,
  // plugins of effects that have left the racks, by id, while their unload
  // callbacks run (so with_plugin still reaches them: see unload)
  unloading: Mutex<HashMap<u64, Arc<Mutex<Box<dyn Hosted>>>>>,
  // where each effect's floating window was last, by id, so it opens there
  // again and is saved with the project. kept after an effect goes, so
  // undoing its removal brings it back with its window's place
  windows: Mutex<HashMap<u64, WindowPosition>>,
  // the settings of synths that have left the list, by id, so one that
  // comes back (undo) comes back with them
  kept_synths: Mutex<HashMap<u64, Vec<u8>>>,
  // while above zero, the synths aren't followed (a project is loading)
  synth_pauses: AtomicUsize,
}

impl Effects {
  pub fn new(mixer: Arc<daw::Mixer>) -> Effects {
    Effects {
      inner: Mutex::new(Inner {
        racks: BTreeMap::new(),
        engine: None,
        sent: BTreeMap::new(),
        paused: false,
        held_by: None,
        rate: DEFAULT_RATE,
        synths: BTreeMap::new(),
        synth_order: Vec::new(),
        followed: None,
      }),
      next_id: AtomicU64::new(1),
      loaded: AtomicUsize::new(0),
      mixer,
      plugins: Arc::new(Plugins::default()),
      backend: Mutex::new(Backend { loader: Arc::new(Vst3), output: Arc::new(LiveOutput), wait: WAIT, settle: SETTLE }),
      on_unload: Mutex::new(None),
      unloading: Mutex::new(HashMap::new()),
      windows: Mutex::new(HashMap::new()),
      kept_synths: Mutex::new(HashMap::new()),
      synth_pauses: AtomicUsize::new(0),
    }
  }

  // load plugins and play chains some other way (the tests)
  #[cfg(test)]
  // (plugins are taken to be set up as soon as they're loaded, unless the
  // test says otherwise with use_settle)
  pub fn use_backend(&self, loader: Arc<dyn Loader>, output: Arc<dyn Output>, wait: Duration) {
    *self.backend.lock().unwrap() = Backend { loader, output, wait, settle: Duration::ZERO };
  }

  #[cfg(test)]
  pub fn use_settle(&self, settle: Duration) {
    self.backend.lock().unwrap().settle = settle;
  }

  fn backend(&self) -> Backend {
    self.backend.lock().unwrap().clone()
  }

  // ---- for the frontend and the rest of the engine

  // every channel that has effects, tracks first, then the master
  pub fn racks(&self) -> Racks {
    let inner = self.inner.lock().unwrap();
    inner.racks.iter()
      .filter(|(_, effects)| !effects.is_empty())
      .map(|(channel, effects)| ChannelRack {
        track: track_of(*channel),
        slots: effects.iter().map(Effect::wire).collect(),
      })
      .collect()
  }

  pub fn has_loaded(&self) -> bool {
    self.loaded.load(Ordering::SeqCst) > 0
  }

  // an effect's name, if it's in a rack (or a synth's)
  pub fn slot_name(&self, slot_id: u64) -> Option<String> {
    let inner = self.inner.lock().unwrap();
    match find(&inner, slot_id) {
      Some((channel, i)) => Some(inner.racks[&channel][i].desc.name.clone()),
      None => inner.synths.get(&slot_id).map(|synth| synth.desc.name.clone()),
    }
  }

  // call `f` with an effect's plugin, if it's in a rack and loaded (and is a
  // real VST3 plugin), or a synth's (its master copy's). on the main thread
  // only. the racks aren't locked
  // while `f` runs, so it may read them (slot_name), but it mustn't change
  // them: the plugin is locked meanwhile
  pub fn with_plugin<R>(&self, slot_id: u64, f: impl FnOnce(&mut plugin::Plugin) -> R) -> Option<R> {
    let hosted = self.hosted(slot_id)?;
    let mut hosted = hosted.lock().unwrap();
    hosted.plugin().map(f)
  }

  // `f` hears an effect's id just before its plugin is torn down (removed,
  // undone, or replaced by another project's), on the main thread, with the
  // racks unlocked. it has left the racks by then, but with_plugin still
  // reaches its plugin
  pub fn on_unload(&self, f: impl Fn(u64) + Send + Sync + 'static) {
    *self.on_unload.lock().unwrap() = Some(Arc::new(f));
  }

  // where an effect's floating window was last, if it's been open (in this
  // session, or when the project was saved)
  pub fn window_position(&self, slot_id: u64) -> Option<WindowPosition> {
    self.windows.lock().unwrap().get(&slot_id).copied()
  }

  pub fn keep_window_position(&self, slot_id: u64, position: WindowPosition) {
    self.windows.lock().unwrap().insert(slot_id, position);
  }

  // an effect's plugin (or a synth's master copy's), while it's in a rack
  // or a synth row, or being unloaded
  fn hosted(&self, slot_id: u64) -> Option<Arc<Mutex<Box<dyn Hosted>>>> {
    {
      let inner = self.inner.lock().unwrap();
      if let Some((channel, i)) = find(&inner, slot_id) {
        return inner.racks[&channel][i].loaded().map(|l| l.plugin.clone());
      }
      if let Some(synth) = inner.synths.get(&slot_id) {
        return synth.master.loaded().map(|l| l.plugin.clone());
      }
    }
    self.unloading.lock().unwrap().get(&slot_id).cloned()
  }

  // a new slot id (an effect's, a synth's or a copy's): unique in the
  // session, never reused
  pub fn new_id(&self) -> u64 {
    self.next_id.fetch_add(1, Ordering::SeqCst)
  }

  // ---- changes (each one undoable: see commands/effect.rs)

  // add an installed effect to a channel, at `index` (or the end). gives the
  // new effect's id. if the plugin can't be loaded, or is an instrument
  // (those are synths: see synths.rs), nothing changes
  pub fn add(&self, channel: Channel, plugin_id: &str, index: Option<usize>) -> Result<u64, String> {
    main_thread::run(|| {
      let info = self.plugins.find(plugin_id)
        .ok_or_else(|| "That plugin isn't among the installed plugins. Try scanning for plugins again".to_string())?;
      if info.kind == PluginKind::Instrument {
        return Err(format!("{} is an instrument: add it to the Pattern Sequencer as a row", info.name));
      }
      let desc = Desc {
        id: self.new_id(),
        plugin_id: info.id.clone(),
        path: info.path.clone(),
        name: info.name.clone(),
        vendor: info.vendor.clone(),
        bypassed: false,
      };
      let rate = self.load_rate();
      let instance = self.load(&desc, None, rate, false)
        .map_err(|e| format!("{} couldn't be loaded: {}", info.name, e))?;
      let id = desc.id;

      let mut inner = self.inner.lock().unwrap();
      let rack = inner.racks.entry(channel).or_default();
      let at = index.unwrap_or(rack.len()).min(rack.len());
      rack.insert(at, Effect { desc, instance });
      self.sync(&mut inner);
      Ok(id)
    })?
  }

  pub fn remove(&self, slot_id: u64) -> Result<(), String> {
    main_thread::run(|| {
      let mut inner = self.inner.lock().unwrap();
      let (channel, i) = find(&inner, slot_id).ok_or_else(gone)?;
      let effect = inner.racks.get_mut(&channel).unwrap().remove(i);
      self.sync(&mut inner);
      let engine = inner.engine.clone();
      drop(inner);
      self.unload(engine, effect.into_loaded().into_iter().collect());
      Ok(())
    })?
  }

  // move an effect within its channel or to another, keeping its plugin
  // and settings. `index` is where it ends up in that channel's chain
  pub fn move_to(&self, slot_id: u64, channel: Channel, index: usize) -> Result<(), String> {
    main_thread::run(|| {
      let mut inner = self.inner.lock().unwrap();
      let (from, i) = find(&inner, slot_id).ok_or_else(gone)?;
      let effect = inner.racks.get_mut(&from).unwrap().remove(i);
      let rack = inner.racks.entry(channel).or_default();
      let at = index.min(rack.len());
      rack.insert(at, effect);
      self.sync(&mut inner);
      Ok(())
    })?
  }

  pub fn set_bypassed(&self, slot_id: u64, bypassed: bool) -> Result<(), String> {
    let mut inner = self.inner.lock().unwrap();
    let (channel, i) = find(&inner, slot_id).ok_or_else(gone)?;
    inner.racks.get_mut(&channel).unwrap()[i].set_bypassed(bypassed);
    Ok(())
  }

  // ---- undo

  pub fn snapshot(&self) -> RackSnap {
    let inner = self.inner.lock().unwrap();
    RackSnap {
      channels: inner.racks.iter()
        .filter(|(_, effects)| !effects.is_empty())
        .map(|(channel, effects)| (*channel, effects.iter().map(|e| match &e.instance {
          Instance::Loaded(_) => SnapSlot { desc: e.desc.clone(), missing: None, state: None },
          Instance::Missing { state, reason } =>
            SnapSlot { desc: e.desc.clone(), missing: Some(reason.clone()), state: Some(state.clone()) },
        }).collect()))
        .collect(),
    }
  }

  // an effect's settings as they are now (a missing one's, as saved)
  pub fn state_of(&self, slot_id: u64) -> Option<Vec<u8>> {
    main_thread::run(|| self.state_here(slot_id)).ok().flatten()
  }

  fn state_here(&self, slot_id: u64) -> Option<Vec<u8>> {
    let hosted = {
      let inner = self.inner.lock().unwrap();
      let instance = match find(&inner, slot_id) {
        Some((channel, i)) => &inner.racks[&channel][i].instance,
        None => &inner.synths.get(&slot_id)?.master,
      };
      match instance {
        Instance::Loaded(loaded) => loaded.plugin.clone(),
        Instance::Missing { state, .. } => return Some(state.clone()),
      }
    };
    let state = hosted.lock().unwrap().state();
    state.map_err(|e| println!("an effect's settings couldn't be read: {}", e)).ok()
  }

  // put the racks back as `target` has them. effects in both keep their
  // plugins and settings; ones that go have their settings kept in `left`
  // (the snapshot being left), and ones that come back are loaded with the
  // settings `target` kept for them (missing if they can't be)
  pub fn restore(&self, target: &RackSnap, left: &mut RackSnap) {
    let done = main_thread::run(|| {
      let wanted = target.ids();
      let live: HashSet<u64> = {
        let inner = self.inner.lock().unwrap();
        inner.racks.values().flatten().map(|e| e.desc.id).collect()
      };
      for &id in live.iter().filter(|id| !wanted.contains(id)) {
        if let Some(state) = self.state_here(id) {
          left.keep_state(id, state);
        }
      }

      let coming: Vec<&SnapSlot> = target.channels.iter().flat_map(|(_, slots)| slots)
        .filter(|s| !live.contains(&s.desc.id))
        .collect();
      let rate = if coming.is_empty() { DEFAULT_RATE } else { self.load_rate() };
      let mut fresh: HashMap<u64, Effect> = coming.into_iter().map(|snap| {
        let instance = self.load(&snap.desc, snap.state.as_deref(), rate, false).unwrap_or_else(|reason| Instance::Missing {
          state: snap.state.clone().unwrap_or_default(),
          reason,
        });
        (snap.desc.id, Effect { desc: snap.desc.clone(), instance })
      }).collect();

      let mut inner = self.inner.lock().unwrap();
      let mut here: HashMap<u64, Effect> = std::mem::take(&mut inner.racks).into_values().flatten()
        .map(|e| (e.desc.id, e))
        .collect();
      for (channel, slots) in &target.channels {
        let rack = inner.racks.entry(*channel).or_default();
        for snap in slots {
          if let Some(mut effect) = here.remove(&snap.desc.id).or_else(|| fresh.remove(&snap.desc.id)) {
            effect.set_bypassed(snap.desc.bypassed);
            rack.push(effect);
          }
        }
      }
      self.sync(&mut inner);
      let engine = inner.engine.clone();
      drop(inner);
      self.unload(engine, here.into_values().filter_map(Effect::into_loaded).collect());
    });
    if let Err(e) = done {
      println!("the effects weren't put back: {}", e);
    }
  }

  // ---- projects

  // every rack, as a project saves it: each loaded plugin's settings as
  // they are now, and missing effects as they were read
  pub fn saved(&self) -> Vec<SavedRack> {
    let taken = main_thread::run(|| {
      let racks: Vec<(Channel, Vec<(Desc, Result<Arc<Mutex<Box<dyn Hosted>>>, Vec<u8>>)>)> = {
        let inner = self.inner.lock().unwrap();
        inner.racks.iter().filter(|(_, effects)| !effects.is_empty()).map(|(channel, effects)| {
          (*channel, effects.iter().map(|e| (e.desc.clone(), match &e.instance {
            Instance::Loaded(loaded) => Ok(loaded.plugin.clone()),
            Instance::Missing { state, .. } => Err(state.clone()),
          })).collect())
        }).collect()
      };
      racks.into_iter().map(|(channel, effects)| SavedRack {
        track: track_of(channel),
        slots: effects.into_iter().map(|(desc, instance)| {
          let state = match instance {
            Ok(hosted) => {
              let state = hosted.lock().unwrap().state();
              state.unwrap_or_else(|e| {
                println!("{}'s settings couldn't be saved: {}", desc.name, e);
                Vec::new()
              })
            }
            Err(state) => state,
          };
          SavedEffect {
            plugin_id: desc.plugin_id,
            name: desc.name,
            vendor: desc.vendor,
            path: desc.path,
            bypassed: desc.bypassed,
            window: self.window_position(desc.id),
            state: base64::encode(state),
          }
        }).collect(),
      }).collect()
    });
    taken.unwrap_or_else(|e| {
      println!("the effects weren't saved: {}", e);
      Vec::new()
    })
  }

  // replace every rack with a project's (and take out every synth: see
  // load_synths). a plugin is found in the catalog by its id (or at the path
  // it was saved with); one that can't be loaded is kept as missing, and
  // listed. an instrument isn't an effect, and is left out
  pub fn load_saved(&self, saved: &[SavedRack]) -> Vec<MissingEffect> {
    let done = main_thread::run(|| {
      self.clear_here();

      let rate = if saved.iter().all(|rack| rack.slots.is_empty()) { DEFAULT_RATE } else { self.load_rate() };
      let mut missing = Vec::new();
      let mut racks: BTreeMap<Channel, Vec<Effect>> = BTreeMap::new();
      let mut windows = HashMap::new();
      for rack in saved {
        for effect in &rack.slots {
          let found = self.plugins.find(&effect.plugin_id);
          if found.as_ref().map_or(false, |info| info.kind == PluginKind::Instrument) {
            println!("left out {} from a rack: it's an instrument", effect.name);
            continue;
          }
          let path = found.as_ref().map_or_else(|| effect.path.clone(), |info| info.path.clone());
          let desc = Desc {
            id: self.new_id(),
            plugin_id: effect.plugin_id.clone(),
            path,
            name: effect.name.clone(),
            vendor: effect.vendor.clone(),
            bypassed: effect.bypassed,
          };
          let state = base64::decode(&effect.state);
          let loaded = match &state {
            Ok(state) => self.load(&desc, Some(state).filter(|s| !s.is_empty()).map(|s| &s[..]), rate, false),
            Err(_) => Err("its saved settings are damaged".to_string()),
          };
          let instance = loaded.unwrap_or_else(|reason| {
            missing.push(MissingEffect {
              track: rack.track,
              name: effect.name.clone(),
              vendor: effect.vendor.clone(),
              path: effect.path.clone(),
              reason: reason.clone(),
            });
            // written back just as it was read
            Instance::Missing { state: state.clone().unwrap_or_default(), reason }
          });
          let desc = match instance {
            Instance::Missing { .. } => Desc { path: effect.path.clone(), ..desc },
            Instance::Loaded(_) => desc,
          };
          // its window opens where it was, under the id it has now
          if let Some(position) = effect.window {
            windows.insert(desc.id, position);
          }
          racks.entry(channel_of(rack.track)).or_default().push(Effect { desc, instance });
        }
      }
      *self.windows.lock().unwrap() = windows;
      let mut inner = self.inner.lock().unwrap();
      inner.racks = racks;
      self.sync(&mut inner);
      missing
    });
    done.unwrap_or_else(|e| {
      println!("the project's effects weren't loaded: {}", e);
      Vec::new()
    })
  }

  // take every effect and synth out
  #[cfg(test)]
  pub fn clear(&self) {
    if let Err(e) = main_thread::run(|| self.clear_here()) {
      println!("the effects weren't cleared: {}", e);
    }
  }

  fn clear_here(&self) {
    let mut inner = self.inner.lock().unwrap();
    let mut doomed: Vec<(u64, Loaded)> = std::mem::take(&mut inner.racks).into_values().flatten()
      .filter_map(Effect::into_loaded)
      .collect();
    doomed.extend(std::mem::take(&mut inner.synths).into_values().flat_map(synths::LiveSynth::into_loaded));
    inner.synth_order.clear();
    inner.followed = None;
    self.sync(&mut inner);
    let engine = inner.engine.clone();
    drop(inner);
    self.kept_synths.lock().unwrap().clear();
    self.unload(engine, doomed);
  }

  // ---- mixdown

  // a second instance of every effect that's loaded and switched on, made at
  // `rate` with the live one's settings copied over, in chains for another
  // engine; and one of each synth for each track that plays it (`synths`:
  // (synth, track)), with its master copy's settings, first in that track's
  // chain. the live plugins aren't touched
  pub fn mixdown_chains(&self, rate: u32, synths: &BTreeSet<(u64, u32)>) -> Result<MixdownEffects, String> {
    // a plugin loaded just now may not have set itself up yet, and its
    // settings then may not be the ones it'll have (a sampler that hasn't
    // loaded its sounds says it's empty, and a copy given that stays empty).
    // so wait until each one is SETTLE old (a mixdown starts away from the
    // main thread, which they need meanwhile)
    let settle = self.backend().settle;
    let youngest = {
      let inner = self.inner.lock().unwrap();
      let synths = synths.iter().filter_map(|(id, _)| inner.synths.get(id)?.master.loaded().map(|l| l.since));
      inner.racks.values().flatten()
        .filter(|e| !e.desc.bypassed)
        .filter_map(|e| e.loaded().map(|l| l.since))
        .chain(synths)
        .max()
    };
    if let Some(since) = youngest {
      let wait = settle.saturating_sub(since.elapsed());
      if !wait.is_zero() { std::thread::sleep(wait); }
    }

    main_thread::run(|| {
      // the synths' copies (one that's muted isn't heard: none), then the
      // effects, in their channels' order
      let live: Vec<(Channel, Desc, Arc<Mutex<Box<dyn Hosted>>>, Option<(u64, f32, f32)>)> = {
        let inner = self.inner.lock().unwrap();
        let copies = synths.iter().filter_map(|&(id, track)| {
          let synth = inner.synths.get(&id).filter(|synth| !synth.muted)?;
          let loaded = synth.master.loaded()?;
          Some((Channel::Track(track), synth.desc.clone(), loaded.plugin.clone(), Some((id, synth.volume, synth.pan))))
        });
        let effects = inner.racks.iter()
          .flat_map(|(channel, effects)| effects.iter().map(move |e| (*channel, e)))
          .filter(|(_, e)| !e.desc.bypassed)
          .filter_map(|(channel, e)| e.loaded().map(|l| (channel, e.desc.clone(), l.plugin.clone(), None)));
        copies.chain(effects).collect()
      };

      let loader = self.backend().loader;
      let mut plugins = Vec::new();
      let mut chains: BTreeMap<Channel, Vec<Arc<Slot>>> = BTreeMap::new();
      let mut copies = daw::CopyIds::new();
      for (channel, desc, hosted, synth) in live {
        let failed = |e: String| format!("{} couldn't be loaded for the mixdown: {}", desc.name, e);
        let state = hosted.lock().unwrap().state().map_err(failed)?;
        let mut copy = loader.load_offline(Path::new(&desc.path), &desc.plugin_id, rate).map_err(failed)?;
        copy.set_state(&state).map_err(failed)?;
        let processor = copy.take_processor().ok_or_else(|| failed("it has no audio processor".into()))?;
        let slot = match synth {
          Some((synth, volume, pan)) => {
            // (an id of its own: a synth has a copy on each of its tracks)
            let slot = Slot::new_instrument(self.new_id(), processor);
            slot.set_gain(volume);
            slot.set_pan(pan);
            copies.insert((synth, track_of(channel)), slot.id());
            slot
          }
          None => Slot::new(desc.id, processor),
        };
        chains.entry(channel).or_default().push(slot);
        plugins.push(copy);
      }
      let settle = if plugins.is_empty() { Duration::ZERO } else { settle };
      Ok(MixdownEffects { chains: chains.into_iter().collect(), copies, plugins: DropOnMain::new(plugins), settle })
    })?
  }

  // ---- the output

  // the output is about to be replaced: every chain comes off the engine
  // (waiting for it to take them), and none is sent until `attach`. if it
  // doesn't take them in time, none is sent to another engine until it has
  // (see held_by)
  pub fn detach(&self) {
    let wait = self.backend().wait;
    let (engine, had_chains) = {
      let mut inner = self.inner.lock().unwrap();
      inner.paused = true;
      let engine = inner.engine.take();
      let had_chains = !inner.sent.is_empty();
      if let Some(engine) = &engine {
        self.empty(engine, &mut inner);
      }
      inner.sent.clear();
      (engine, had_chains)
    };
    if let Some(engine) = engine {
      if !engine.wait_for_chains(wait) && had_chains {
        self.inner.lock().unwrap().held_by = Some(engine);
      }
    }
  }

  // an output has just been opened: send it the chains, if they had nowhere
  // to go until now (see follow_output)
  pub fn output_opened(&self) {
    let done = main_thread::run(|| {
      let mut inner = self.inner.lock().unwrap();
      self.sync(&mut inner);
    });
    if let Err(e) = done {
      println!("the effects weren't put on the new output: {}", e);
    }
  }

  // send every chain to the engine that's open now, with every plugin
  // prepared for its rate
  pub fn attach(&self) {
    let done = main_thread::run(|| {
      let mut inner = self.inner.lock().unwrap();
      inner.paused = false;
      self.sync(&mut inner);
    });
    if let Err(e) = done {
      println!("the effects weren't put back on the output: {}", e);
    }
  }

  // give every loaded plugin time on the main thread (it passes its own
  // parameter changes on, and restarts if it asked to), and tell the engine
  // which have their interface showing
  pub fn idle(&self) {
    let _ = main_thread::run(|| {
      let plugins: Vec<(Arc<Mutex<Box<dyn Hosted>>>, Arc<Slot>)> = {
        let inner = self.inner.lock().unwrap();
        all_loaded(&inner).map(|l| (l.plugin.clone(), l.slot.clone())).collect()
      };
      for (plugin, slot) in plugins {
        match plugin.try_lock() {
          Ok(mut plugin) => {
            plugin.idle();
            // a plugin whose interface is showing keeps its channel
            // running through silence (see Slot::set_showing)
            slot.set_showing(plugin.editor_open());
          }
          // (one busy elsewhere on this thread, its window say, waits for
          // the next round, and counts as showing)
          Err(_) => slot.set_showing(true),
        }
      }
    });
  }

  // ---- inside (on the main thread)

  // the rate to load a plugin at: the output's (opening it, as the first
  // effect does), or the last one known
  fn load_rate(&self) -> u32 {
    match self.backend().output.engine() {
      Ok(engine) => engine.sample_rate(),
      Err(e) => {
        println!("effects are loaded without an audio output: {}", e);
        self.inner.lock().unwrap().rate
      }
    }
  }

  // load an effect's plugin (or an instrument's: a synth's copy), ready for
  // a chain
  fn load(&self, desc: &Desc, state: Option<&[u8]>, rate: u32, instrument: bool) -> Result<Instance, String> {
    let loader = self.backend().loader;
    let mut hosted = loader.load(Path::new(&desc.path), &desc.plugin_id, rate)?;
    if let Some(state) = state {
      hosted.set_state(state).map_err(|e| format!("its settings couldn't be put back ({})", e))?;
    }
    let processor = hosted.take_processor().ok_or_else(|| "it has no audio processor".to_string())?;
    let slot = if instrument { Slot::new_instrument(desc.id, processor) } else { Slot::new(desc.id, processor) };
    slot.set_bypassed(desc.bypassed);
    let has_editor = hosted.has_editor();
    Ok(Instance::Loaded(Loaded { plugin: Arc::new(Mutex::new(hosted)), slot, has_editor, rate, since: Instant::now() }))
  }

  // tear down plugins that have left the racks (or the synths), by id, once
  // the engine has let go of their slots. the unload callbacks run first,
  // while with_plugin can still reach each plugin (through `unloading`), so
  // a window showing one is closed with its plugin there to let go of it
  fn unload(&self, engine: Option<Arc<daw::Engine>>, effects: Vec<(u64, Loaded)>) {
    let leaving: Vec<(u64, Arc<Mutex<Box<dyn Hosted>>>)> = effects.iter()
      .map(|(id, loaded)| (*id, loaded.plugin.clone()))
      .collect();
    if leaving.is_empty() { return; }
    self.unloading.lock().unwrap().extend(leaving.iter().cloned());
    if let Some(engine) = engine {
      engine.wait_for_chains(self.backend().wait);
    }
    let on_unload = self.on_unload.lock().unwrap().clone();
    if let Some(on_unload) = on_unload {
      for (id, _) in &leaving { on_unload(*id); }
    }
    {
      let mut unloading = self.unloading.lock().unwrap();
      for (id, _) in &leaving { unloading.remove(id); }
    }
    drop(leaving);
    drop(effects);
  }

  // empty every chain sent to `engine`
  fn empty(&self, engine: &daw::Engine, inner: &mut Inner) {
    let channels: Vec<Channel> = inner.sent.keys().copied().collect();
    for channel in channels {
      self.send_chain(engine, channel, Vec::new(), inner);
    }
  }

  fn send_chain(&self, engine: &daw::Engine, channel: Channel, ids: Vec<u64>, inner: &mut Inner) {
    let slots: Vec<Arc<Slot>> = ids.iter().filter_map(|id| {
      let in_rack = inner.racks.get(&channel).and_then(|rack| rack.iter().find(|e| e.desc.id == *id));
      match in_rack {
        Some(effect) => effect.loaded().map(|l| l.slot.clone()),
        None => synths::slot_in(inner, *id),
      }
    }).collect();
    let sent = match channel {
      Channel::Track(n) => engine.set_track_chain(&self.mixer.track_route(n), slots),
      Channel::Master => engine.set_master_chain(slots),
    };
    match sent {
      Ok(()) if ids.is_empty() => { inner.sent.remove(&channel); }
      Ok(()) => { inner.sent.insert(channel, ids); }
      Err(e) => println!("an effects chain didn't reach the engine: {}", e),
    }
  }

  // bring the engine's chains in line with the racks. a slot moving from
  // one channel to another is taken out of the first, and the engine waited
  // for, before it's put in the second
  fn sync(&self, inner: &mut Inner) {
    inner.racks.retain(|_, effects| !effects.is_empty());
    let loaded = all_loaded(inner).count();
    self.loaded.store(loaded, Ordering::SeqCst);
    if inner.paused { return; }

    let backend = self.backend();
    let engine = if loaded > 0 {
      backend.output.engine().map_err(|e| println!("effects can't be heard: {}", e)).ok()
    } else {
      backend.output.engine_if_open()
    };
    let same = match (&inner.engine, &engine) {
      (Some(a), Some(b)) => Arc::ptr_eq(a, b),
      (None, None) => true,
      _ => false,
    };
    if !same {
      // (an output that's been replaced without us hearing of it)
      if let Some(old) = inner.engine.take() {
        let had_chains = !inner.sent.is_empty();
        self.empty(&old, inner);
        if !old.wait_for_chains(backend.wait) && had_chains {
          inner.held_by = Some(old);
        }
      }
      inner.sent.clear();
      inner.engine = engine.clone();
    }
    let Some(engine) = engine else { return };

    // nothing goes to this engine, nor is any plugin touched, while another
    // may still be running the slots. the effects pass sound dry until the
    // other lets go (or is gone), which the next sync looks at again
    if let Some(old) = inner.held_by.clone() {
      if Arc::ptr_eq(&old, &engine) || old.wait_for_chains(backend.wait) {
        inner.held_by = None;
      } else {
        println!("effects are left off the audio output (heard dry) until the old output lets go of them");
        return;
      }
    }

    // every plugin prepared for the engine's rate
    let rate = engine.sample_rate();
    inner.rate = rate;
    for loaded in all_loaded_mut(inner) {
      if loaded.rate != rate {
        if let Err(e) = loaded.plugin.lock().unwrap().set_sample_rate(rate) {
          println!("a plugin can't run at {} Hz: {}", rate, e);
        }
        loaded.rate = rate;
      }
    }

    // each chain: the synths' copies first, then the rack's effects
    let mut wanted: BTreeMap<Channel, Vec<u64>> = synths::chains(inner);
    for (channel, effects) in &inner.racks {
      wanted.entry(*channel).or_default().extend(effects.iter().filter(|e| e.loaded().is_some()).map(|e| e.desc.id));
    }
    let was_in: HashMap<u64, Channel> = inner.sent.iter()
      .flat_map(|(channel, ids)| ids.iter().map(move |id| (*id, *channel)))
      .collect();
    let channels: BTreeSet<Channel> = wanted.keys().chain(inner.sent.keys()).copied().collect();
    let chain_of = |channel: &Channel| wanted.get(channel).cloned().unwrap_or_default();
    let differs = |inner: &Inner, channel: &Channel, ids: &Vec<u64>| {
      inner.sent.get(channel).map_or(!ids.is_empty(), |sent| sent != ids)
    };

    // first without the slots that are still in another channel's chain
    let mut moving = false;
    for channel in &channels {
      let all = chain_of(channel);
      let first: Vec<u64> = all.iter().copied()
        .filter(|id| was_in.get(id).map_or(true, |c| c == channel))
        .collect();
      moving |= first.len() != all.len();
      if differs(inner, channel, &first) {
        self.send_chain(&engine, *channel, first, inner);
      }
    }
    if moving {
      engine.wait_for_chains(backend.wait);
      for channel in &channels {
        let all = chain_of(channel);
        if differs(inner, channel, &all) {
          self.send_chain(&engine, *channel, all, inner);
        }
      }
    }
  }
}

fn find(inner: &Inner, slot_id: u64) -> Option<(Channel, usize)> {
  inner.racks.iter().find_map(|(channel, effects)| {
    effects.iter().position(|e| e.desc.id == slot_id).map(|i| (*channel, i))
  })
}

// every plugin loaded: the racks', and the synths' copies
fn all_loaded(inner: &Inner) -> impl Iterator<Item = &Loaded> {
  inner.racks.values().flatten().filter_map(Effect::loaded)
    .chain(inner.synths.values().flat_map(synths::LiveSynth::loaded))
}

fn all_loaded_mut(inner: &mut Inner) -> impl Iterator<Item = &mut Loaded> {
  let racks = inner.racks.values_mut().flatten().filter_map(|e| match &mut e.instance {
    Instance::Loaded(loaded) => Some(loaded),
    Instance::Missing { .. } => None,
  });
  racks.chain(inner.synths.values_mut().flat_map(synths::LiveSynth::loaded_mut))
}

fn gone() -> String {
  "That effect isn't in the mixer any more".to_string()
}

// give loaded plugins time on the main thread, about twenty times a second,
// for as long as the app runs
pub fn keep_idling(effects: Arc<Effects>) {
  let started = std::thread::Builder::new().name("plugin idle".into()).spawn(move || loop {
    std::thread::sleep(IDLE_EVERY);
    if effects.has_loaded() {
      effects.idle();
    }
  });
  if let Err(e) = started {
    println!("plugins won't be given idle time: {}", e);
  }
}

// send the chains to an output as soon as it opens, so effects added (or a
// project loaded) while no output could be opened are heard once one is.
// it's done on a thread of its own: the output can be opened from inside a
// sync, with the racks locked, and this waits for that to finish (a sync
// with nothing new to send sends nothing)
pub fn follow_output(effects: &Arc<Effects>) {
  let racks = Arc::downgrade(effects);
  daw::on_output_opened(move || {
    let racks = racks.clone();
    let started = std::thread::Builder::new().name("effects to output".into()).spawn(move || {
      if let Some(effects) = racks.upgrade() { effects.output_opened(); }
    });
    if let Err(e) = started {
      println!("the effects weren't put on the new output: {}", e);
    }
  });
}

// fake plugins for tests: a gain, whose settings are the gain, and a synth
// that plays its "gain" as a steady level while any note is held
#[cfg(test)]
pub mod testing {
  use super::*;
  use crate::plugin::scan::Catalog;
  use crate::plugin::{Event, PluginInfo, Transport};
  use std::sync::atomic::{AtomicBool, AtomicU32};

  pub const RATE: u32 = 48_000;
  // the gain a fake plugin starts with
  pub const START_GAIN: f32 = 0.5;

  fn info(n: u32, name: &str, kind: PluginKind, path: &str) -> PluginInfo {
    PluginInfo {
      id: format!("vst3:{:032x}", n),
      path: path.into(),
      name: name.into(),
      vendor: "Fake Audio".into(),
      version: "1.0".into(),
      kind,
      category: "Other".into(),
      subcategories: vec!["Fx".into()],
    }
  }

  // a gain that loads; a gain whose bundle isn't there; an instrument, and
  // one whose bundle isn't there
  pub fn gain() -> PluginInfo { info(1, "Gain", PluginKind::Effect, "/fake/Gain.vst3") }
  pub fn absent() -> PluginInfo { info(2, "Absent", PluginKind::Effect, "/gone/Absent.vst3") }
  pub fn synth() -> PluginInfo { info(3, "Synth", PluginKind::Instrument, "/fake/Synth.vst3") }
  pub fn gone_synth() -> PluginInfo { info(4, "Gone Synth", PluginKind::Instrument, "/gone/Synth.vst3") }

  pub fn catalog() -> Catalog {
    Catalog { plugins: vec![gain(), absent(), synth(), gone_synth()], ..Default::default() }
  }

  struct GainProcessor(Arc<AtomicU32>);

  impl Processor for GainProcessor {
    fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[Event], _: &Transport) {
      let gain = f32::from_bits(self.0.load(Ordering::Relaxed));
      for (output, input) in output.into_iter().zip(input) {
        for (o, i) in output.iter_mut().zip(input) { *o = i * gain; }
      }
    }
  }

  // 0 until a note starts, then its level until every note has ended, each
  // on the frame it's given (what comes in is ignored: the engine adds it).
  // silent until its plugin has set itself up, and for good if it was given
  // the settings of one that hadn't
  struct SynthProcessor {
    level: Arc<AtomicU32>,
    held: [bool; 128],
    ready_at: Instant,
    empty: Arc<AtomicBool>,
  }

  impl Processor for SynthProcessor {
    fn process(&mut self, _: [&[f32]; 2], output: [&mut [f32]; 2], events: &[Event], _: &Transport) {
      let ready = Instant::now() >= self.ready_at && !self.empty.load(Ordering::Relaxed);
      let level = if ready { f32::from_bits(self.level.load(Ordering::Relaxed)) } else { 0. };
      let [left, right] = output;
      let mut next = events.iter().peekable();
      for i in 0..left.len() {
        while let Some(event) = next.next_if(|e| e.offset() as usize <= i) {
          match *event {
            Event::NoteOn { pitch, .. } => self.held[pitch as usize] = true,
            Event::NoteOff { pitch, .. } => self.held[pitch as usize] = false,
          }
        }
        let out = if self.held.iter().any(|&h| h) { level } else { 0. };
        left[i] = out;
        right[i] = out;
      }
    }
  }

  pub struct FakeGain {
    gain: Arc<AtomicU32>,
    processor: Option<Box<dyn Processor>>,
    alive: Arc<AtomicUsize>,
    // when it's set itself up (as a sampler loading its sounds): until then
    // its settings are empty
    ready_at: Instant,
    empty: Arc<AtomicBool>,
    // whether its interface is showing (the loader's, for every fake)
    open: Arc<AtomicBool>,
  }

  impl Drop for FakeGain {
    fn drop(&mut self) {
      self.alive.fetch_sub(1, Ordering::SeqCst);
    }
  }

  impl Hosted for FakeGain {
    fn take_processor(&mut self) -> Option<Box<dyn Processor>> {
      self.processor.take()
    }

    fn state(&self) -> Result<Vec<u8>, String> {
      if Instant::now() < self.ready_at { return Ok(Vec::new()); }
      Ok(self.gain.load(Ordering::Relaxed).to_le_bytes().to_vec())
    }

    fn set_state(&mut self, blob: &[u8]) -> Result<(), String> {
      self.empty.store(blob.is_empty(), Ordering::Relaxed);
      if blob.is_empty() { return Ok(()); }
      let bytes: [u8; 4] = blob.try_into().map_err(|_| "not a gain".to_string())?;
      self.gain.store(u32::from_le_bytes(bytes), Ordering::Relaxed);
      Ok(())
    }

    fn set_sample_rate(&mut self, _rate: u32) -> Result<(), String> {
      Ok(())
    }

    fn has_editor(&self) -> bool {
      false
    }

    fn idle(&mut self) {}

    fn editor_open(&self) -> bool {
      self.open.load(Ordering::SeqCst)
    }

    fn plugin(&mut self) -> Option<&mut plugin::Plugin> {
      None
    }
  }

  // loads a FakeGain from any bundle under /fake (unless it's been made
  // `broken`), and counts them. Synth.vst3's plays a synth
  #[derive(Default)]
  pub struct FakeLoader {
    pub broken: AtomicBool,
    // how long each takes to set itself up once it's loaded
    pub settles_in: Mutex<Duration>,
    pub loads: AtomicUsize,
    pub alive: Arc<AtomicUsize>,
    // the rates they were loaded at
    pub rates: Mutex<Vec<u32>>,
    // whether every fake's interface is showing
    pub editors_open: Arc<AtomicBool>,
  }

  impl Loader for FakeLoader {
    fn load(&self, path: &Path, _id: &str, rate: u32) -> Result<Box<dyn Hosted>, String> {
      if !path.starts_with("/fake") || self.broken.load(Ordering::SeqCst) {
        return Err("not installed".into());
      }
      self.loads.fetch_add(1, Ordering::SeqCst);
      self.alive.fetch_add(1, Ordering::SeqCst);
      self.rates.lock().unwrap().push(rate);
      let gain = Arc::new(AtomicU32::new(START_GAIN.to_bits()));
      let ready_at = Instant::now() + *self.settles_in.lock().unwrap();
      let empty = Arc::new(AtomicBool::new(false));
      let processor: Box<dyn Processor> = if path.ends_with("Synth.vst3") {
        Box::new(SynthProcessor { level: gain.clone(), held: [false; 128], ready_at, empty: empty.clone() })
      } else {
        Box::new(GainProcessor(gain.clone()))
      };
      Ok(Box::new(FakeGain { processor: Some(processor), gain, alive: self.alive.clone(), ready_at, empty, open: self.editors_open.clone() }))
    }
  }

  // always the same engine (one without a device, rendered by the test)
  pub struct Fixed(pub Arc<daw::Engine>);

  impl Output for Fixed {
    fn engine(&self) -> Result<Arc<daw::Engine>, String> {
      Ok(self.0.clone())
    }

    fn engine_if_open(&self) -> Option<Arc<daw::Engine>> {
      Some(self.0.clone())
    }
  }

  // a state whose effects are fake gains, played by an engine the test
  // renders
  pub struct Rig {
    pub state: Arc<daw::InnerState>,
    pub engine: Arc<daw::Engine>,
    pub renderer: daw::Renderer,
    pub loader: Arc<FakeLoader>,
    pub run: u64,
  }

  pub fn rig() -> Rig {
    let state = Arc::new(daw::InnerState::default());
    let (engine, renderer) = daw::Engine::new(RATE);
    let engine = Arc::new(engine);
    let loader = Arc::new(FakeLoader::default());
    state.playlist.effects.use_backend(loader.clone(), Arc::new(Fixed(engine.clone())), Duration::ZERO);
    state.playlist.effects.plugins.set(catalog());
    Rig { state, engine, renderer, loader, run: 0 }
  }

  impl Rig {
    pub fn effects(&self) -> &Effects {
      &self.state.playlist.effects
    }

    // what a steady 0.2 on a track (none: straight to the master) comes
    // out as
    pub fn level(&mut self, track: Option<u32>) -> f32 {
      use rodio::buffer::SamplesBuffer;
      self.run += 1;
      let source = SamplesBuffer::new(2, RATE, vec![0.2f32; 400]);
      let route = match track {
        Some(n) => daw::Route::Track(self.state.playlist.mixer.track_route(n)),
        None => daw::Route::Master,
      };
      let voice = self.engine.voice(source, Some(self.run), 0, route);
      self.engine.start_run(self.run, self.state.playlist.mixer.master(), vec![voice]).unwrap();
      // all of it, so it's over before the next one
      let mut out = vec![0f32; 400];
      self.renderer.render(&mut out, std::time::Instant::now(), Duration::ZERO);
      out[200]
    }

    // what a synth's copy on `track` (none: its master copy) plays,
    // rendered: `notes` (pitch, from, to, in frames), as a run, `frames`
    // long. the left side. silence if there's no such copy
    pub fn hear_notes(&mut self, synth: u64, track: Option<u32>, notes: &[(u8, u64, u64)], frames: usize) -> Vec<f32> {
      self.run += 1;
      self.engine.start_run(self.run, self.state.playlist.mixer.master(), vec![]).unwrap();
      let timed: Vec<daw::TimedNote> = notes.iter()
        .flat_map(|&(pitch, from, to)| [daw::TimedNote::on(from, pitch, 1.), daw::TimedNote::off(to, pitch)])
        .collect();
      if let Some(&copy) = self.effects().copy_ids().get(&(synth, track)) {
        match track {
          Some(n) => self.engine.play_notes(&self.state.playlist.mixer.track_route(n), copy, Some(self.run), timed).unwrap(),
          None => self.engine.play_master_notes(copy, Some(self.run), timed).unwrap(),
        }
      }
      let mut out = vec![0f32; frames * 2];
      self.renderer.render(&mut out, std::time::Instant::now(), Duration::ZERO);
      out.chunks(2).map(|f| f[0]).collect()
    }

    // a fake synth's copy's gain (its settings), on a track (none: the
    // master copy)
    pub fn copy_gain(&self, synth: u64, track: Option<u32>) -> Option<f32> {
      let copy = *self.effects().copy_ids().get(&(synth, track))?;
      let inner = self.effects().inner.lock().unwrap();
      let plugin = inner.synths.get(&synth)?.loaded().find(|l| l.slot.id() == copy)?.plugin.clone();
      drop(inner);
      let state = plugin.lock().unwrap().state().ok()?;
      Some(f32::from_le_bytes(state.try_into().ok()?))
    }

    // a fake plugin's gain, through its settings

    pub fn gain_of(&self, slot_id: u64) -> f32 {
      let state = self.effects().state_of(slot_id).unwrap();
      f32::from_le_bytes(state.try_into().unwrap())
    }

    // a synth row's plugin's slot
    pub fn slot_of(&self, row: u64) -> u64 {
      let sequencer = self.state.playlist.sequencer.lock().unwrap();
      sequencer.row(row).and_then(|row| row.synth()).expect("a synth row").slot
    }

    // turn a fake plugin's knob (not undoable)
    pub fn turn(&self, slot_id: u64, gain: f32) {
      let hosted = self.effects().hosted(slot_id).unwrap();
      hosted.lock().unwrap().set_state(&gain.to_le_bytes()).unwrap();
    }

    // the ids in each channel's chain, in order
    pub fn layout(&self) -> Vec<(Option<u32>, Vec<u64>)> {
      self.effects().racks().into_iter().map(|r| (r.track, r.slots.iter().map(|s| s.id).collect())).collect()
    }
  }
}

#[cfg(test)]
mod tests;
