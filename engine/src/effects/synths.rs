// the synth rows' plugins
//
// the synth rows themselves (names, volume, pan, mute, order) are the
// sequencer's (see daw_core/sequencer.rs), and undo and projects go through
// them. this keeps each synth's plugins in line with those rows:
// follow_synths is told the synths (by slot id) and which tracks play
// which, and loads or lets go of plugins to match
//
// one plugin instance mixes every note it's sent into one output, so a
// synth that two tracks play needs two instances, or each track would hear
// the other's notes. so each synth has copies of its plugin:
//
//   - its master copy, first in the master's chain, which plays a pattern on
//     its own and keys auditioned. its slot id is the synth's slot, and its
//     interface is the one shown (with_plugin, slot_name and so on reach it)
//   - a copy on each track that has a clip with notes for it, first in that
//     track's chain (before its effects), with a slot id of its own
//
// a copy is made as soon as a track needs it, as instruments take a second
// or more to load their sounds. one no track needs any more is taken out of
// its chain and kept a while (SPARE_FOR) in case it's needed again (a clip
// moved back, an undo), then torn down
//
// the master copy's settings are passed on to the other copies: while its
// interface is showing (and once more when it's closed), about once a
// second, and always before playing, a mixdown or a save (push_synth_states)
//
// a synth whose plugin can't be loaded is kept, missing, with its settings:
// it has no copies, and its notes aren't heard
use super::*;
use crate::daw::SynthStatus;

// how often a synth whose interface is showing has its settings passed on
const PUSH_EVERY: Duration = Duration::from_secs(1);
// how long a copy no track needs is kept, in case one needs it again
const SPARE_FOR: Duration = Duration::from_secs(30);

// a synth's plugins
pub(super) struct LiveSynth {
  pub(super) desc: Desc,
  pub(super) volume: f32,
  pub(super) pan: f32,
  pub(super) muted: bool,
  pub(super) master: Instance,
  // by track
  copies: BTreeMap<u32, Copy>,
  // when its settings were last passed on, and whether its interface was
  // showing then
  pushed_at: Option<Instant>,
  was_open: bool,
}

struct Copy {
  id: u64,
  loaded: Loaded,
  // the settings it was last given
  given: Vec<u8>,
  // since when no track has needed it (none: one does)
  spare_since: Option<Instant>,
}

impl LiveSynth {
  fn gain(&self) -> f32 {
    if self.muted { 0. } else { self.volume }
  }

  // its volume, mute and pan, on every copy
  fn set_gains(&self) {
    let gain = self.gain();
    for loaded in self.loaded() {
      loaded.slot.set_gain(gain);
      loaded.slot.set_pan(self.pan);
    }
  }

  pub(super) fn loaded(&self) -> impl Iterator<Item = &Loaded> {
    self.master.loaded().into_iter().chain(self.copies.values().map(|copy| &copy.loaded))
  }

  pub(super) fn loaded_mut(&mut self) -> impl Iterator<Item = &mut Loaded> {
    let master = match &mut self.master {
      Instance::Loaded(loaded) => Some(loaded),
      Instance::Missing { .. } => None,
    };
    master.into_iter().chain(self.copies.values_mut().map(|copy| &mut copy.loaded))
  }

  // every plugin, by slot id, to be torn down (see Effects::unload)
  pub(super) fn into_loaded(self) -> Vec<(u64, Loaded)> {
    let id = self.desc.id;
    let mut all: Vec<(u64, Loaded)> = self.copies.into_values().map(|copy| (copy.id, copy.loaded)).collect();
    if let Instance::Loaded(loaded) = self.master { all.push((id, loaded)); }
    all
  }
}

// what the synths were last brought in line with
#[derive(Clone, PartialEq)]
pub(super) struct Followed {
  list: Vec<daw::Synth>,
  needed: BTreeSet<(u64, u32)>,
}

// a synth's copy's slot (its master's too), by slot id
pub(super) fn slot_in(inner: &Inner, id: u64) -> Option<Arc<Slot>> {
  if let Some(synth) = inner.synths.get(&id) {
    return synth.master.loaded().map(|l| l.slot.clone());
  }
  inner.synths.values()
    .flat_map(|synth| synth.copies.values())
    .find(|copy| copy.id == id)
    .map(|copy| copy.loaded.slot.clone())
}

// the copies at the head of each channel's chain, in the list's order: the
// master copies on the master, each track's copies on it
pub(super) fn chains(inner: &Inner) -> BTreeMap<Channel, Vec<u64>> {
  let mut chains: BTreeMap<Channel, Vec<u64>> = BTreeMap::new();
  for synth in inner.synth_order.iter().filter_map(|id| inner.synths.get(id)) {
    if synth.master.loaded().is_some() {
      chains.entry(Channel::Master).or_default().push(synth.desc.id);
    }
    for (track, copy) in synth.copies.iter().filter(|(_, copy)| copy.spare_since.is_none()) {
      chains.entry(Channel::Track(*track)).or_default().push(copy.id);
    }
  }
  chains
}

// the synths aren't followed while one of these is held (see pause_synths)
pub struct SynthPause<'a>(&'a Effects);

impl Drop for SynthPause<'_> {
  fn drop(&mut self) {
    self.0.synth_pauses.fetch_sub(1, Ordering::SeqCst);
  }
}

impl daw::SynthPlugins for Effects {
  fn synth_status(&self, id: u64) -> SynthStatus {
    let inner = self.inner.lock().unwrap();
    let loaded = inner.synths.get(&id).and_then(|synth| synth.master.loaded());
    SynthStatus { missing: loaded.is_none(), has_editor: loaded.map_or(false, |l| l.has_editor) }
  }
}

impl Effects {
  // stop following the synth rows until what's returned is dropped (while
  // a project replaces them, plugins first and rows after)
  pub fn pause_synths(&self) -> SynthPause<'_> {
    self.synth_pauses.fetch_add(1, Ordering::SeqCst);
    SynthPause(self)
  }

  // load an installed instrument as a new synth's master copy, under a new
  // slot id. it's heard once a synth row has it (see follow_synths). gives
  // the synth for the row (named after the plugin)
  pub fn load_synth(&self, plugin_id: &str) -> Result<daw::Synth, String> {
    main_thread::run(|| {
      let info = self.plugins.find(plugin_id)
        .ok_or_else(|| "That plugin isn't among the installed plugins. Try scanning for plugins again".to_string())?;
      if info.kind != PluginKind::Instrument {
        return Err(format!("{} isn't an instrument: add it to a mixer channel's effects", info.name));
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
      let master = self.load(&desc, None, rate, true)
        .map_err(|e| format!("{} couldn't be loaded: {}", info.name, e))?;
      let synth = daw::Synth {
        id: desc.id,
        plugin_id: info.id.clone(),
        path: info.path.clone(),
        name: info.name.clone(),
        vendor: info.vendor.clone(),
        volume: daw::ROW_VOLUME,
        pan: 0.,
        muted: false,
      };
      let mut inner = self.inner.lock().unwrap();
      inner.synths.insert(desc.id, live(desc, &synth, master));
      self.loaded.store(all_loaded(&inner).count(), Ordering::SeqCst);
      Ok(synth)
    })?
  }

  // a project's synth rows' plugins, with their settings, replacing any
  // there are (the rows follow: see follow_synths). one whose plugin can't
  // be loaded is kept as missing, with its settings, and listed
  pub fn load_synths(&self, rows: &[daw::SavedRow]) -> Vec<MissingEffect> {
    let saved: Vec<(&daw::SavedRow, &daw::SavedSynth)> = rows.iter()
      .filter_map(|row| Some((row, row.instrument.synth()?)))
      .collect();
    let done = main_thread::run(|| {
      let old: Vec<(u64, Loaded)> = {
        let mut inner = self.inner.lock().unwrap();
        inner.synth_order.clear();
        inner.followed = None;
        std::mem::take(&mut inner.synths).into_values().flat_map(LiveSynth::into_loaded).collect()
      };
      if !old.is_empty() {
        let engine = {
          let mut inner = self.inner.lock().unwrap();
          self.sync(&mut inner);
          inner.engine.clone()
        };
        self.unload(engine, old);
      }

      let rate = if saved.is_empty() { DEFAULT_RATE } else { self.load_rate() };
      let mut missing = Vec::new();
      let mut loaded = Vec::new();
      for &(row, synth) in &saved {
        let found = self.plugins.find(&synth.plugin_id);
        let path = found.as_ref().map_or_else(|| synth.path.clone(), |info| info.path.clone());
        let desc = Desc {
          id: synth.slot,
          plugin_id: synth.plugin_id.clone(),
          path,
          name: synth.name.clone(),
          vendor: synth.vendor.clone(),
          bypassed: false,
        };
        let state = base64::decode(&synth.state);
        let master = match (&state, &found) {
          (Ok(_), Some(info)) if info.kind != PluginKind::Instrument => Err("it isn't an instrument".to_string()),
          (Ok(state), _) => self.load(&desc, Some(state).filter(|s| !s.is_empty()).map(|s| &s[..]), rate, true),
          (Err(_), _) => Err("its saved settings are damaged".to_string()),
        };
        let master = master.unwrap_or_else(|reason| {
          missing.push(MissingEffect {
            track: None,
            name: synth.name.clone(),
            vendor: synth.vendor.clone(),
            path: synth.path.clone(),
            reason: reason.clone(),
          });
          Instance::Missing { state: state.clone().unwrap_or_default(), reason }
        });
        let desc = match master {
          Instance::Missing { .. } => Desc { path: synth.path.clone(), ..desc },
          Instance::Loaded(_) => desc,
        };
        if let Some(position) = synth.window {
          self.keep_window_position(desc.id, position);
        }
        let spec = daw::Synth {
          id: synth.slot,
          plugin_id: synth.plugin_id.clone(),
          path: desc.path.clone(),
          name: synth.name.clone(),
          vendor: synth.vendor.clone(),
          volume: row.volume,
          pan: row.pan,
          muted: row.muted,
        };
        loaded.push(live(desc, &spec, master));
      }

      let mut inner = self.inner.lock().unwrap();
      for synth in loaded {
        inner.synths.insert(synth.desc.id, synth);
      }
      self.loaded.store(all_loaded(&inner).count(), Ordering::SeqCst);
      missing
    });
    done.unwrap_or_else(|e| {
      println!("the project's synths weren't loaded: {}", e);
      Vec::new()
    })
  }

  // bring the synths' plugins in line with the synth rows' (`list`, by
  // slot) and the (slot, track) pairs that need a copy: load the synths new
  // to the list (with the settings they had, if they've been in it before),
  // let go of ones that have left it (keeping their settings), make the
  // copies that are needed and put aside the ones that aren't, and set
  // every copy's volume
  pub fn follow_synths(&self, list: &[daw::Synth], needed: &BTreeSet<(u64, u32)>) {
    if self.synth_pauses.load(Ordering::SeqCst) > 0 { return; }
    {
      let inner = self.inner.lock().unwrap();
      let same = inner.followed.as_ref().map_or(list.is_empty() && needed.is_empty(), |f| f.list == list && f.needed == *needed)
        && inner.synths.len() == list.len()
        && list.iter().all(|s| inner.synths.contains_key(&s.id));
      if same { return; }
    }
    if let Err(e) = main_thread::run(|| self.follow_here(list, needed)) {
      println!("the synths weren't brought in line: {}", e);
    }
  }

  fn follow_here(&self, list: &[daw::Synth], needed: &BTreeSet<(u64, u32)>) {
    let ids: BTreeSet<u64> = list.iter().map(|s| s.id).collect();
    let coming: Vec<&daw::Synth> = {
      let inner = self.inner.lock().unwrap();
      list.iter().filter(|s| !inner.synths.contains_key(&s.id)).collect()
    };
    // (the output's rate, found once, and only if something's loaded)
    let mut known_rate = None;
    let mut rate = || *known_rate.get_or_insert_with(|| self.load_rate());

    // synths back in the list (an undo), with the settings they went with
    let mut arrived = Vec::new();
    for synth in coming {
      let state = self.kept_synths.lock().unwrap().remove(&synth.id).unwrap_or_default();
      let desc = desc_of(synth);
      let master = self.load(&desc, Some(&state[..]).filter(|s| !s.is_empty()), rate(), true)
        .unwrap_or_else(|reason| Instance::Missing { state, reason });
      arrived.push(live(desc, synth, master));
    }

    // and ones that have left it, with the settings they had
    let leaving: Vec<LiveSynth> = {
      let mut inner = self.inner.lock().unwrap();
      for synth in arrived {
        inner.synths.insert(synth.desc.id, synth);
      }
      let gone: Vec<u64> = inner.synths.keys().copied().filter(|id| !ids.contains(id)).collect();
      gone.iter().filter_map(|id| inner.synths.remove(id)).collect()
    };
    for synth in &leaving {
      let state = match &synth.master {
        Instance::Loaded(loaded) => loaded.plugin.lock().unwrap().state()
          .map_err(|e| println!("{}'s settings couldn't be kept: {}", synth.desc.name, e))
          .unwrap_or_default(),
        Instance::Missing { state, .. } => state.clone(),
      };
      self.kept_synths.lock().unwrap().insert(synth.desc.id, state);
    }

    // the copies wanted: made where there's none, put aside where no track
    // needs one any more
    let to_make: Vec<(u64, u32, Desc, Arc<Mutex<Box<dyn Hosted>>>)> = {
      let mut inner = self.inner.lock().unwrap();
      let now = Instant::now();
      for (id, synth) in inner.synths.iter_mut() {
        for (track, copy) in synth.copies.iter_mut() {
          let wanted = needed.contains(&(*id, *track));
          copy.spare_since = match (wanted, copy.spare_since) {
            (true, _) => None,
            (false, None) => Some(now),
            (false, since) => since,
          };
        }
      }
      needed.iter().filter_map(|&(id, track)| {
        let synth = inner.synths.get(&id)?;
        if synth.copies.contains_key(&track) { return None; }
        let master = synth.master.loaded()?;
        Some((id, track, synth.desc.clone(), master.plugin.clone()))
      }).collect()
    };
    let mut states: HashMap<u64, Vec<u8>> = HashMap::new();
    let mut made = Vec::new();
    for (id, track, desc, master) in to_make {
      let state = states.entry(id).or_insert_with(|| master.lock().unwrap().state().unwrap_or_default()).clone();
      let desc = Desc { id: self.new_id(), ..desc };
      match self.load(&desc, Some(&state[..]).filter(|s| !s.is_empty()), rate(), true) {
        Ok(Instance::Loaded(loaded)) => made.push((id, track, Copy { id: desc.id, loaded, given: state, spare_since: None })),
        Ok(Instance::Missing { .. }) => {}
        Err(e) => println!("{} couldn't be loaded for track {}: {}", desc.name, track + 1, e),
      }
    }

    let mut inner = self.inner.lock().unwrap();
    for (id, track, copy) in made {
      if let Some(synth) = inner.synths.get_mut(&id) {
        synth.copies.insert(track, copy);
      }
    }
    for spec in list {
      if let Some(synth) = inner.synths.get_mut(&spec.id) {
        synth.desc.name = spec.name.clone();
        synth.volume = spec.volume;
        synth.pan = spec.pan;
        synth.muted = spec.muted;
        synth.set_gains();
      }
    }
    inner.synth_order = list.iter().map(|s| s.id).collect();
    inner.followed = Some(Followed { list: list.to_vec(), needed: needed.clone() });
    self.sync(&mut inner);
    let engine = inner.engine.clone();
    drop(inner);
    self.unload(engine, leaving.into_iter().flat_map(LiveSynth::into_loaded).collect());
  }

  // the slot id of each synth's copy that plays: (synth, none) its master
  // copy, (synth, track) its copy on that track
  pub fn copy_ids(&self) -> daw::CopyIds {
    let inner = self.inner.lock().unwrap();
    let mut ids = daw::CopyIds::new();
    for synth in inner.synth_order.iter().filter_map(|id| inner.synths.get(id)) {
      let id = synth.desc.id;
      if synth.master.loaded().is_none() { continue; }
      ids.insert((id, None), id);
      for (track, copy) in synth.copies.iter().filter(|(_, copy)| copy.spare_since.is_none()) {
        ids.insert((id, Some(*track)), copy.id);
      }
    }
    ids
  }

  // pass each synth's master copy's settings on to its other copies, where
  // they differ: every synth's (`every`), or those whose interface is
  // showing or was just closed, at most once a PUSH_EVERY
  pub fn push_synth_states(&self, every: bool) {
    let due: Vec<u64> = {
      let mut inner = self.inner.lock().unwrap();
      let now = Instant::now();
      inner.synths.iter_mut().filter_map(|(id, synth)| {
        if synth.copies.is_empty() { return None; }
        let master = synth.master.loaded()?;
        if !every && synth.pushed_at.map_or(false, |at| now - at < PUSH_EVERY) { return None; }
        // (one busy on the main thread, its window say, counts as showing)
        let open = master.plugin.try_lock().map_or(true, |plugin| plugin.editor_open());
        let due = every || open || synth.was_open;
        synth.was_open = open;
        if due { synth.pushed_at = Some(now); }
        due.then_some(*id)
      }).collect()
    };
    if due.is_empty() { return; }
    let pushed = main_thread::run(|| {
      for id in due { self.push_here(id); }
    });
    if let Err(e) = pushed {
      println!("the synths' settings weren't passed on: {}", e);
    }
  }

  fn push_here(&self, id: u64) {
    let (master, copies) = {
      let inner = self.inner.lock().unwrap();
      let Some(synth) = inner.synths.get(&id) else { return };
      let Some(master) = synth.master.loaded() else { return };
      let copies: Vec<(u64, Arc<Mutex<Box<dyn Hosted>>>)> = synth.copies.values()
        .map(|copy| (copy.id, copy.loaded.plugin.clone()))
        .collect();
      (master.plugin.clone(), copies)
    };
    let state = match master.lock().unwrap().state() {
      // (empty: it hasn't set itself up yet)
      Ok(state) if !state.is_empty() => state,
      Ok(_) => return,
      Err(e) => return println!("a synth's settings couldn't be read: {}", e),
    };
    for (copy_id, plugin) in copies {
      let differs = {
        let inner = self.inner.lock().unwrap();
        inner.synths.get(&id).and_then(|s| s.copies.values().find(|c| c.id == copy_id)).map_or(false, |c| c.given != state)
      };
      if !differs { continue; }
      if let Err(e) = plugin.lock().unwrap().set_state(&state) {
        println!("a synth's settings couldn't be passed on to a copy: {}", e);
        continue;
      }
      let mut inner = self.inner.lock().unwrap();
      if let Some(copy) = inner.synths.get_mut(&id).and_then(|s| s.copies.values_mut().find(|c| c.id == copy_id)) {
        copy.given = state.clone();
      }
    }
  }

  // tear down copies no track has needed for SPARE_FOR
  pub fn drop_spare_copies(&self) {
    self.drop_copies_spare_for(SPARE_FOR);
  }

  pub(super) fn drop_copies_spare_for(&self, long: Duration) {
    let any = {
      let inner = self.inner.lock().unwrap();
      inner.synths.values().flat_map(|s| s.copies.values()).any(|c| c.spare_since.map_or(false, |at| at.elapsed() >= long))
    };
    if !any { return; }
    let done = main_thread::run(|| {
      let mut inner = self.inner.lock().unwrap();
      let mut doomed = Vec::new();
      for synth in inner.synths.values_mut() {
        let old: Vec<u32> = synth.copies.iter()
          .filter(|(_, c)| c.spare_since.map_or(false, |at| at.elapsed() >= long))
          .map(|(track, _)| *track)
          .collect();
        for track in old {
          if let Some(copy) = synth.copies.remove(&track) { doomed.push((copy.id, copy.loaded)); }
        }
      }
      self.sync(&mut inner);
      let engine = inner.engine.clone();
      drop(inner);
      self.unload(engine, doomed);
    });
    if let Err(e) = done {
      println!("spare copies of synths weren't let go of: {}", e);
    }
  }
}

fn desc_of(synth: &daw::Synth) -> Desc {
  Desc {
    id: synth.id,
    plugin_id: synth.plugin_id.clone(),
    path: synth.path.clone(),
    name: synth.name.clone(),
    vendor: synth.vendor.clone(),
    bypassed: false,
  }
}

fn live(desc: Desc, spec: &daw::Synth, master: Instance) -> LiveSynth {
  let synth = LiveSynth {
    desc,
    volume: spec.volume,
    pan: spec.pan,
    muted: spec.muted,
    master,
    copies: BTreeMap::new(),
    pushed_at: None,
    was_open: false,
  };
  synth.set_gains();
  synth
}

#[cfg(test)]
mod tests;
