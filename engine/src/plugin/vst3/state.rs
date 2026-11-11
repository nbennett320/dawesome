// a plugin's saved state as one blob: the component's state and the
// controller's, each as the plugin wrote it
//
//   "DWV3"            4 bytes
//   version           u32, little-endian (1)
//   component length  u64, little-endian, then its bytes
//   controller length u64, little-endian, then its bytes

const MAGIC: &[u8; 4] = b"DWV3";
const VERSION: u32 = 1;

pub fn pack(component: &[u8], controller: &[u8]) -> Vec<u8> {
  let mut blob = Vec::with_capacity(24 + component.len() + controller.len());
  blob.extend_from_slice(MAGIC);
  blob.extend_from_slice(&VERSION.to_le_bytes());
  for part in [component, controller] {
    blob.extend_from_slice(&(part.len() as u64).to_le_bytes());
    blob.extend_from_slice(part);
  }
  blob
}

// the component's and the controller's parts of a blob
pub fn unpack(blob: &[u8]) -> Result<(&[u8], &[u8]), String> {
  if blob.len() < 8 || &blob[..4] != MAGIC {
    return Err("not a plugin state".into());
  }
  let version = u32::from_le_bytes(blob[4..8].try_into().unwrap());
  if version != VERSION {
    return Err(format!("plugin state version {} isn't supported", version));
  }
  let mut rest = &blob[8..];
  let mut part = || -> Result<&[u8], String> {
    if rest.len() < 8 {
      return Err("plugin state is cut short".into());
    }
    let len = u64::from_le_bytes(rest[..8].try_into().unwrap());
    let len = usize::try_from(len).map_err(|_| "plugin state is too long".to_string())?;
    if rest.len() - 8 < len {
      return Err("plugin state is cut short".into());
    }
    let (bytes, after) = rest[8..].split_at(len);
    rest = after;
    Ok(bytes)
  };
  let component = part()?;
  let controller = part()?;
  Ok((component, controller))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_state_blob_round_trips() {
    let blob = pack(b"component bytes", b"ctl");
    assert_eq!(unpack(&blob).unwrap(), (&b"component bytes"[..], &b"ctl"[..]));
    let empty = pack(b"", b"");
    assert_eq!(unpack(&empty).unwrap(), (&b""[..], &b""[..]));
  }

  #[test]
  fn test_state_blob_has_magic_and_version() {
    let blob = pack(b"a", b"b");
    assert_eq!(&blob[..4], b"DWV3");
    assert_eq!(&blob[4..8], &1u32.to_le_bytes());
    assert_eq!(blob.len(), 4 + 4 + 8 + 1 + 8 + 1);
  }

  #[test]
  fn test_bad_state_blobs_are_refused() {
    assert!(unpack(b"").is_err());
    assert!(unpack(b"JUNKJUNKJUNK").is_err());
    let mut newer = pack(b"a", b"b");
    newer[4] = 2;
    assert!(unpack(&newer).unwrap_err().contains("version"));
    let whole = pack(b"component", b"controller");
    for cut in 8..whole.len() {
      assert!(unpack(&whole[..cut]).is_err(), "cut at {}", cut);
    }
  }
}
