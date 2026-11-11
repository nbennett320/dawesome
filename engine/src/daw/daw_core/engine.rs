// the audio engine: one output stream for the whole app, whose callback is
// the clock everything plays by
//
// the callback renders sounds (voices) it's been sent, each starting on an
// exact frame of a run (a stretch of playback from play to stop). a run's
// frame 0 is wherever the callback is when it gets the run's start, and the
// scheduler (see daw::run_playlist) sends each beat's voices a little before
// the callback reaches them, tagged with the frame they start on
//
// it renders a block at a time, through buses. each track's voices are summed
// on the track's bus, which runs its chain of effects and then the track's
// strip (fader, pan, mute, solo, meter). the tracks, and voices sent straight
// to the master, are summed on the master bus, which runs its own chain, the
// master strip and the limiter. voices routed direct (the metronome,
// previews) skip all of that:
//
//   track voices ─→ track bus → chain → strip ─┐
//   master voices ─────────────────────────────┴→ master bus → chain → strip → limiter ─┐
//   direct voices ───────────────────────────────────────────────────────────────────────┴→ out
//
// effects come before the fader, so a muted track's effects still run. buses
// last as long as the engine, and one with effects runs every block, playing
// or not, so their tails ring on after a stop. one with no effects runs only
// in the blocks its voices sound in
//
// a chain, a track's or the master's, can start with any number of
// instruments, each playing only the notes sent to it by its slot's id (see
// play_notes). each note is timed to a frame of a run, as a voice is, and is
// handed to its instrument in the block it falls in, with its offset in the
// block. what each instrument plays, at its own gain, is added to the bus's
// voices, and the sum goes through the effects after them:
//
//   bus voices ──────────────────────┐
//   notes for a → instrument a × gain ┤
//   notes for b → instrument b × gain ┴→ (+) → effects → strip
//
// a run that stops (or is replaced) ends the notes it was holding, so none
// hangs. notes played without a run (a key auditioned) are ended only by
// their own note-off
//
// the callback never takes a lock, touches the disk, or allocates or frees
// memory: voices, buses and chains arrive through a lock-free queue, and what
// it's done with is sent back to be freed by whoever sends the next command
use crate::daw;
use crate::plugin::{self, Processor, Transport};
use super::audio_settings::{buffer_choices, AudioSettings};
use rodio::cpal::{
  self,
  traits::{DeviceTrait, HostTrait, StreamTrait},
  FromSample,
  SizedSample,
};
use rodio::source::UniformSourceIterator;
use rodio::Source;
use rtrb::{Consumer, Producer, RingBuffer};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// a sound sent to the engine
pub type VoiceSource = Box<dyn Source<Item = f32> + Send>;

// where a voice is heard
#[derive(Clone, Debug)]
pub enum Route {
  // on its track's bus (the playlist's clips and pattern hits)
  Track(daw::TrackRoute),
  // straight on the master bus (a pattern played in the Sequencer tab)
  Master,
  // straight to the output, after the limiter (the metronome and previews)
  Direct,
}

// a bus, as its chain is named
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Channel {
  Track(u32),
  Master,
}

// an effect or instrument in a chain: its processor, and whether it's
// bypassed (a bypassed slot passes the signal through untouched, so a
// bypassed instrument adds nothing). an instrument's output, times its gain,
// is added to what comes into it, rather than replacing it
//
// the processor (and what an instrument is playing) is only ever touched by
// the audio thread of the engine whose chain holds the slot, so it needs no
// lock. that holds as long as a slot is in at most one engine's chain, once:
// then one thread runs it, one call at a time. the last Arc of a slot is
// never dropped on the audio thread (a chain that's replaced goes back
// through the garbage queue)
pub struct Slot {
  id: u64,
  bypassed: AtomicBool,
  // its interface is on screen: its bus doesn't rest (see Bus::rests)
  showing: AtomicBool,
  // something about it changed that its bus should run again for
  nudged: AtomicBool,
  // an instrument's gain, as an f32's bits
  gain: AtomicU32,
  processor: UnsafeCell<Box<dyn Processor>>,
  // what an instrument is playing (an effect has none)
  playing: Option<UnsafeCell<Playing>>,
}

// see above: only one thread ever reaches the processor
unsafe impl Sync for Slot {}

// an instrument's notes, as its chain's audio thread keeps them. it goes with
// the slot, so a chain rebuilt around the same instrument keeps what it holds
struct Playing {
  // what it plays this block, in order of their offsets. room for every
  // note a bus can hold is made up front, and a pitch's worth more for
  // ending held notes at a stop
  events: Vec<plugin::Event>,
  // how many notes of each pitch are held, by the run playing and by
  // auditions
  held_by_run: [u16; 128],
  held_by_audition: [u16; 128],
  // pitches a run that stopped held, to be ended at the next block's start
  released: [bool; 128],
  // the gain it played at, at the end of the last block it ran (none before
  // its first, which starts at its gain rather than ramping to it)
  gain: Option<f32>,
}

impl Playing {
  // whether it has anything to play this block, or a note held down
  fn busy(&self) -> bool {
    !self.events.is_empty()
      || self.held_by_run.iter().chain(&self.held_by_audition).any(|&held| held > 0)
  }
}

impl Slot {
  pub fn new(id: u64, processor: Box<dyn Processor>) -> Arc<Slot> {
    Slot::make(id, processor, None)
  }

  // an instrument, for the head of a chain
  pub fn new_instrument(id: u64, processor: Box<dyn Processor>) -> Arc<Slot> {
    let playing = Playing {
      events: Vec::with_capacity(NOTE_ROOM),
      held_by_run: [0; 128],
      held_by_audition: [0; 128],
      released: [false; 128],
      gain: None,
    };
    Slot::make(id, processor, Some(UnsafeCell::new(playing)))
  }

  fn make(id: u64, processor: Box<dyn Processor>, playing: Option<UnsafeCell<Playing>>) -> Arc<Slot> {
    Arc::new(Slot {
      id,
      bypassed: AtomicBool::new(false),
      showing: AtomicBool::new(false),
      nudged: AtomicBool::new(false),
      gain: AtomicU32::new(1f32.to_bits()),
      processor: UnsafeCell::new(processor),
      playing,
    })
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn instrument(&self) -> bool {
    self.playing.is_some()
  }

  pub fn bypassed(&self) -> bool {
    self.bypassed.load(Ordering::Relaxed)
  }

  pub fn set_bypassed(&self, bypassed: bool) {
    self.bypassed.store(bypassed, Ordering::Relaxed);
    // (switched back on, it may have something to say with no input)
    self.nudged.store(true, Ordering::Relaxed);
  }

  // whether its interface is on screen. while it is, its bus keeps running
  // however quiet it is: the plugin may play from its own keyboard, or show
  // meters
  pub fn showing(&self) -> bool {
    self.showing.load(Ordering::Relaxed)
  }

  pub fn set_showing(&self, showing: bool) {
    self.showing.store(showing, Ordering::Relaxed);
  }

  // an instrument's gain (1 to start with), which its output is multiplied
  // by before it's added to the bus. a change is ramped to across the next
  // block, so it doesn't click. (an effect's is kept but not used)
  pub fn gain(&self) -> f32 {
    f32::from_bits(self.gain.load(Ordering::Relaxed))
  }

  pub fn set_gain(&self, gain: f32) {
    let gain = if gain.is_finite() { gain.max(0.) } else { 0. };
    self.gain.store(gain.to_bits(), Ordering::Relaxed);
  }

  // only for the audio thread running the chain that holds the slot
  #[allow(clippy::mut_from_ref)]
  unsafe fn processor(&self) -> &mut dyn Processor {
    &mut **self.processor.get()
  }

  // the same
  #[allow(clippy::mut_from_ref)]
  unsafe fn playing(&self) -> Option<&mut Playing> {
    self.playing.as_ref().map(|playing| &mut *playing.get())
  }
}

// where a run is in the song, for the transport the processors see
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunStart {
  // beats a minute, and the time signature
  pub tempo: f64,
  pub beats_per_bar: u16,
  pub beat_unit: u16,
  // the song position of the run's frame 0, in frames (before the song's
  // start, with a count-in)
  pub origin: i64,
}

impl Default for RunStart {
  fn default() -> Self {
    let transport = Transport::default();
    RunStart {
      tempo: transport.tempo,
      beats_per_bar: transport.beats_per_bar,
      beat_unit: transport.beat_unit,
      origin: 0,
    }
  }
}

pub struct Voice {
  // the run it belongs to, and the frame of that run it starts on. a voice
  // without a run (a preview) starts as soon as the engine gets it, and
  // isn't stopped with the run
  run: Option<u64>,
  at: u64,
  route: Route,
  // stereo, at the engine's rate
  samples: UniformSourceIterator<VoiceSource, f32>,
  // the engine's frame it starts on, once it's been placed
  start: u64,
  // frames left in its fade out, once it's been stopped
  fade_left: Option<u32>,
  // what it's summed on, once it's been placed
  target: Target,
}

// a note for a track's instrument, `at` frames into a run (or, without a
// run, as soon as the engine gets it)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedNote {
  pub at: u64,
  pub pitch: u8,
  pub kind: NoteKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoteKind {
  // velocity from 0 to 1
  On { velocity: f32 },
  Off,
}

impl TimedNote {
  pub fn on(at: u64, pitch: u8, velocity: f32) -> TimedNote {
    TimedNote { at, pitch, kind: NoteKind::On { velocity } }
  }

  pub fn off(at: u64, pitch: u8) -> TimedNote {
    TimedNote { at, pitch, kind: NoteKind::Off }
  }
}

// who a note on a bus belongs to
#[derive(Clone, Copy, Debug, PartialEq)]
enum Owner {
  Run(u64),
  // played without a run
  Audition,
}

// a note waiting on a bus for its block
#[derive(Clone, Copy, Debug)]
struct PendingNote {
  owner: Owner,
  // the id of the instrument slot it's for
  instrument: u64,
  // the engine's frame it falls on
  frame: u64,
  pitch: u8,
  kind: NoteKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
  // a track's bus, by its place in the renderer's list
  Bus(usize),
  Master,
  Direct,
}

// where voices are summed, then run through a chain and a strip. made off the
// audio thread with everything it needs, so the callback never allocates
struct Bus {
  channel: Channel,
  // two stereo pairs, MAX_BLOCK long. the voices are summed in the first,
  // and the chain passes the signal from one to the other (a processor never
  // reads and writes the same buffer)
  buffers: [[Vec<f32>; 2]; 2],
  strip: daw::Strip,
  chain: Vec<Arc<Slot>>,
  // notes still to come for its instruments, in no order. room for
  // MAX_NOTES of them is made up front
  notes: Vec<PendingNote>,
  // whether a voice was summed on it this block
  sounding: bool,
  // whether it ran last block (one that's been idle starts its strip afresh)
  awake: bool,
  // how many frames in a row nothing has gone into its chain and nothing
  // has come out (see rests)
  quiet: u64,
  // whether its chain has been left out of a block since it last ran
  rested: bool,
}

// a chain rests once nothing has gone in or come out of it for this long
// (and for as long again as its plugins say they ring on)
const REST_AFTER: Duration = Duration::from_secs(2);
// quieter than this is silence: -100 dB
const SILENCE: f32 = 1e-5;

// whether a block of stereo has anything in it
fn sounds(pair: &[Vec<f32>; 2], frames: usize) -> bool {
  pair.iter().any(|side| side[..frames].iter().any(|s| s.abs() > SILENCE))
}

impl Bus {
  fn new(channel: Channel, strip: daw::Strip) -> Box<Bus> {
    let pair = || [vec![0f32; plugin::MAX_BLOCK], vec![0f32; plugin::MAX_BLOCK]];
    Box::new(Bus {
      channel,
      buffers: [pair(), pair()],
      strip,
      chain: Vec::new(),
      notes: Vec::with_capacity(MAX_NOTES),
      sounding: false,
      awake: false,
      quiet: 0,
      rested: false,
    })
  }

  // whether the chain is left out of this block: nothing has gone into it
  // and nothing has come out of it for `rest_frames` (and its plugins'
  // tails), so running it would only make more silence. it runs again from
  // the block in which there's
  // - sound to go through it (`fed`),
  // - a note for one of its instruments, or one held,
  // - a parameter turned on one of its plugins, or one switched back on,
  // and it never rests while one of its plugins' interfaces is on screen,
  // or one says it never stops sounding. bypassed slots don't count
  //
  // call once a block, after take_notes and before run
  fn rests(&mut self, fed: bool, rest_frames: u64) -> bool {
    if self.chain.is_empty() { return false; }

    let mut wake = fed;
    let mut tail = Some(0u64);
    for slot in self.chain.iter() {
      if slot.nudged.swap(false, Ordering::Relaxed) { wake = true; }
      if slot.bypassed() { continue; }
      if slot.showing() { wake = true; }
      // safe: this is the callback of the engine whose chain holds the slot
      let processor = unsafe { slot.processor() };
      if processor.has_pending() { wake = true; }
      if unsafe { slot.playing() }.map_or(false, |playing| playing.busy()) { wake = true; }
      tail = match (tail, processor.tail_frames()) {
        (Some(longest), Some(frames)) => Some(longest.max(frames)),
        _ => None,
      };
    }
    if wake { self.quiet = 0; }
    let rests = match tail {
      Some(tail) => self.quiet >= rest_frames.saturating_add(tail),
      None => false,
    };
    if rests { self.rested = true; }
    rests
  }

  // silence the first `frames` of the mix
  fn clear(&mut self, frames: usize) {
    for side in self.buffers[0].iter_mut() {
      side[..frames].fill(0.);
    }
  }

  // the notes that fall in the block from `from` to `end` (frames), moved to
  // their instruments' events in order: by offset, and at the same offset
  // note-offs first, so a note ending where the same pitch starts again is
  // heard starting again. a note for an instrument the chain doesn't hold is
  // dropped once its block comes (one still to come waits, in case its
  // instrument's chain is on its way)
  fn take_notes(&mut self, from: u64, end: u64) {
    // each instrument's block starts by ending what a stopped run held
    for slot in self.chain.iter() {
      // safe: this is the callback of the engine whose chain holds the slot
      let Some(playing) = (unsafe { slot.playing() }) else { continue };
      playing.events.clear();
      for pitch in 0..128 {
        if !playing.released[pitch] { continue; }
        playing.released[pitch] = false;
        // (room is kept for these)
        if playing.events.len() < playing.events.capacity() {
          playing.events.push(plugin::Event::NoteOff { offset: 0, pitch: pitch as u8 });
        }
      }
    }
    if self.notes.is_empty() { return; }

    // note-ons first, then note-offs, so a held count never goes below what's
    // held (the order the instrument hears them in is set below)
    for ons in [true, false] {
      let mut i = 0;
      while i < self.notes.len() {
        let note = self.notes[i];
        if note.frame >= end || matches!(note.kind, NoteKind::On { .. }) != ons {
          i += 1;
          continue;
        }
        self.notes.swap_remove(i);

        // (safe, as above. ids are unique in a chain, so this is the only
        // reference to it)
        let playing = self.chain.iter()
          .find(|slot| slot.id() == note.instrument)
          .and_then(|slot| unsafe { slot.playing() });
        let Some(playing) = playing else { continue };
        let held = match note.owner {
          Owner::Run(_) => &mut playing.held_by_run[note.pitch as usize],
          Owner::Audition => &mut playing.held_by_audition[note.pitch as usize],
        };
        let offset = note.frame.saturating_sub(from) as u32;
        let event = match note.kind {
          NoteKind::On { velocity } => {
            *held = held.saturating_add(1);
            plugin::Event::NoteOn { offset, pitch: note.pitch, velocity }
          }
          NoteKind::Off => {
            *held = held.saturating_sub(1);
            plugin::Event::NoteOff { offset, pitch: note.pitch }
          }
        };
        // (there's always room: events has more than notes)
        if playing.events.len() < playing.events.capacity() { playing.events.push(event); }
      }
    }
    for slot in self.chain.iter() {
      let Some(playing) = (unsafe { slot.playing() }) else { continue };
      // (sort_unstable never allocates)
      playing.events.sort_unstable_by_key(|event| (event.offset(), matches!(event, plugin::Event::NoteOn { .. })));
    }
  }

  // a run stopped: its notes still to come are dropped, and every pitch it
  // holds on each instrument is ended at the next block's start, unless an
  // audition holds it too (whose own note-off ends it)
  fn end_run_notes(&mut self) {
    self.notes.retain(|note| !matches!(note.owner, Owner::Run(_)));
    for slot in self.chain.iter() {
      // safe: this is the callback of the engine whose chain holds the slot
      let Some(playing) = (unsafe { slot.playing() }) else { continue };
      for pitch in 0..128 {
        if playing.held_by_run[pitch] == 0 { continue; }
        playing.held_by_run[pitch] = 0;
        if playing.held_by_audition[pitch] == 0 { playing.released[pitch] = true; }
      }
    }
  }

  // run the chain on the first `frames` of the mix, then the strip. returns
  // the pair the result ended up in
  fn run(&mut self, frames: usize, transport: &Transport) -> usize {
    let mut at = 0;
    for slot in self.chain.iter() {
      if slot.bypassed() { continue; }

      let (first, second) = self.buffers.split_at_mut(1);
      let (input, output) = if at == 0 { (&first[0], &mut second[0]) } else { (&second[0], &mut first[0]) };
      let [in_l, in_r] = input;
      let [out_l, out_r] = output;
      // safe: this is the callback of the engine whose chain holds the slot
      let processor = unsafe { slot.processor() };
      let playing = unsafe { slot.playing() };
      // an instrument hears its own notes, an effect none
      let events: &[plugin::Event] = match &playing {
        Some(playing) => &playing.events,
        None => &[],
      };
      processor.process(
        [&in_l[..frames], &in_r[..frames]],
        [&mut out_l[..frames], &mut out_r[..frames]],
        events,
        transport,
      );
      // an instrument plays over the bus's voices (and the instruments
      // before it), at its gain, ramped across the block from where it was
      if let Some(playing) = playing {
        let to = slot.gain();
        let from = playing.gain.unwrap_or(to);
        playing.gain = Some(to);
        let step = (to - from) / frames.max(1) as f32;
        for (out, input) in [(out_l, in_l), (out_r, in_r)] {
          for (i, (o, s)) in out[..frames].iter_mut().zip(&input[..frames]).enumerate() {
            *o = *o * (from + step * (i + 1) as f32) + s;
          }
        }
      }
      at = 1 - at;
    }

    // (before the strip: a muted track's reverb is still sounding)
    if sounds(&self.buffers[at], frames) {
      self.quiet = 0;
    } else {
      self.quiet = self.quiet.saturating_add(frames as u64);
    }

    let [l, r] = &mut self.buffers[at];
    self.strip.process(&mut l[..frames], &mut r[..frames]);
    at
  }
}

enum Command {
  // start a run, stopping any other, with the master strip's controls,
  // where it is in the song, and its first voices
  Start { run: u64, master: Arc<daw::ChannelControls>, start: RunStart, voices: Vec<Box<Voice>> },
  // stop a run (if it's the one playing), fading out what's sounding
  Stop { run: u64 },
  Play(Vec<Box<Voice>>),
  // notes for an instrument on a bus, by its slot's id (a track's bus is
  // sent before them)
  Notes { channel: Channel, instrument: u64, run: Option<u64>, notes: Vec<TimedNote> },
  // a track's bus, sent before the first of its voices
  Bus(Box<Bus>),
  // a longer list for the buses, which move into it (so adding a bus never
  // allocates on the callback)
  Buses(Vec<Box<Bus>>),
  // a track's bus plays by other controls (the mixer was replaced)
  Retarget(daw::TrackRoute),
  // the master strip's controls
  Master(Arc<daw::ChannelControls>),
  // a bus's whole chain, replacing the one it has
  Chain { channel: Channel, slots: Vec<Arc<Slot>> },
}

// what the callback is done with, sent back to be freed off the audio thread
// (only held, to be dropped)
#[allow(dead_code)]
enum Garbage {
  Voice(Box<Voice>),
  Voices(Vec<Box<Voice>>),
  Notes(Vec<TimedNote>),
  Bus(Box<Bus>),
  Buses(Vec<Box<Bus>>),
  Route(daw::TrackRoute),
  Controls(Arc<daw::ChannelControls>, Option<Arc<AtomicUsize>>),
  Chain(Vec<Arc<Slot>>),
}

// how many voices can sound at once. more are dropped (and counted)
const MAX_VOICES: usize = 1024;
const QUEUE_LEN: usize = 4096;
// how long stopping takes to fade out, so it doesn't click
const STOP_FADE: Duration = Duration::from_millis(10);
// the most frames rendered in one go (a block). longer buffers are rendered
// in parts
const MAX_CHUNK: usize = plugin::MAX_BLOCK;
// how many track buses there's room for at first (more are made room for as
// they're needed)
const BUS_ROOM: usize = 256;
// how many notes a bus holds waiting for their block. more are dropped (and
// counted). a run sends a beat's worth at a time, so this is a lot of notes
const MAX_NOTES: usize = 4096;
// and room in an instrument's events for the note-offs a stop sends, one per
// pitch
const NOTE_ROOM: usize = MAX_NOTES + 128;

// what the callback reports, readable from any thread
struct Shared {
  // when the engine started, which the instants below count from
  epoch: Instant,
  // frames rendered so far
  frames: AtomicU64,
  // from rendering a frame to hearing it
  latency_ns: AtomicU64,
  // how many frames the device asks for at a time
  buffer_frames: AtomicU64,
  // the last run started (plus one, zero for none), its first frame, and
  // when that frame is heard (ns since `epoch`). the run is stored last, so
  // whoever sees it sees the rest
  anchor_run: AtomicU64,
  anchor_frame: AtomicU64,
  anchor_heard_ns: AtomicU64,
  // voices dropped because too many were sounding, and voices that arrived
  // after they should have started
  dropped: AtomicU64,
  late: AtomicU64,
  // notes dropped because a bus had too many waiting
  dropped_notes: AtomicU64,
  // chains sent to the callback, and chains it has put in place
  chains_sent: AtomicU64,
  chains_applied: AtomicU64,
  // how many chains were left out of the last block, resting
  resting: AtomicU64,
}

// where a run started, in the engine's frames and in time
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
  pub frame: u64,
  // when its first frame comes out of the speakers
  pub heard: Instant,
}

// the track buses that have been sent to the callback
struct Buses {
  // by track number, with the controls each plays by
  tracks: HashMap<u32, daw::TrackRoute>,
  // how many the callback's list has room for
  room: usize,
}

// the side of the engine everything but the callback uses
pub struct Engine {
  commands: Mutex<Producer<Command>>,
  garbage: Mutex<Consumer<Garbage>>,
  shared: Arc<Shared>,
  // locked before `commands`, never after
  buses: Mutex<Buses>,
  sample_rate: u32,
  // the output device it plays through
  device_name: String,
}

impl Engine {
  // an engine and the renderer its callback runs, without a device
  // (the output stream drives the renderer, or a test does)
  pub fn new(sample_rate: u32) -> (Engine, Renderer) {
    let (commands_tx, commands_rx) = RingBuffer::new(QUEUE_LEN);
    let (garbage_tx, garbage_rx) = RingBuffer::new(QUEUE_LEN);
    let shared = Arc::new(Shared {
      epoch: Instant::now(),
      frames: AtomicU64::new(0),
      latency_ns: AtomicU64::new(0),
      buffer_frames: AtomicU64::new(0),
      anchor_run: AtomicU64::new(0),
      anchor_frame: AtomicU64::new(0),
      anchor_heard_ns: AtomicU64::new(0),
      dropped: AtomicU64::new(0),
      late: AtomicU64::new(0),
      dropped_notes: AtomicU64::new(0),
      chains_sent: AtomicU64::new(0),
      chains_applied: AtomicU64::new(0),
      resting: AtomicU64::new(0),
    });

    let engine = Engine {
      commands: Mutex::new(commands_tx),
      garbage: Mutex::new(garbage_rx),
      shared: shared.clone(),
      buses: Mutex::new(Buses { tracks: HashMap::new(), room: BUS_ROOM }),
      sample_rate,
      device_name: "no device".into(),
    };
    let renderer = Renderer::new(sample_rate, commands_rx, garbage_tx, shared);

    (engine, renderer)
  }

  pub fn sample_rate(&self) -> u32 {
    self.sample_rate
  }

  pub fn device_name(&self) -> &str {
    &self.device_name
  }

  // frames the callback has rendered
  pub fn frames(&self) -> u64 {
    self.shared.frames.load(Ordering::Acquire)
  }

  // from rendering a frame to hearing it, as last reported by the device
  pub fn latency(&self) -> Duration {
    Duration::from_nanos(self.shared.latency_ns.load(Ordering::Relaxed))
  }

  // how many frames the device asks for at a time (the most seen)
  pub fn buffer_frames(&self) -> u64 {
    self.shared.buffer_frames.load(Ordering::Relaxed)
  }

  // a duration in frames at the engine's rate
  pub fn frames_in(&self, dur: Duration) -> u64 {
    (dur.as_secs_f64() * self.sample_rate as f64).round() as u64
  }

  // a sound for the engine to play, `at` frames into a run (or right away,
  // without one)
  pub fn voice<S>(&self, source: S, run: Option<u64>, at: u64, route: Route) -> Box<Voice>
  where
    S: Source<Item = f32> + Send + 'static,
  {
    self.boxed_voice(Box::new(source), run, at, route)
  }

  // (a voice for a track makes sure the track's bus is there first: it's
  // sent ahead of the voice)
  pub fn boxed_voice(&self, source: VoiceSource, run: Option<u64>, at: u64, route: Route) -> Box<Voice> {
    if let Route::Track(track) = &route {
      if let Err(e) = self.track_bus(track) {
        println!("audio engine: track {} plays without its strip: {}", track.number, e);
      }
    }

    Box::new(Voice {
      run,
      at,
      route,
      samples: UniformSourceIterator::new(source, 2, self.sample_rate),
      start: 0,
      fade_left: None,
      target: Target::Master,
    })
  }

  // send a track's bus to the callback the first time it's needed, or its
  // new controls if the mixer has been replaced since
  fn track_bus(&self, track: &daw::TrackRoute) -> Result<(), String> {
    let mut buses = self.buses.lock().unwrap();
    match buses.tracks.get(&track.number) {
      Some(sent) if Arc::ptr_eq(&sent.controls, &track.controls)
        && Arc::ptr_eq(&sent.solo_count, &track.solo_count) => return Ok(()),
      Some(_) => self.send(Command::Retarget(track.clone()))?,
      None => {
        if buses.tracks.len() >= buses.room {
          let room = buses.room * 2;
          self.send(Command::Buses(Vec::with_capacity(room)))?;
          buses.room = room;
        }
        let strip = daw::Strip::new(track.controls.clone(), Some(track.solo_count.clone()), self.sample_rate, true);
        self.send(Command::Bus(Bus::new(Channel::Track(track.number), strip)))?;
      }
    }
    buses.tracks.insert(track.number, track.clone());
    Ok(())
  }

  // start a run with its first voices, played through the master strip with
  // these controls. any run that's playing stops. the processors are told
  // it's at the song's start, at 120 bpm in 4/4 (the app's runs all use
  // start_run_at; the tests use this)
  #[allow(dead_code)]
  pub fn start_run(
    &self,
    run: u64,
    master: Arc<daw::ChannelControls>,
    voices: Vec<Box<Voice>>,
  ) -> Result<(), String> {
    self.start_run_at(run, master, RunStart::default(), voices)
  }

  // the same, telling the processors where the run is in the song
  pub fn start_run_at(
    &self,
    run: u64,
    master: Arc<daw::ChannelControls>,
    start: RunStart,
    voices: Vec<Box<Voice>>,
  ) -> Result<(), String> {
    self.send(Command::Start { run, master, start, voices })
  }

  // stop a run, if it's the one playing
  pub fn stop_run(&self, run: u64) -> Result<(), String> {
    self.send(Command::Stop { run })
  }

  pub fn play(&self, voices: Vec<Box<Voice>>) -> Result<(), String> {
    if voices.is_empty() { return Ok(()); }
    self.send(Command::Play(voices))
  }

  // notes for one instrument on a track's bus, by its slot's id, each `at`
  // frames into a run, or, without one, straight away (a key auditioned: see
  // the top of the file). no other instrument hears them. the track's bus is
  // sent first if it hasn't been
  pub fn play_notes(&self, route: &daw::TrackRoute, instrument: u64, run: Option<u64>, notes: Vec<TimedNote>) -> Result<(), String> {
    if notes.is_empty() { return Ok(()); }
    self.track_bus(route)?;
    self.send(Command::Notes { channel: Channel::Track(route.number), instrument, run, notes })
  }

  // the same, for an instrument on the master bus (a synth's master copy)
  pub fn play_master_notes(&self, instrument: u64, run: Option<u64>, notes: Vec<TimedNote>) -> Result<(), String> {
    if notes.is_empty() { return Ok(()); }
    self.send(Command::Notes { channel: Channel::Master, instrument, run, notes })
  }

  // play a sound once, straight to the output
  pub fn preview<S>(&self, source: S) -> Result<(), String>
  where
    S: Source<Item = f32> + Send + 'static,
  {
    self.play(vec![self.voice(source, None, 0, Route::Direct)])
  }

  // the master strip's controls, before any run sets them (a run's start
  // sets them too, and they stay after it ends)
  #[allow(dead_code)]
  pub fn set_master(&self, controls: Arc<daw::ChannelControls>) -> Result<(), String> {
    self.send(Command::Master(controls))
  }

  // replace a track's chain of effects (making its bus if need be). the old
  // slots come back to be dropped here; see wait_for_chains
  pub fn set_track_chain(&self, route: &daw::TrackRoute, slots: Vec<Arc<Slot>>) -> Result<(), String> {
    self.track_bus(route)?;
    self.send_chain(Channel::Track(route.number), slots)
  }

  // replace the master's chain of effects
  pub fn set_master_chain(&self, slots: Vec<Arc<Slot>>) -> Result<(), String> {
    self.send_chain(Channel::Master, slots)
  }

  fn send_chain(&self, channel: Channel, slots: Vec<Arc<Slot>>) -> Result<(), String> {
    self.collect_garbage();
    let mut commands = self.commands.lock().unwrap();
    commands
      .push(Command::Chain { channel, slots })
      .map_err(|_| "the audio engine is busy (its queue is full)".to_string())?;
    self.shared.chains_sent.fetch_add(1, Ordering::AcqRel);
    Ok(())
  }

  // how many buses' chains were resting in the last block rendered: silent
  // for long enough that their plugins aren't being run (see Bus::rests)
  pub fn resting_chains(&self) -> u64 {
    self.shared.resting.load(Ordering::Relaxed)
  }

  // wait until the callback has put in place every chain sent so far, so no
  // slot taken out of one is still being run. returns whether it got there
  // (with no output running it never does), or the renderer is gone (its
  // stream has closed, and its chains went with it: nothing can run them)
  pub fn wait_for_chains(&self, timeout: Duration) -> bool {
    let sent = self.shared.chains_sent.load(Ordering::Acquire);
    let deadline = Instant::now() + timeout;
    loop {
      if self.shared.chains_applied.load(Ordering::Acquire) >= sent || self.renderer_gone() {
        self.collect_garbage();
        return true;
      }
      if Instant::now() >= deadline { return false; }
      thread::sleep(Duration::from_millis(1));
    }
  }

  // whether the renderer has been dropped (the command queue's other end is
  // gone). a renderer is only dropped when it isn't rendering
  fn renderer_gone(&self) -> bool {
    self.commands.lock().unwrap().is_abandoned()
  }

  fn send(&self, command: Command) -> Result<(), String> {
    self.collect_garbage();
    self.commands.lock().unwrap()
      .push(command)
      .map_err(|_| "the audio engine is busy (its queue is full)".to_string())
  }

  // free what the callback is done with
  fn collect_garbage(&self) {
    let mut garbage = self.garbage.lock().unwrap();
    while let Ok(item) = garbage.pop() {
      drop(item);
    }

    let dropped = self.shared.dropped.swap(0, Ordering::Relaxed);
    if dropped > 0 { println!("audio engine: dropped {} voices (too many at once)", dropped); }
    let late = self.shared.late.swap(0, Ordering::Relaxed);
    if late > 0 { println!("audio engine: {} voices arrived late", late); }
    let dropped = self.shared.dropped_notes.swap(0, Ordering::Relaxed);
    if dropped > 0 { println!("audio engine: dropped {} notes (too many waiting at once)", dropped); }
  }

  // where a run started, once the callback has started it
  pub fn anchor(&self, run: u64) -> Option<Anchor> {
    if self.shared.anchor_run.load(Ordering::Acquire) != run + 1 { return None; }

    let at = |ns: &AtomicU64| self.shared.epoch + Duration::from_nanos(ns.load(Ordering::Relaxed));
    Some(Anchor {
      frame: self.shared.anchor_frame.load(Ordering::Relaxed),
      heard: at(&self.shared.anchor_heard_ns),
    })
  }

  // wait for the callback to start a run
  pub fn wait_for_anchor(&self, run: u64, timeout: Duration) -> Result<Anchor, String> {
    let deadline = Instant::now() + timeout;
    loop {
      if let Some(anchor) = self.anchor(run) { return Ok(anchor); }
      if Instant::now() >= deadline {
        return Err("the audio output isn't running".to_string());
      }
      thread::sleep(Duration::from_millis(1));
    }
  }

  // sleep until the callback has rendered up to `frame`, or `keep_waiting`
  // says to give up. returns whether it got there
  pub fn wait_for_frame(&self, frame: u64, keep_waiting: impl Fn() -> bool) -> bool {
    // checked now and then, so a stop isn't kept waiting
    const MAX_NAP: Duration = Duration::from_millis(20);

    loop {
      let now = self.frames();
      if now >= frame { return true; }
      if !keep_waiting() { return false; }

      let left = Duration::from_secs_f64((frame - now) as f64 / self.sample_rate as f64);
      thread::sleep(left.min(MAX_NAP).max(Duration::from_micros(500)));
    }
  }
}

// the run playing
struct Run {
  id: u64,
  // the engine's frame it started on
  frame: u64,
  start: RunStart,
}

// the callback's side of the engine
pub struct Renderer {
  commands: Consumer<Command>,
  garbage: Producer<Garbage>,
  shared: Arc<Shared>,
  voices: Vec<Box<Voice>>,
  run: Option<Run>,
  // frames rendered
  frame: u64,
  // every track's bus that's been used, in the order they came
  buses: Vec<Box<Bus>>,
  // the master bus. its strip's controls are set by each run (the limiter
  // comes after it)
  master: Box<Bus>,
  // what's routed direct, this block
  direct: [Vec<f32>; 2],
  // what the processors are told
  transport: Transport,
  fade_frames: u32,
  // how long a chain is silent for before it rests (see Bus::rests)
  rest_frames: u64,
}

impl Renderer {
  fn new(
    sample_rate: u32,
    commands: Consumer<Command>,
    garbage: Producer<Garbage>,
    shared: Arc<Shared>,
  ) -> Self {
    // until a run sets its controls, the master passes the mix unchanged
    let master = daw::Strip::new(Arc::new(daw::ChannelControls::default()), None, sample_rate, true);
    Renderer {
      commands,
      garbage,
      shared,
      voices: Vec::with_capacity(MAX_VOICES),
      run: None,
      frame: 0,
      buses: Vec::with_capacity(BUS_ROOM),
      master: Bus::new(Channel::Master, master),
      direct: [vec![0f32; MAX_CHUNK], vec![0f32; MAX_CHUNK]],
      transport: Transport { sample_rate, ..Transport::default() },
      fade_frames: ((STOP_FADE.as_secs_f64() * sample_rate as f64) as u32).max(1),
      rest_frames: (REST_AFTER.as_secs_f64() * sample_rate as f64) as u64,
    }
  }

  // the device asked for a buffer this many frames long
  pub fn saw_buffer(&self, frames: usize) {
    self.shared.buffer_frames.fetch_max(frames as u64, Ordering::Relaxed);
  }

  // fill `out` (interleaved stereo) with what plays next
  // - now: when the callback was called
  // - latency: from now until the first frame of `out` is heard
  pub fn render(&mut self, out: &mut [f32], now: Instant, latency: Duration) {
    self.shared.latency_ns.store(latency.as_nanos() as u64, Ordering::Relaxed);

    // what's arrived since the last buffer
    while let Ok(command) = self.commands.pop() {
      self.apply(command, now, latency);
    }

    for block in out.chunks_mut(MAX_CHUNK * 2) {
      self.render_block(block);
    }

    // take out finished voices, and those that have faded out
    let mut i = 0;
    while i < self.voices.len() {
      if self.voices[i].fade_left == Some(0) {
        let voice = self.voices.swap_remove(i);
        self.throw_away(Garbage::Voice(voice));
      } else {
        i += 1;
      }
    }

    self.shared.frames.store(self.frame, Ordering::Release);
  }

  // one block, at most MAX_CHUNK frames
  fn render_block(&mut self, out: &mut [f32]) {
    let frames = out.len() / 2;
    let end = self.frame + frames as u64;

    // where the run is in the song at the block's start. stopped, it stays
    // where the last run got to
    if let Some(run) = &self.run {
      self.transport.playing = true;
      self.transport.position = run.start.origin + (self.frame - run.frame) as i64;
    } else {
      self.transport.playing = false;
    }

    // and each bus's notes in this block
    for bus in self.buses.iter_mut() {
      bus.sounding = false;
      bus.take_notes(self.frame, end);
    }
    self.master.take_notes(self.frame, end);
    self.master.clear(frames);
    for side in self.direct.iter_mut() {
      side[..frames].fill(0.);
    }

    // each voice's frames in this block, summed on its bus from the frame it
    // starts on
    let fade_frames = self.fade_frames;
    for voice in self.voices.iter_mut() {
      if voice.start >= end || voice.fade_left == Some(0) { continue; }

      // its first frame, so one that has just run out doesn't wake its bus
      let mut first = next_frame(&mut voice.samples);
      if first.is_none() {
        // done: it's taken out after this buffer
        voice.fade_left = Some(0);
        continue;
      }

      let [left, right] = match voice.target {
        Target::Bus(i) => {
          let bus = &mut self.buses[i];
          if !bus.sounding {
            bus.clear(frames);
            bus.sounding = true;
          }
          &mut bus.buffers[0]
        }
        Target::Master => &mut self.master.buffers[0],
        Target::Direct => &mut self.direct,
      };

      let from = voice.start.saturating_sub(self.frame) as usize;
      for i in from..frames {
        let (l, r) = match first.take().or_else(|| next_frame(&mut voice.samples)) {
          Some(lr) => lr,
          None => {
            voice.fade_left = Some(0);
            break;
          }
        };
        let gain = match voice.fade_left {
          Some(0) => break,
          Some(frames_left) => {
            voice.fade_left = Some(frames_left - 1);
            frames_left as f32 / fade_frames as f32
          }
          None => 1.,
        };

        left[i] += l * gain;
        right[i] += r * gain;
      }
    }

    // each track bus with something to run, onto the master bus
    let transport = self.transport;
    let rest_frames = self.rest_frames;
    let mut resting = 0;
    let [master_l, master_r] = &mut self.master.buffers[0];
    for bus in self.buses.iter_mut() {
      if !bus.sounding && bus.chain.is_empty() {
        bus.awake = false;
        continue;
      }
      if !bus.sounding { bus.clear(frames); }
      // a chain that's been silent, with silence going in, is left out
      let fed = bus.sounding && sounds(&bus.buffers[0], frames);
      if bus.rests(fed, rest_frames) {
        bus.awake = false;
        resting += 1;
        continue;
      }
      if !bus.awake {
        bus.strip.jump();
        bus.awake = true;
      }

      let at = bus.run(frames, &transport);
      let [l, r] = &bus.buffers[at];
      for i in 0..frames {
        master_l[i] += l[i];
        master_r[i] += r[i];
      }
    }

    // the master's chain rests the same way, when nothing reaches it
    let [direct_l, direct_r] = &self.direct;
    let fed = sounds(&self.master.buffers[0], frames);
    if self.master.rests(fed, rest_frames) {
      // (`awake` is only for a bus that has rested: the master's strip
      // otherwise smooths on from block to block, as it always has)
      self.master.awake = false;
      for (i, frame) in out.chunks_exact_mut(2).enumerate() {
        frame[0] = direct_l[i];
        frame[1] = direct_r[i];
      }
      self.shared.resting.store(resting + 1, Ordering::Relaxed);
      self.frame = end;
      return;
    }
    self.shared.resting.store(resting, Ordering::Relaxed);
    // back from a rest, its strip starts where its fader is now
    if !self.master.awake && self.master.rested {
      self.master.strip.jump();
      self.master.rested = false;
    }
    self.master.awake = true;

    let at = self.master.run(frames, &transport);
    let [l, r] = &self.master.buffers[at];
    for (i, frame) in out.chunks_exact_mut(2).enumerate() {
      frame[0] = daw::soft_limit(l[i]) + direct_l[i];
      frame[1] = daw::soft_limit(r[i]) + direct_r[i];
    }

    self.frame = end;
  }

  fn apply(&mut self, command: Command, now: Instant, latency: Duration) {
    match command {
      Command::Start { run, master, start, voices } => {
        if let Some(playing) = self.run.as_ref().map(|run| run.id) { self.stop(playing); }
        self.run = Some(Run { id: run, frame: self.frame, start });
        self.transport.tempo = start.tempo;
        self.transport.beats_per_bar = start.beats_per_bar;
        self.transport.beat_unit = start.beat_unit;
        self.transport.position = start.origin;

        // each run starts its master strip at its volume, as a new strip would
        self.set_master(master);
        self.master.strip.jump();

        let since_epoch = |t: Instant| t.duration_since(self.shared.epoch).as_nanos() as u64;
        self.shared.anchor_frame.store(self.frame, Ordering::Relaxed);
        self.shared.anchor_heard_ns.store(since_epoch(now + latency), Ordering::Relaxed);
        self.shared.anchor_run.store(run + 1, Ordering::Release);

        self.add(voices);
      }
      Command::Stop { run } => self.stop(run),
      Command::Play(voices) => self.add(voices),
      Command::Notes { channel, instrument, run, notes } => self.add_notes(channel, instrument, run, notes),
      Command::Bus(bus) => {
        // the engine makes room before sending more buses than fit
        if self.buses.len() < self.buses.capacity() {
          self.buses.push(bus);
        } else {
          self.throw_away(Garbage::Bus(bus));
        }
      }
      Command::Buses(mut buses) => {
        buses.extend(self.buses.drain(..));
        let old = std::mem::replace(&mut self.buses, buses);
        self.throw_away(Garbage::Buses(old));
      }
      Command::Retarget(track) => {
        match self.buses.iter_mut().find(|bus| bus.channel == Channel::Track(track.number)) {
          Some(bus) => {
            let (controls, solo_count) = bus.strip.set_controls(track.controls, Some(track.solo_count));
            self.throw_away(Garbage::Controls(controls, solo_count));
          }
          None => self.throw_away(Garbage::Route(track)),
        }
      }
      Command::Master(controls) => self.set_master(controls),
      Command::Chain { channel, slots } => {
        let bus = match channel {
          Channel::Master => Some(&mut self.master),
          Channel::Track(_) => self.buses.iter_mut().find(|bus| bus.channel == channel),
        };
        let old = match bus {
          Some(bus) => {
            // a chain that's changed is run again: what's new in it may
            // have something to say
            bus.quiet = 0;
            std::mem::replace(&mut bus.chain, slots)
          }
          None => slots,
        };
        self.throw_away(Garbage::Chain(old));
        self.shared.chains_applied.fetch_add(1, Ordering::AcqRel);
      }
    }
  }

  fn set_master(&mut self, controls: Arc<daw::ChannelControls>) {
    if Arc::ptr_eq(self.master.strip.controls(), &controls) {
      self.throw_away(Garbage::Controls(controls, None));
    } else {
      let (old, _) = self.master.strip.set_controls(controls, None);
      self.throw_away(Garbage::Controls(old, None));
    }
  }

  // stop a run if it's playing: what's sounding fades out, what hasn't
  // started yet won't. the transport stays where the run got to
  fn stop(&mut self, run: u64) {
    match &self.run {
      Some(playing) if playing.id == run => {
        self.transport.position = playing.start.origin + (self.frame - playing.frame) as i64;
        self.transport.playing = false;
        self.run = None;
      }
      _ => return,
    }

    // and the notes it holds end now
    for bus in self.buses.iter_mut() {
      bus.end_run_notes();
    }
    self.master.end_run_notes();

    for voice in self.voices.iter_mut() {
      if voice.run != Some(run) { continue; }

      if voice.start > self.frame {
        voice.fade_left = Some(0);
      } else if voice.fade_left.is_none() {
        voice.fade_left = Some(self.fade_frames);
      }
    }
  }

  fn add(&mut self, mut voices: Vec<Box<Voice>>) {
    for mut voice in voices.drain(..) {
      voice.start = match (voice.run, &self.run) {
        // a preview starts now
        (None, _) => self.frame,
        (Some(run), Some(playing)) if run == playing.id => playing.frame + voice.at,
        // its run has stopped, or been replaced
        _ => {
          self.throw_away(Garbage::Voice(voice));
          continue;
        }
      };

      // the bus it's summed on (sent before it). without one (the engine
      // couldn't send it) it's heard on the master, without its strip
      voice.target = match &voice.route {
        Route::Track(track) => self.buses.iter()
          .position(|bus| bus.channel == Channel::Track(track.number))
          .map_or(Target::Master, Target::Bus),
        Route::Master => Target::Master,
        Route::Direct => Target::Direct,
      };

      // it should have started already: play it from where it would be now
      if voice.start < self.frame {
        self.shared.late.fetch_add(1, Ordering::Relaxed);
        for _ in voice.start..self.frame {
          if next_frame(&mut voice.samples).is_none() { break; }
        }
        voice.start = self.frame;
      }

      if self.voices.len() < MAX_VOICES {
        self.voices.push(voice);
      } else {
        self.shared.dropped.fetch_add(1, Ordering::Relaxed);
        self.throw_away(Garbage::Voice(voice));
      }
    }

    // the list itself is freed elsewhere too
    self.throw_away(Garbage::Voices(voices));
  }

  fn add_notes(&mut self, channel: Channel, instrument: u64, run: Option<u64>, mut notes: Vec<TimedNote>) {
    // where they're counted from: the run's first frame, or now. a run that's
    // stopped or been replaced has nothing to play them in
    let (owner, zero) = match (run, &self.run) {
      (None, _) => (Owner::Audition, self.frame),
      (Some(run), Some(playing)) if run == playing.id => (Owner::Run(run), playing.frame),
      _ => {
        self.throw_away(Garbage::Notes(notes));
        return;
      }
    };

    // a note that should have started already starts now. one that should
    // have ended already too isn't played at all: its note-on and note-off
    // would both land now, where the note-off comes first (see take_notes),
    // and it would hang. so each pitch's late notes are paired in time order,
    // each note-off with the earliest late note-on still waiting, and the
    // pairs are left out. a late note-off with no late note-on to pair with
    // ends a note that started earlier, so it comes now, before any late
    // note-on of the pitch that's still played (sorting in place doesn't
    // allocate)
    let frame = self.frame;
    let late = |note: &TimedNote| zero + note.at < frame;
    notes.sort_unstable_by_key(|note| (note.pitch.min(127), note.at, matches!(note.kind, NoteKind::On { .. })));
    let bus = match channel {
      Channel::Master => Some(&mut self.master),
      Channel::Track(_) => self.buses.iter_mut().find(|bus| bus.channel == channel),
    };
    match bus {
      Some(bus) => {
        let mut from = 0;
        while from < notes.len() {
          let pitch = notes[from].pitch.min(127);
          let to = from + notes[from..].iter().take_while(|note| note.pitch.min(127) == pitch).count();
          let same_pitch = &notes[from..to];
          from = to;

          // how many of its late notes pair up
          let (mut waiting, mut pairs) = (0, 0);
          for note in same_pitch.iter().filter(|note| late(note)) {
            match note.kind {
              NoteKind::On { .. } => waiting += 1,
              NoteKind::Off if waiting > 0 => {
                waiting -= 1;
                pairs += 1;
              }
              NoteKind::Off => {}
            }
          }

          // the same walk again, leaving the pairs out (any `pairs` of the
          // late note-ons will do: they'd all land now)
          let (mut waiting, mut ons_left_out) = (0, 0);
          for note in same_pitch {
            if late(note) {
              self.shared.late.fetch_add(1, Ordering::Relaxed);
              match note.kind {
                NoteKind::On { .. } => {
                  waiting += 1;
                  if ons_left_out < pairs {
                    ons_left_out += 1;
                    continue;
                  }
                }
                NoteKind::Off if waiting > 0 => {
                  waiting -= 1;
                  continue;
                }
                NoteKind::Off => {}
              }
            }
            if bus.notes.len() >= MAX_NOTES {
              self.shared.dropped_notes.fetch_add(1, Ordering::Relaxed);
              continue;
            }
            bus.notes.push(PendingNote { owner, instrument, frame: (zero + note.at).max(frame), pitch, kind: note.kind });
          }
        }
      }
      // (the engine couldn't send its bus)
      None => { self.shared.dropped_notes.fetch_add(notes.len() as u64, Ordering::Relaxed); }
    }
    self.throw_away(Garbage::Notes(notes));
  }

  fn throw_away(&mut self, garbage: Garbage) {

    // with the queue full there's no choice but to free it here
    let _ = self.garbage.push(garbage);
  }
}

fn next_frame(samples: &mut impl Iterator<Item = f32>) -> Option<(f32, f32)> {
  let l = samples.next()?;
  let r = samples.next()?;
  Some((l, r))
}

// the app's output: the engine, and the thread holding its stream (which
// closes the stream when `close` is dropped)
struct Output {
  engine: Arc<Engine>,
  _close: mpsc::Sender<()>,
}

struct Outputs {
  open: Option<Output>,
  // what it was opened with, or will be
  settings: AudioSettings,
  // something to tell the user about the output, like a saved device
  // that's gone missing
  notice: Option<String>,
}

// the app's engine, opened the first time it's needed. if the output can't
// be opened, the next call tries again
lazy_static! {
  static ref OUTPUTS: Mutex<Outputs> = Mutex::new(Outputs {
    open: None,
    settings: AudioSettings::load_default(),
    notice: None,
  });
  // told each time engine() opens an output (see on_output_opened)
  static ref ON_OPENED: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(None);
}

// `f` hears each time engine() opens an output, once it's open, with no
// lock of this module held (so it may call engine()). whoever had nothing
// to play on until then (the effects' chains) can send it now
pub fn on_output_opened(f: impl Fn() + Send + Sync + 'static) {
  *ON_OPENED.lock().unwrap() = Some(Arc::new(f));
}

// the rate of the output that's open, without opening one
pub fn open_engine_rate() -> Option<u32> {
  OUTPUTS.lock().unwrap().open.as_ref().map(|output| output.engine.sample_rate())
}

pub fn engine() -> Result<Arc<Engine>, String> {
  let mut outputs = OUTPUTS.lock().unwrap();
  if let Some(output) = outputs.open.as_ref() { return Ok(output.engine.clone()); }

  // a saved device that's gone (or won't take the saved buffer size) gives
  // way to the system's default
  let settings = outputs.settings.clone();
  let output = match open_output(&settings) {
    Ok(output) => {
      outputs.notice = None;
      output
    }
    Err(e) if settings.output_device.is_some() || settings.buffer_frames.is_some() => {
      let fallback = AudioSettings { output_device: None, buffer_frames: None, ..settings };
      let output = open_output(&fallback)?;
      outputs.notice = Some(format!(
        "The saved audio output couldn't be used ({}), so the system default is playing instead", e));
      output
    }
    Err(e) => return Err(e),
  };

  let engine = output.engine.clone();
  outputs.open = Some(output);
  drop(outputs);

  let opened = ON_OPENED.lock().unwrap().clone();
  if let Some(opened) = opened { opened(); }
  Ok(engine)
}

// the engine, if it's been opened (so stopping doesn't open it)
pub fn engine_if_open() -> Option<Arc<Engine>> {
  OUTPUTS.lock().unwrap().open.as_ref().map(|output| output.engine.clone())
}

// the settings the engine opens with
pub fn audio_settings() -> AudioSettings {
  OUTPUTS.lock().unwrap().settings.clone()
}

pub fn audio_notice() -> Option<String> {
  OUTPUTS.lock().unwrap().notice.clone()
}

// change the devices or buffer size, reopening the output when they change
// it, and save them. the new output is opened before the old one closes, so
// settings that don't work leave things as they were. stop playing first:
// a run playing on the old output stops hearing it
pub fn apply_audio_settings(settings: AudioSettings) -> Result<(), String> {
  let mut outputs = OUTPUTS.lock().unwrap();
  let same_output = outputs.open.is_some()
    && outputs.settings.output_device == settings.output_device
    && outputs.settings.buffer_frames == settings.buffer_frames;

  if !same_output {
    // the old output closes as it's replaced
    outputs.open = Some(open_output(&settings)?);
  }
  outputs.settings = settings;
  outputs.notice = outputs.settings.save_default()
    .err()
    .map(|e| format!("The audio settings are in use, but couldn't be saved for next time: {}", e));

  Ok(())
}

// an output device by name, or the system's default
pub fn output_device(name: Option<&str>) -> Result<cpal::Device, String> {
  let host = cpal::default_host();
  match name {
    None => host.default_output_device().ok_or_else(|| "there's no audio output device".to_string()),
    Some(name) => host
      .output_devices()
      .map_err(|e| format!("could not list the audio outputs: {}", e))?
      .find(|device| device.name().map(|n| n == name).unwrap_or(false))
      .ok_or_else(|| format!("there's no audio output named '{}'", name)),
  }
}

// open an output and start rendering to it. the stream lives on its own
// thread (it can't be moved between threads) until the output is dropped
fn open_output(settings: &AudioSettings) -> Result<Output, String> {
  let (opened_tx, opened_rx) = mpsc::channel();
  let (close_tx, close_rx) = mpsc::channel::<()>();
  let settings = settings.clone();

  thread::Builder::new()
    .name("audio output".into())
    .spawn(move || {
      let stream = match build_output(&settings) {
        Ok((engine, stream)) => {
          let _ = opened_tx.send(Ok(engine));
          stream
        }
        Err(e) => {
          let _ = opened_tx.send(Err(e));
          return;
        }
      };

      // until the output is dropped (nothing is ever sent)
      let _ = close_rx.recv();
      drop(stream);
      println!("audio output closed");
    })
    .map_err(|e| format!("could not start the audio output thread: {}", e))?;

  let engine = Arc::new(
    opened_rx.recv().map_err(|_| "the audio output thread exited".to_string())??);

  // give the device a moment to start, so its latency is known
  let started = Instant::now();
  while engine.frames() == 0 && started.elapsed() < Duration::from_secs(1) {
    thread::sleep(Duration::from_millis(2));
  }

  Ok(Output { engine, _close: close_tx })
}

fn build_output(settings: &AudioSettings) -> Result<(Engine, cpal::Stream), String> {
  let device = output_device(settings.output_device.as_deref())?;
  let name = device.name().unwrap_or_else(|_| "Unknown output".into());
  let supported = device.default_output_config()
    .map_err(|e| format!("could not read the settings of '{}': {}", name, e))?;
  let format = supported.sample_format();
  let choices = buffer_choices(supported.buffer_size());
  let mut config: cpal::StreamConfig = supported.into();

  if let Some(frames) = settings.buffer_frames {
    if !choices.contains(&frames) {
      return Err(format!("'{}' can't use a buffer of {} frames", name, frames));
    }
    config.buffer_size = cpal::BufferSize::Fixed(frames);
  }

  println!(
    "audio output: {} ({} Hz, {} channels, {:?}, buffer {:?})",
    name, config.sample_rate.0, config.channels, format, config.buffer_size,
  );

  let (mut engine, renderer) = Engine::new(config.sample_rate.0);
  engine.device_name = name;
  let stream = match format {
    cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, renderer),
    cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, renderer),
    cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, renderer),
    cpal::SampleFormat::I32 => build_stream::<i32>(&device, &config, renderer),
    other => return Err(format!("unsupported output sample format: {:?}", other)),
  }?;
  stream.play().map_err(|e| format!("could not start the audio output: {}", e))?;

  Ok((engine, stream))
}

fn build_stream<T>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  mut renderer: Renderer,
) -> Result<cpal::Stream, String>
where
  T: SizedSample + FromSample<f32>,
{
  let channels = config.channels.max(1) as usize;
  let sample_rate = config.sample_rate.0 as f64;
  // rendered here first, allocated once
  let mut mix = vec![0f32; MAX_CHUNK * 2];

  device.build_output_stream(
    config,
    move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
      let now = Instant::now();
      let stamp = info.timestamp();
      let latency = stamp.playback.duration_since(&stamp.callback).unwrap_or_default();
      renderer.saw_buffer(data.len() / channels);

      for (i, out) in data.chunks_mut(MAX_CHUNK * channels).enumerate() {
        let frames = out.len() / channels;
        let mix = &mut mix[..frames * 2];
        let offset = Duration::from_secs_f64((i * MAX_CHUNK) as f64 / sample_rate);
        renderer.render(mix, now, latency + offset);

        for (frame, lr) in out.chunks_mut(channels).zip(mix.chunks_exact(2)) {
          match frame.len() {
            1 => frame[0] = T::from_sample((lr[0] + lr[1]) * 0.5),
            _ => {
              frame[0] = T::from_sample(lr[0]);
              frame[1] = T::from_sample(lr[1]);
              for extra in frame[2..].iter_mut() { *extra = T::from_sample(0f32); }
            }
          }
        }
      }
    },
    |e| eprintln!("audio output error: {}", e),
    None,
  ).map_err(|e| format!("could not open the audio output: {}", e))
}

#[cfg(test)]
mod tests {
  use super::*;
  use rodio::buffer::SamplesBuffer;

  const RATE: u32 = 1_000;

  fn master() -> Arc<daw::ChannelControls> {
    daw::Mixer::default().master()
  }

  // a stereo click: `frames` frames of 1.0 at the test rate
  fn click(frames: usize) -> SamplesBuffer<f32> {
    SamplesBuffer::new(2, RATE, vec![1.; frames * 2])
  }

  // render `frames` frames, returning the left channel
  fn render(renderer: &mut Renderer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.; frames * 2];
    renderer.render(&mut out, Instant::now(), Duration::ZERO);
    out.chunks(2).map(|f| f[0]).collect()
  }

  fn first_sound(samples: &[f32]) -> Option<usize> {
    samples.iter().position(|&s| s != 0.)
  }

  #[test]
  fn test_voices_start_on_their_frame() {
    let (engine, mut renderer) = Engine::new(RATE);
    // the run starts after some frames have played
    render(&mut renderer, 37);

    engine.start_run(1, master(), vec![engine.voice(click(3), Some(1), 10, Route::Direct)]).unwrap();
    // a voice sent later lands on its frame too, even mid-buffer
    engine.play(vec![engine.voice(click(3), Some(1), 45, Route::Direct)]).unwrap();
    let out = render(&mut renderer, 64);

    assert_eq!(engine.anchor(1).unwrap().frame, 37);
    assert_eq!(&out[8..14], &[0., 0., 1., 1., 1., 0.]);
    assert_eq!(first_sound(&out[14..]), Some(45 - 14));
    assert_eq!(engine.frames(), 37 + 64);
  }

  #[test]
  fn test_voices_can_start_in_a_later_buffer() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![engine.voice(click(1), Some(1), 100, Route::Direct)]).unwrap();

    let mut out = render(&mut renderer, 64);
    out.extend(render(&mut renderer, 64));

    assert_eq!(first_sound(&out), Some(100));
  }

  #[test]
  fn test_a_late_voice_plays_from_where_it_would_be() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![]).unwrap();
    render(&mut renderer, 10);

    // should have started at frame 5: its first 5 frames are skipped
    let ramp = SamplesBuffer::new(2, RATE, (0..40).map(|i| (i / 2 + 1) as f32).collect::<Vec<_>>());
    engine.play(vec![engine.voice(ramp, Some(1), 5, Route::Direct)]).unwrap();
    let out = render(&mut renderer, 4);

    assert_eq!(out, vec![6., 7., 8., 9.]);
  }

  #[test]
  fn test_stopping_fades_out_and_drops_what_has_not_started() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![
      engine.voice(click(500), Some(1), 0, Route::Direct),
      engine.voice(click(500), Some(1), 200, Route::Direct),
    ]).unwrap();
    render(&mut renderer, 50);

    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 300);

    // a 10 frame fade at 1kHz, then nothing, and the later voice never plays
    assert!(out[0] > 0.8 && out[0] <= 1.);
    assert!(out[5] < out[0]);
    assert!(out[10..].iter().all(|&s| s == 0.));
    assert!(renderer.voices.is_empty());
  }

  #[test]
  fn test_a_new_run_replaces_the_old_one_and_stale_voices_are_dropped() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, master(), vec![]).unwrap();
    render(&mut renderer, 10);

    engine.start_run(2, master(), vec![engine.voice(click(1), Some(2), 5, Route::Direct)]).unwrap();
    // the old run's scheduler hasn't noticed yet
    engine.play(vec![engine.voice(click(1), Some(1), 12, Route::Direct)]).unwrap();
    // stopping the old run again does nothing to the new one
    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 20);

    assert_eq!(engine.anchor(2).unwrap().frame, 10);
    assert!(engine.anchor(1).is_none());
    assert_eq!(out.iter().filter(|&&s| s != 0.).count(), 1);
    assert_eq!(first_sound(&out), Some(5));
  }

  #[test]
  fn test_previews_play_now_and_ignore_runs() {
    let (engine, mut renderer) = Engine::new(RATE);
    render(&mut renderer, 10);
    engine.preview(click(3)).unwrap();
    engine.stop_run(0).unwrap();

    let out = render(&mut renderer, 8);
    assert_eq!(&out[..4], &[1., 1., 1., 0.]);
  }

  #[test]
  fn test_the_anchor_is_when_the_run_is_heard() {
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(3, master(), vec![]).unwrap();
    let now = Instant::now();
    let mut out = vec![0.; 16];
    renderer.render(&mut out, now, Duration::from_millis(12));

    let heard = engine.anchor(3).unwrap().heard.duration_since(now);
    assert!(heard >= Duration::from_micros(11_999) && heard <= Duration::from_micros(12_001));
    assert_eq!(engine.latency(), Duration::from_millis(12));
  }

  #[test]
  fn test_voices_are_resampled_to_the_engine_rate() {
    let (engine, mut renderer) = Engine::new(RATE);
    // 10 frames at twice the engine's rate last 5 of its frames
    let fast = SamplesBuffer::new(2, RATE * 2, vec![1f32; 20]);
    engine.preview(fast).unwrap();

    let out = render(&mut renderer, 10);
    let sounding = out.iter().filter(|&&s| s != 0.).count();
    assert!((4..=6).contains(&sounding), "{} frames sounded", sounding);
  }

  // render `frames` frames, returning (left, right)
  fn render_stereo(renderer: &mut Renderer, frames: usize) -> Vec<(f32, f32)> {
    let mut out = vec![0.; frames * 2];
    renderer.render(&mut out, Instant::now(), Duration::ZERO);
    out.chunks(2).map(|f| (f[0], f[1])).collect()
  }

  // `frames` frames of `level` on both sides
  fn steady(frames: usize, level: f32) -> SamplesBuffer<f32> {
    SamplesBuffer::new(2, RATE, vec![level; frames * 2])
  }

  fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
  }

  // multiplies by a gain
  struct Gain(f32);

  impl Processor for Gain {
    fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
      for (output, input) in output.into_iter().zip(input) {
        for (o, i) in output.iter_mut().zip(input) { *o = i * self.0; }
      }
    }
  }

  // plays its input back `frames` frames later (and only that)
  struct Delay {
    line: [Vec<f32>; 2],
    at: usize,
  }

  impl Delay {
    fn new(frames: usize) -> Self {
      Delay { line: [vec![0.; frames], vec![0.; frames]], at: 0 }
    }
  }

  impl Processor for Delay {
    fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
      let len = self.line[0].len();
      for (side, (output, input)) in output.into_iter().zip(input).enumerate() {
        for (i, (o, s)) in output.iter_mut().zip(input).enumerate() {
          let at = (self.at + i) % len;
          *o = self.line[side][at];
          self.line[side][at] = *s;
        }
      }
      self.at = (self.at + input[0].len()) % len;
    }
  }

  // passes the signal through, noting the transport and block length it was given
  struct Watch(Arc<Mutex<Vec<(Transport, usize)>>>);

  impl Processor for Watch {
    fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], transport: &Transport) {
      self.0.lock().unwrap().push((*transport, input[0].len()));
      for (output, input) in output.into_iter().zip(input) { output.copy_from_slice(input); }
    }
  }

  fn track(engine: &Engine, mixer: &daw::Mixer, number: u32, source: SamplesBuffer<f32>, at: u64) -> Box<Voice> {
    engine.voice(source, Some(1), at, Route::Track(mixer.track_route(number)))
  }

  #[test]
  fn test_track_buses_mix_as_gain_per_voice() {
    let mixer = daw::Mixer::default();
    mixer.set_volume_db(0, -6.);
    mixer.set_pan(0, -0.5);
    mixer.set_volume_db(1, -12.);
    mixer.set_pan(1, 0.75);

    // a different shape on each side, so pan shows
    let ramp = |frames: usize, level: f32| -> Vec<f32> {
      (0..frames).flat_map(|i| {
        let s = level * (i + 1) as f32 / frames as f32;
        [s, -0.5 * s]
      }).collect()
    };
    let voices = [(0, 0, ramp(40, 0.3)), (1, 10, ramp(50, 0.25)), (0, 30, ramp(20, 0.2))];

    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, mixer.master(), voices.iter()
      .map(|(number, at, samples)| track(&engine, &mixer, *number, SamplesBuffer::new(2, RATE, samples.clone()), *at))
      .collect()).unwrap();
    // in uneven buffers, so voices cross them
    let mut out = render_stereo(&mut renderer, 37);
    out.extend(render_stereo(&mut renderer, 63));

    let mut expected = vec![(0f32, 0f32); 100];
    for (number, at, samples) in &voices {
      let controls = mixer.track(*number);
      let volume = daw::db_to_gain(controls.volume_db());
      let pan = controls.pan();
      let gain = (volume * (1. - pan).min(1.), volume * (1. + pan).min(1.));
      for (i, frame) in samples.chunks(2).enumerate() {
        expected[*at as usize + i].0 += frame[0] * gain.0;
        expected[*at as usize + i].1 += frame[1] * gain.1;
      }
    }
    for (i, (got, want)) in out.iter().zip(&expected).enumerate() {
      assert!(close(got.0, want.0) && close(got.1, want.1), "frame {}: {:?} != {:?}", i, got, want);
    }
  }

  #[test]
  fn test_a_track_chain_changes_only_its_track_and_the_master_chain_all_but_direct() {
    let mixer = daw::Mixer::default();
    let play = |engine: &Engine, renderer: &mut Renderer| -> Vec<f32> {
      engine.start_run(1, mixer.master(), vec![
        track(engine, &mixer, 0, steady(10, 0.2), 0),
        track(engine, &mixer, 1, steady(10, 0.4), 20),
        engine.voice(steady(10, 0.3), Some(1), 40, Route::Direct),
      ]).unwrap();
      let out = render(renderer, 60);
      vec![out[5], out[25], out[45]]
    };

    let (engine, mut renderer) = Engine::new(RATE);
    engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(Gain(0.5)))]).unwrap();
    let out = play(&engine, &mut renderer);
    assert!(close(out[0], 0.1) && close(out[1], 0.4) && close(out[2], 0.3), "{:?}", out);

    let (engine, mut renderer) = Engine::new(RATE);
    engine.set_master_chain(vec![Slot::new(1, Box::new(Gain(0.5)))]).unwrap();
    let out = play(&engine, &mut renderer);
    assert!(close(out[0], 0.1) && close(out[1], 0.2) && close(out[2], 0.3), "{:?}", out);
  }

  #[test]
  fn test_an_effect_tail_rings_on_after_the_voices_end_and_after_a_stop() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(Delay::new(50)))]).unwrap();

    // the voice is over by frame 5, its echo comes at 50
    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 0)]).unwrap();
    let out = render(&mut renderer, 20);
    assert!(out.iter().all(|&s| s == 0.));
    assert!(renderer.voices.is_empty());

    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 60);
    assert_eq!(first_sound(&out), Some(50 - 20));
    assert!(out[30..35].iter().all(|&s| close(s, 0.5)));
    assert!(out[35..].iter().all(|&s| s == 0.));
  }

  #[test]
  fn test_a_bypassed_slot_passes_the_signal_through() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let silence = Slot::new(7, Box::new(Gain(0.)));
    silence.set_bypassed(true);
    assert_eq!(silence.id(), 7);
    engine.set_track_chain(&mixer.track_route(0), vec![silence.clone()]).unwrap();

    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(100, 0.5), 0)]).unwrap();
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| s == 0.5));

    silence.set_bypassed(false);
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| s == 0.));
  }

  #[test]
  fn test_a_replaced_chain_comes_back_to_be_dropped() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let first = Slot::new(1, Box::new(Gain(1.)));
    let master = Slot::new(2, Box::new(Gain(1.)));
    engine.set_track_chain(&mixer.track_route(3), vec![first.clone()]).unwrap();
    engine.set_master_chain(vec![master.clone()]).unwrap();
    // nothing has rendered, so nothing has taken them
    assert!(!engine.wait_for_chains(Duration::from_millis(5)));

    render(&mut renderer, 8);
    assert!(engine.wait_for_chains(Duration::from_secs(1)));
    assert_eq!(Arc::strong_count(&first), 2);
    assert_eq!(Arc::strong_count(&master), 2);

    engine.set_track_chain(&mixer.track_route(3), vec![Slot::new(3, Box::new(Gain(1.)))]).unwrap();
    engine.set_master_chain(vec![]).unwrap();
    render(&mut renderer, 8);
    assert!(engine.wait_for_chains(Duration::from_secs(1)));
    assert_eq!(Arc::strong_count(&first), 1);
    assert_eq!(Arc::strong_count(&master), 1);
  }

  #[test]
  fn test_a_track_voice_lands_on_its_frame() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    render(&mut renderer, 37);

    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 2, steady(3, 0.5), 10)]).unwrap();
    // one past a whole block, in the same buffer
    let late = MAX_CHUNK as u64 + 5;
    engine.play(vec![track(&engine, &mixer, 2, steady(1, 0.5), late)]).unwrap();
    let out = render(&mut renderer, MAX_CHUNK + 64);

    assert_eq!(&out[8..14], &[0., 0., 0.5, 0.5, 0.5, 0.]);
    assert_eq!(first_sound(&out[14..]).map(|i| i + 14), Some(late as usize));
  }

  #[test]
  fn test_processors_see_the_transport() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let watch = Slot::new(1, Box::new(Watch(seen.clone())));
    // (kept running through the silence: at this rate one block is longer
    // than the two seconds a silent chain rests after)
    watch.set_showing(true);
    engine.set_master_chain(vec![watch]).unwrap();
    render(&mut renderer, 16);

    let start = RunStart { tempo: 93.5, beats_per_bar: 3, beat_unit: 8, origin: 1_000 };
    engine.start_run_at(1, mixer.master(), start, vec![]).unwrap();
    render(&mut renderer, 64);
    // longer than a block: two calls
    render(&mut renderer, MAX_CHUNK + 10);
    engine.stop_run(1).unwrap();
    render(&mut renderer, 16);

    let seen = seen.lock().unwrap();
    let blocks: Vec<(bool, i64, usize)> = seen.iter().map(|(t, frames)| (t.playing, t.position, *frames)).collect();
    assert_eq!(blocks, vec![
      (false, 0, 16),
      (true, 1_000, 64),
      (true, 1_064, MAX_CHUNK),
      (true, 1_064 + MAX_CHUNK as i64, 10),
      // stopped where the run got to
      (false, 1_074 + MAX_CHUNK as i64, 16),
    ]);
    let (t, _) = seen[1];
    assert_eq!((t.tempo, t.beats_per_bar, t.beat_unit, t.sample_rate), (93.5, 3, 8, RATE));
  }

  #[test]
  fn test_mute_solo_and_meters_work_per_track() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let play = |renderer: &mut Renderer| -> Vec<f32> {
      // a moment of nothing first, so the buses start afresh, as new clips would
      render(renderer, 1);
      engine.start_run(1, mixer.master(), vec![
        track(&engine, &mixer, 0, steady(100, 0.5), 0),
        track(&engine, &mixer, 1, steady(100, 0.5), 100),
        track(&engine, &mixer, 2, steady(100, 0.5), 200),
      ]).unwrap();
      let out = render(renderer, 300);
      vec![out[50], out[150], out[250]]
    };

    mixer.set_volume_db(0, -6.);
    mixer.set_mute(1, true);
    let quieter = 0.5 * daw::db_to_gain(-6.);
    let out = play(&mut renderer);
    assert!(close(out[0], quieter) && out[1] == 0. && close(out[2], 0.5), "{:?}", out);
    // each meter shows its bus, after the fader
    let levels = mixer.take_levels();
    assert_eq!(levels.tracks.iter().map(|&(n, _)| n).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert!(close(levels.tracks[0].1, quieter));
    assert_eq!(levels.tracks[1].1, 0.);
    assert!(close(levels.tracks[2].1, 0.5));
    assert!(close(levels.master, 0.5));

    mixer.set_mute(1, false);
    mixer.set_solo(1, true);
    let out = play(&mut renderer);
    assert!(out[0] == 0. && close(out[1], 0.5) && out[2] == 0., "{:?}", out);
  }

  // an instrument: 0.25 while any pitch is held (each event at its offset),
  // ignoring what comes in
  struct Tone {
    held: [bool; 128],
  }

  impl Tone {
    fn slot(id: u64) -> Arc<Slot> {
      Slot::new_instrument(id, Box::new(Tone { held: [false; 128] }))
    }
  }

  impl Processor for Tone {
    fn process(&mut self, _: [&[f32]; 2], output: [&mut [f32]; 2], events: &[plugin::Event], _: &Transport) {
      let [left, right] = output;
      let mut next = events.iter().peekable();
      for i in 0..left.len() {
        while let Some(event) = next.next_if(|e| e.offset() as usize <= i) {
          match *event {
            plugin::Event::NoteOn { pitch, .. } => self.held[pitch as usize] = true,
            plugin::Event::NoteOff { pitch, .. } => self.held[pitch as usize] = false,
          }
        }
        let level = if self.held.iter().any(|&h| h) { 0.25 } else { 0. };
        left[i] = level;
        right[i] = level;
      }
    }
  }

  // an engine with a Tone at the head of track 0's chain, and the mixer
  fn with_tone() -> (Engine, Renderer, daw::Mixer) {
    let mixer = daw::Mixer::default();
    let (engine, renderer) = Engine::new(RATE);
    engine.set_track_chain(&mixer.track_route(0), vec![Tone::slot(1)]).unwrap();
    (engine, renderer, mixer)
  }

  // what the instrument at `at` in a bus's chain is playing
  fn playing(bus: &Bus, at: usize) -> &Playing {
    // safe: the test is the only thread rendering
    unsafe { bus.chain[at].playing().unwrap() }
  }

  // the frames that sound, as runs of (from, to)
  fn sounding(samples: &[f32]) -> Vec<(usize, usize)> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (i, &s) in samples.iter().enumerate() {
      if s == 0. { continue; }
      match runs.last_mut() {
        Some(run) if run.1 == i => run.1 = i + 1,
        _ => runs.push((i, i + 1)),
      }
    }
    runs
  }

  #[test]
  fn test_notes_start_and_end_on_their_frames() {
    let (engine, mut renderer, mixer) = with_tone();
    render(&mut renderer, 37);

    let route = mixer.track_route(0);
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    // mid-block, and one past a whole block, in the same buffer
    let late = MAX_CHUNK as u64 + 5;
    engine.play_notes(&route, 1, Some(1), vec![
      TimedNote::on(10, 60, 0.8),
      TimedNote::off(15, 60),
      TimedNote::on(late, 64, 0.8),
      TimedNote::off(late + 3, 64),
    ]).unwrap();
    let out = render(&mut renderer, MAX_CHUNK + 64);

    assert_eq!(sounding(&out), vec![(10, 15), (late as usize, late as usize + 3)]);
    // nothing held is left
    assert!(playing(&renderer.buses[0], 0).held_by_run.iter().all(|&n| n == 0));
  }

  #[test]
  fn test_a_note_can_end_where_the_same_pitch_starts_again() {
    let (engine, mut renderer, mixer) = with_tone();
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    // sent in the order a run sends them: the second note before the first ends
    engine.play_notes(&mixer.track_route(0), 1, Some(1), vec![
      TimedNote::on(5, 60, 1.),
      TimedNote::on(10, 60, 1.),
      TimedNote::off(10, 60),
      TimedNote::off(20, 60),
    ]).unwrap();
    let out = render(&mut renderer, 30);

    assert_eq!(sounding(&out), vec![(5, 20)]);
  }

  #[test]
  fn test_a_note_over_before_it_arrives_is_left_out_not_hung() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    // the notes come a buffer late (as a run's first slice can, if the
    // callback runs between the run's start and its notes)
    render(&mut renderer, 20);
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(5, 60, 1.), TimedNote::off(10, 60)]).unwrap();
    let out = render(&mut renderer, 30);

    assert!(out.iter().all(|&s| s == 0.));
    let bus = &renderer.buses[0];
    assert!(bus.notes.is_empty());
    assert!(playing(bus, 0).held_by_run.iter().all(|&n| n == 0));
    // and a stop leaves nothing sounding
    engine.stop_run(1).unwrap();
    assert!(render(&mut renderer, 30).iter().all(|&s| s == 0.));
  }

  #[test]
  fn test_a_late_note_that_hasnt_ended_plays_from_now_to_its_end() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    render(&mut renderer, 20);
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(5, 60, 1.), TimedNote::off(40, 60)]).unwrap();
    let out = render(&mut renderer, 30);

    assert_eq!(sounding(&out), vec![(0, 20)]);
    assert!(playing(&renderer.buses[0], 0).held_by_run.iter().all(|&n| n == 0));
  }

  #[test]
  fn test_a_late_note_ending_where_the_same_pitch_starts_again_still_plays_on() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(5, 60, 1.)]).unwrap();
    let mut out = render(&mut renderer, 20);
    // late: the first note's end and the next one's start (at a clip
    // boundary), and a short note that's over already
    engine.play_notes(&route, 1, Some(1), vec![
      TimedNote::on(12, 60, 1.),
      TimedNote::off(12, 60),
      TimedNote::on(14, 60, 1.),
      TimedNote::off(16, 60),
      TimedNote::off(30, 60),
    ]).unwrap();
    out.extend(render(&mut renderer, 20));

    assert_eq!(sounding(&out), vec![(5, 30)]);
    assert!(playing(&renderer.buses[0], 0).held_by_run.iter().all(|&n| n == 0));
  }

  #[test]
  fn test_a_stop_ends_the_notes_a_run_holds() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    // one held past the stop, one that hasn't started by then
    engine.play_notes(&route, 1, Some(1), vec![
      TimedNote::on(2, 60, 1.),
      TimedNote::off(500, 60),
      TimedNote::on(100, 62, 1.),
    ]).unwrap();
    let out = render(&mut renderer, 20);
    assert_eq!(sounding(&out), vec![(2, 20)]);

    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 200);
    assert!(out.iter().all(|&s| s == 0.));
    assert!(renderer.buses[0].notes.is_empty());
  }

  #[test]
  fn test_a_new_run_ends_the_old_ones_notes_and_drops_its_late_ones() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(0, 60, 1.)]).unwrap();
    render(&mut renderer, 10);

    engine.start_run(2, mixer.master(), vec![]).unwrap();
    // the old run's scheduler hasn't noticed yet
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(12, 61, 1.), TimedNote::off(14, 61)]).unwrap();
    engine.play_notes(&route, 1, Some(2), vec![TimedNote::on(30, 62, 1.), TimedNote::off(35, 62)]).unwrap();
    let out = render(&mut renderer, 50);

    assert_eq!(sounding(&out), vec![(30, 35)]);
  }

  #[test]
  fn test_an_audition_plays_now_and_outlives_a_stop() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    render(&mut renderer, 10);
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    engine.play_notes(&route, 1, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
    // the run holds the same pitch, and another
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(0, 60, 1.), TimedNote::on(0, 70, 1.)]).unwrap();
    let out = render(&mut renderer, 10);
    assert_eq!(sounding(&out), vec![(0, 10)]);

    // the run's notes end, but not the audition's
    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 10);
    assert_eq!(sounding(&out), vec![(0, 10)]);
    assert_eq!(playing(&renderer.buses[0], 0).held_by_audition[60], 1);

    engine.play_notes(&route, 1, None, vec![TimedNote::off(0, 60)]).unwrap();
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| s == 0.));
  }

  #[test]
  fn test_an_instrument_is_added_to_the_clips_and_both_go_through_the_effects() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    let instrument = Tone::slot(1);
    engine.set_track_chain(&route, vec![instrument.clone(), Slot::new(2, Box::new(Gain(0.5)))]).unwrap();
    mixer.set_volume_db(0, -6.);
    let fader = daw::db_to_gain(-6.);

    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(100, 0.2), 0)]).unwrap();
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(20, 60, 1.), TimedNote::off(40, 60)]).unwrap();
    let out = render(&mut renderer, 60);
    assert!(close(out[10], 0.2 * 0.5 * fader), "{}", out[10]);
    assert!(close(out[30], 0.45 * 0.5 * fader), "{}", out[30]);
    assert!(close(out[50], 0.2 * 0.5 * fader), "{}", out[50]);

    // bypassed, the clip passes through, and the notes aren't heard
    instrument.set_bypassed(true);
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(65, 60, 1.)]).unwrap();
    let out = render(&mut renderer, 20);
    assert!(out.iter().all(|&s| close(s, 0.2 * 0.5 * fader)));
  }

  #[test]
  fn test_too_many_notes_waiting_are_dropped_not_made_room_for() {
    let (engine, mut renderer, mixer) = with_tone();
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    let notes: Vec<TimedNote> = (0..MAX_NOTES as u64 + 10).map(|n| TimedNote::on(1_000 + n, 60, 1.)).collect();
    engine.play_notes(&mixer.track_route(0), 1, Some(1), notes).unwrap();
    render(&mut renderer, 1);
    let bus = &renderer.buses[0];
    assert_eq!(bus.notes.len(), MAX_NOTES);
    assert_eq!(bus.notes.capacity(), MAX_NOTES);
  }

  #[test]
  fn test_two_instruments_on_a_bus_each_hear_only_their_own_notes() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let route = mixer.track_route(0);
    let (a, b) = (Tone::slot(1), Tone::slot(2));
    // set before it first plays, so it starts there rather than ramping
    b.set_gain(2.);
    engine.set_track_chain(&route, vec![a, b, Slot::new(3, Box::new(Gain(0.5)))]).unwrap();

    // the same pitch on both: a's note-off doesn't end b's note
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(10, 60, 1.), TimedNote::off(20, 60)]).unwrap();
    engine.play_notes(&route, 2, Some(1), vec![TimedNote::on(15, 60, 1.), TimedNote::off(30, 60)]).unwrap();
    let out = render(&mut renderer, 40);

    assert!(out[..10].iter().all(|&s| s == 0.));
    assert!(out[10..15].iter().all(|&s| close(s, 0.25 * 0.5)), "{:?}", &out[10..15]);
    assert!(out[15..20].iter().all(|&s| close(s, (0.25 + 0.5) * 0.5)), "{:?}", &out[15..20]);
    assert!(out[20..30].iter().all(|&s| close(s, 0.5 * 0.5)), "{:?}", &out[20..30]);
    assert!(out[30..].iter().all(|&s| s == 0.));
    for at in 0..2 {
      assert!(playing(&renderer.buses[0], at).held_by_run.iter().all(|&n| n == 0));
    }
  }

  #[test]
  fn test_an_instrument_on_the_master_plays_through_the_master_chain() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    engine.set_master_chain(vec![Tone::slot(5), Slot::new(6, Box::new(Gain(0.5)))]).unwrap();

    // with a track's clip and a hit straight on the master
    engine.start_run(1, mixer.master(), vec![
      track(&engine, &mixer, 0, steady(100, 0.2), 0),
      engine.voice(steady(100, 0.1), Some(1), 0, Route::Master),
    ]).unwrap();
    engine.play_master_notes(5, Some(1), vec![TimedNote::on(20, 60, 1.), TimedNote::off(40, 60)]).unwrap();
    let out = render(&mut renderer, 60);
    assert!(close(out[10], 0.3 * 0.5), "{}", out[10]);
    assert!(close(out[30], 0.55 * 0.5), "{}", out[30]);
    assert!(close(out[50], 0.3 * 0.5), "{}", out[50]);

    // a key auditioned on it, with nothing else playing
    engine.stop_run(1).unwrap();
    render(&mut renderer, 20);
    engine.play_master_notes(5, None, vec![TimedNote::on(0, 64, 1.)]).unwrap();
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| close(s, 0.25 * 0.5)), "{:?}", out);
    engine.play_master_notes(5, None, vec![TimedNote::off(0, 64)]).unwrap();
    assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
  }

  #[test]
  fn test_an_instruments_gain_ramps_to_a_new_value_across_a_block() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);
    let instrument = Tone::slot(1);
    engine.set_track_chain(&route, vec![instrument.clone()]).unwrap();
    engine.play_notes(&route, 1, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| s == 0.25));

    instrument.set_gain(0.5);
    let out = render(&mut renderer, 10);
    // falling a step a frame, rather than all at once
    assert!(out[0] < 0.25 && out[0] > 0.125, "{:?}", out);
    assert!(out.windows(2).all(|w| w[1] < w[0]), "{:?}", out);
    assert!(close(out[9], 0.125), "{:?}", out);
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| close(s, 0.125)), "{:?}", out);

    // bypassed, it adds nothing
    instrument.set_bypassed(true);
    assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
  }

  #[test]
  fn test_a_stop_ends_what_each_instrument_holds_for_the_run() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let route = mixer.track_route(0);
    let (a, b) = (Tone::slot(1), Tone::slot(2));
    b.set_gain(2.);
    engine.set_track_chain(&route, vec![a, b]).unwrap();

    // a holds a note of the run's, b one of the run's and a key auditioned
    // (of the pitch a holds, which an audition on a doesn't keep)
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    engine.play_notes(&route, 1, Some(1), vec![TimedNote::on(0, 60, 1.), TimedNote::off(500, 60)]).unwrap();
    engine.play_notes(&route, 2, Some(1), vec![TimedNote::on(0, 62, 1.), TimedNote::off(500, 62)]).unwrap();
    engine.play_notes(&route, 2, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| close(s, 0.75)), "{:?}", out);

    engine.stop_run(1).unwrap();
    let out = render(&mut renderer, 10);
    assert!(out.iter().all(|&s| close(s, 0.5)), "{:?}", out);
    let bus = &renderer.buses[0];
    assert!(playing(bus, 0).held_by_run.iter().all(|&n| n == 0));
    assert!(playing(bus, 1).held_by_run.iter().all(|&n| n == 0));
    assert_eq!(playing(bus, 1).held_by_audition[60], 1);
    assert!(bus.notes.is_empty());

    engine.play_notes(&route, 2, None, vec![TimedNote::off(0, 60)]).unwrap();
    assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
  }

  #[test]
  fn test_notes_for_an_instrument_not_there_are_dropped_or_wait_for_its_chain() {
    let (engine, mut renderer, mixer) = with_tone();
    let route = mixer.track_route(0);

    // played now, for an id the chain hasn't, and on a master with none
    engine.play_notes(&route, 9, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
    engine.play_master_notes(9, None, vec![TimedNote::on(0, 60, 1.)]).unwrap();
    assert!(render(&mut renderer, 10).iter().all(|&s| s == 0.));
    assert!(renderer.buses[0].notes.is_empty());
    assert!(renderer.master.notes.is_empty());
    assert!(playing(&renderer.buses[0], 0).held_by_audition.iter().all(|&n| n == 0));

    // still to come, for an instrument whose chain comes before they do
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    engine.play_notes(&route, 3, Some(1), vec![TimedNote::on(30, 60, 1.), TimedNote::off(40, 60)]).unwrap();
    let mut out = render(&mut renderer, 20);
    engine.set_track_chain(&route, vec![Tone::slot(1), Tone::slot(3)]).unwrap();
    out.extend(render(&mut renderer, 30));
    assert_eq!(sounding(&out), vec![(30, 40)]);
  }

  // ---- resting: a chain that's silent, in and out, is left out

  // an effect that passes its input through and counts the frames it's
  // run for. it can say how long it rings on, and that a parameter is
  // waiting (which a block takes)
  struct Counted {
    frames: Arc<AtomicUsize>,
    tail: Option<u64>,
    pending: Arc<AtomicBool>,
  }

  impl Counted {
    fn new(tail: Option<u64>) -> (Counted, Arc<AtomicUsize>, Arc<AtomicBool>) {
      let frames = Arc::new(AtomicUsize::new(0));
      let pending = Arc::new(AtomicBool::new(false));
      (Counted { frames: frames.clone(), tail, pending: pending.clone() }, frames, pending)
    }
  }

  impl Processor for Counted {
    fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
      self.frames.fetch_add(input[0].len(), Ordering::Relaxed);
      self.pending.store(false, Ordering::Relaxed);
      for (output, input) in output.into_iter().zip(input) { output.copy_from_slice(input); }
    }

    fn tail_frames(&self) -> Option<u64> { self.tail }

    fn has_pending(&self) -> bool { self.pending.load(Ordering::Relaxed) }
  }

  // an effect that hums whatever comes in (a tape machine's hiss), counting
  // its frames
  struct Hum(Arc<AtomicUsize>);

  impl Processor for Hum {
    fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], _: &[plugin::Event], _: &Transport) {
      self.0.fetch_add(input[0].len(), Ordering::Relaxed);
      for output in output { output.fill(0.01); }
    }
  }

  // render `frames`, a hundred at a time (a rest is counted in blocks)
  fn render_blocks(renderer: &mut Renderer, frames: usize) -> Vec<f32> {
    (0..frames / 100).flat_map(|_| render(renderer, 100)).collect()
  }

  // two seconds, at the tests' rate
  const REST: usize = 2 * RATE as usize;

  #[test]
  fn test_a_silent_chain_rests_after_two_seconds_and_sound_wakes_it_at_once() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let (counted, frames, _) = Counted::new(Some(0));
    engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(counted))]).unwrap();

    // a voice five frames long: the chain runs for its block, and for two
    // seconds of silence after it
    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 0)]).unwrap();
    let out = render_blocks(&mut renderer, 5_000);
    assert_eq!(sounding(&out), vec![(0, 5)]);
    assert_eq!(frames.load(Ordering::Relaxed), 100 + REST);

    // and not again, however long the silence
    assert_eq!(engine.resting_chains(), 1);
    render_blocks(&mut renderer, 3_000);
    assert_eq!(frames.load(Ordering::Relaxed), 100 + REST);

    // a voice in the middle of a block wakes it for that block, and is
    // heard from its own frame
    engine.play(vec![track(&engine, &mixer, 0, steady(5, 0.5), 8_000 + 37)]).unwrap();
    let out = render_blocks(&mut renderer, 300);
    assert_eq!(engine.resting_chains(), 0);
    assert_eq!(sounding(&out), vec![(37, 42)]);
    assert!(out[37..42].iter().all(|&s| close(s, 0.5)));
    assert_eq!(frames.load(Ordering::Relaxed), 100 + REST + 300);
  }

  #[test]
  fn test_an_echo_within_two_seconds_is_heard_and_a_longer_one_needs_its_tail_said() {
    for (delay, tail, heard) in [(1_500, Some(0), true), (2_500, Some(0), false), (2_500, Some(2_500), true), (2_500, None, true)] {
      let mixer = daw::Mixer::default();
      let (engine, mut renderer) = Engine::new(RATE);
      let (counted, _, _) = Counted::new(tail);
      engine.set_track_chain(&mixer.track_route(0), vec![
        Slot::new(1, Box::new(Delay::new(delay))),
        // (says how long the chain rings on)
        Slot::new(2, Box::new(counted)),
      ]).unwrap();
      engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 0)]).unwrap();
      let out = render_blocks(&mut renderer, 6_000);
      let echo = if heard { vec![(delay, delay + 5)] } else { vec![] };
      assert_eq!(sounding(&out), echo, "a delay of {} with a tail of {:?}", delay, tail);
    }
  }

  #[test]
  fn test_a_plugin_that_never_stops_or_that_sounds_by_itself_never_rests() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let (endless, endless_frames, _) = Counted::new(None);
    let hum_frames = Arc::new(AtomicUsize::new(0));
    engine.set_track_chain(&mixer.track_route(0), vec![Slot::new(1, Box::new(endless))]).unwrap();
    engine.set_track_chain(&mixer.track_route(1), vec![Slot::new(2, Box::new(Hum(hum_frames.clone())))]).unwrap();
    // (the hum's track is muted: its chain is still sounding, before the
    // fader, so it's still run)
    mixer.set_mute(1, true);

    let out = render_blocks(&mut renderer, 6_000);
    assert!(out.iter().all(|&s| s == 0.));
    assert_eq!(endless_frames.load(Ordering::Relaxed), 6_000);
    assert_eq!(hum_frames.load(Ordering::Relaxed), 6_000);

    // unmuted, it's there at once
    mixer.set_mute(1, false);
    let out = render_blocks(&mut renderer, 1_000);
    assert!(close(*out.last().unwrap(), 0.01), "{:?}", out.last());
  }

  #[test]
  fn test_a_note_wakes_a_resting_instrument_on_its_frame_and_a_held_one_keeps_it_awake() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    // (an instrument that's silent even while it holds a note: a pad with
    // a slow attack)
    let (counted, frames, _) = Counted::new(Some(0));
    engine.set_track_chain(&mixer.track_route(0), vec![
      Slot::new_instrument(1, Box::new(counted)),
      Tone::slot(2),
    ]).unwrap();
    let route = mixer.track_route(0);
    render_blocks(&mut renderer, 5_000);
    assert_eq!(frames.load(Ordering::Relaxed), REST);

    // a note for the tone, 37 frames into a run: heard from that frame
    engine.start_run(1, mixer.master(), vec![]).unwrap();
    engine.play_notes(&route, 2, Some(1), vec![TimedNote::on(37, 60, 1.), TimedNote::off(60, 60)]).unwrap();
    let out = render_blocks(&mut renderer, 100);
    assert_eq!(sounding(&out), vec![(37, 60)]);

    // a note held on the silent one keeps the chain running for as long as
    // it's held, then for two seconds from the block that ends it
    engine.play_notes(&route, 1, None, vec![TimedNote::on(0, 64, 1.)]).unwrap();
    let before = frames.load(Ordering::Relaxed);
    render_blocks(&mut renderer, 6_000);
    assert_eq!(frames.load(Ordering::Relaxed), before + 6_000);
    engine.play_notes(&route, 1, None, vec![TimedNote::off(0, 64)]).unwrap();
    render_blocks(&mut renderer, 6_000);
    assert_eq!(frames.load(Ordering::Relaxed), before + 6_000 + REST);
  }

  #[test]
  fn test_a_showing_interface_a_turned_knob_a_switch_and_a_new_chain_run_a_resting_chain() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let (counted, frames, pending) = Counted::new(Some(0));
    let slot = Slot::new(1, Box::new(counted));
    engine.set_track_chain(&mixer.track_route(0), vec![slot.clone()]).unwrap();
    let ran = |renderer: &mut Renderer, frames_in: usize| {
      let before = frames.load(Ordering::Relaxed);
      render_blocks(renderer, frames_in);
      frames.load(Ordering::Relaxed) - before
    };
    assert_eq!(ran(&mut renderer, 5_000), REST);

    // its interface on screen: it runs for as long as it's there, and for
    // two seconds from the last block it was
    slot.set_showing(true);
    assert_eq!(ran(&mut renderer, 5_000), 5_000);
    slot.set_showing(false);
    assert_eq!(ran(&mut renderer, 5_000), REST - 100);

    // a knob turned: two seconds from the block that takes it
    pending.store(true, Ordering::Relaxed);
    assert_eq!(ran(&mut renderer, 5_000), REST);
    assert!(!pending.load(Ordering::Relaxed));

    // switched off, it isn't run at all; switched back on, it is again
    slot.set_bypassed(true);
    assert_eq!(ran(&mut renderer, 5_000), 0);
    slot.set_bypassed(false);
    assert_eq!(ran(&mut renderer, 5_000), REST);

    // and a chain that's changed
    let (other, other_frames, _) = Counted::new(Some(0));
    engine.set_track_chain(&mixer.track_route(0), vec![slot.clone(), Slot::new(2, Box::new(other))]).unwrap();
    assert_eq!(ran(&mut renderer, 5_000), REST);
    assert_eq!(other_frames.load(Ordering::Relaxed), REST);
  }

  #[test]
  fn test_the_master_chain_rests_too_and_what_goes_round_it_is_still_heard() {
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    let (counted, frames, _) = Counted::new(Some(0));
    engine.set_master_chain(vec![Slot::new(1, Box::new(counted))]).unwrap();
    let out = render_blocks(&mut renderer, 5_000);
    assert!(out.iter().all(|&s| s == 0.));
    assert_eq!(frames.load(Ordering::Relaxed), REST);

    // a preview goes straight to the output, and doesn't wake it
    engine.preview(click(3)).unwrap();
    let out = render_blocks(&mut renderer, 100);
    assert_eq!(&out[..4], &[1., 1., 1., 0.]);
    assert_eq!(frames.load(Ordering::Relaxed), REST);

    // a track's voice does, and comes out at its level from its frame
    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 20)]).unwrap();
    let out = render_blocks(&mut renderer, 100);
    assert_eq!(sounding(&out), vec![(20, 25)]);
    assert!(out[20..25].iter().all(|&s| close(s, daw::soft_limit(0.5))), "{:?}", &out[20..25]);
    assert_eq!(frames.load(Ordering::Relaxed), REST + 100);
  }

  #[test]
  fn test_a_bus_with_no_chain_is_as_it_was() {
    // nothing to rest: a voice two seconds into silence plays as ever
    let mixer = daw::Mixer::default();
    let (engine, mut renderer) = Engine::new(RATE);
    engine.start_run(1, mixer.master(), vec![track(&engine, &mixer, 0, steady(5, 0.5), 4_010)]).unwrap();
    let out = render_blocks(&mut renderer, 5_000);
    assert_eq!(sounding(&out), vec![(4_010, 4_015)]);
  }

  // plays through the real output: run with `cargo test -- --ignored`
  #[test]
  #[ignore = "needs an audio output device"]
  fn test_the_device_output_runs() {

    let engine = engine().expect("could not open the output");
    let start = engine.frames();
    engine.start_run(99, master(), vec![
      engine.voice(daw::METRONOME_TICK_SOURCE.convert_samples(), Some(99), 0, Route::Direct),
    ]).unwrap();

    let anchor = engine.wait_for_anchor(99, Duration::from_secs(2)).unwrap();
    assert!(anchor.frame >= start);
    assert!(engine.wait_for_frame(anchor.frame + engine.frames_in(Duration::from_millis(300)), || true));
    engine.stop_run(99).unwrap();

    println!(
      "rate {} Hz, buffer {} frames, output latency {:?}, anchored at frame {}",
      engine.sample_rate(), engine.buffer_frames(), engine.latency(), anchor.frame,
    );
    assert!(engine.buffer_frames() > 0);
  }

  // opens real outputs: run with `cargo test -- --ignored`. doesn't touch
  // the saved settings
  #[test]
  #[ignore = "needs an audio output device"]
  fn test_outputs_open_with_settings() {
    for frames in [128, 512] {
      let output = open_output(&AudioSettings { buffer_frames: Some(frames), ..Default::default() })
        .expect("could not open the output");
      println!(
        "{}: asked for {} frames, got {} ({:?} latency)",
        output.engine.device_name(), frames, output.engine.buffer_frames(), output.engine.latency(),
      );
      // some devices go no lower than they can (Bluetooth outputs stop at
      // 256, whatever range they claim), but never ask for less
      assert!(output.engine.buffer_frames() >= frames as u64);
    }

    let missing = open_output(&AudioSettings { output_device: Some("no such device".into()), ..Default::default() });
    assert!(missing.err().unwrap().contains("no such device"));
    let odd = open_output(&AudioSettings { buffer_frames: Some(100), ..Default::default() });
    assert!(odd.err().unwrap().contains("100 frames"));
  }
}
