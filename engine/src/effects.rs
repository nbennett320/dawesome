// the mixer's effects: every channel (each track, and the master) has a rack,
// its effects in the order the sound goes through them
//
// a track's rack can start with an instrument (a plugin whose kind is
// instrument), which plays the notes of the pattern clips on that track. a
// track has at most one, always first, and the master none. adding another
// to a track replaces the one there
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
  // "effect" or "instrument" (first in a track's chain)
  pub kind: PluginKind,
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
  // an instrument is saved as one, so it's known as one even when it can't
  // be loaded (left out for effects, as projects had it before)
  #[serde(default = "effect_kind", skip_serializing_if = "is_effect")]
  pub kind: PluginKind,
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
// marked missing
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MissingEffect {
  // none is the master
  pub track: Option<u32>,
  pub name: String,
  pub vendor: String,
  pub path: String,
  pub reason: String,
}

fn effect_kind() -> PluginKind {
  PluginKind::Effect
}

fn is_effect(kind: &PluginKind) -> bool {
  *kind == PluginKind::Effect
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
  kind: PluginKind,
  bypassed: bool,
}

impl Desc {
  fn instrument(&self) -> bool {
    self.kind == PluginKind::Instrument
  }
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

struct Effect {
  desc: Desc,
  instance: Instance,
}

impl Effect {
  fn loaded(&self) -> Option<&Loaded> {
    match &self.instance {
      Instance::Loaded(loaded) => Some(loaded),
      Instance::Missing { .. } => None,
    }
  }

  fn wire(&self) -> EffectSlot {
    EffectSlot {
      id: self.desc.id,
      plugin_id: self.desc.plugin_id.clone(),
      name: self.desc.name.clone(),
      vendor: self.desc.vendor.clone(),
      kind: self.desc.kind,
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
      }),
      next_id: AtomicU64::new(1),
      loaded: AtomicUsize::new(0),
      mixer,
      plugins: Arc::new(Plugins::default()),
      backend: Mutex::new(Backend { loader: Arc::new(Vst3), output: Arc::new(LiveOutput), wait: WAIT, settle: SETTLE }),
      on_unload: Mutex::new(None),
      unloading: Mutex::new(HashMap::new()),
      windows: Mutex::new(HashMap::new()),
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

  // an effect's name, if it's in a rack
  pub fn slot_name(&self, slot_id: u64) -> Option<String> {
    let inner = self.inner.lock().unwrap();
    find(&inner, slot_id).map(|(channel, i)| inner.racks[&channel][i].desc.name.clone())
  }

  // call `f` with an effect's plugin, if it's in a rack and loaded (and is a
  // real VST3 plugin). on the main thread only. the racks aren't locked
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

  // an effect's plugin, while it's in a rack or being unloaded
  fn hosted(&self, slot_id: u64) -> Option<Arc<Mutex<Box<dyn Hosted>>>> {
    {
      let inner = self.inner.lock().unwrap();
      if let Some((channel, i)) = find(&inner, slot_id) {
        return inner.racks[&channel][i].loaded().map(|l| l.plugin.clone());
      }
    }
    self.unloading.lock().unwrap().get(&slot_id).cloned()
  }

  // ---- changes (each one undoable: see effect_commands.rs)

  // the instrument that adding this plugin to this channel would replace,
  // if it's an instrument and the channel has one
  pub fn replaced_by(&self, channel: Channel, plugin_id: &str) -> Option<u64> {
    let info = self.plugins.find(plugin_id)?;
    if info.kind != PluginKind::Instrument { return None; }
    let inner = self.inner.lock().unwrap();
    inner.racks.get(&channel)?.first().filter(|e| e.desc.instrument()).map(|e| e.desc.id)
  }

  // add an installed plugin to a channel, at `index` (or the end), but
  // never above an instrument. an instrument goes first, replacing the
  // track's instrument if it has one (`index` doesn't matter). gives the new
  // effect's id. if the plugin can't be loaded nothing changes
  pub fn add(&self, channel: Channel, plugin_id: &str, index: Option<usize>) -> Result<u64, String> {
    main_thread::run(|| {
      let info = self.plugins.find(plugin_id)
        .ok_or_else(|| "That plugin isn't among the installed plugins. Try scanning for plugins again".to_string())?;
      if info.kind == PluginKind::Instrument && channel == Channel::Master {
        return Err(NO_MASTER_INSTRUMENT.into());
      }
      let desc = Desc {
        id: self.next_id.fetch_add(1, Ordering::SeqCst),
        plugin_id: info.id.clone(),
        path: info.path.clone(),
        name: info.name.clone(),
        vendor: info.vendor.clone(),
        kind: info.kind,
        bypassed: false,
      };
      let rate = self.load_rate();
      let instance = self.load(&desc, None, rate)
        .map_err(|e| format!("{} couldn't be loaded: {}", info.name, e))?;
      let id = desc.id;

      let mut inner = self.inner.lock().unwrap();
      let rack = inner.racks.entry(channel).or_default();
      let has_instrument = rack.first().map_or(false, |e| e.desc.instrument());
      let mut replaced = Vec::new();
      if desc.instrument() {
        if has_instrument { replaced.push(rack.remove(0)); }
        rack.insert(0, Effect { desc, instance });
      } else {
        let lowest = if has_instrument { 1 } else { 0 };
        let at = index.unwrap_or(rack.len()).clamp(lowest, rack.len());
        rack.insert(at, Effect { desc, instance });
      }
      self.sync(&mut inner);
      let engine = inner.engine.clone();
      drop(inner);
      self.unload(engine, replaced);
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
      self.unload(engine, vec![effect]);
      Ok(())
    })?
  }

  // move an effect within its channel or to another, keeping its plugin
  // and settings. `index` is where it ends up in that channel's chain, but
  // never above an instrument. an instrument stays first: moved to another
  // track it becomes that track's instrument, if it hasn't one already
  pub fn move_to(&self, slot_id: u64, channel: Channel, index: usize) -> Result<(), String> {
    main_thread::run(|| {
      let mut inner = self.inner.lock().unwrap();
      let (from, i) = find(&inner, slot_id).ok_or_else(gone)?;
      let instrument = inner.racks[&from][i].desc.instrument();
      if instrument && from != channel {
        let taken = inner.racks.get(&channel).and_then(|rack| rack.first()).map_or(false, |e| e.desc.instrument());
        match channel {
          Channel::Master => return Err(NO_MASTER_INSTRUMENT.into()),
          Channel::Track(n) if taken => return Err(format!("Track {} already has an instrument", n + 1)),
          Channel::Track(_) => {}
        }
      }

      let effect = inner.racks.get_mut(&from).unwrap().remove(i);
      let rack = inner.racks.entry(channel).or_default();
      let at = if instrument { 0 } else {
        let lowest = if rack.first().map_or(false, |e| e.desc.instrument()) { 1 } else { 0 };
        index.clamp(lowest, rack.len())
      };
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
      let (channel, i) = find(&inner, slot_id)?;
      match &inner.racks[&channel][i].instance {
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
        let instance = self.load(&snap.desc, snap.state.as_deref(), rate).unwrap_or_else(|reason| Instance::Missing {
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
      self.unload(engine, here.into_values().collect());
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
            kind: desc.kind,
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

  // replace every rack with a project's. a plugin is found in the catalog
  // by its id (or at the path it was saved with); one that can't be loaded
  // is kept as missing, and listed
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
          let path = found.as_ref().map_or_else(|| effect.path.clone(), |info| info.path.clone());
          let desc = Desc {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            plugin_id: effect.plugin_id.clone(),
            path,
            name: effect.name.clone(),
            vendor: effect.vendor.clone(),
            // (the catalog knows best, but a missing plugin is as it was saved)
            kind: found.map_or(effect.kind, |info| info.kind),
            bypassed: effect.bypassed,
          };
          let state = base64::decode(&effect.state);
          let loaded = match &state {
            Ok(state) => self.load(&desc, Some(state).filter(|s| !s.is_empty()).map(|s| &s[..]), rate),
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
      // a track's instrument first, whatever order the file has (sort_by_key
      // keeps the rest in order)
      for effects in racks.values_mut() {
        effects.sort_by_key(|e| !e.desc.instrument());
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

  // take every effect out
  pub fn clear(&self) {
    if let Err(e) = main_thread::run(|| self.clear_here()) {
      println!("the effects weren't cleared: {}", e);
    }
  }

  fn clear_here(&self) {
    let mut inner = self.inner.lock().unwrap();
    let doomed: Vec<Effect> = std::mem::take(&mut inner.racks).into_values().flatten().collect();
    self.sync(&mut inner);
    let engine = inner.engine.clone();
    drop(inner);
    self.unload(engine, doomed);
  }

  // ---- mixdown

  // a second instance of every effect that's loaded and switched on, made at
  // `rate` with the live one's settings copied over, in chains for another
  // engine. the live effects aren't touched
  pub fn mixdown_chains(&self, rate: u32) -> Result<MixdownEffects, String> {
    // a plugin loaded just now may not have set itself up yet, and its
    // settings then may not be the ones it'll have (a sampler that hasn't
    // loaded its sounds says it's empty, and a copy given that stays empty).
    // so wait until each one is SETTLE old (a mixdown starts away from the
    // main thread, which they need meanwhile)
    let settle = self.backend().settle;
    let youngest = {
      let inner = self.inner.lock().unwrap();
      inner.racks.values().flatten()
        .filter(|e| !e.desc.bypassed)
        .filter_map(|e| e.loaded().map(|l| l.since))
        .max()
    };
    if let Some(since) = youngest {
      let wait = settle.saturating_sub(since.elapsed());
      if !wait.is_zero() { std::thread::sleep(wait); }
    }

    main_thread::run(|| {
      let live: Vec<(Channel, Desc, Arc<Mutex<Box<dyn Hosted>>>)> = {
        let inner = self.inner.lock().unwrap();
        inner.racks.iter()
          .flat_map(|(channel, effects)| effects.iter().map(move |e| (*channel, e)))
          .filter(|(_, e)| !e.desc.bypassed)
          .filter_map(|(channel, e)| e.loaded().map(|l| (channel, e.desc.clone(), l.plugin.clone())))
          .collect()
      };

      let loader = self.backend().loader;
      let mut plugins = Vec::new();
      let mut chains: Vec<(Channel, Vec<Arc<Slot>>)> = Vec::new();
      for (channel, desc, hosted) in live {
        let failed = |e: String| format!("{} couldn't be loaded for the mixdown: {}", desc.name, e);
        let state = hosted.lock().unwrap().state().map_err(failed)?;
        let mut copy = loader.load_offline(Path::new(&desc.path), &desc.plugin_id, rate).map_err(failed)?;
        copy.set_state(&state).map_err(failed)?;
        let processor = copy.take_processor().ok_or_else(|| failed("it has no audio processor".into()))?;
        let slot = if desc.instrument() { Slot::new_instrument(desc.id, processor) } else { Slot::new(desc.id, processor) };
        match chains.last_mut() {
          Some((c, slots)) if *c == channel => slots.push(slot),
          _ => chains.push((channel, vec![slot])),
        }
        plugins.push(copy);
      }
      let settle = if plugins.is_empty() { Duration::ZERO } else { settle };
      Ok(MixdownEffects { chains, plugins: DropOnMain::new(plugins), settle })
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
  // parameter changes on, and restarts if it asked to)
  pub fn idle(&self) {
    let _ = main_thread::run(|| {
      let plugins: Vec<Arc<Mutex<Box<dyn Hosted>>>> = {
        let inner = self.inner.lock().unwrap();
        inner.racks.values().flatten().filter_map(|e| e.loaded().map(|l| l.plugin.clone())).collect()
      };
      for plugin in plugins {
        // (one busy elsewhere on this thread, its window say, waits for
        // the next round)
        if let Ok(mut plugin) = plugin.try_lock() {
          plugin.idle();
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

  // load an effect's plugin, ready for a chain
  fn load(&self, desc: &Desc, state: Option<&[u8]>, rate: u32) -> Result<Instance, String> {
    let loader = self.backend().loader;
    let mut hosted = loader.load(Path::new(&desc.path), &desc.plugin_id, rate)?;
    if let Some(state) = state {
      hosted.set_state(state).map_err(|e| format!("its settings couldn't be put back ({})", e))?;
    }
    let processor = hosted.take_processor().ok_or_else(|| "it has no audio processor".to_string())?;
    let slot = if desc.instrument() { Slot::new_instrument(desc.id, processor) } else { Slot::new(desc.id, processor) };
    slot.set_bypassed(desc.bypassed);
    let has_editor = hosted.has_editor();
    Ok(Instance::Loaded(Loaded { plugin: Arc::new(Mutex::new(hosted)), slot, has_editor, rate, since: Instant::now() }))
  }

  // tear down effects that have left the racks, once the engine has let go
  // of their slots. the unload callbacks run first, while with_plugin can
  // still reach each plugin (through `unloading`), so a window showing one
  // is closed with its plugin there to let go of it
  fn unload(&self, engine: Option<Arc<daw::Engine>>, effects: Vec<Effect>) {
    let leaving: Vec<(u64, Arc<Mutex<Box<dyn Hosted>>>)> = effects.iter()
      .filter_map(|e| e.loaded().map(|l| (e.desc.id, l.plugin.clone())))
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
      inner.racks.get(&channel)?.iter().find(|e| e.desc.id == *id)?.loaded().map(|l| l.slot.clone())
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
    let loaded = inner.racks.values().flatten().filter(|e| e.loaded().is_some()).count();
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
    for effect in inner.racks.values_mut().flatten() {
      if let Instance::Loaded(loaded) = &mut effect.instance {
        if loaded.rate != rate {
          if let Err(e) = loaded.plugin.lock().unwrap().set_sample_rate(rate) {
            println!("{} can't run at {} Hz: {}", effect.desc.name, rate, e);
          }
          loaded.rate = rate;
        }
      }
    }

    let wanted: BTreeMap<Channel, Vec<u64>> = inner.racks.iter()
      .map(|(channel, effects)| (*channel, effects.iter().filter(|e| e.loaded().is_some()).map(|e| e.desc.id).collect()))
      .collect();
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

pub const NO_MASTER_INSTRUMENT: &str = "The master can't hold an instrument";

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
      Ok(Box::new(FakeGain { processor: Some(processor), gain, alive: self.alive.clone(), ready_at, empty }))
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
    run: u64,
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

    // what a track's instrument plays, rendered: `notes` (pitch, from, to,
    // in frames) on track `track`, as a run, `frames` long. the left side
    pub fn hear_notes(&mut self, track: u32, notes: &[(u8, u64, u64)], frames: usize) -> Vec<f32> {
      self.run += 1;
      let route = self.state.playlist.mixer.track_route(track);
      self.engine.start_run(self.run, self.state.playlist.mixer.master(), vec![]).unwrap();
      let timed = notes.iter()
        .flat_map(|&(pitch, from, to)| [daw::TimedNote::on(from, pitch, 1.), daw::TimedNote::off(to, pitch)])
        .collect();
      self.engine.play_notes(&route, Some(self.run), timed).unwrap();
      let mut out = vec![0f32; frames * 2];
      self.renderer.render(&mut out, std::time::Instant::now(), Duration::ZERO);
      out.chunks(2).map(|f| f[0]).collect()
    }

    // a fake plugin's gain, through its settings

    pub fn gain_of(&self, slot_id: u64) -> f32 {
      let state = self.effects().state_of(slot_id).unwrap();
      f32::from_le_bytes(state.try_into().unwrap())
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
mod tests {
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
      kind: PluginKind::Effect,
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
    // replaced by another instrument
    let s = effects.add(Channel::Track(0), &synth().id, None).unwrap();
    effects.add(Channel::Track(0), &synth().id, None).unwrap();
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
  fn test_an_instrument_goes_first_and_replaces_the_one_there() {
    let rig = rig();
    let effects = rig.effects();
    let a = effects.add(Channel::Track(1), &gain().id, None).unwrap();
    let s = effects.add(Channel::Track(1), &synth().id, Some(5)).unwrap();
    assert_eq!(rig.layout(), vec![(Some(1), vec![s, a])]);
    let json = serde_json::to_value(effects.racks()).unwrap();
    assert_eq!(json[0]["slots"][0]["kind"], "instrument");
    assert_eq!(json[0]["slots"][1]["kind"], "effect");

    // an effect can't go above it
    let b = effects.add(Channel::Track(1), &gain().id, Some(0)).unwrap();
    assert_eq!(rig.layout(), vec![(Some(1), vec![s, b, a])]);

    // another instrument replaces it, and it's torn down
    assert_eq!(effects.replaced_by(Channel::Track(1), &synth().id), Some(s));
    assert_eq!(effects.replaced_by(Channel::Track(1), &gain().id), None);
    assert_eq!(effects.replaced_by(Channel::Track(2), &synth().id), None);
    let alive = rig.loader.alive.load(Ordering::SeqCst);
    let t = effects.add(Channel::Track(1), &synth().id, None).unwrap();
    assert_eq!(rig.layout(), vec![(Some(1), vec![t, b, a])]);
    assert_eq!(rig.loader.alive.load(Ordering::SeqCst), alive);

    assert_eq!(effects.add(Channel::Master, &synth().id, None).unwrap_err(), NO_MASTER_INSTRUMENT);
  }

  #[test]
  fn test_moving_keeps_an_instrument_first_and_one_to_a_track() {
    let rig = rig();
    let effects = rig.effects();
    let s = effects.add(Channel::Track(0), &synth().id, None).unwrap();
    let a = effects.add(Channel::Track(0), &gain().id, None).unwrap();
    let t = effects.add(Channel::Track(2), &synth().id, None).unwrap();
    let m = effects.add(Channel::Master, &gain().id, None).unwrap();

    // within its chain it stays first; an effect can't go above it
    effects.move_to(s, Channel::Track(0), 1).unwrap();
    effects.move_to(a, Channel::Track(0), 0).unwrap();
    assert_eq!(rig.layout()[0], (Some(0), vec![s, a]));

    // not to the master, nor to a track that has one
    assert_eq!(effects.move_to(s, Channel::Master, 0).unwrap_err(), NO_MASTER_INSTRUMENT);
    assert_eq!(effects.move_to(s, Channel::Track(2), 0).unwrap_err(), "Track 3 already has an instrument");
    // to a track without one, it becomes that track's, first
    effects.move_to(m, Channel::Track(4), 0).unwrap();
    effects.move_to(s, Channel::Track(4), 1).unwrap();
    // an effect moved in goes after it
    effects.move_to(a, Channel::Track(2), 0).unwrap();
    assert_eq!(rig.layout(), vec![(Some(2), vec![t, a]), (Some(4), vec![s, m])]);
  }

  #[test]
  fn test_an_instrument_plays_notes_over_the_tracks_clips() {
    let mut rig = rig();
    let s = rig.effects().add(Channel::Track(0), &synth().id, None).unwrap();
    rig.turn(s, 0.3);
    // a clip on the track passes through it
    assert!(close(rig.level(Some(0)), 0.2));

    let out = rig.hear_notes(0, &[(60, 100, 300)], 400);
    assert_eq!(out.iter().position(|&s| s != 0.), Some(100));
    assert!(close(out[200], 0.3));
    assert!(out[300..].iter().all(|&s| s == 0.));
    // only its own track
    assert!(rig.hear_notes(1, &[(60, 0, 300)], 400).iter().all(|&s| s == 0.));
    // and through the effects after it
    rig.effects().add(Channel::Track(0), &gain().id, None).unwrap();
    assert!(close(rig.hear_notes(0, &[(60, 0, 300)], 400)[200], 0.3 * START_GAIN));
  }

  #[test]
  fn test_an_instrument_is_saved_and_one_that_cant_load_stays_first_and_an_instrument() {
    let rig = rig();
    let effects = rig.effects();
    let a = effects.add(Channel::Track(0), &gain().id, None).unwrap();
    let s = effects.add(Channel::Track(0), &synth().id, None).unwrap();
    rig.turn(s, 0.125);
    let saved = effects.saved();
    let json = serde_json::to_value(&saved).unwrap();
    assert_eq!(json[0]["slots"][0]["kind"], "instrument");
    // effects are saved as they were before instruments
    assert!(json[0]["slots"][1].get("kind").is_none());
    assert_eq!(saved[0].slots[1].plugin_id, gain().id);
    let _ = (a, s);

    // loaded back: the instrument first, with its settings
    let missing = effects.load_saved(&saved);
    assert!(missing.is_empty());
    let racks = effects.racks();
    assert_eq!(racks[0].slots[0].kind, PluginKind::Instrument);
    assert!(close(rig.gain_of(racks[0].slots[0].id), 0.125));

    // a file with it second, whose instrument isn't installed (nor in the
    // catalog): it's put first, still an instrument, and kept as saved
    let mut odd = saved.clone();
    odd[0].slots.reverse();
    odd[0].slots[1].plugin_id = "vst3:unknown".into();
    odd[0].slots[1].path = "/gone/Other.vst3".into();
    let missing = effects.load_saved(&odd);
    assert_eq!(missing.len(), 1);
    let racks = effects.racks();
    assert_eq!(racks[0].slots[0].kind, PluginKind::Instrument);
    assert!(racks[0].slots[0].missing);
    assert_eq!(racks[0].slots[1].kind, PluginKind::Effect);
    assert_eq!(effects.saved()[0].slots[0], odd[0].slots[1]);

    // a project from before instruments loads its effects as effects
    let old: Vec<SavedRack> = serde_json::from_value(serde_json::json!([
      { "track": 0, "slots": [{ "pluginId": "vst3:unknown", "name": "Old", "path": "/gone/Old.vst3" }] },
    ])).unwrap();
    assert_eq!(old[0].slots[0].kind, PluginKind::Effect);
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

    let mixdown = rig.effects().mixdown_chains(96_000).unwrap();
    assert_eq!(mixdown.chains.len(), 1);
    drop(mixdown);

    rig.effects().remove(id).unwrap();
    assert!((rms(&hear(&rig, 1)) - dry).abs() < 1e-2);
    running.store(false, Ordering::SeqCst);
    render.join().unwrap();
  }
}
