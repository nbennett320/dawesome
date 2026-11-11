// the objects a host hands a plugin: the application (its name, and messages
// for a plugin's two halves to talk through), the component handler (how a
// plugin's own window reports a knob being turned) and the frame its window
// sits in
use ::vst3::Steinberg::Vst::IAttributeList_::AttrID;
use ::vst3::Steinberg::Vst::{
  IAttributeList, IAttributeListTrait, IComponentHandler, IComponentHandlerTrait, IHostApplication,
  IHostApplicationTrait, IMessage, IMessageTrait, ParamID, ParamValue, String128, TChar,
};
use ::vst3::Steinberg::{
  int32, int64, kInvalidArgument, kResultFalse, kResultOk, tresult, uint32, FIDString, FUnknown,
  IPlugFrame, IPlugFrameTrait, IPlugView, IPlugViewTrait, ViewRect, TUID,
};
use ::vst3::{Class, ComRef, ComWrapper, Interface};
use rtrb::Producer;
use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

pub const HOST_NAME: &str = "Dawesome";

// a parameter's new value (normalized)
pub type ParamChange = (ParamID, ParamValue);

// what the main-thread half shares with the objects the plugin calls into
pub struct Shared {
  // parameter changes on their way to the processor. only the main thread
  // pushes, so the lock is never contended
  pub to_processor: Mutex<Producer<ParamChange>>,
  // restartComponent flags not yet handled (see Plugin::idle)
  pub restart: AtomicI32,
}

impl Shared {
  pub fn send(&self, id: ParamID, value: ParamValue) {
    let mut queue = self.to_processor.lock().unwrap_or_else(|e| e.into_inner());
    // a full queue means the processor isn't running; the controller still
    // has the value
    let _ = queue.push((id, value));
  }
}

fn guid(tuid: *const TUID) -> Option<[u8; 16]> {
  if tuid.is_null() {
    return None;
  }
  // safe: a TUID is 16 bytes, and the pointer was checked
  Some(unsafe { std::array::from_fn(|i| (*tuid)[i] as u8) })
}

fn c_text(text: *const std::ffi::c_char) -> Option<String> {
  if text.is_null() {
    return None;
  }
  // safe: the plugin hands us a NUL-terminated string
  Some(unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned())
}

// a UTF-16 string into a String128, cut to fit
pub fn write_string128(text: &str, out: &mut String128) {
  let mut len = 0;
  for (i, c) in text.encode_utf16().take(out.len() - 1).enumerate() {
    out[i] = c;
    len = i + 1;
  }
  out[len] = 0;
}

pub struct HostApplication;

impl Class for HostApplication {
  type Interfaces = (IHostApplication,);
}

impl IHostApplicationTrait for HostApplication {
  unsafe fn getName(&self, name: *mut String128) -> tresult {
    if name.is_null() {
      return kInvalidArgument;
    }
    write_string128(HOST_NAME, &mut *name);
    kResultOk
  }

  unsafe fn createInstance(&self, cid: *mut TUID, iid: *mut TUID, obj: *mut *mut c_void) -> tresult {
    if obj.is_null() {
      return kInvalidArgument;
    }
    *obj = std::ptr::null_mut();
    let (Some(cid), Some(iid)) = (guid(cid), guid(iid)) else {
      return kInvalidArgument;
    };
    // a plugin asks for the interface it's making, or FUnknown
    let wants = |i: [u8; 16]| iid == i || iid == FUnknown::IID;
    if cid == IMessage::IID && wants(IMessage::IID) {
      if let Some(message) = ComWrapper::new(Message::default()).to_com_ptr::<IMessage>() {
        *obj = message.into_raw() as *mut c_void;
        return kResultOk;
      }
    }
    if cid == IAttributeList::IID && wants(IAttributeList::IID) {
      if let Some(list) = ComWrapper::new(AttributeList::default()).to_com_ptr::<IAttributeList>() {
        *obj = list.into_raw() as *mut c_void;
        return kResultOk;
      }
    }
    kResultFalse
  }
}

// a message between a plugin's component and its controller
pub struct Message {
  id: Mutex<Option<CString>>,
  attributes: ComWrapper<AttributeList>,
}

impl Default for Message {
  fn default() -> Self {
    Message { id: Mutex::new(None), attributes: ComWrapper::new(AttributeList::default()) }
  }
}

impl Class for Message {
  type Interfaces = (IMessage,);
}

impl IMessageTrait for Message {
  unsafe fn getMessageID(&self) -> FIDString {
    // the string lives in the message until the id is set again
    match &*self.id.lock().unwrap_or_else(|e| e.into_inner()) {
      Some(id) => id.as_ptr(),
      None => std::ptr::null(),
    }
  }

  unsafe fn setMessageID(&self, id: FIDString) {
    let id = if id.is_null() { None } else { Some(CStr::from_ptr(id).to_owned()) };
    *self.id.lock().unwrap_or_else(|e| e.into_inner()) = id;
  }

  unsafe fn getAttributes(&self) -> *mut IAttributeList {
    // borrowed, as the SDK's own host does it (no reference is added)
    self.attributes.as_com_ref::<IAttributeList>().map_or(std::ptr::null_mut(), |a| a.as_ptr())
  }
}

enum Attribute {
  Int(i64),
  Float(f64),
  // UTF-16, without the NUL
  Text(Vec<u16>),
  Binary(Vec<u8>),
}

#[derive(Default)]
pub struct AttributeList {
  values: Mutex<HashMap<String, Attribute>>,
}

impl Class for AttributeList {
  type Interfaces = (IAttributeList,);
}

impl AttributeList {
  fn set(&self, id: AttrID, value: Attribute) -> tresult {
    let Some(id) = c_text(id) else { return kInvalidArgument };
    self.values.lock().unwrap_or_else(|e| e.into_inner()).insert(id, value);
    kResultOk
  }

  fn with<R>(&self, id: AttrID, f: impl FnOnce(&Attribute) -> Option<R>) -> Option<R> {
    let id = c_text(id)?;
    let values = self.values.lock().unwrap_or_else(|e| e.into_inner());
    values.get(&id).and_then(f)
  }
}

impl IAttributeListTrait for AttributeList {
  unsafe fn setInt(&self, id: AttrID, value: int64) -> tresult {
    self.set(id, Attribute::Int(value))
  }

  unsafe fn getInt(&self, id: AttrID, value: *mut int64) -> tresult {
    match self.with(id, |a| if let Attribute::Int(v) = a { Some(*v) } else { None }) {
      Some(v) if !value.is_null() => {
        *value = v;
        kResultOk
      }
      _ => kResultFalse,
    }
  }

  unsafe fn setFloat(&self, id: AttrID, value: f64) -> tresult {
    self.set(id, Attribute::Float(value))
  }

  unsafe fn getFloat(&self, id: AttrID, value: *mut f64) -> tresult {
    match self.with(id, |a| if let Attribute::Float(v) = a { Some(*v) } else { None }) {
      Some(v) if !value.is_null() => {
        *value = v;
        kResultOk
      }
      _ => kResultFalse,
    }
  }

  unsafe fn setString(&self, id: AttrID, string: *const TChar) -> tresult {
    if string.is_null() {
      return kInvalidArgument;
    }
    let mut text = Vec::new();
    let mut i = 0;
    // safe: the plugin hands us a NUL-terminated UTF-16 string
    while *string.add(i) != 0 {
      text.push(*string.add(i));
      i += 1;
    }
    self.set(id, Attribute::Text(text))
  }

  unsafe fn getString(&self, id: AttrID, string: *mut TChar, size_in_bytes: uint32) -> tresult {
    let room = size_in_bytes as usize / 2;
    if string.is_null() || room == 0 {
      return kInvalidArgument;
    }
    let found = self.with(id, |a| {
      if let Attribute::Text(text) = a {
        let n = text.len().min(room - 1);
        // safe: the plugin says the buffer holds size_in_bytes
        std::ptr::copy_nonoverlapping(text.as_ptr(), string, n);
        *string.add(n) = 0;
        Some(())
      } else {
        None
      }
    });
    if found.is_some() {
      kResultOk
    } else {
      kResultFalse
    }
  }

  unsafe fn setBinary(&self, id: AttrID, data: *const c_void, size_in_bytes: uint32) -> tresult {
    if data.is_null() && size_in_bytes > 0 {
      return kInvalidArgument;
    }
    let bytes = if size_in_bytes == 0 {
      Vec::new()
    } else {
      // safe: the plugin says data holds size_in_bytes
      std::slice::from_raw_parts(data as *const u8, size_in_bytes as usize).to_vec()
    };
    self.set(id, Attribute::Binary(bytes))
  }

  unsafe fn getBinary(&self, id: AttrID, data: *mut *const c_void, size_in_bytes: *mut uint32) -> tresult {
    if data.is_null() || size_in_bytes.is_null() {
      return kInvalidArgument;
    }
    // the bytes stay where they are until the attribute is set again, which
    // is what the SDK promises the plugin
    match self.with(id, |a| if let Attribute::Binary(b) = a { Some((b.as_ptr(), b.len())) } else { None }) {
      Some((ptr, len)) => {
        *data = ptr as *const c_void;
        *size_in_bytes = len as uint32;
        kResultOk
      }
      None => kResultFalse,
    }
  }
}

// how a plugin's controller (usually its own window) tells the host about a
// parameter it changed, and asks for the component to be restarted
pub struct ComponentHandler {
  pub shared: std::sync::Arc<Shared>,
}

impl Class for ComponentHandler {
  type Interfaces = (IComponentHandler,);
}

impl IComponentHandlerTrait for ComponentHandler {
  unsafe fn beginEdit(&self, _id: ParamID) -> tresult {
    kResultOk
  }

  unsafe fn performEdit(&self, id: ParamID, value: ParamValue) -> tresult {
    self.shared.send(id, value);
    kResultOk
  }

  unsafe fn endEdit(&self, _id: ParamID) -> tresult {
    kResultOk
  }

  unsafe fn restartComponent(&self, flags: int32) -> tresult {
    self.shared.restart.fetch_or(flags, Ordering::SeqCst);
    kResultOk
  }
}

pub type ResizeCallback = Box<dyn Fn(u32, u32) + Send>;

// what a plugin's window sits in. the plugin asks it for a new size
pub struct PlugFrame {
  pub resized: Mutex<Option<ResizeCallback>>,
}

impl Class for PlugFrame {
  type Interfaces = (IPlugFrame,);
}

impl IPlugFrameTrait for PlugFrame {
  unsafe fn resizeView(&self, view: *mut IPlugView, size: *mut ViewRect) -> tresult {
    if view.is_null() || size.is_null() {
      return kInvalidArgument;
    }
    let rect = *size;
    let (width, height) = ((rect.right - rect.left).max(0) as u32, (rect.bottom - rect.top).max(0) as u32);
    // whoever holds the window makes room first, then the view is told
    if let Some(resized) = &*self.resized.lock().unwrap_or_else(|e| e.into_inner()) {
      resized(width, height);
    }
    // safe: the plugin passes its own live view
    if let Some(view) = ComRef::from_raw(view) {
      view.onSize(size);
    }
    kResultOk
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ::vst3::ComPtr;

  #[test]
  fn test_host_says_its_name() {
    let host: ComPtr<IHostApplication> = ComWrapper::new(HostApplication).to_com_ptr().unwrap();
    let mut name: String128 = [0x41; 128];
    unsafe {
      assert_eq!(host.getName(&mut name), kResultOk);
    }
    let end = name.iter().position(|&c| c == 0).unwrap();
    assert_eq!(String::from_utf16_lossy(&name[..end]), HOST_NAME);
  }

  #[test]
  fn test_host_makes_messages_with_attributes() {
    let host: ComPtr<IHostApplication> = ComWrapper::new(HostApplication).to_com_ptr().unwrap();
    let mut cid: TUID = std::array::from_fn(|i| IMessage::IID[i] as _);
    let mut iid = cid;
    let mut obj = std::ptr::null_mut();
    unsafe {
      assert_eq!(host.createInstance(&mut cid, &mut iid, &mut obj), kResultOk);
      let message = ComPtr::<IMessage>::from_raw(obj as *mut IMessage).unwrap();
      message.setMessageID(b"hello\0".as_ptr() as FIDString);
      assert_eq!(CStr::from_ptr(message.getMessageID()).to_str(), Ok("hello"));

      let attrs = ComRef::<IAttributeList>::from_raw(message.getAttributes()).unwrap();
      let key = b"answer\0".as_ptr() as AttrID;
      attrs.setInt(key, 42);
      let mut v = 0;
      assert_eq!(attrs.getInt(key, &mut v), kResultOk);
      assert_eq!(v, 42);
      let mut f = 0.;
      assert_eq!(attrs.getFloat(key, &mut f), kResultFalse);

      let text: Vec<u16> = "hi there\0".encode_utf16().collect();
      attrs.setString(key, text.as_ptr());
      let mut out = [0u16; 4];
      assert_eq!(attrs.getString(key, out.as_mut_ptr(), 8), kResultOk);
      assert_eq!(&out, &[104, 105, 32, 0]);

      let bytes = [1u8, 2, 3];
      attrs.setBinary(key, bytes.as_ptr() as *const c_void, 3);
      let (mut ptr, mut len) = (std::ptr::null(), 0);
      assert_eq!(attrs.getBinary(key, &mut ptr, &mut len), kResultOk);
      assert_eq!(std::slice::from_raw_parts(ptr as *const u8, len as usize), &bytes);
    }
  }

  #[test]
  fn test_host_doesnt_make_unknown_classes() {
    let host: ComPtr<IHostApplication> = ComWrapper::new(HostApplication).to_com_ptr().unwrap();
    let mut cid: TUID = [7; 16];
    let mut iid = cid;
    let mut obj = 1 as *mut c_void;
    unsafe {
      assert_eq!(host.createInstance(&mut cid, &mut iid, &mut obj), kResultFalse);
    }
    assert!(obj.is_null());
  }

  #[test]
  fn test_string128_is_cut_and_terminated() {
    let mut out: String128 = [1; 128];
    write_string128(&"x".repeat(300), &mut out);
    assert_eq!(out[126], 'x' as u16);
    assert_eq!(out[127], 0);
  }
}
