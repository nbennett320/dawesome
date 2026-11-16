// a built-in effect at work: its parameters' values, shared between the app
// (which sets them) and the audio thread (which reads them), and the Runner
// that smooths them and calls the effect
//
// the Store is the only thing the two sides share: a plain value per
// parameter (so what was typed or saved is kept exactly), as f32 bits in
// atomics, and a version raised by every change.
// setting a value never waits on the audio thread, and reading one never
// waits on the app
//
// the Runner reads the store at the start of each block it's given. a
// switch or a choice jumps there; a continuous value glides (smooth.rs).
// while something glides, the effect is called once per GRID frames, each
// call with the values as they are at its start. the grid is counted from
// the first frame the runner ran, not from each block's start, so how the
// engine cuts its sound into blocks makes no difference to what comes out.
// while nothing glides, the effect gets each block whole

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use crate::effect::{Effect, Entry, Io, Values};
use crate::smooth::Smoother;
use crate::tap::Tap;
use crate::transport::Transport;

// how many frames each step of a glide lasts. short, because each step
// changes filters' coefficients at once: at 32, sweeping a notch's Q under
// a big output gain clicked (the conformance suite's knob test). it only
// costs anything while something glides
pub const GRID: usize = 4;

pub struct Store {
  entry: &'static Entry,
  // what the effect puts out, for the app's analyzers (tap.rs)
  tap: Tap,
  // each parameter's plain value (f32 bits), by id
  values: Box<[AtomicU32]>,
  version: AtomicU64,
}

impl Store {
  // every parameter at its default
  pub fn new(entry: &'static Entry) -> Arc<Store> {
    let values = entry.params.iter()
      .map(|p| AtomicU32::new(p.default.to_bits()))
      .collect();
    Arc::new(Store { entry, tap: Tap::new(), values, version: AtomicU64::new(0) })
  }

  pub fn entry(&self) -> &'static Entry {
    self.entry
  }

  pub fn tap(&self) -> &Tap {
    &self.tap
  }

  // how many changes there have been
  pub fn version(&self) -> u64 {
    self.version.load(Ordering::Acquire)
  }

  // a parameter's plain value (0 if there's no such parameter)
  pub fn plain(&self, id: usize) -> f32 {
    self.values.get(id).map_or(0., |v| f32::from_bits(v.load(Ordering::Relaxed)))
  }

  // and normalized
  pub fn get(&self, id: usize) -> f32 {
    self.entry.params.get(id).map_or(0., |p| p.normalize(self.plain(id)))
  }

  pub fn plains(&self) -> Vec<f32> {
    (0..self.values.len()).map(|id| self.plain(id)).collect()
  }

  // set a parameter by its normalized value, made one it can have (a
  // choice's nearest step). gives the plain value it became. none if
  // there's no such parameter
  pub fn set(&self, id: usize, normalized: f32) -> Option<f32> {
    let desc = self.entry.params.get(id)?;
    self.set_plain(id, desc.plain(normalized))
  }

  // the same, by its plain value
  pub fn set_plain(&self, id: usize, plain: f32) -> Option<f32> {
    let desc = self.entry.params.get(id)?;
    let plain = desc.clamp(plain);
    self.values[id].store(plain.to_bits(), Ordering::Relaxed);
    self.version.fetch_add(1, Ordering::Release);
    Some(plain)
  }

  // set every parameter at once, by id (as one change)
  pub fn set_plains(&self, plains: &[f32]) {
    for (id, (desc, value)) in self.entry.params.iter().zip(&self.values).enumerate() {
      let plain = plains.get(id).copied().unwrap_or(desc.default);
      value.store(desc.clamp(plain).to_bits(), Ordering::Relaxed);
    }
    self.version.fetch_add(1, Ordering::Release);
  }
}

pub struct Runner {
  store: Arc<Store>,
  effect: Box<dyn Effect>,
  rate: f32,
  // the store's version last read
  seen: u64,
  // each parameter's normalized value, gliding to the store's
  smoothers: Vec<Smoother>,
  // each parameter's plain value as it is for the frames being run
  plain: Vec<f32>,
  // and as the store had it when last read: where glides land (exactly,
  // not by way of a normalized value)
  targets: Vec<f32>,
  // something is gliding: the effect is run a grid cell at a time
  gliding: bool,
  // frames run so far, which the grid is counted from
  frames: u64,
  // tail_frames as of the last block
  tail: Option<u64>,
}

impl Runner {
  // the effect's own, at `rate`, starting from the store's values
  pub fn new(store: Arc<Store>, rate: f32) -> Runner {
    let entry = store.entry();
    let effect = (entry.make)(rate);
    let smoothers: Vec<Smoother> = (0..entry.params.len())
      .map(|id| Smoother::new(store.get(id), GRID, rate))
      .collect();
    let plain = store.plains();
    let seen = store.version();
    let tail = effect.tail_frames(&Values(&plain), rate);
    let targets = plain.clone();
    Runner { store, effect, rate, seen, smoothers, plain, targets, gliding: false, frames: 0, tail }
  }

  pub fn store(&self) -> &Arc<Store> {
    &self.store
  }

  // a value has been set that no block has read yet
  pub fn has_pending(&self) -> bool {
    self.store.version() != self.seen
  }

  pub fn tail_frames(&self) -> Option<u64> {
    self.tail
  }

  // forget all sound it holds, and land every glide
  pub fn reset(&mut self) {
    self.effect.reset();
    for s in &mut self.smoothers { s.jump_to(s.target()); }
    self.plain.copy_from_slice(&self.targets);
    self.gliding = false;
  }

  // run a block (any length). `output` is filled
  pub fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], transport: &Transport) {
    let rate = transport.sample_rate as f32;
    if rate != self.rate && rate > 0. {
      self.rate = rate;
      self.effect.set_sample_rate(rate);
      for s in &mut self.smoothers { s.set_rate(GRID, rate); }
      self.reset();
    }
    self.read_store();

    let frames = input[0].len().min(input[1].len()).min(output[0].len()).min(output[1].len());
    let [out_l, out_r] = output;
    let params = self.store.entry().params;
    let mut at = 0;
    while at < frames {
      let in_cell = (self.frames % GRID as u64) as usize;
      // a grid cell starts: its values are the glides' as they are now
      if in_cell == 0 && self.gliding {
        let mut stepped = false;
        for (id, s) in self.smoothers.iter_mut().enumerate() {
          if !params[id].smoothed() { continue; }
          self.plain[id] = if s.moving() { params[id].plain(s.value()) } else { self.targets[id] };
          if s.moving() {
            s.step();
            stepped = true;
          }
        }
        // (one more cell to pick up where the last step landed)
        self.gliding = stepped;
      }
      let len = if self.gliding { (GRID - in_cell).min(frames - at) } else { frames - at };
      let end = at + len;
      let io = Io {
        input: [&input[0][at..end], &input[1][at..end]],
        output: [&mut out_l[at..end], &mut out_r[at..end]],
        sidechain: None,
      };
      let transport = Transport { position: transport.position + at as i64, ..*transport };
      self.effect.process(io, &Values(&self.plain), &transport);
      at = end;
      self.frames += len as u64;
    }
    self.store.tap.write(&out_l[..frames], &out_r[..frames], transport.sample_rate);
    self.tail = self.effect.tail_frames(&Values(&self.plain), self.rate);
  }

  // take the values set since the last block. before it's run a frame,
  // they're where it starts, not somewhere to glide to: a project's saved
  // settings (or a mixdown's copy of them) are put in after it's made
  fn read_store(&mut self) {
    let version = self.store.version();
    if version == self.seen { return; }
    self.seen = version;
    let starting = self.frames == 0;
    let params = self.store.entry().params;
    for (id, s) in self.smoothers.iter_mut().enumerate() {
      let plain = self.store.plain(id);
      self.targets[id] = plain;
      let target = params[id].normalize(plain);
      if params[id].smoothed() && !starting {
        s.glide_to(target);
        if s.moving() { self.gliding = true; }
      } else {
        s.jump_to(target);
        self.plain[id] = plain;
      }
    }
  }
}
