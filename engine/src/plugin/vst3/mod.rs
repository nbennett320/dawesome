// the VST3 host
//
// module opens a plugin's bundle, classes reads what's in it, and plugin and
// processor are the two halves of a loaded plugin. the rest are the objects
// a host hands a plugin: host (the application, messages, the component
// handler, the window's frame), stream (saved state) and queues (parameter
// changes and note events for a block)
//
// `vst3` alone means the bindings crate, so it's written `::vst3` in here

mod classes;
mod host;
mod module;
mod plugin;
mod processor;
mod queues;
mod state;
mod stream;

#[allow(unused_imports)]
pub use classes::{category_of, class_id_hex, kind_of, parse_plugin_id, plugin_id};
pub use module::{bundle_binary, Module};
pub use plugin::Plugin;

use super::PluginInfo;
use std::path::Path;

// the plugins in a bundle, without making any of them
pub fn scan_bundle(path: &Path) -> Result<Vec<PluginInfo>, String> {
  let module = Module::open(path)?;
  Ok(classes::classes(&module).into_iter().map(|(_, info)| info).collect())
}
