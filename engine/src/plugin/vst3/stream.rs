// a stream in memory (IBStream): what a plugin writes its state into and
// reads it back from
use ::vst3::Steinberg::IBStream_::IStreamSeekMode_::{kIBSeekCur, kIBSeekEnd, kIBSeekSet};
use ::vst3::Steinberg::{int32, int64, kInvalidArgument, kResultOk, tresult, IBStream, IBStreamTrait};
use ::vst3::{Class, ComWrapper};
use std::ffi::c_void;
use std::sync::Mutex;

pub struct MemoryStream {
  // the bytes, and where the next read or write starts
  inner: Mutex<(Vec<u8>, usize)>,
}

impl Class for MemoryStream {
  type Interfaces = (IBStream,);
}

impl MemoryStream {
  pub fn new() -> ComWrapper<MemoryStream> {
    MemoryStream::from_bytes(Vec::new())
  }

  // a stream to be read from the start
  pub fn from_bytes(bytes: Vec<u8>) -> ComWrapper<MemoryStream> {
    ComWrapper::new(MemoryStream { inner: Mutex::new((bytes, 0)) })
  }

  pub fn bytes(&self) -> Vec<u8> {
    self.lock().0.clone()
  }

  pub fn rewind(&self) {
    self.lock().1 = 0;
  }

  fn lock(&self) -> std::sync::MutexGuard<'_, (Vec<u8>, usize)> {
    self.inner.lock().unwrap_or_else(|e| e.into_inner())
  }
}

impl IBStreamTrait for MemoryStream {
  unsafe fn read(&self, buffer: *mut c_void, bytes: int32, read: *mut int32) -> tresult {
    if bytes < 0 || (buffer.is_null() && bytes > 0) {
      return kInvalidArgument;
    }
    let mut inner = self.lock();
    let (data, pos) = &mut *inner;
    let start = (*pos).min(data.len());
    let n = (bytes as usize).min(data.len() - start);
    if n > 0 {
      // safe: the plugin says buffer holds `bytes`, and n is no more
      std::ptr::copy_nonoverlapping(data.as_ptr().add(start), buffer as *mut u8, n);
    }
    *pos = start + n;
    if !read.is_null() {
      *read = n as int32;
    }
    kResultOk
  }

  unsafe fn write(&self, buffer: *mut c_void, bytes: int32, written: *mut int32) -> tresult {
    if bytes < 0 || (buffer.is_null() && bytes > 0) {
      return kInvalidArgument;
    }
    let mut inner = self.lock();
    let (data, pos) = &mut *inner;
    let n = bytes as usize;
    let end = *pos + n;
    if data.len() < end {
      data.resize(end, 0);
    }
    if n > 0 {
      // safe: the plugin says buffer holds `bytes`
      std::ptr::copy_nonoverlapping(buffer as *const u8, data.as_mut_ptr().add(*pos), n);
    }
    *pos = end;
    if !written.is_null() {
      *written = n as int32;
    }
    kResultOk
  }

  unsafe fn seek(&self, offset: int64, mode: int32, result: *mut int64) -> tresult {
    let mut inner = self.lock();
    let (data, pos) = &mut *inner;
    let base = match mode as u32 {
      m if m == kIBSeekSet as u32 => 0,
      m if m == kIBSeekCur as u32 => *pos as int64,
      m if m == kIBSeekEnd as u32 => data.len() as int64,
      _ => return kInvalidArgument,
    };
    let to = base + offset;
    if to < 0 {
      return kInvalidArgument;
    }
    // seeking past the end is allowed: a write there fills the gap with 0s
    *pos = to as usize;
    if !result.is_null() {
      *result = to;
    }
    kResultOk
  }

  unsafe fn tell(&self, pos: *mut int64) -> tresult {
    if pos.is_null() {
      return kInvalidArgument;
    }
    *pos = self.lock().1 as int64;
    kResultOk
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // the stream as a plugin sees it, through its COM interface
  fn com(stream: &ComWrapper<MemoryStream>) -> ::vst3::ComPtr<IBStream> {
    stream.to_com_ptr::<IBStream>().unwrap()
  }

  #[test]
  fn test_stream_reads_back_what_was_written() {
    let stream = MemoryStream::new();
    let s = com(&stream);
    let mut written = 0;
    let mut data = *b"tape model";
    unsafe {
      assert_eq!(s.write(data.as_mut_ptr() as *mut c_void, 10, &mut written), kResultOk);
    }
    assert_eq!(written, 10);
    assert_eq!(stream.bytes(), b"tape model");

    let mut pos = -1;
    unsafe {
      s.seek(5, kIBSeekSet as int32, &mut pos);
    }
    assert_eq!(pos, 5);
    let mut out = [0u8; 16];
    let mut read = 0;
    unsafe {
      s.read(out.as_mut_ptr() as *mut c_void, 16, &mut read);
    }
    assert_eq!(read, 5);
    assert_eq!(&out[..5], b"model");
    unsafe {
      s.read(out.as_mut_ptr() as *mut c_void, 16, &mut read);
    }
    assert_eq!(read, 0);
  }

  #[test]
  fn test_stream_seeks_from_the_end_and_current_position() {
    let stream = MemoryStream::from_bytes(b"0123456789".to_vec());
    let s = com(&stream);
    let mut pos = 0;
    unsafe {
      s.seek(-3, kIBSeekEnd as int32, &mut pos);
      assert_eq!(pos, 7);
      s.seek(-2, kIBSeekCur as int32, &mut pos);
      assert_eq!(pos, 5);
      assert_eq!(s.seek(-6, kIBSeekCur as int32, &mut pos), kInvalidArgument);
      s.tell(&mut pos);
      assert_eq!(pos, 5);
    }
  }

  #[test]
  fn test_writing_past_the_end_fills_the_gap() {
    let stream = MemoryStream::from_bytes(b"ab".to_vec());
    let s = com(&stream);
    let mut byte = *b"z";
    unsafe {
      s.seek(4, kIBSeekSet as int32, std::ptr::null_mut());
      s.write(byte.as_mut_ptr() as *mut c_void, 1, std::ptr::null_mut());
    }
    assert_eq!(stream.bytes(), b"ab\0\0z");
  }

  #[test]
  fn test_overwriting_in_the_middle_keeps_the_rest() {
    let stream = MemoryStream::from_bytes(b"abcdef".to_vec());
    let s = com(&stream);
    let mut bytes = *b"XY";
    unsafe {
      s.seek(2, kIBSeekSet as int32, std::ptr::null_mut());
      s.write(bytes.as_mut_ptr() as *mut c_void, 2, std::ptr::null_mut());
    }
    assert_eq!(stream.bytes(), b"abXYef");
  }
}
