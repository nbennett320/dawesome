// the mixer's effects: every channel (each track, and the master) has a rack,
// its effects in the order the sound goes through them
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
use std::time::Duration;

use crate::daw::{self, Channel, Slot};
use crate::main_thread::{self, DropOnMain};
use crate::plugin::{self, PluginKind, Processor};
use crate::plugin_catalog::Plugins;

// how long to wait for the engine to take a chain before carrying on (with
// no output running it never does; a plugin is still safe to tear down then,
// as its processor stops calling it first)
const WAIT: Duration = Duration::from_millis(500);
// how often loaded plugins are given time on the main thread
const IDLE_EVERY: Duration = Duration::from_millis(50);
// the rate plugins are prepared for until an output is open
const DEFAULT_RATE: u32 = 44_100;

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
}

struct Vst3;

impl Loader for Vst3 {
  fn load(&self, path: &Path, id: &str, rate: u32) -> Result<Box<dyn Hosted>, String> {
    plugin::Plugin::load(path, id, rate).map(|p| Box::new(p) as Box<dyn Hosted>)
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
}

#[derive(Clone)]
struct Backend {
  loader: Arc<dyn Loader>,
  output: Arc<dyn Output>,
  wait: Duration,
}

struct Inner {
  racks: BTreeMap<Channel, Vec<Effect>>,
  // the engine the chains have been sent to, and what each of its chains
  // holds (slot ids), as far as we've sent
  engine: Option<Arc<daw::Engine>>,
  sent: BTreeMap<Channel, Vec<u64>>,
  // the output is being replaced: nothing is sent until it's back
  paused: bool,
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
}

impl Effects {
  pub fn new(mixer: Arc<daw::Mixer>) -> Effects {
    Effects {
      inner: Mutex::new(Inner {
        racks: BTreeMap::new(),
        engine: None,
        sent: BTreeMap::new(),
        paused: false,
        rate: DEFAULT_RATE,
      }),
      next_id: AtomicU64::new(1),
      loaded: AtomicUsize::new(0),
      mixer,
      plugins: Arc::new(Plugins::default()),
      backend: Mutex::new(Backend { loader: Arc::new(Vst3), output: Arc::new(LiveOutput), wait: WAIT }),
      on_unload: Mutex::new(None),
    }
  }

  // load plugins and play chains some other way (the tests)
  #[cfg(test)]
  pub fn use_backend(&self, loader: Arc<dyn Loader>, output: Arc<dyn Output>, wait: Duration) {
    *self.backend.lock().unwrap() = Backend { loader, output, wait };
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
  // racks unlocked
  pub fn on_unload(&self, f: impl Fn(u64) + Send + Sync + 'static) {
    *self.on_unload.lock().unwrap() = Some(Arc::new(f));
  }

  fn hosted(&self, slot_id: u64) -> Option<Arc<Mutex<Box<dyn Hosted>>>> {
    let inner = self.inner.lock().unwrap();
    let (channel, i) = find(&inner, slot_id)?;
    inner.racks[&channel][i].loaded().map(|l| l.plugin.clone())
  }

  // ---- changes (each one undoable: see effect_commands.rs)

  // add an installed plugin to a channel, at `index` (or the end). gives
  // the new effect's id. if the plugin can't be loaded nothing changes
  pub fn add(&self, channel: Channel, plugin_id: &str, index: Option<usize>) -> Result<u64, String> {
    main_thread::run(|| {
      let info = self.plugins.find(plugin_id)
        .ok_or_else(|| "That plugin isn't among the installed plugins. Try scanning for plugins again".to_string())?;
      if info.kind == PluginKind::Instrument {
        return Err(format!("{} is an instrument, and only effects can go in an effects rack for now", info.name));
      }
      let desc = Desc {
        id: self.next_id.fetch_add(1, Ordering::SeqCst),
        plugin_id: info.id.clone(),
        path: info.path.clone(),
        name: info.name.clone(),
        vendor: info.vendor.clone(),
        bypassed: false,
      };
      let rate = self.load_rate();
      let instance = self.load(&desc, None, rate)
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
      self.unload(engine, vec![effect]);
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
      for rack in saved {
        for effect in &rack.slots {
          let path = self.plugins.find(&effect.plugin_id).map_or_else(|| effect.path.clone(), |info| info.path);
          let desc = Desc {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            plugin_id: effect.plugin_id.clone(),
            path,
            name: effect.name.clone(),
            vendor: effect.vendor.clone(),
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
          racks.entry(channel_of(rack.track)).or_default().push(Effect { desc, instance });
        }
      }

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
        let mut copy = loader.load(Path::new(&desc.path), &desc.plugin_id, rate).map_err(failed)?;
        copy.set_state(&state).map_err(failed)?;
        let processor = copy.take_processor().ok_or_else(|| failed("it has no audio processor".into()))?;
        let slot = Slot::new(desc.id, processor);
        match chains.last_mut() {
          Some((c, slots)) if *c == channel => slots.push(slot),
          _ => chains.push((channel, vec![slot])),
        }
        plugins.push(copy);
      }
      Ok(MixdownEffects { chains, plugins: DropOnMain::new(plugins) })
    })?
  }

  // ---- the output

  // the output is about to be replaced: every chain comes off the engine
  // (waiting for it to take them), and none is sent until `attach`
  pub fn detach(&self) {
    let wait = self.backend().wait;
    let engine = {
      let mut inner = self.inner.lock().unwrap();
      inner.paused = true;
      let engine = inner.engine.take();
      if let Some(engine) = &engine {
        self.empty(engine, &mut inner);
      }
      inner.sent.clear();
      engine
    };
    if let Some(engine) = engine {
      engine.wait_for_chains(wait);
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
    let slot = Slot::new(desc.id, processor);
    slot.set_bypassed(desc.bypassed);
    let has_editor = hosted.has_editor();
    Ok(Instance::Loaded(Loaded { plugin: Arc::new(Mutex::new(hosted)), slot, has_editor, rate }))
  }

  // tear down effects that have left the racks, once the engine has let go
  // of their slots
  fn unload(&self, engine: Option<Arc<daw::Engine>>, effects: Vec<Effect>) {
    let ids: Vec<u64> = effects.iter().filter(|e| e.loaded().is_some()).map(|e| e.desc.id).collect();
    if ids.is_empty() { return; }
    if let Some(engine) = engine {
      engine.wait_for_chains(self.backend().wait);
    }
    let on_unload = self.on_unload.lock().unwrap().clone();
    if let Some(on_unload) = on_unload {
      for &id in &ids { on_unload(id); }
    }
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
        self.empty(&old, inner);
        old.wait_for_chains(backend.wait);
      }
      inner.sent.clear();
      inner.engine = engine.clone();
    }
    let Some(engine) = engine else { return };

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

// a fake plugin for tests: a gain, whose settings are the gain
#[cfg(test)]
pub mod testing {
  use super::*;
  use crate::plugin::scan::Catalog;
  use crate::plugin::{Event, PluginInfo, Transport};
  use std::sync::atomic::AtomicU32;

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

  // a gain that loads; a gain whose bundle isn't there; an instrument
  pub fn gain() -> PluginInfo { info(1, "Gain", PluginKind::Effect, "/fake/Gain.vst3") }
  pub fn absent() -> PluginInfo { info(2, "Absent", PluginKind::Effect, "/gone/Absent.vst3") }
  pub fn synth() -> PluginInfo { info(3, "Synth", PluginKind::Instrument, "/fake/Synth.vst3") }

  pub fn catalog() -> Catalog {
    Catalog { plugins: vec![gain(), absent(), synth()], ..Default::default() }
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

  pub struct FakeGain {
    gain: Arc<AtomicU32>,
    processor: Option<GainProcessor>,
    alive: Arc<AtomicUsize>,
  }

  impl Drop for FakeGain {
    fn drop(&mut self) {
      self.alive.fetch_sub(1, Ordering::SeqCst);
    }
  }

  impl Hosted for FakeGain {
    fn take_processor(&mut self) -> Option<Box<dyn Processor>> {
      self.processor.take().map(|p| Box::new(p) as Box<dyn Processor>)
    }

    fn state(&self) -> Result<Vec<u8>, String> {
      Ok(self.gain.load(Ordering::Relaxed).to_le_bytes().to_vec())
    }

    fn set_state(&mut self, blob: &[u8]) -> Result<(), String> {
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
  // `broken`), and counts them
  #[derive(Default)]
  pub struct FakeLoader {
    pub broken: std::sync::atomic::AtomicBool,
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
      Ok(Box::new(FakeGain {
        processor: Some(GainProcessor(gain.clone())),
        gain,
        alive: self.alive.clone(),
      }))
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
