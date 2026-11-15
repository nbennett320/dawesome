// every built-in effect. to add one, write it in a file here and put its
// Entry in REGISTRY: the app lists it from there, and the conformance suite
// (testing.rs) checks it

use crate::effect::Entry;

pub mod eq;

pub static REGISTRY: &[&Entry] = &[&eq::ENTRY];

// a built-in effect by its id ("eq")
pub fn find(id: &str) -> Option<&'static Entry> {
  REGISTRY.iter().copied().find(|e| e.info.id == id)
}
