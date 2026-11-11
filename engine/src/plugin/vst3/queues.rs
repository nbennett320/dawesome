// what goes in and out of a plugin with each block besides audio: parameter
// changes (IParameterChanges, a queue of points per parameter) and note
// events (IEventList)
//
// these are made once, when a processor is, with room for everything a block
// can hold, and cleared before each block: the audio thread never allocates.
// a change or event that doesn't fit is dropped
//
// they're only ever touched by the thread running the processor: by us
// before and after a block, and by the plugin during its process call. that's
// what makes the unsynchronized cells below sound
use ::vst3::Steinberg::Vst::{
  Event as VstEvent, IEventList, IEventListTrait, IParamValueQueue, IParamValueQueueTrait,
  IParameterChanges, IParameterChangesTrait, ParamID, ParamValue,
};
use ::vst3::Steinberg::{int32, kInvalidArgument, kResultFalse, kResultOk, tresult};
use ::vst3::{Class, ComWrapper};
use std::cell::UnsafeCell;

// points a parameter can have in one block
const POINTS: usize = 64;
// parameters that can change in one block
pub const QUEUES: usize = 512;
// note events in one block
pub const EVENTS: usize = 1024;

pub struct ParamQueue {
  data: UnsafeCell<QueueData>,
}

struct QueueData {
  id: ParamID,
  len: usize,
  // in order of their offset
  points: [(int32, ParamValue); POINTS],
}

// safe: see the top of the file. only one thread at a time touches a queue
unsafe impl Send for ParamQueue {}
unsafe impl Sync for ParamQueue {}

impl Class for ParamQueue {
  type Interfaces = (IParamValueQueue,);
}

impl ParamQueue {
  #[allow(clippy::mut_from_ref)]
  fn data(&self) -> &mut QueueData {
    // safe: see the top of the file, and no reference to the data is kept
    // past a single call
    unsafe { &mut *self.data.get() }
  }

  // the value it ends the block on
  pub fn last(&self) -> Option<ParamValue> {
    let data = self.data();
    data.points[..data.len].last().map(|p| p.1)
  }

  fn add(&self, offset: int32, value: ParamValue) -> Option<usize> {
    let data = self.data();
    let at = data.points[..data.len].partition_point(|p| p.0 < offset);
    if at < data.len && data.points[at].0 == offset {
      data.points[at].1 = value;
      return Some(at);
    }
    if data.len == POINTS {
      return None;
    }
    data.points.copy_within(at..data.len, at + 1);
    data.points[at] = (offset, value);
    data.len += 1;
    Some(at)
  }
}

impl IParamValueQueueTrait for ParamQueue {
  unsafe fn getParameterId(&self) -> ParamID {
    self.data().id
  }

  unsafe fn getPointCount(&self) -> int32 {
    self.data().len as int32
  }

  unsafe fn getPoint(&self, index: int32, offset: *mut int32, value: *mut ParamValue) -> tresult {
    let data = self.data();
    if index < 0 || index as usize >= data.len {
      return kInvalidArgument;
    }
    let (o, v) = data.points[index as usize];
    if !offset.is_null() {
      *offset = o;
    }
    if !value.is_null() {
      *value = v;
    }
    kResultOk
  }

  unsafe fn addPoint(&self, offset: int32, value: ParamValue, index: *mut int32) -> tresult {
    match self.add(offset, value) {
      Some(at) => {
        if !index.is_null() {
          *index = at as int32;
        }
        kResultOk
      }
      None => kResultFalse,
    }
  }
}

pub struct ParamChanges {
  data: UnsafeCell<ChangesData>,
}

struct ChangesData {
  queues: Vec<ComWrapper<ParamQueue>>,
  // how many queues are in use this block
  used: usize,
}

// safe: see the top of the file
unsafe impl Send for ParamChanges {}
unsafe impl Sync for ParamChanges {}

impl Class for ParamChanges {
  type Interfaces = (IParameterChanges,);
}

impl ParamChanges {
  pub fn new() -> ComWrapper<ParamChanges> {
    let queues = (0..QUEUES)
      .map(|_| {
        ComWrapper::new(ParamQueue {
          data: UnsafeCell::new(QueueData { id: 0, len: 0, points: [(0, 0.); POINTS] }),
        })
      })
      .collect();
    ComWrapper::new(ParamChanges { data: UnsafeCell::new(ChangesData { queues, used: 0 }) })
  }

  #[allow(clippy::mut_from_ref)]
  fn data(&self) -> &mut ChangesData {
    // safe: see the top of the file, and no reference to the data is kept
    // past a single call
    unsafe { &mut *self.data.get() }
  }

  pub fn clear(&self) {
    let data = self.data();
    for queue in &data.queues[..data.used] {
      queue.data().len = 0;
    }
    data.used = 0;
  }

  // the queue for a parameter, started if it has none yet
  fn queue(&self, id: ParamID) -> Option<usize> {
    let data = self.data();
    if let Some(i) = data.queues[..data.used].iter().position(|q| q.data().id == id) {
      return Some(i);
    }
    if data.used == data.queues.len() {
      return None;
    }
    let i = data.used;
    let queue = data.queues[i].data();
    queue.id = id;
    queue.len = 0;
    data.used += 1;
    Some(i)
  }

  // a change at the start of the block (a later one for the same parameter
  // replaces it)
  pub fn set(&self, id: ParamID, value: ParamValue) -> bool {
    match self.queue(id) {
      Some(i) => self.data().queues[i].add(0, value).is_some(),
      None => false,
    }
  }

  // each changed parameter and the value it ends the block on
  pub fn last_values(&self, mut f: impl FnMut(ParamID, ParamValue)) {
    let data = self.data();
    for queue in &data.queues[..data.used] {
      if let Some(value) = queue.last() {
        f(queue.data().id, value);
      }
    }
  }

  fn queue_ptr(&self, i: usize) -> *mut IParamValueQueue {
    self.data().queues[i].as_com_ref::<IParamValueQueue>().map_or(std::ptr::null_mut(), |q| q.as_ptr())
  }
}

impl IParameterChangesTrait for ParamChanges {
  unsafe fn getParameterCount(&self) -> int32 {
    self.data().used as int32
  }

  unsafe fn getParameterData(&self, index: int32) -> *mut IParamValueQueue {
    if index < 0 || index as usize >= self.data().used {
      return std::ptr::null_mut();
    }
    self.queue_ptr(index as usize)
  }

  unsafe fn addParameterData(&self, id: *const ParamID, index: *mut int32) -> *mut IParamValueQueue {
    if id.is_null() {
      return std::ptr::null_mut();
    }
    match self.queue(*id) {
      Some(i) => {
        if !index.is_null() {
          *index = i as int32;
        }
        self.queue_ptr(i)
      }
      None => std::ptr::null_mut(),
    }
  }
}

pub struct EventList {
  // made with room for EVENTS, and never pushed past it
  events: UnsafeCell<Vec<VstEvent>>,
}

// safe: see the top of the file
unsafe impl Send for EventList {}
unsafe impl Sync for EventList {}

impl Class for EventList {
  type Interfaces = (IEventList,);
}

impl EventList {
  pub fn new() -> ComWrapper<EventList> {
    ComWrapper::new(EventList { events: UnsafeCell::new(Vec::with_capacity(EVENTS)) })
  }

  #[allow(clippy::mut_from_ref)]
  fn events(&self) -> &mut Vec<VstEvent> {
    // safe: see the top of the file, and no reference to the list is kept
    // past a single call
    unsafe { &mut *self.events.get() }
  }

  pub fn clear(&self) {
    self.events().clear();
  }

  // false when the block's full
  pub fn push(&self, event: VstEvent) -> bool {
    let events = self.events();
    if events.len() == EVENTS {
      return false;
    }
    // within the capacity, so this doesn't allocate
    events.push(event);
    true
  }

  pub fn len(&self) -> usize {
    self.events().len()
  }

  pub fn get(&self, i: usize) -> Option<VstEvent> {
    self.events().get(i).copied()
  }
}

impl IEventListTrait for EventList {
  unsafe fn getEventCount(&self) -> int32 {
    self.len() as int32
  }

  unsafe fn getEvent(&self, index: int32, event: *mut VstEvent) -> tresult {
    if index < 0 || event.is_null() {
      return kInvalidArgument;
    }
    match self.get(index as usize) {
      Some(e) => {
        *event = e;
        kResultOk
      }
      None => kInvalidArgument,
    }
  }

  unsafe fn addEvent(&self, event: *mut VstEvent) -> tresult {
    if event.is_null() {
      return kInvalidArgument;
    }
    if self.push(*event) {
      kResultOk
    } else {
      kResultFalse
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ::vst3::ComPtr;

  #[test]
  fn test_changes_keep_one_queue_per_parameter() {
    let changes = ParamChanges::new();
    assert!(changes.set(7, 0.25));
    assert!(changes.set(9, 0.5));
    // the later value at offset 0 replaces the first
    assert!(changes.set(7, 0.75));
    let com: ComPtr<IParameterChanges> = changes.to_com_ptr().unwrap();
    unsafe {
      assert_eq!(com.getParameterCount(), 2);
      let queue = ComPtr::<IParamValueQueue>::from_raw(com.getParameterData(0)).unwrap();
      // from_raw took a reference the list didn't give, so give it back
      let queue = std::mem::ManuallyDrop::new(queue);
      assert_eq!(queue.getParameterId(), 7);
      assert_eq!(queue.getPointCount(), 1);
      let (mut offset, mut value) = (-1, -1.);
      assert_eq!(queue.getPoint(0, &mut offset, &mut value), kResultOk);
      assert_eq!((offset, value), (0, 0.75));
      assert!(com.getParameterData(2).is_null());
    }
    let mut seen = vec![];
    changes.last_values(|id, v| seen.push((id, v)));
    assert_eq!(seen, vec![(7, 0.75), (9, 0.5)]);
    changes.clear();
    let mut seen = vec![];
    changes.last_values(|id, v| seen.push((id, v)));
    assert!(seen.is_empty());
  }

  #[test]
  fn test_plugin_written_points_stay_in_order() {
    let changes = ParamChanges::new();
    let com: ComPtr<IParameterChanges> = changes.to_com_ptr().unwrap();
    unsafe {
      let mut index = -1;
      let id: ParamID = 3;
      let queue = com.addParameterData(&id, &mut index);
      assert_eq!(index, 0);
      // the same parameter again gets the same queue
      assert_eq!(com.addParameterData(&id, &mut index), queue);
      let queue = std::mem::ManuallyDrop::new(ComPtr::<IParamValueQueue>::from_raw(queue).unwrap());
      queue.addPoint(100, 0.9, &mut index);
      queue.addPoint(10, 0.1, &mut index);
      assert_eq!(index, 0);
      queue.addPoint(50, 0.5, &mut index);
      assert_eq!(index, 1);
      let mut offsets = vec![];
      for i in 0..queue.getPointCount() {
        let (mut o, mut v) = (0, 0.);
        queue.getPoint(i, &mut o, &mut v);
        offsets.push(o);
      }
      assert_eq!(offsets, vec![10, 50, 100]);
    }
    let mut last = vec![];
    changes.last_values(|id, v| last.push((id, v)));
    assert_eq!(last, vec![(3, 0.9)]);
  }

  #[test]
  fn test_full_changes_and_queues_drop_the_extra() {
    let changes = ParamChanges::new();
    for id in 0..QUEUES as u32 {
      assert!(changes.set(id, 0.));
    }
    assert!(!changes.set(QUEUES as u32, 0.));
    // an existing parameter can still change
    assert!(changes.set(0, 1.));

    let queue = ComWrapper::new(ParamQueue {
      data: UnsafeCell::new(QueueData { id: 0, len: 0, points: [(0, 0.); POINTS] }),
    });
    for i in 0..POINTS as i32 {
      assert!(queue.add(i, 0.).is_some());
    }
    assert!(queue.add(POINTS as i32, 0.).is_none());
  }

  #[test]
  fn test_event_list_holds_a_block_of_events() {
    let list = EventList::new();
    let com: ComPtr<IEventList> = list.to_com_ptr().unwrap();
    // safe: Event is plain data
    let mut event: VstEvent = unsafe { std::mem::zeroed() };
    event.sampleOffset = 12;
    unsafe {
      assert_eq!(com.addEvent(&mut event), kResultOk);
      assert_eq!(com.getEventCount(), 1);
      let mut out: VstEvent = std::mem::zeroed();
      assert_eq!(com.getEvent(0, &mut out), kResultOk);
      assert_eq!(out.sampleOffset, 12);
      assert_eq!(com.getEvent(1, &mut out), kInvalidArgument);
    }
    for _ in 1..EVENTS {
      assert!(list.push(event));
    }
    assert!(!list.push(event));
    list.clear();
    assert_eq!(list.len(), 0);
  }
}
