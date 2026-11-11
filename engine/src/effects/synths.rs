// the synths' plugins
//
// the synth list itself (names, volume, mute, order) is the sequencer's (see
// daw_core/sequencer.rs), and undo and projects go through it. this keeps
// each synth's plugins in line with that list: follow_synths is told the
// list and which tracks play which synths, and loads or lets go of plugins
// to match
//
// one plugin instance mixes every note it's sent into one output, so a
// synth that two tracks play needs two instances, or each track would hear
// the other's notes. so each synth has copies of its plugin:
//
//   - its master copy, first in the master's chain, which plays a pattern on
//     its own and keys auditioned. its slot id is the synth's id, and its
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

  // its volume and mute, on every copy
  fn set_gains(&self) {
    let gain = self.gain();
    for loaded in self.loaded() {
      loaded.slot.set_gain(gain);
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
  // stop following the synth list until what's returned is dropped (while
  // a project replaces it, plugins first and list after)
  pub fn pause_synths(&self) -> SynthPause<'_> {
    self.synth_pauses.fetch_add(1, Ordering::SeqCst);
    SynthPause(self)
  }

  // load an installed instrument as a new synth's master copy, under a new
  // id. it's heard once the synth list has it (see follow_synths). gives the
  // synth as the list will have it (named after the plugin)
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
        muted: false,
      };
      let mut inner = self.inner.lock().unwrap();
      inner.synths.insert(desc.id, live(desc, &synth, master));
      self.loaded.store(all_loaded(&inner).count(), Ordering::SeqCst);
      Ok(synth)
    })?
  }

  // a project's synths, with their settings, replacing any there are (the
  // list follows: see follow_synths). one whose plugin can't be loaded is
  // kept as missing, with its settings, and listed
  pub fn load_synths(&self, saved: &[daw::SavedSynth]) -> Vec<MissingEffect> {
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
      for synth in saved {
        let found = self.plugins.find(&synth.plugin_id);
        let path = found.as_ref().map_or_else(|| synth.path.clone(), |info| info.path.clone());
        let desc = Desc {
          id: synth.id,
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
          id: synth.id,
          plugin_id: synth.plugin_id.clone(),
          path: desc.path.clone(),
          name: synth.name.clone(),
          vendor: synth.vendor.clone(),
          volume: synth.volume,
          muted: synth.muted,
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

  // bring the synths' plugins in line with the synth list and the (synth,
  // track) pairs that need a copy: load the synths new to the list (with
  // the settings they had, if they've been in it before), let go of ones
  // that have left it (keeping their settings), make the copies that are
  // needed and put aside the ones that aren't, and set every copy's volume
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
mod tests {
  use super::super::testing::*;
  use super::*;
  use crate::synth_commands as synths;
  use std::sync::atomic::Ordering::SeqCst;

  fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
  }

  fn note(pitch: u8, start: f64, length: f64) -> daw::PatternNote {
    daw::PatternNote { pitch, start, length, velocity: 1. }
  }

  // a synth (master copy at `gain`) and a note pattern with a note for it
  fn synth_with_notes(rig: &Rig, gain: f32) -> (u64, u64) {
    let synth = synths::add(&rig.state, &synth().id).unwrap().synth_id;
    rig.turn(synth, gain);
    let pattern = rig.state.playlist.sequencer.lock().unwrap().add_pattern(1, 4, daw::PatternKind::Note).unwrap();
    crate::sequencer_commands::set_notes(&rig.state, pattern, synth, vec![note(60, 0., 4.)], None).unwrap();
    (synth, pattern)
  }

  fn clip(rig: &Rig, pattern: u64, track: u32) -> u64 {
    crate::sequencer_commands::add_pattern_clip_for_tests(&rig.state, pattern, track, Some(0.))
  }

  fn steps(rig: &Rig) -> usize {
    rig.state.history.lock().unwrap().len()
  }

  // what a copy plays once a change of gain (ramped across a block) has
  // been made: the second time round
  fn heard(rig: &mut Rig, synth: u64, track: Option<u32>) -> Vec<f32> {
    rig.hear_notes(synth, track, &[(60, 0, 300)], 400);
    rig.hear_notes(synth, track, &[(60, 0, 300)], 400)
  }

  #[test]
  fn test_a_synth_is_an_instrument_with_a_unique_name_and_its_master_copy_on_the_master() {
    let mut rig = rig();
    let a = synths::add(&rig.state, &synth().id).unwrap();
    let b = synths::add(&rig.state, &synth().id).unwrap();
    let names: Vec<&str> = b.sequencer.synths.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["Synth", "Synth 2"]);
    assert!(!b.sequencer.synths[0].missing);
    let json = serde_json::to_value(&b.sequencer.synths[1]).unwrap();
    assert_eq!(json["pluginId"], synth().id);
    assert_eq!(json["hasEditor"], false);
    assert_eq!(json["volume"], 1.);

    // its id is a slot id: its name and plugin are reached by it
    assert_eq!(rig.effects().slot_name(b.synth_id).as_deref(), Some("Synth 2"));
    rig.turn(a.synth_id, 0.3);
    assert!(close(rig.gain_of(a.synth_id), 0.3));
    // its master copy plays notes straight to the master; no track has one
    assert!(close(rig.hear_notes(a.synth_id, None, &[(60, 0, 300)], 400)[100], 0.3));
    assert!(rig.effects().copy_ids().get(&(a.synth_id, Some(0))).is_none());
    // racks hold effects only
    assert!(rig.effects().racks().is_empty());

    let err = synths::add(&rig.state, &gain().id).unwrap_err();
    assert!(err.contains("isn't an instrument"), "{}", err);
    assert!(synths::add(&rig.state, &gone_synth().id).is_err());
    assert_eq!(steps(&rig), 2);
  }

  #[test]
  fn test_a_copy_appears_when_a_clip_lands_and_its_notes_play_through_its_track() {
    let mut rig = rig();
    let (synth, pattern) = synth_with_notes(&rig, 0.4);
    // no clip: no copy
    assert_eq!(rig.loader.alive.load(SeqCst), 1);
    clip(&rig, pattern, 2);
    assert_eq!(rig.loader.alive.load(SeqCst), 2);
    // with the master copy's settings, on the track's bus
    assert_eq!(rig.copy_gain(synth, Some(2)), Some(0.4));
    assert!(close(rig.hear_notes(synth, Some(2), &[(60, 0, 300)], 400)[100], 0.4));
    // and through the track's effects, after the copy
    rig.effects().add(Channel::Track(2), &gain().id, None).unwrap();
    assert!(close(heard(&mut rig, synth, Some(2))[100], 0.4 * START_GAIN));

    // a clip of a pattern without its notes needs none
    let empty = rig.state.playlist.sequencer.lock().unwrap().add_pattern(1, 4, daw::PatternKind::Note).unwrap();
    clip(&rig, empty, 5);
    assert!(rig.effects().copy_ids().get(&(synth, Some(5))).is_none());
  }

  #[test]
  fn test_two_tracks_playing_a_synth_each_hear_only_their_own_notes() {
    let mut rig = rig();
    let (synth, pattern) = synth_with_notes(&rig, 0.5);
    clip(&rig, pattern, 0);
    clip(&rig, pattern, 1);
    let copies = rig.effects().copy_ids();
    let (a, b) = (copies[&(synth, Some(0))], copies[&(synth, Some(1))]);
    assert!(a != b && a != synth && b != synth);
    rig.state.playlist.mixer.set_volume_db(1, -6.);

    // notes for track 0's copy: heard on track 0 only (track 1's fader
    // makes the difference)
    rig.run += 1;
    let run = rig.run;
    rig.engine.start_run(run, rig.state.playlist.mixer.master(), vec![]).unwrap();
    let mut slice = vec![
      daw::SliceNote { on: Duration::ZERO, off: Duration::from_millis(2), synth, route: Some(rig.state.playlist.mixer.track_route(0)), pitch: 60, velocity: 1. },
    ];
    daw::play_slice_notes(&rig.engine, run, Duration::ZERO, slice.clone(), &copies).unwrap();
    let mut out = vec![0f32; 400 * 2];
    rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
    assert!(close(out[20], 0.5));

    // and track 1's on track 1, through its fader
    rig.run += 1;
    let run = rig.run;
    rig.engine.start_run(run, rig.state.playlist.mixer.master(), vec![]).unwrap();
    slice[0].route = Some(rig.state.playlist.mixer.track_route(1));
    daw::play_slice_notes(&rig.engine, run, Duration::ZERO, slice, &copies).unwrap();
    rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
    assert!((out[20] / 0.5 - 0.501).abs() < 0.01, "{}", out[20]);
  }

  #[test]
  fn test_the_master_copys_settings_reach_the_copies() {
    let rig = rig();
    let (synth, pattern) = synth_with_notes(&rig, 0.4);
    clip(&rig, pattern, 0);
    clip(&rig, pattern, 3);
    rig.turn(synth, 0.7);
    assert_eq!(rig.copy_gain(synth, Some(0)), Some(0.4));

    // not while its interface isn't showing (but always before playing)
    rig.effects().push_synth_states(false);
    assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.4));
    synths::before_playing(&rig.state);
    assert_eq!(rig.copy_gain(synth, Some(0)), Some(0.7));
    assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.7));

    // while it's showing, about once a second; once more as it's closed
    rig.effects().inner.lock().unwrap().synths.get_mut(&synth).unwrap().pushed_at = None;
    rig.loader.editors_open.store(true, SeqCst);
    rig.turn(synth, 0.2);
    rig.effects().push_synth_states(false);
    assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.2));
    rig.turn(synth, 0.1);
    rig.effects().push_synth_states(false);
    assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.2));
    rig.effects().inner.lock().unwrap().synths.get_mut(&synth).unwrap().pushed_at = None;
    rig.loader.editors_open.store(false, SeqCst);
    rig.effects().push_synth_states(false);
    assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.1));
    rig.turn(synth, 0.6);
    rig.effects().inner.lock().unwrap().synths.get_mut(&synth).unwrap().pushed_at = None;
    rig.effects().push_synth_states(false);
    assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.1));

    // and a copy made now starts with them
    clip(&rig, pattern, 6);
    assert_eq!(rig.copy_gain(synth, Some(6)), Some(0.6));
  }

  #[test]
  fn test_volume_and_mute_reach_every_copy() {
    let mut rig = rig();
    let (synth, pattern) = synth_with_notes(&rig, 0.5);
    clip(&rig, pattern, 1);
    let had = steps(&rig);

    // a drag of the volume is one step
    for v in [0.9, 0.7, 0.5] { synths::set_volume(&rig.state, synth, v, Some(5)).unwrap(); }
    assert_eq!(steps(&rig), had + 1);
    assert!(close(heard(&mut rig, synth, Some(1))[100], 0.25));
    assert!(close(heard(&mut rig, synth, None)[100], 0.25));

    let seq = synths::set_muted(&rig.state, synth, true).unwrap();
    assert!(seq.synths[0].muted);
    assert!(heard(&mut rig, synth, Some(1)).iter().all(|&s| s == 0.));
    assert!(heard(&mut rig, synth, None).iter().all(|&s| s == 0.));

    crate::history::undo(&rig.state).unwrap();
    assert!(close(heard(&mut rig, synth, Some(1))[100], 0.25));
    crate::history::undo(&rig.state).unwrap();
    assert!(close(heard(&mut rig, synth, Some(1))[100], 0.5));
    assert!(synths::set_volume(&rig.state, 999, 1., None).is_err());
  }

  #[test]
  fn test_removing_a_synth_and_undoing_it_brings_back_its_notes_and_settings() {
    let mut rig = rig();
    let (synth, pattern) = synth_with_notes(&rig, 0.35);
    clip(&rig, pattern, 2);
    let at = WindowPosition { x: 10., y: 20. };
    rig.effects().keep_window_position(synth, at);
    let heard = Arc::new(Mutex::new(Vec::new()));
    let h = heard.clone();
    rig.effects().on_unload(move |id| h.lock().unwrap().push(id));
    let had = steps(&rig);

    let seq = synths::remove(&rig.state, synth).unwrap();
    assert!(seq.synths.is_empty());
    assert!(seq.pattern(pattern).unwrap().notes.is_empty());
    assert_eq!(steps(&rig), had + 1);
    // its master copy and its copy are torn down
    assert_eq!(rig.loader.alive.load(SeqCst), 0);
    assert!(heard.lock().unwrap().contains(&synth));
    assert!(rig.effects().slot_name(synth).is_none());

    crate::history::undo(&rig.state).unwrap();
    let seq = synths::sequencer_state(&rig.state);
    assert_eq!(seq.synths[0].id, synth);
    assert_eq!(seq.pattern(pattern).unwrap().notes[&synth], vec![note(60, 0., 4.)]);
    assert!(close(rig.gain_of(synth), 0.35));
    assert_eq!(rig.copy_gain(synth, Some(2)), Some(0.35));
    assert_eq!(rig.effects().window_position(synth), Some(at));
    assert!(close(rig.hear_notes(synth, Some(2), &[(60, 0, 300)], 400)[100], 0.35));

    // redone and undone again: the same
    crate::history::redo(&rig.state).unwrap();
    assert_eq!(rig.loader.alive.load(SeqCst), 0);
    crate::history::undo(&rig.state).unwrap();
    assert!(close(rig.gain_of(synth), 0.35));
  }

  #[test]
  fn test_adding_a_synth_on_a_track_is_one_step() {
    let rig = rig();
    let before = crate::project::capture(&rig.state, None);
    let had = steps(&rig);
    let added = synths::add_on_track(&rig.state, &synth().id, 4, Some(8.)).unwrap();
    assert_eq!(steps(&rig), had + 1);

    let pattern = added.project.sequencer.pattern(added.pattern_id).unwrap();
    assert_eq!((pattern.kind, pattern.length), (daw::PatternKind::Note, 16));
    assert!(pattern.notes.is_empty());
    assert_eq!(added.project.sequencer.synths[0].id, added.synth_id);
    let clip = &added.project.pattern_clips[0];
    assert_eq!((clip.pattern_id, clip.track_number, clip.start_beat), (added.pattern_id, 4, 8.));
    let json = serde_json::to_value(&added).unwrap();
    assert!(json.get("synthId").is_some() && json.get("patternId").is_some() && json["project"].get("pattern_clips").is_none());
    let after = crate::project::capture(&rig.state, None);

    // its first note makes the track's copy
    crate::sequencer_commands::set_notes(&rig.state, added.pattern_id, added.synth_id, vec![note(60, 0., 1.)], None).unwrap();
    assert!(rig.effects().copy_ids().contains_key(&(added.synth_id, Some(4))));
    crate::history::undo(&rig.state).unwrap();

    crate::history::undo(&rig.state).unwrap();
    assert_eq!(crate::project::capture(&rig.state, None), before);
    assert_eq!(rig.loader.alive.load(SeqCst), 0);
    crate::history::redo(&rig.state).unwrap();
    assert_eq!(crate::project::capture(&rig.state, None), after);
    assert!(synths::add_on_track(&rig.state, &gain().id, 0, None).is_err());
    assert_eq!(steps(&rig), had + 1);
  }

  #[test]
  fn test_a_key_plays_a_synths_master_copy_until_its_let_go() {
    let mut rig = rig();
    let (synth, pattern) = synth_with_notes(&rig, 0.25);
    clip(&rig, pattern, 1);
    rig.state.playlist.mixer.set_mute(1, true);
    let engine = rig.engine.clone();

    synths::play_note_on(&rig.state, &engine, synth, 60, 0.9).unwrap();
    let mut out = vec![0f32; 100 * 2];
    rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
    assert!(out.iter().all(|&s| close(s, 0.25)));

    synths::stop_note_on(&rig.state, &engine, synth, 60).unwrap();
    rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
    assert!(out.iter().all(|&s| s == 0.));
    assert!(synths::play_note_on(&rig.state, &engine, synth, 128, 0.9).is_err());
    assert!(synths::play_note_on(&rig.state, &engine, 999, 60, 0.9).is_err());
  }

  #[test]
  fn test_a_copy_no_track_needs_is_kept_a_while_then_let_go() {
    let rig = rig();
    let (synth, pattern) = synth_with_notes(&rig, 0.5);
    let placed = clip(&rig, pattern, 2);
    let copy = rig.effects().copy_ids()[&(synth, Some(2))];
    let loads = rig.loader.loads.load(SeqCst);

    // moved away and back: the same copy, not loaded again
    rig.state.playlist.audiograph.lock().unwrap().pattern_nodes.iter_mut().find(|n| n.id == placed).unwrap().track_number = 3;
    synths::follow(&rig.state);
    assert!(rig.effects().copy_ids().get(&(synth, Some(2))).is_none());
    assert_eq!(rig.loader.alive.load(SeqCst), 3);
    rig.state.playlist.audiograph.lock().unwrap().pattern_nodes.iter_mut().find(|n| n.id == placed).unwrap().track_number = 2;
    synths::follow(&rig.state);
    assert_eq!(rig.effects().copy_ids()[&(synth, Some(2))], copy);
    assert_eq!(rig.loader.loads.load(SeqCst), loads + 1);

    // the one on track 3 goes once it's been spare long enough
    rig.effects().drop_copies_spare_for(Duration::ZERO);
    assert_eq!(rig.loader.alive.load(SeqCst), 2);
    assert_eq!(rig.effects().copy_ids()[&(synth, Some(2))], copy);
  }
}
