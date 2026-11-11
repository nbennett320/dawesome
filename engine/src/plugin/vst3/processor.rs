// the audio half of a VST3 plugin: runs IAudioProcessor::process a block at
// a time on the audio thread
//
// everything a block needs (the parameter-change and event lists, the bus
// and channel tables, buffers for buses we don't use) is made when the
// processor is, so a block never allocates or locks. parameter changes come
// from the main half through one lock-free queue and the plugin's own go
// back through another (see Plugin::idle)
//
// the plugin object itself belongs to the main half (Plugin), which keeps it
// alive and tears it down. the two meet at the gate: a block only calls the
// plugin while the gate is open, and the main half closes it (waiting out a
// block in progress) before it stops, re-prepares or tears down the plugin.
// a processor whose gate is closed passes its input through (an effect) or
// is silent (an instrument), and is safe to keep or drop on any thread
use super::host::ParamChange;
use super::queues::{EventList, ParamChanges};
use crate::plugin::{Event, Processor, Transport, MAX_BLOCK};
use ::vst3::Steinberg::Vst::Event_::EventTypes_::{kNoteOffEvent, kNoteOnEvent};
use ::vst3::Steinberg::Vst::Event_::EventFlags_::kIsLive;
use ::vst3::Steinberg::Vst::ProcessContext_::StatesAndFlags_::{
  kBarPositionValid, kContTimeValid, kPlaying, kProjectTimeMusicValid, kTempoValid, kTimeSigValid,
};
use ::vst3::Steinberg::Vst::ProcessModes_::{kOffline, kRealtime};
use ::vst3::Steinberg::Vst::SymbolicSampleSizes_::kSample32;
use ::vst3::Steinberg::Vst::{
  AudioBusBuffers, AudioBusBuffers__type0, Event as VstEvent, IAudioProcessor, IAudioProcessorTrait,
  IEventList, IParameterChanges, ProcessContext, ProcessData,
};
use ::vst3::Steinberg::kResultOk;
use ::vst3::{ComRef, ComWrapper};
use rtrb::{Consumer, Producer};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

// whether the processor may call the plugin. see the top of the file
pub struct Gate {
  open: AtomicBool,
  // a block is calling the plugin
  busy: AtomicBool,
}

impl Gate {
  pub fn new() -> Gate {
    Gate { open: AtomicBool::new(true), busy: AtomicBool::new(false) }
  }

  // the audio thread, before calling the plugin. both sides store then load
  // (SeqCst), so either the block sees the gate closed or close() sees the
  // block busy and waits
  fn enter(&self) -> bool {
    self.busy.store(true, Ordering::SeqCst);
    if self.open.load(Ordering::SeqCst) {
      true
    } else {
      self.busy.store(false, Ordering::SeqCst);
      false
    }
  }

  fn leave(&self) {
    self.busy.store(false, Ordering::SeqCst);
  }

  // stop blocks calling the plugin, waiting out one in progress (at most a
  // block's time)
  pub fn close(&self) {
    self.open.store(false, Ordering::SeqCst);
    while self.busy.load(Ordering::SeqCst) {
      std::thread::yield_now();
    }
  }

  pub fn open(&self) {
    self.open.store(true, Ordering::SeqCst);
  }
}

// the channels of each of a plugin's audio buses, main bus first
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
  pub inputs: Vec<usize>,
  pub outputs: Vec<usize>,
  // whether the main input bus is used (effects). an instrument's inputs, if
  // it has any, are given silence
  pub main_input: bool,
}

pub struct VstProcessor {
  // the plugin's processor, kept alive by the main half while the gate's
  // open
  audio: *mut IAudioProcessor,
  gate: Arc<Gate>,
  from_main: Consumer<ParamChange>,
  to_main: Producer<ParamChange>,
  input_changes: ComWrapper<ParamChanges>,
  output_changes: ComWrapper<ParamChanges>,
  input_events: ComWrapper<EventList>,
  output_events: ComWrapper<EventList>,
  // a block's bus tables. their channel tables (below) don't move, so these
  // point into them from the start
  inputs: Vec<AudioBusBuffers>,
  outputs: Vec<AudioBusBuffers>,
  // each bus's channel pointers. the main buses' first two are pointed at
  // the block's own buffers before each block; the rest point into scratch
  input_channels: Vec<Vec<*mut f32>>,
  output_channels: Vec<Vec<*mut f32>>,
  // a buffer for each channel of each bus, inputs first
  scratch: Vec<Vec<f32>>,
  // how many of the scratch buffers belong to inputs (cleared each block)
  input_scratch: usize,
  main_input: bool,
  context: ProcessContext,
  // frames processed since the processor was made
  continuous: i64,
  // realtime, or offline for a mixdown (see Plugin::load_offline)
  mode: i32,
  // how long the plugin says it rings on, in frames (IAudioProcessor's
  // getTailSamples), kept up to date by the main half
  tail: Arc<AtomicU32>,
}

// what getTailSamples answers for a plugin that never stops sounding
// (kInfiniteTail)
pub const INFINITE_TAIL: u32 = u32::MAX;

// safe: the raw pointers are to the processor's own buffers (which move with
// it) and to the plugin's processor, which is only used while the gate is
// open, from whichever one thread is running the processor
unsafe impl Send for VstProcessor {}

impl VstProcessor {
  pub fn new(
    audio: *mut IAudioProcessor,
    gate: Arc<Gate>,
    from_main: Consumer<ParamChange>,
    to_main: Producer<ParamChange>,
    layout: &Layout,
    tail: Arc<AtomicU32>,
  ) -> VstProcessor {
    let mut scratch = Vec::new();
    let mut tables = |buses: &[usize]| -> Vec<Vec<*mut f32>> {
      buses
        .iter()
        .map(|&channels| {
          (0..channels)
            .map(|_| {
              let mut buffer = vec![0f32; MAX_BLOCK];
              let ptr = buffer.as_mut_ptr();
              // the Vec's heap buffer doesn't move when the Vec does
              scratch.push(buffer);
              ptr
            })
            .collect()
        })
        .collect()
    };
    let mut input_channels = tables(&layout.inputs);
    let input_scratch = layout.inputs.iter().sum();
    let mut output_channels = tables(&layout.outputs);
    let bus = |channels: &mut Vec<*mut f32>| AudioBusBuffers {
      numChannels: channels.len() as i32,
      silenceFlags: 0,
      __field0: AudioBusBuffers__type0 { channelBuffers32: channels.as_mut_ptr() },
    };
    let inputs = input_channels.iter_mut().map(bus).collect();
    let outputs = output_channels.iter_mut().map(bus).collect();

    VstProcessor {
      audio,
      gate,
      from_main,
      to_main,
      input_changes: ParamChanges::new(),
      output_changes: ParamChanges::new(),
      input_events: EventList::new(),
      output_events: EventList::new(),
      inputs,
      outputs,
      input_channels,
      output_channels,
      scratch,
      input_scratch,
      main_input: layout.main_input,
      // safe: ProcessContext is plain data, and all zeros is a valid one
      context: unsafe { std::mem::zeroed() },
      continuous: 0,
      mode: kRealtime as i32,
      tail,
    }
  }

  // for a plugin that was set up to render offline
  pub fn offline(mut self) -> VstProcessor {
    self.mode = kOffline as i32;
    self
  }

  // what a block sounds like without the plugin
  fn bypass(&self, input: [&[f32]; 2], output: [&mut [f32]; 2], n: usize) {
    for (out, inp) in output.into_iter().zip(input) {
      if self.main_input {
        out[..n].copy_from_slice(&inp[..n]);
      } else {
        out[..n].fill(0.);
      }
    }
  }
}

impl Processor for VstProcessor {
  fn process(&mut self, input: [&[f32]; 2], output: [&mut [f32]; 2], events: &[Event], transport: &Transport) {
    let n = output[0].len().min(output[1].len()).min(input[0].len()).min(input[1].len()).min(MAX_BLOCK);
    if !self.gate.enter() {
      self.bypass(input, output, n);
      return;
    }

    self.input_changes.clear();
    while let Ok((id, value)) = self.from_main.pop() {
      self.input_changes.set(id, value);
    }
    self.output_changes.clear();
    self.input_events.clear();
    for event in events {
      self.input_events.push(vst_event(event, n));
    }
    self.output_events.clear();
    fill_context(&mut self.context, transport, self.continuous);
    self.continuous += n as i64;

    // aux inputs hear silence, even if the plugin wrote to them last block
    for buffer in &mut self.scratch[..self.input_scratch] {
      buffer[..n].fill(0.);
    }
    // the main buses use the block's own buffers. a plugin doesn't write to
    // its inputs, so the input pointers are only read through
    if self.main_input {
      if let Some(channels) = self.input_channels.first_mut() {
        for (c, ptr) in channels.iter_mut().take(2).enumerate() {
          *ptr = input[c].as_ptr() as *mut f32;
        }
      }
    }
    let mono_out = self.output_channels.first().map_or(false, |c| c.len() == 1);
    if let Some(channels) = self.output_channels.first_mut() {
      for (c, ptr) in channels.iter_mut().take(2).enumerate() {
        *ptr = output[c].as_ptr() as *mut f32;
      }
    }

    let mut data = ProcessData {
      processMode: self.mode,
      symbolicSampleSize: kSample32 as i32,
      numSamples: n as i32,
      numInputs: self.inputs.len() as i32,
      numOutputs: self.outputs.len() as i32,
      inputs: if self.inputs.is_empty() { std::ptr::null_mut() } else { self.inputs.as_mut_ptr() },
      outputs: if self.outputs.is_empty() { std::ptr::null_mut() } else { self.outputs.as_mut_ptr() },
      inputParameterChanges: com_ptr::<_, IParameterChanges>(&self.input_changes),
      outputParameterChanges: com_ptr::<_, IParameterChanges>(&self.output_changes),
      inputEvents: com_ptr::<_, IEventList>(&self.input_events),
      outputEvents: com_ptr::<_, IEventList>(&self.output_events),
      processContext: &mut self.context,
    };
    // safe: the gate is open, so the main half is keeping the plugin alive
    // and prepared, and everything data points at lives through the call
    let result = unsafe { ComRef::from_raw(self.audio).map(|audio| audio.process(&mut data)) };
    self.gate.leave();

    if result != Some(kResultOk) || self.outputs.is_empty() {
      self.bypass(input, output, n);
      return;
    }
    if mono_out {
      let [left, right] = output;
      right[..n].copy_from_slice(&left[..n]);
    }
    let to_main = &mut self.to_main;
    self.output_changes.last_values(|id, value| {
      // a full queue drops the change: the main half is behind
      let _ = to_main.push((id, value));
    });
  }

  fn tail_frames(&self) -> Option<u64> {
    match self.tail.load(Ordering::Relaxed) {
      INFINITE_TAIL => None,
      frames => Some(frames as u64),
    }
  }

  // a parameter the main half changed, which the plugin's processor only
  // hears of with a block
  fn has_pending(&self) -> bool {
    !self.from_main.is_empty()
  }
}

fn com_ptr<C: ::vst3::Class, I: ::vst3::Interface>(wrapper: &ComWrapper<C>) -> *mut I {
  wrapper.as_com_ref::<I>().map_or(std::ptr::null_mut(), |r| r.as_ptr())
}

// a note for the plugin, on the first event bus, channel 1
pub fn vst_event(event: &Event, frames: usize) -> VstEvent {
  // safe: Event is plain data, and all zeros is a valid one
  let mut e: VstEvent = unsafe { std::mem::zeroed() };
  e.busIndex = 0;
  e.sampleOffset = (event.offset() as usize).min(frames.saturating_sub(1)) as i32;
  e.flags = kIsLive as u16;
  match *event {
    Event::NoteOn { pitch, velocity, .. } => {
      e.r#type = kNoteOnEvent as u16;
      e.__field0.noteOn.channel = 0;
      e.__field0.noteOn.pitch = pitch as i16;
      e.__field0.noteOn.velocity = velocity;
      e.__field0.noteOn.noteId = -1;
    }
    Event::NoteOff { pitch, .. } => {
      e.r#type = kNoteOffEvent as u16;
      e.__field0.noteOff.channel = 0;
      e.__field0.noteOff.pitch = pitch as i16;
      e.__field0.noteOff.velocity = 0.;
      e.__field0.noteOff.noteId = -1;
    }
  }
  e
}

// where the block is in the song, for the plugin
pub fn fill_context(context: &mut ProcessContext, transport: &Transport, continuous: i64) {
  let rate = transport.sample_rate as f64;
  let quarters = if rate > 0. { transport.position as f64 / rate * transport.tempo / 60. } else { 0. };
  // a bar's length in quarter notes (6/8 is 3)
  let bar = transport.beats_per_bar as f64 * 4. / transport.beat_unit.max(1) as f64;
  context.state = (kTempoValid | kTimeSigValid | kProjectTimeMusicValid | kBarPositionValid | kContTimeValid) as u32;
  if transport.playing {
    context.state |= kPlaying as u32;
  }
  context.sampleRate = rate;
  context.projectTimeSamples = transport.position;
  context.continousTimeSamples = continuous;
  context.projectTimeMusic = quarters;
  context.barPositionMusic = if bar > 0. { (quarters / bar).floor() * bar } else { 0. };
  context.tempo = transport.tempo;
  context.timeSigNumerator = transport.beats_per_bar as i32;
  context.timeSigDenominator = transport.beat_unit as i32;
}

#[cfg(test)]
mod tests {
  use super::*;
  use rtrb::RingBuffer;

  #[test]
  fn test_context_has_musical_position_from_frames_and_tempo() {
    let mut context: ProcessContext = unsafe { std::mem::zeroed() };
    let transport = Transport {
      playing: true,
      tempo: 120.,
      beats_per_bar: 4,
      beat_unit: 4,
      // 5.5 seconds at 120 bpm is 11 quarter notes, in the third bar
      position: 5 * 48_000 + 24_000,
      sample_rate: 48_000,
    };
    fill_context(&mut context, &transport, 99);
    assert_eq!(context.projectTimeMusic, 11.);
    assert_eq!(context.barPositionMusic, 8.);
    assert_eq!(context.projectTimeSamples, 264_000);
    assert_eq!(context.continousTimeSamples, 99);
    assert_eq!(context.tempo, 120.);
    assert_eq!((context.timeSigNumerator, context.timeSigDenominator), (4, 4));
    assert_ne!(context.state & kPlaying as u32, 0);
    assert_ne!(context.state & kTempoValid as u32, 0);

    let stopped = Transport { playing: false, beats_per_bar: 6, beat_unit: 8, ..transport };
    fill_context(&mut context, &stopped, 0);
    assert_eq!(context.state & kPlaying as u32, 0);
    // a 6/8 bar is three quarter notes
    assert_eq!(context.barPositionMusic, 9.);
  }

  #[test]
  fn test_notes_become_vst_events_inside_the_block() {
    let on = vst_event(&Event::NoteOn { offset: 10, pitch: 60, velocity: 0.5 }, 64);
    assert_eq!(on.r#type, kNoteOnEvent as u16);
    assert_eq!(on.sampleOffset, 10);
    unsafe {
      assert_eq!(on.__field0.noteOn.pitch, 60);
      assert_eq!(on.__field0.noteOn.velocity, 0.5);
      assert_eq!(on.__field0.noteOn.noteId, -1);
    }
    let off = vst_event(&Event::NoteOff { offset: 500, pitch: 61 }, 64);
    assert_eq!(off.r#type, kNoteOffEvent as u16);
    // an offset past the block is kept inside it
    assert_eq!(off.sampleOffset, 63);
    unsafe {
      assert_eq!(off.__field0.noteOff.pitch, 61);
    }
  }

  fn processor(main_input: bool) -> VstProcessor {
    let (_to, from_main) = RingBuffer::new(4);
    let (to_main, _from) = RingBuffer::new(4);
    let layout = Layout { inputs: vec![2, 1], outputs: vec![2], main_input };
    VstProcessor::new(std::ptr::null_mut(), Arc::new(Gate::new()), from_main, to_main, &layout, Arc::new(AtomicU32::new(0)))
  }

  #[test]
  fn test_the_tail_is_what_the_main_half_last_read() {
    let mut p = processor(true);
    assert_eq!(p.tail_frames(), Some(0));
    p.tail.store(4_800, Ordering::Relaxed);
    assert_eq!(p.tail_frames(), Some(4_800));
    // a plugin that never stops
    p.tail.store(INFINITE_TAIL, Ordering::Relaxed);
    assert_eq!(p.tail_frames(), None);
    let _ = &mut p;
  }

  #[test]
  fn test_a_parameter_change_from_the_main_half_is_pending_until_a_block() {
    let (mut to, from_main) = RingBuffer::new(4);
    let (to_main, _from) = RingBuffer::new(4);
    let layout = Layout { inputs: vec![2], outputs: vec![2], main_input: true };
    let gate = Arc::new(Gate::new());
    let mut p = VstProcessor::new(std::ptr::null_mut(), gate.clone(), from_main, to_main, &layout, Arc::new(AtomicU32::new(0)));
    assert!(!p.has_pending());
    to.push((7, 0.5)).unwrap();
    assert!(p.has_pending());

    // a block takes it (there's no plugin to call here, so the block itself
    // fails after that and passes the sound through)
    let (l, r) = ([0f32; 8], [0f32; 8]);
    let (mut ol, mut or) = ([0f32; 8], [0f32; 8]);
    p.process([&l, &r], [&mut ol, &mut or], &[], &Transport::default());
    assert!(!p.has_pending());
  }

  #[test]
  fn test_tables_have_a_buffer_for_every_channel() {
    let p = processor(true);
    assert_eq!(p.inputs.len(), 2);
    assert_eq!(p.inputs[1].numChannels, 1);
    // three input channels, two output
    assert_eq!(p.scratch.len(), 5);
    assert_eq!(p.input_scratch, 3);
    unsafe {
      assert_eq!(*p.inputs[1].__field0.channelBuffers32, p.input_channels[1][0]);
    }
  }

  #[test]
  fn test_closed_gate_passes_effects_through_and_silences_instruments() {
    let input = [vec![0.5f32; 32], vec![-0.5f32; 32]];
    for main_input in [true, false] {
      let mut p = processor(main_input);
      p.gate.close();
      let (mut left, mut right) = (vec![9f32; 32], vec![9f32; 32]);
      p.process([&input[0], &input[1]], [&mut left, &mut right], &[], &Transport::default());
      let expect = if main_input { (0.5, -0.5) } else { (0., 0.) };
      assert!(left.iter().all(|&s| s == expect.0));
      assert!(right.iter().all(|&s| s == expect.1));
    }
  }

  #[test]
  fn test_a_missing_plugin_processor_counts_as_a_failed_block() {
    // a null plugin pointer can't be called, so the block is bypassed
    let input = [vec![0.25f32; 16], vec![0.25f32; 16]];
    let mut p = processor(true);
    let (mut left, mut right) = (vec![0f32; 16], vec![0f32; 16]);
    p.process([&input[0], &input[1]], [&mut left, &mut right], &[], &Transport::default());
    assert!(left.iter().chain(&right).all(|&s| s == 0.25));
    // and the gate is left for the next block
    assert!(p.gate.enter());
  }
}
