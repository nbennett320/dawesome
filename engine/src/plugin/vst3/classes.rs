// what a module's factory says it has: its classes, kept when they're audio
// modules (effects and instruments), and turned into PluginInfo
//
// a class is known by its 16-byte id. projects save it as "vst3:" and 32 hex
// digits, written the way the VST3 SDK writes them, so a project names the
// same plugin on every system
use super::module::Module;
use crate::plugin::{PluginInfo, PluginKind};
use ::vst3::Steinberg::{
  char16, char8, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3, IPluginFactory3Trait,
  IPluginFactoryTrait, PClassInfo, PClassInfo2, PClassInfoW, PFactoryInfo, TUID,
};
use ::vst3::Steinberg::kResultOk;

// the class category of effects and instruments (kVstAudioEffectClass)
pub const AUDIO_MODULE_CLASS: &str = "Audio Module Class";

// every audio module class in a module, with its id
pub fn classes(module: &Module) -> Vec<(TUID, PluginInfo)> {
  let factory = module.factory();
  let path = module.path().to_string_lossy().into_owned();
  let factory2 = factory.cast::<IPluginFactory2>();
  let factory3 = factory.cast::<IPluginFactory3>();

  // the factory's vendor, for classes that don't name their own
  // safe (here and below): the structs are plain data the factory fills in
  let mut factory_info: PFactoryInfo = unsafe { std::mem::zeroed() };
  let factory_vendor = if unsafe { factory.getFactoryInfo(&mut factory_info) } == kResultOk {
    text8(&factory_info.vendor)
  } else {
    String::new()
  };

  let mut found = Vec::new();
  let count = unsafe { factory.countClasses() };
  for i in 0..count.max(0) {
    let mut class = None;
    if let Some(factory3) = &factory3 {
      let mut info: PClassInfoW = unsafe { std::mem::zeroed() };
      if unsafe { factory3.getClassInfoUnicode(i, &mut info) } == kResultOk {
        class = Some(Class {
          cid: info.cid,
          category: text8(&info.category),
          name: text16(&info.name),
          subcategories: text8(&info.subCategories),
          vendor: text16(&info.vendor),
          version: text16(&info.version),
        });
      }
    }
    if class.is_none() {
      if let Some(factory2) = &factory2 {
        let mut info: PClassInfo2 = unsafe { std::mem::zeroed() };
        if unsafe { factory2.getClassInfo2(i, &mut info) } == kResultOk {
          class = Some(Class {
            cid: info.cid,
            category: text8(&info.category),
            name: text8(&info.name),
            subcategories: text8(&info.subCategories),
            vendor: text8(&info.vendor),
            version: text8(&info.version),
          });
        }
      }
    }
    if class.is_none() {
      let mut info: PClassInfo = unsafe { std::mem::zeroed() };
      if unsafe { factory.getClassInfo(i, &mut info) } == kResultOk {
        class = Some(Class {
          cid: info.cid,
          category: text8(&info.category),
          name: text8(&info.name),
          subcategories: String::new(),
          vendor: String::new(),
          version: String::new(),
        });
      }
    }

    let Some(class) = class else { continue };
    if class.category != AUDIO_MODULE_CLASS {
      continue;
    }
    let subcategories: Vec<String> = class
      .subcategories
      .split('|')
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .collect();
    let kind = kind_of(&subcategories);
    found.push((
      class.cid,
      PluginInfo {
        id: plugin_id(&class.cid),
        path: path.clone(),
        name: class.name,
        vendor: if class.vendor.is_empty() { factory_vendor.clone() } else { class.vendor },
        version: class.version,
        kind,
        category: category_of(kind, &subcategories),
        subcategories,
      },
    ));
  }
  found
}

struct Class {
  cid: TUID,
  category: String,
  name: String,
  subcategories: String,
  vendor: String,
  version: String,
}

// a fixed-size C string from a struct
pub fn text8(chars: &[char8]) -> String {
  let bytes: Vec<u8> = chars.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
  String::from_utf8_lossy(&bytes).trim().to_string()
}

// a fixed-size UTF-16 string from a struct
pub fn text16(chars: &[char16]) -> String {
  let end = chars.iter().position(|&c| c == 0).unwrap_or(chars.len());
  String::from_utf16_lossy(&chars[..end]).trim().to_string()
}

// an instrument says so in its subcategories ("Instrument|Synth")
pub fn kind_of(subcategories: &[String]) -> PluginKind {
  if subcategories.iter().any(|s| s.eq_ignore_ascii_case("Instrument")) {
    PluginKind::Instrument
  } else {
    PluginKind::Effect
  }
}

// where the browser files a plugin, from its subcategories. the first one
// that names a known group wins ("Fx|Dynamics" → Dynamics)
pub fn category_of(kind: PluginKind, subcategories: &[String]) -> String {
  let groups: &[(&str, &str)] = match kind {
    PluginKind::Instrument => &[
      ("Drum", "Drum"),
      ("Piano", "Piano"),
      ("Sampler", "Sampler"),
      ("Synth", "Synth"),
    ],
    PluginKind::Effect => &[
      ("Analyzer", "Analyzer"),
      ("Delay", "Delay"),
      ("Distortion", "Distortion"),
      ("Dynamics", "Dynamics"),
      ("EQ", "EQ"),
      ("Filter", "Filter"),
      ("Generator", "Generator"),
      ("Mastering", "Mastering"),
      ("Modulation", "Modulation"),
      ("Pitch Shift", "Pitch Shift"),
      ("Restoration", "Restoration"),
      ("Reverb", "Reverb"),
      ("Spatial", "Spatial"),
      ("Surround", "Spatial"),
      ("Tools", "Tools"),
    ],
  };
  // instruments are matched by how specific the group is (a drum sampler is
  // a Drum), effects by the plugin's own order
  match kind {
    PluginKind::Instrument => groups
      .iter()
      .find(|(sub, _)| subcategories.iter().any(|s| s.eq_ignore_ascii_case(sub)))
      .map(|(_, group)| group.to_string()),
    PluginKind::Effect => subcategories.iter().find_map(|s| {
      groups.iter().find(|(sub, _)| s.eq_ignore_ascii_case(sub)).map(|(_, group)| group.to_string())
    }),
  }
  .unwrap_or_else(|| "Other".to_string())
}

// the bytes of a class id in the order the SDK prints them. on Windows the
// first three fields of a TUID are stored as a COM GUID's (little-endian);
// everywhere else they're already in order. the swap is its own inverse
fn printed_order(cid: &TUID) -> [u8; 16] {
  let bytes: [u8; 16] = std::array::from_fn(|i| cid[i] as u8);
  if cfg!(target_os = "windows") {
    let order = [3, 2, 1, 0, 5, 4, 7, 6, 8, 9, 10, 11, 12, 13, 14, 15];
    std::array::from_fn(|i| bytes[order[i]])
  } else {
    bytes
  }
}

pub fn class_id_hex(cid: &TUID) -> String {
  printed_order(cid).iter().map(|b| format!("{:02X}", b)).collect()
}

pub fn plugin_id(cid: &TUID) -> String {
  format!("vst3:{}", class_id_hex(cid))
}

// the class id a plugin id names
pub fn parse_plugin_id(id: &str) -> Option<TUID> {
  let hex = id.strip_prefix("vst3:")?;
  if hex.len() != 32 || !hex.is_ascii() {
    return None;
  }
  let mut cid: TUID = [0; 16];
  for (i, byte) in cid.iter_mut().enumerate() {
    *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()? as _;
  }
  // back from printed order (the same swap)
  let bytes = printed_order(&cid);
  Some(std::array::from_fn(|i| bytes[i] as _))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn subs(s: &str) -> Vec<String> {
    s.split('|').map(String::from).collect()
  }

  #[test]
  fn test_effect_category_comes_from_its_subcategories() {
    assert_eq!(category_of(PluginKind::Effect, &subs("Fx|Dynamics")), "Dynamics");
    assert_eq!(category_of(PluginKind::Effect, &subs("Fx|Reverb|Delay")), "Reverb");
    assert_eq!(category_of(PluginKind::Effect, &subs("Fx|Pitch Shift")), "Pitch Shift");
    assert_eq!(category_of(PluginKind::Effect, &subs("Fx|Surround")), "Spatial");
    assert_eq!(category_of(PluginKind::Effect, &subs("Fx|eq")), "EQ");
  }

  #[test]
  fn test_unknown_or_missing_subcategory_is_other() {
    assert_eq!(category_of(PluginKind::Effect, &subs("Fx")), "Other");
    assert_eq!(category_of(PluginKind::Effect, &[]), "Other");
    assert_eq!(category_of(PluginKind::Instrument, &subs("Instrument")), "Other");
  }

  #[test]
  fn test_instrument_category_prefers_the_specific_group() {
    assert_eq!(category_of(PluginKind::Instrument, &subs("Instrument|Synth")), "Synth");
    assert_eq!(category_of(PluginKind::Instrument, &subs("Instrument|Synth|Sampler")), "Sampler");
    assert_eq!(category_of(PluginKind::Instrument, &subs("Instrument|Sampler|Drum")), "Drum");
    assert_eq!(category_of(PluginKind::Instrument, &subs("Instrument|Piano")), "Piano");
  }

  #[test]
  fn test_instrument_subcategory_makes_an_instrument() {
    assert_eq!(kind_of(&subs("Instrument|Synth")), PluginKind::Instrument);
    assert_eq!(kind_of(&subs("Fx|Dynamics")), PluginKind::Effect);
    assert_eq!(kind_of(&[]), PluginKind::Effect);
  }

  #[test]
  fn test_class_id_is_32_hex_digits_and_parses_back() {
    let cid: TUID = std::array::from_fn(|i| (i as u8 * 17) as _);
    let id = plugin_id(&cid);
    assert_eq!(id.len(), 5 + 32);
    assert!(id.starts_with("vst3:"));
    assert_eq!(parse_plugin_id(&id), Some(cid));
    if !cfg!(target_os = "windows") {
      assert_eq!(&id[5..13], "00112233");
    }
  }

  #[test]
  fn test_bad_plugin_ids_dont_parse() {
    assert_eq!(parse_plugin_id("vst3:1234"), None);
    assert_eq!(parse_plugin_id("clap:00112233445566778899AABBCCDDEEFF"), None);
    assert_eq!(parse_plugin_id("vst3:0011223344556677889900AABBCCDDZZ"), None);
  }

  #[test]
  fn test_fixed_size_strings_stop_at_nul() {
    let mut name: [char8; 8] = [0; 8];
    for (i, b) in b"Tape".iter().enumerate() {
      name[i] = *b as char8;
    }
    assert_eq!(text8(&name), "Tape");
    let wide: Vec<char16> = "Délai\0junk".encode_utf16().collect();
    assert_eq!(text16(&wide), "Délai");
  }
}
