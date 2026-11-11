// a plugin's module: the bundle (or library) a VST3 plugin ships in, opened
// once and shared by everything made from it
//
// opening one calls its entry point (bundleEntry on macOS, ModuleEntry on
// Linux, InitDll on Windows) and asks it for its factory, which is what makes
// the plugin's objects. when the last Arc to a module goes, the factory is
// released, the exit point is called, and the library is let go
//
// a plugin torn down while others from its module are still alive is
// parked here, terminated but not released, until the module goes. some
// plugins can't have one instance destroyed while another lives on:
// DecentSampler, once it has loaded its sounds, crashes on the main run loop
// a moment after (in a callback it queued itself), which a mixdown's copy or
// a removed instrument would set off. released along with the module, with
// no turn of the run loop in between, they're safe
use ::vst3::ComPtr;
use ::vst3::Steinberg::IPluginFactory;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

pub struct Module {
  path: PathBuf,
  // released before the module's exit point is called
  factory: Option<ComPtr<IPluginFactory>>,
  // plugins' objects that have been torn down, released before the factory
  parked: Mutex<Vec<Box<dyn std::any::Any>>>,
  library: platform::Library,
}

// safe: a module's factory is meant to be used from any thread, CFBundle is
// a thread-safe CoreFoundation object, and libraries are process-wide
unsafe impl Send for Module {}
unsafe impl Sync for Module {}

// the modules that are open, so a second plugin from the same bundle shares
// the first one's
static OPEN: Mutex<Vec<(PathBuf, Weak<Module>)>> = Mutex::new(Vec::new());

// held while a module is being opened or closed, so a bundle's entry point is
// never called while its exit point is still running
static OPENING: Mutex<()> = Mutex::new(());

impl Module {
  pub fn open(path: &Path) -> Result<Arc<Module>, String> {
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let _opening = OPENING.lock().unwrap_or_else(|e| e.into_inner());
    let mut open = OPEN.lock().unwrap_or_else(|e| e.into_inner());
    open.retain(|(_, module)| module.strong_count() > 0);
    if let Some(module) = open.iter().find(|(p, _)| *p == key).and_then(|(_, m)| m.upgrade()) {
      return Ok(module);
    }

    let (library, factory) = platform::Library::open(&key)?;
    // GetPluginFactory hands over a reference of its own
    // safe: the pointer is checked for null and comes from the module
    let Some(factory) = (unsafe { ComPtr::from_raw(factory) }) else {
      // the library's own drop calls its exit point
      return Err(format!("{} has no plugin factory", path.display()));
    };
    let module = Arc::new(Module { path: key.clone(), factory: Some(factory), parked: Mutex::new(Vec::new()), library });
    open.push((key, Arc::downgrade(&module)));
    Ok(module)
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  // keep a torn-down plugin's objects until the module goes (see the top)
  pub fn park(&self, objects: Box<dyn std::any::Any>) {
    self.parked.lock().unwrap_or_else(|e| e.into_inner()).push(objects);
  }

  pub fn factory(&self) -> &ComPtr<IPluginFactory> {
    // a module without a factory is never handed out (see open)
    self.factory.as_ref().expect("module without a factory")
  }
}

impl Drop for Module {
  fn drop(&mut self) {
    let _opening = OPENING.lock().unwrap_or_else(|e| e.into_inner());
    self.parked.get_mut().unwrap_or_else(|e| e.into_inner()).clear();
    self.factory = None;
    self.library.exit();
  }
}

// the binary inside a bundle, for this machine
pub fn bundle_binary(bundle: &Path) -> PathBuf {
  binary_in_bundle(bundle, std::env::consts::OS, std::env::consts::ARCH, bundle.is_dir())
}

// where a bundle keeps its binary on the given system (the folder names are
// the VST3 SDK's). on Windows a plugin can also be a single .vst3 file
pub(crate) fn binary_in_bundle(bundle: &Path, os: &str, arch: &str, is_dir: bool) -> PathBuf {
  let name = bundle.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
  let contents = bundle.join("Contents");
  match os {
    "macos" => contents.join("MacOS").join(name),
    "windows" => {
      if !is_dir {
        return bundle.to_path_buf();
      }
      let folder = match arch {
        "aarch64" => "arm64-win",
        "x86" => "x86-win",
        _ => "x86_64-win",
      };
      contents.join(folder).join(format!("{}.vst3", name))
    }
    _ => contents.join(format!("{}-linux", arch)).join(format!("{}.so", name)),
  }
}

#[cfg(target_os = "macos")]
mod platform {
  use ::vst3::Steinberg::IPluginFactory;
  use core_foundation::base::TCFType;
  use core_foundation::bundle::{CFBundle, CFBundleLoadExecutable, CFBundleRef};
  use core_foundation::string::CFString;
  use core_foundation::url::CFURL;
  use std::ffi::c_void;
  use std::path::Path;

  type BundleEntry = unsafe extern "C" fn(CFBundleRef) -> bool;
  type BundleExit = unsafe extern "C" fn() -> bool;
  type GetPluginFactory = unsafe extern "system" fn() -> *mut IPluginFactory;

  pub struct Library {
    bundle: CFBundle,
    // whether bundleEntry was called (and so bundleExit is owed)
    entered: bool,
  }

  impl Library {
    pub fn open(path: &Path) -> Result<(Library, *mut IPluginFactory), String> {
      let url = CFURL::from_path(path, true).ok_or_else(|| format!("{} isn't a path", path.display()))?;
      let bundle = CFBundle::new(url).ok_or_else(|| format!("{} isn't a bundle", path.display()))?;
      // safe: the bundle ref is alive for the call
      if unsafe { CFBundleLoadExecutable(bundle.as_concrete_TypeRef()) } == 0 {
        return Err(format!("couldn't load {}", path.display()));
      }
      let mut library = Library { bundle, entered: false };

      let entry = library.function("bundleEntry");
      if !entry.is_null() {
        // safe: bundleEntry has this signature in every VST3 module
        let entry: BundleEntry = unsafe { std::mem::transmute(entry) };
        if !unsafe { entry(library.bundle.as_concrete_TypeRef()) } {
          return Err("the plugin's bundleEntry failed".into());
        }
        library.entered = true;
      }

      let factory = library.function("GetPluginFactory");
      if factory.is_null() {
        return Err(format!("{} has no GetPluginFactory", path.display()));
      }
      // safe: GetPluginFactory has this signature in every VST3 module
      let factory: GetPluginFactory = unsafe { std::mem::transmute(factory) };
      Ok((library, unsafe { factory() }))
    }

    fn function(&self, name: &str) -> *const c_void {
      self.bundle.function_pointer_for_name(CFString::new(name))
    }

    pub fn exit(&mut self) {
      if !self.entered {
        return;
      }
      self.entered = false;
      let exit = self.function("bundleExit");
      if !exit.is_null() {
        // safe: bundleExit has this signature, and bundleEntry was called
        let exit: BundleExit = unsafe { std::mem::transmute(exit) };
        unsafe { exit() };
      }
      // the executable stays loaded: code that registered Objective-C
      // classes can't be unloaded, and CoreFoundation keeps the bundle
    }
  }

  impl Drop for Library {
    fn drop(&mut self) {
      self.exit();
    }
  }
}

#[cfg(target_os = "linux")]
mod platform {
  use super::bundle_binary;
  use ::vst3::Steinberg::IPluginFactory;
  use libloading::os::unix::Library as Lib;
  use std::ffi::c_void;
  use std::path::Path;

  type ModuleEntry = unsafe extern "C" fn(*mut c_void) -> bool;
  type ModuleExit = unsafe extern "C" fn() -> bool;
  type GetPluginFactory = unsafe extern "system" fn() -> *mut IPluginFactory;

  pub struct Library {
    library: Option<Lib>,
    entered: bool,
  }

  impl Library {
    pub fn open(path: &Path) -> Result<(Library, *mut IPluginFactory), String> {
      let binary = bundle_binary(path);
      // safe: loading a library runs its initializers, which is what
      // loading a plugin means
      let lib = unsafe { Lib::new(&binary) }.map_err(|e| format!("couldn't load {}: {}", binary.display(), e))?;
      let handle = lib.into_raw();
      // safe: the handle was just taken from a Library
      let lib = unsafe { Lib::from_raw(handle) };
      let mut library = Library { library: Some(lib), entered: false };
      let lib = library.library.as_ref().unwrap();

      // safe: the symbols have these signatures in every VST3 module
      let entry = unsafe { lib.get::<ModuleEntry>(b"ModuleEntry\0") }.ok().map(|s| *s);
      let factory = unsafe { lib.get::<GetPluginFactory>(b"GetPluginFactory\0") }.ok().map(|s| *s);
      if let Some(entry) = entry {
        if !unsafe { entry(handle) } {
          return Err("the plugin's ModuleEntry failed".into());
        }
        library.entered = true;
      }
      let factory = factory.ok_or_else(|| format!("{} has no GetPluginFactory", binary.display()))?;
      Ok((library, unsafe { factory() }))
    }

    pub fn exit(&mut self) {
      if !self.entered {
        return;
      }
      self.entered = false;
      if let Some(lib) = &self.library {
        // safe: ModuleExit has this signature, and ModuleEntry was called
        if let Ok(exit) = unsafe { lib.get::<ModuleExit>(b"ModuleExit\0") } {
          unsafe { exit() };
        }
      }
    }
  }

  impl Drop for Library {
    fn drop(&mut self) {
      self.exit();
      self.library = None;
    }
  }
}

#[cfg(target_os = "windows")]
mod platform {
  use super::bundle_binary;
  use ::vst3::Steinberg::IPluginFactory;
  use libloading::Library as Lib;
  use std::path::Path;

  type InitDll = unsafe extern "system" fn() -> bool;
  type ExitDll = unsafe extern "system" fn() -> bool;
  type GetPluginFactory = unsafe extern "system" fn() -> *mut IPluginFactory;

  pub struct Library {
    library: Option<Lib>,
    entered: bool,
  }

  impl Library {
    pub fn open(path: &Path) -> Result<(Library, *mut IPluginFactory), String> {
      let binary = bundle_binary(path);
      // safe: loading a library runs its initializers, which is what
      // loading a plugin means
      let lib = unsafe { Lib::new(&binary) }.map_err(|e| format!("couldn't load {}: {}", binary.display(), e))?;
      let mut library = Library { library: Some(lib), entered: false };
      let lib = library.library.as_ref().unwrap();

      // safe: the symbols have these signatures in every VST3 module
      let entry = unsafe { lib.get::<InitDll>(b"InitDll\0") }.ok().map(|s| *s);
      let factory = unsafe { lib.get::<GetPluginFactory>(b"GetPluginFactory\0") }.ok().map(|s| *s);
      if let Some(entry) = entry {
        if !unsafe { entry() } {
          return Err("the plugin's InitDll failed".into());
        }
        library.entered = true;
      }
      let factory = factory.ok_or_else(|| format!("{} has no GetPluginFactory", binary.display()))?;
      Ok((library, unsafe { factory() }))
    }

    pub fn exit(&mut self) {
      if !self.entered {
        return;
      }
      self.entered = false;
      if let Some(lib) = &self.library {
        // safe: ExitDll has this signature, and InitDll was called
        if let Ok(exit) = unsafe { lib.get::<ExitDll>(b"ExitDll\0") } {
          unsafe { exit() };
        }
      }
    }
  }

  impl Drop for Library {
    fn drop(&mut self) {
      self.exit();
      self.library = None;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_bundle_binary_on_each_system() {
    let bundle = Path::new("/plugins/Tape Model.vst3");
    assert_eq!(
      binary_in_bundle(bundle, "macos", "aarch64", true),
      Path::new("/plugins/Tape Model.vst3/Contents/MacOS/Tape Model")
    );
    assert_eq!(
      binary_in_bundle(bundle, "linux", "x86_64", true),
      Path::new("/plugins/Tape Model.vst3/Contents/x86_64-linux/Tape Model.so")
    );
    assert_eq!(
      binary_in_bundle(bundle, "windows", "x86_64", true),
      Path::new("/plugins/Tape Model.vst3/Contents/x86_64-win/Tape Model.vst3")
    );
  }

  #[test]
  fn test_single_file_plugin_on_windows_is_its_own_binary() {
    let file = Path::new("C:/VST3/Old.vst3");
    assert_eq!(binary_in_bundle(file, "windows", "x86_64", false), file);
  }

  #[test]
  fn test_opening_a_missing_bundle_fails() {
    assert!(Module::open(Path::new("/nowhere/Missing.vst3")).is_err());
  }
}
