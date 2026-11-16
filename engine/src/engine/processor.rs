// what the engine runs in a channel's chain, a block at a time: a plugin's
// audio half (plugin/ hosts VST3 ones, native.rs the built-in effects), and
// what it's told

// the most frames a processor is ever given at once
pub const MAX_BLOCK: usize = 4096;

// what a processor is told about the moment it's playing (the built-in
// effects' crate has it, as they're told the same)
pub use dsp::Transport;

// something for an instrument to play, `offset` frames into the block
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
  // velocity from 0 to 1
  NoteOn { offset: u32, pitch: u8, velocity: f32 },
  NoteOff { offset: u32, pitch: u8 },
}

impl Event {
  pub fn offset(&self) -> u32 {
    match *self {
      Event::NoteOn { offset, .. } | Event::NoteOff { offset, .. } => offset,
    }
  }
}

// the half of a plugin the audio thread runs
pub trait Processor: Send {
  // run one block: stereo in (left, right), stereo out, all four the same
  // length (at most MAX_BLOCK). `output` holds whatever was there before, and
  // must be filled. `events` are in order of their offset
  //
  // called on the audio thread: it mustn't lock, allocate, or touch the disk
  fn process(
    &mut self,
    input: [&[f32]; 2],
    output: [&mut [f32]; 2],
    events: &[Event],
    transport: &Transport,
  );

  // how long it can go on sounding after its input stops, in frames, as far
  // as it says: none for one that never stops. a chain isn't rested (see
  // Bus::rests in engine.rs) until it's been silent for this long as well
  //
  // called on the audio thread, like process
  fn tail_frames(&self) -> Option<u64> {
    Some(0)
  }

  // whether something's waiting that only a block can hand it (a parameter
  // turned on the main thread): a chain that's resting runs again for it
  //
  // called on the audio thread, like process
  fn has_pending(&self) -> bool {
    false
  }
}
