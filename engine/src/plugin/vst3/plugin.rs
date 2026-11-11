// the main-thread half of a VST3 plugin: makes the plugin's objects, and
// holds its parameters, its saved state and its window. its Processor (see
// processor.rs) is the audio half
//
// a VST3 plugin is a component (the audio side, which is also its
// IAudioProcessor) and an edit controller (parameters and the window). they
// are either one object, or two that we connect so they can talk
//
// every method here is for the main (UI) thread: many plugins (JUCE's in
// particular) expect their non-audio calls on the process's main thread
use super::classes::{classes, parse_plugin_id, text16};
use super::host::{ComponentHandler, HostApplication, ParamChange, PlugFrame, Shared};
use super::module::Module;
use super::processor::{Gate, Layout, VstProcessor};
use super::state;
use super::stream::MemoryStream;
use crate::plugin::{ParamInfo, PluginInfo, PluginKind, Processor, MAX_BLOCK};
use ::vst3::Steinberg::Vst::BusDirections_::{kInput, kOutput};
use ::vst3::Steinberg::Vst::MediaTypes_::{kAudio, kEvent};
use ::vst3::Steinberg::Vst::ParameterInfo_::ParameterFlags_::{kCanAutomate, kIsBypass, kIsHidden, kIsReadOnly};
use ::vst3::Steinberg::Vst::ProcessModes_::{kOffline, kRealtime};
use ::vst3::Steinberg::Vst::RestartFlags_::{kLatencyChanged, kReloadComponent};
use ::vst3::Steinberg::Vst::SpeakerArr::kStereo;
use ::vst3::Steinberg::Vst::SymbolicSampleSizes_::kSample32;
use ::vst3::Steinberg::Vst::{
  IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler, IComponentTrait, IConnectionPoint,
  IConnectionPointTrait, IEditController, IEditControllerTrait, IHostApplication, ParameterInfo, ProcessSetup,
  SpeakerArrangement, String128,
};
use ::vst3::Steinberg::{
  kResultOk, kResultTrue, FIDString, FUnknown, IBStream, IPlugFrame, IPlugView, IPlugViewTrait,
  IPluginBaseTrait, IPluginFactory3, IPluginFactory3Trait, IPluginFactoryTrait, ViewRect, TUID,
};
use ::vst3::{Class, ComPtr, ComWrapper, Interface};
use rtrb::{Consumer, RingBuffer};
use std::cell::Cell;
use std::ffi::c_void;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

// how many parameter changes can wait between the halves
const QUEUE: usize = 4096;

pub struct Plugin {
  info: PluginInfo,
  sample_rate: u32,
  latency: u32,
  // how long it says it rings on after its input stops, in frames, which
  // its processor reads on the audio thread (see Processor::tail_frames)
  tail: Arc<AtomicU32>,
  // the open window's view
  editor: Option<ComPtr<IPlugView>>,
  has_editor: Cell<Option<bool>>,
  frame: ComWrapper<PlugFrame>,
  shared: Arc<Shared>,
  gate: Arc<Gate>,
  from_processor: Consumer<ParamChange>,
  // until it's taken
  processor: Option<VstProcessor>,
  com: Com,
  handler: ComWrapper<ComponentHandler>,
  host: ComWrapper<HostApplication>,
}

// safe to move between threads (it sits in a registry behind a mutex), but
// every method is still for the main thread: see the top of the file
unsafe impl Send for Plugin {}

// the plugin's objects, and how far they got. dropping it undoes what was
// done, in the order the SDK asks: stop processing, deactivate, disconnect,
// terminate, release (which may wait for the module: see module.rs). the
// module goes last
struct Com {
  controller: Option<ComPtr<IEditController>>,
  // the controller is its own object (and so was initialized by us)
  separate: bool,
  connected: Option<(ComPtr<IConnectionPoint>, ComPtr<IConnectionPoint>)>,
  audio: Option<ComPtr<IAudioProcessor>>,
  component: ComPtr<IComponent>,
  active: bool,
  processing: bool,
  // realtime, or offline (a mixdown's copy)
  mode: i32,
  module: Arc<Module>,
}

impl Com {
  fn audio(&self) -> &ComPtr<IAudioProcessor> {
    // load doesn't make a Plugin without one
    self.audio.as_ref().expect("plugin without an audio processor")
  }

  // set up for a sample rate, activate, and start processing
  fn start(&mut self, sample_rate: u32) -> Result<(), String> {
    let mut setup = ProcessSetup {
      processMode: self.mode,
      symbolicSampleSize: kSample32 as i32,
      maxSamplesPerBlock: MAX_BLOCK as i32,
      sampleRate: sample_rate as f64,
    };
    // safe (here and below): the plugin's objects are alive and initialized
    if unsafe { self.audio().setupProcessing(&mut setup) } != kResultOk {
      return Err(format!("the plugin can't run at {} Hz", sample_rate));
    }
    if unsafe { self.component.setActive(1) } != kResultOk {
      return Err("the plugin wouldn't activate".into());
    }
    self.active = true;
    // some plugins don't implement setProcessing, which is fine
    unsafe { self.audio().setProcessing(1) };
    self.processing = true;
    Ok(())
  }

  fn stop(&mut self) {
    if self.processing {
      unsafe { self.audio().setProcessing(0) };
      self.processing = false;
    }
    if self.active {
      unsafe { self.component.setActive(0) };
      self.active = false;
    }
  }
}

impl Drop for Com {
  fn drop(&mut self) {
    self.stop();
    // safe: the objects are alive until the fields drop after this
    unsafe {
      if let Some((component, controller)) = self.connected.take() {
        component.disconnect(controller.as_ptr());
        controller.disconnect(component.as_ptr());
      }
      if let Some(controller) = &self.controller {
        controller.setComponentHandler(std::ptr::null_mut());
        if self.separate {
          controller.terminate();
        }
      }
      self.component.terminate();
    }
    // with other plugins from the module still alive, the objects are kept
    // until it goes (see module.rs). otherwise the fields drop in order:
    // controller, audio, component, then module
    if Arc::strong_count(&self.module) > 1 {
      let objects = (self.controller.take(), self.audio.take(), self.component.clone());
      self.module.park(Box::new(objects));
    }
  }
}

// a COM pointer to an object of ours, as the plugin takes it
fn ours<C: Class, I: Interface>(wrapper: &ComWrapper<C>) -> *mut I {
  wrapper.as_com_ref::<I>().map_or(std::ptr::null_mut(), |r| r.as_ptr())
}

fn create<I: Interface>(module: &Module, cid: &TUID) -> Option<ComPtr<I>> {
  let mut obj = std::ptr::null_mut();
  // safe: the factory fills obj with an object of interface I, with a
  // reference for us, or leaves it null
  unsafe {
    let result =
      module.factory().createInstance(cid.as_ptr() as FIDString, I::IID.as_ptr() as FIDString, &mut obj);
    if result != kResultOk {
      return None;
    }
    ComPtr::from_raw(obj as *mut I)
  }
}

fn channels(arrangement: SpeakerArrangement) -> usize {
  arrangement.count_ones() as usize
}

// ask for stereo on the main buses, and turn on only the buses we use: the
// main output, the main input for an effect, and the first event input
fn setup_buses(component: &ComPtr<IComponent>, audio: &ComPtr<IAudioProcessor>, kind: PluginKind) -> Layout {
  let use_input = kind == PluginKind::Effect;
  // safe (here and below): the objects are initialized and not yet active
  let count = |media, dir| unsafe { component.getBusCount(media as i32, dir as i32) }.max(0);
  let arrangement = |dir, i| {
    let mut arr: SpeakerArrangement = kStereo;
    if unsafe { audio.getBusArrangement(dir as i32, i, &mut arr) } == kResultOk {
      arr
    } else {
      kStereo
    }
  };
  let (ins, outs) = (count(kAudio, kInput), count(kAudio, kOutput));
  let mut inputs: Vec<SpeakerArrangement> = (0..ins).map(|i| arrangement(kInput, i)).collect();
  let mut outputs: Vec<SpeakerArrangement> = (0..outs).map(|i| arrangement(kOutput, i)).collect();
  if use_input && !inputs.is_empty() {
    inputs[0] = kStereo;
  }
  if !outputs.is_empty() {
    outputs[0] = kStereo;
  }
  // a plugin that won't take stereo keeps what it had (mono is handled)
  unsafe {
    audio.setBusArrangements(inputs.as_mut_ptr(), ins, outputs.as_mut_ptr(), outs);
  }
  let inputs: Vec<usize> = (0..ins).map(|i| channels(arrangement(kInput, i))).collect();
  let outputs: Vec<usize> = (0..outs).map(|i| channels(arrangement(kOutput, i))).collect();

  unsafe {
    for i in 0..ins {
      component.activateBus(kAudio as i32, kInput as i32, i, (use_input && i == 0) as u8);
    }
    for i in 0..outs {
      component.activateBus(kAudio as i32, kOutput as i32, i, (i == 0) as u8);
    }
    for i in 0..count(kEvent, kInput) {
      component.activateBus(kEvent as i32, kInput as i32, i, (i == 0) as u8);
    }
    for i in 0..count(kEvent, kOutput) {
      component.activateBus(kEvent as i32, kOutput as i32, i, 0);
    }
  }
  let main_input = use_input && inputs.first().map_or(false, |&c| c > 0);
  Layout { inputs, outputs, main_input }
}

// the window system's name for the parent handed to open_editor
fn platform_type() -> FIDString {
  #[cfg(target_os = "macos")]
  return ::vst3::Steinberg::kPlatformTypeNSView;
  #[cfg(target_os = "windows")]
  return ::vst3::Steinberg::kPlatformTypeHWND;
  #[cfg(not(any(target_os = "macos", target_os = "windows")))]
  return ::vst3::Steinberg::kPlatformTypeX11EmbedWindowID;
}

const EDITOR: FIDString = b"editor\0".as_ptr() as FIDString;

fn size_of(rect: &ViewRect) -> (u32, u32) {
  ((rect.right - rect.left).max(0) as u32, (rect.bottom - rect.top).max(0) as u32)
}

impl Plugin {
  // load the plugin `id` from the bundle at `path`, and get it ready to
  // process: stereo in and out (an instrument: notes in, stereo out), 32-bit
  // float, realtime, at most MAX_BLOCK frames a block
  pub fn load(path: &Path, id: &str, sample_rate: u32) -> Result<Plugin, String> {
    Plugin::load_in(path, id, sample_rate, kRealtime as i32)
  }

  // the same, but the plugin is told it's rendering offline: faster than
  // it's heard, with no hurry, so it needn't cut corners. (it doesn't make
  // one that loads its sounds late have them any sooner: see SETTLE in
  // effects.rs)
  pub fn load_offline(path: &Path, id: &str, sample_rate: u32) -> Result<Plugin, String> {
    Plugin::load_in(path, id, sample_rate, kOffline as i32)
  }

  fn load_in(path: &Path, id: &str, sample_rate: u32, mode: i32) -> Result<Plugin, String> {
    let cid = parse_plugin_id(id).ok_or_else(|| format!("{} isn't a VST3 plugin id", id))?;
    let module = Module::open(path)?;
    let info = classes(&module)
      .into_iter()
      .find(|(c, _)| *c == cid)
      .map(|(_, info)| info)
      .ok_or_else(|| format!("{} isn't in {}", id, path.display()))?;

    let host = ComWrapper::new(HostApplication);
    let context: *mut FUnknown = ours::<_, IHostApplication>(&host) as *mut FUnknown;
    if let Some(factory3) = module.factory().cast::<IPluginFactory3>() {
      // safe: the host object outlives the module's use of it (the plugin
      // takes its own reference if it keeps it)
      unsafe { factory3.setHostContext(context) };
    }

    let component =
      create::<IComponent>(&module, &cid).ok_or_else(|| format!("{} couldn't be made", info.name))?;
    // safe (here and below): the objects were just made, and are used as the
    // SDK describes
    if unsafe { component.initialize(context) } != kResultOk {
      return Err(format!("{} wouldn't initialize", info.name));
    }
    let mut com = Com {
      controller: None,
      separate: false,
      connected: None,
      audio: None,
      component,
      active: false,
      processing: false,
      mode,
      module,
    };

    // the controller: the component itself, or a second object it names
    if let Some(controller) = com.component.cast::<IEditController>() {
      com.controller = Some(controller);
    } else {
      let mut controller_id: TUID = [0; 16];
      let named = unsafe { com.component.getControllerClassId(&mut controller_id) } == kResultOk
        && controller_id != [0; 16];
      if let Some(controller) = named.then(|| create::<IEditController>(&com.module, &controller_id)).flatten() {
        if unsafe { controller.initialize(context) } == kResultOk {
          com.controller = Some(controller);
          com.separate = true;
        }
      }
    }

    let (to_processor, from_main) = RingBuffer::new(QUEUE);
    let (to_main, from_processor) = RingBuffer::new(QUEUE);
    let shared = Arc::new(Shared { to_processor: Mutex::new(to_processor), restart: Default::default() });
    let handler = ComWrapper::new(ComponentHandler { shared: shared.clone() });
    if let Some(controller) = &com.controller {
      unsafe { controller.setComponentHandler(ours::<_, IComponentHandler>(&handler)) };
      if com.separate {
        // connect the two halves, so they can send each other messages
        if let (Some(a), Some(b)) = (com.component.cast::<IConnectionPoint>(), controller.cast::<IConnectionPoint>()) {
          unsafe {
            a.connect(b.as_ptr());
            b.connect(a.as_ptr());
          }
          com.connected = Some((a, b));
        }
        // and tell the controller what the component starts out as
        let stream = MemoryStream::new();
        if unsafe { com.component.getState(ours::<_, IBStream>(&stream)) } == kResultOk {
          stream.rewind();
          unsafe { controller.setComponentState(ours::<_, IBStream>(&stream)) };
        }
      }
    }

    let audio = com
      .component
      .cast::<IAudioProcessor>()
      .ok_or_else(|| format!("{} doesn't process audio", info.name))?;
    if unsafe { audio.canProcessSampleSize(kSample32 as i32) } != kResultOk {
      return Err(format!("{} can't process 32-bit audio", info.name));
    }
    let layout = setup_buses(&com.component, &audio, info.kind);
    if layout.outputs.first().map_or(true, |&c| c == 0) {
      return Err(format!("{} has no audio output", info.name));
    }
    com.audio = Some(audio);
    com.start(sample_rate)?;
    let latency = unsafe { com.audio().getLatencySamples() };
    let tail = Arc::new(AtomicU32::new(unsafe { com.audio().getTailSamples() }));

    let gate = Arc::new(Gate::new());
    let processor = VstProcessor::new(com.audio().as_ptr(), gate.clone(), from_main, to_main, &layout, tail.clone());
    let processor = if mode == kOffline as i32 { processor.offline() } else { processor };
    Ok(Plugin {
      info,
      sample_rate,
      latency,
      tail,
      editor: None,
      has_editor: Cell::new(None),
      frame: ComWrapper::new(PlugFrame { resized: Mutex::new(None) }),
      shared,
      gate,
      from_processor,
      processor: Some(processor),
      com,
      handler,
      host,
    })
  }

  pub fn info(&self) -> &PluginInfo {
    &self.info
  }

  // the audio half. there's only one, so this gives it once
  pub fn take_processor(&mut self) -> Option<Box<dyn Processor>> {
    self.processor.take().map(|p| Box::new(p) as Box<dyn Processor>)
  }

  pub fn sample_rate(&self) -> u32 {
    self.sample_rate
  }

  // prepare the plugin for another sample rate. a block in progress is
  // waited out, and blocks are bypassed while this runs. if it fails, the
  // plugin stays bypassed
  pub fn set_sample_rate(&mut self, sample_rate: u32) -> Result<(), String> {
    self.gate.close();
    self.com.stop();
    self.com.start(sample_rate)?;
    self.sample_rate = sample_rate;
    self.latency = unsafe { self.com.audio().getLatencySamples() };
    self.tail.store(unsafe { self.com.audio().getTailSamples() }, Ordering::Relaxed);
    self.gate.open();
    Ok(())
  }

  // frames the plugin delays its output by
  pub fn latency(&self) -> u32 {
    self.latency
  }

  // the component's and the controller's state, in one blob (see state.rs)
  pub fn state(&self) -> Result<Vec<u8>, String> {
    let component = MemoryStream::new();
    // safe (here and below): the objects are alive, and the streams outlive
    // the calls
    if unsafe { self.com.component.getState(ours::<_, IBStream>(&component)) } != kResultOk {
      return Err(format!("{} wouldn't save its state", self.info.name));
    }
    let controller = MemoryStream::new();
    let controller_bytes = match &self.com.controller {
      Some(c) if unsafe { c.getState(ours::<_, IBStream>(&controller)) } == kResultOk => controller.bytes(),
      _ => Vec::new(),
    };
    Ok(state::pack(&component.bytes(), &controller_bytes))
  }

  pub fn set_state(&mut self, blob: &[u8]) -> Result<(), String> {
    let (component, controller) = state::unpack(blob)?;
    let stream = MemoryStream::from_bytes(component.to_vec());
    if unsafe { self.com.component.setState(ours::<_, IBStream>(&stream)) } != kResultOk {
      return Err(format!("{} wouldn't take the saved state", self.info.name));
    }
    if let Some(c) = &self.com.controller {
      let stream = MemoryStream::from_bytes(component.to_vec());
      unsafe { c.setComponentState(ours::<_, IBStream>(&stream)) };
      if !controller.is_empty() {
        let stream = MemoryStream::from_bytes(controller.to_vec());
        unsafe { c.setState(ours::<_, IBStream>(&stream)) };
      }
    }
    Ok(())
  }

  pub fn params(&self) -> Vec<ParamInfo> {
    let Some(c) = &self.com.controller else { return Vec::new() };
    let mut params = Vec::new();
    // safe: the controller is alive, and info is plain data it fills in
    for i in 0..unsafe { c.getParameterCount() }.max(0) {
      let mut info: ParameterInfo = unsafe { std::mem::zeroed() };
      if unsafe { c.getParameterInfo(i, &mut info) } != kResultOk {
        continue;
      }
      let flag = |f: u32| info.flags as u32 & f != 0;
      params.push(ParamInfo {
        id: info.id,
        name: text16(&info.title),
        units: text16(&info.units),
        default: info.defaultNormalizedValue,
        step_count: info.stepCount,
        automatable: flag(kCanAutomate as u32),
        read_only: flag(kIsReadOnly as u32),
        bypass: flag(kIsBypass as u32),
        hidden: flag(kIsHidden as u32),
      });
    }
    params
  }

  // a parameter's value, normalized
  pub fn param(&self, id: u32) -> f64 {
    match &self.com.controller {
      // safe: the controller is alive
      Some(c) => unsafe { c.getParamNormalized(id) },
      None => 0.,
    }
  }

  // change a parameter: the controller hears now, the processor at the
  // start of its next block
  pub fn set_param(&mut self, id: u32, value: f64) {
    let value = value.clamp(0., 1.);
    if let Some(c) = &self.com.controller {
      // safe: the controller is alive
      unsafe { c.setParamNormalized(id, value) };
    }
    self.shared.send(id, value);
  }

  // how the plugin shows a parameter at a value ("-6.0 dB")
  pub fn param_text(&self, id: u32, value: f64) -> String {
    let mut text: String128 = [0; 128];
    match &self.com.controller {
      // safe: the controller is alive, and text is the size it expects
      Some(c) if unsafe { c.getParamStringByValue(id, value, &mut text) } == kResultOk => text16(&text),
      _ => format!("{:.2}", value),
    }
  }

  // call regularly on the main thread: gives the controller the parameter
  // changes the processor made, and handles the plugin asking to be
  // restarted (its latency changed, ...)
  pub fn idle(&mut self) {
    while let Ok((id, value)) = self.from_processor.pop() {
      if let Some(c) = &self.com.controller {
        // safe: the controller is alive
        unsafe { c.setParamNormalized(id, value) };
      }
    }
    let flags = self.shared.restart.swap(0, Ordering::SeqCst);
    // a new latency is read after the plugin's been restarted. a reload is
    // treated the same way (its buses aren't looked at again)
    if flags & (kLatencyChanged | kReloadComponent) as i32 != 0 {
      let _ = self.set_sample_rate(self.sample_rate);
    }
  }

  pub fn has_editor(&self) -> bool {
    if self.editor.is_some() {
      return true;
    }
    if let Some(has) = self.has_editor.get() {
      return has;
    }
    let has = match &self.com.controller {
      // safe: the controller is alive; the view it makes is released at once
      Some(c) => unsafe { ComPtr::from_raw(c.createView(EDITOR)) }.is_some(),
      None => false,
    };
    self.has_editor.set(Some(has));
    has
  }

  // open the plugin's own window contents inside `parent` (an NSView* on
  // macOS, an HWND on Windows, an X11 window id on Linux). gives the view's
  // size
  pub fn open_editor(&mut self, parent: *mut c_void) -> Result<(u32, u32), String> {
    if parent.is_null() {
      return Err("no window to open the plugin in".into());
    }
    self.close_editor();
    let Some(c) = &self.com.controller else {
      return Err(format!("{} has no window", self.info.name));
    };
    // safe (here and below): the controller is alive, the view is used as
    // the SDK describes, and parent is the caller's live view
    let view = unsafe { ComPtr::from_raw(c.createView(EDITOR)) }
      .ok_or_else(|| format!("{} has no window", self.info.name))?;
    if unsafe { view.isPlatformTypeSupported(platform_type()) } != kResultTrue {
      return Err(format!("{} can't open its window here", self.info.name));
    }
    unsafe { view.setFrame(ours::<_, IPlugFrame>(&self.frame)) };
    if unsafe { view.attached(parent, platform_type()) } != kResultOk {
      unsafe { view.setFrame(std::ptr::null_mut()) };
      return Err(format!("{} couldn't open its window", self.info.name));
    }
    let mut rect = ViewRect { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe { view.getSize(&mut rect) };
    self.editor = Some(view);
    Ok(size_of(&rect))
  }

  pub fn close_editor(&mut self) {
    if let Some(view) = self.editor.take() {
      // safe: the view is attached and alive
      unsafe {
        view.removed();
        view.setFrame(std::ptr::null_mut());
      }
    }
  }

  // whether its interface is open (in a window or a tab)
  pub fn editor_open(&self) -> bool {
    self.editor.is_some()
  }

  pub fn editor_size(&self) -> Option<(u32, u32)> {
    let view = self.editor.as_ref()?;
    let mut rect = ViewRect { left: 0, top: 0, right: 0, bottom: 0 };
    // safe: the view is alive
    (unsafe { view.getSize(&mut rect) } == kResultOk).then(|| size_of(&rect))
  }

  pub fn editor_can_resize(&self) -> bool {
    // safe: the view is alive
    self.editor.as_ref().map_or(false, |view| unsafe { view.canResize() } == kResultTrue)
  }

  // ask the open view to take a size. it may adjust it; gives what it took
  // (its current size if it can't resize, (0, 0) with no window open)
  pub fn resize_editor(&mut self, width: u32, height: u32) -> (u32, u32) {
    if !self.editor_can_resize() {
      return self.editor_size().unwrap_or((0, 0));
    }
    let Some(view) = &self.editor else { return (0, 0) };
    let mut rect = ViewRect { left: 0, top: 0, right: width as i32, bottom: height as i32 };
    // safe: the view is alive
    unsafe {
      view.checkSizeConstraint(&mut rect);
      view.onSize(&mut rect);
    }
    size_of(&rect)
  }

  // called with the size the plugin asked its window to be (it's been told
  // it has it). make the window that size
  pub fn on_editor_resize(&mut self, f: Box<dyn Fn(u32, u32) + Send>) {
    *self.frame.resized.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
  }
}

impl Drop for Plugin {
  fn drop(&mut self) {
    self.close_editor();
    // the processor (wherever it is) stops calling the plugin, and the
    // plugin's objects are torn down as `com` drops
    self.gate.close();
  }
}
