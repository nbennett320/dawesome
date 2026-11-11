// color themes: the stock ones are embedded, and a theme file may give just
// the required colors, with the rest derived from them
//
// the format and every key are described in themes/README.md. the tables
// below follow its order, and a test checks that they still match it
use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

pub const DEFAULT_LIGHT: &str = "slate-light";
pub const DEFAULT_DARK: &str = "charcoal";

#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
  Light,
  Dark,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTheme {
  pub id: String,
  pub name: String,
  pub appearance: Appearance,
  // core keys by name, section keys as "section.key"
  pub colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

impl Rgba {
  // #rgb, #rrggbb or #rrggbbaa, in either case
  pub fn parse(text: &str) -> Option<Rgba> {
    let hex = text.strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
      return None;
    }
    let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).unwrap();
    let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
    match hex.len() {
      3 => Some(Rgba { r: digit(0) * 17, g: digit(1) * 17, b: digit(2) * 17, a: 255 }),
      6 => Some(Rgba { r: pair(0), g: pair(2), b: pair(4), a: 255 }),
      8 => Some(Rgba { r: pair(0), g: pair(2), b: pair(4), a: pair(6) }),
      _ => None,
    }
  }

  // lowercase, and the alpha only when it isn't ff
  pub fn to_hex(self) -> String {
    if self.a == 255 {
      format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    } else {
      format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }
  }

  // per channel, alpha included
  fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let channel = |x: u8, y: u8| (x as f64 * (1.0 - t) + y as f64 * t).round() as u8;
    Rgba {
      r: channel(a.r, b.r),
      g: channel(a.g, b.g),
      b: channel(a.b, b.b),
      a: channel(a.a, b.a),
    }
  }

  fn with_alpha(self, alpha: f64) -> Rgba {
    Rgba { a: (alpha * 255.0).round() as u8, ..self }
  }

  // WCAG 2 relative luminance (alpha ignored)
  #[cfg(test)]
  fn luminance(self) -> f64 {
    let linear = |c: u8| {
      let c = c as f64 / 255.0;
      if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
  }
}

// how a key is worked out when the theme doesn't give it. a key name with
// no dot is a core key; "fourtrack.ledRedOn" is a key of a section
enum Derive {
  Required,
  Ref(&'static str),
  Mix(&'static str, &'static str, f64),
  Alpha(&'static str, f64),
  Literal(&'static str),
}

use Derive::*;

type Table = &'static [(&'static str, Derive)];

// the tables, in README order. a key only uses keys above it, or in earlier
// tables
static COLORS: Table = &[
  ("surface", Required),
  ("panel", Required),
  ("raised", Required),
  ("sunken", Required),
  ("line", Required),
  ("lineStrong", Required),
  ("inkStrong", Required),
  ("ink", Required),
  ("inkSoft", Required),
  ("inkFaint", Required),
  ("accent", Required),
  ("accentSoft", Required),
  ("accentTint", Required),
  ("accentInk", Required),
  ("led", Required),
  ("ledTint", Required),
  ("ledInk", Required),
  ("danger", Required),
  ("dangerTint", Required),
  ("surfaceAlt", Mix("surface", "panel", 0.5)),
  ("inkMid", Mix("ink", "inkSoft", 0.5)),
  ("accentFocus", Mix("accent", "accentSoft", 0.5)),
  ("accentStrong", Mix("accent", "accentInk", 0.5)),
  ("accentTintStrong", Mix("accentTint", "accentSoft", 0.25)),
  ("ledLine", Mix("led", "ledTint", 0.45)),
  ("dangerBright", Ref("danger")),
  ("dangerLine", Mix("dangerTint", "danger", 0.25)),
  ("dangerInk", Mix("danger", "inkStrong", 0.6)),
  ("ok", Literal("#84cc16")),
  ("shadow", Literal("#000000")),
];

static PLAYLIST: Table = &[
  ("background", Ref("surface")),
  ("laneAlt", Ref("surfaceAlt")),
  ("laneLine", Ref("line")),
  ("beatLine", Ref("line")),
  ("barLine", Ref("lineStrong")),
  ("ruler", Ref("panel")),
  ("rulerLine", Ref("lineStrong")),
  ("rulerText", Ref("inkFaint")),
  ("rulerBarText", Ref("inkMid")),
  ("playhead", Ref("accent")),
  ("playheadStopped", Ref("inkFaint")),
  ("loopEnd", Ref("inkMid")),
  ("clip", Ref("raised")),
  ("clipBorder", Ref("lineStrong")),
  ("clipLabel", Ref("panel")),
  ("clipText", Ref("ink")),
  ("clipWave", Ref("inkSoft")),
  ("modified", Ref("led")),
  ("patternClip", Ref("ledTint")),
  ("patternLabel", Mix("ledTint", "led", 0.15)),
  ("patternBorder", Mix("led", "ledLine", 0.5)),
  ("patternHit", Mix("led", "ledInk", 0.25)),
  ("patternBeat", Mix("ledLine", "ledTint", 0.4)),
  ("selectedClip", Ref("accentTint")),
  ("selectedLabel", Ref("accentTintStrong")),
  ("selectedBorder", Ref("accent")),
  ("lassoFill", Ref("accent")),
  ("accent", Ref("accent")),
  ("accentSoft", Ref("accentSoft")),
  ("idle", Ref("inkFaint")),
  ("scrollbar", Ref("inkSoft")),
  ("debug", Literal("#ffbb99")),
  ("debugText", Ref("inkStrong")),
];

static WAVEFORM: Table = &[
  ("background", Ref("surface")),
  ("ruler", Ref("panel")),
  ("rulerLine", Ref("lineStrong")),
  ("grid", Ref("line")),
  ("beat", Ref("inkFaint")),
  ("centre", Ref("lineStrong")),
  ("wave", Ref("accentStrong")),
  ("dimmed", Ref("lineStrong")),
  ("wash", Alpha("panel", 0.6)),
  ("fill", Alpha("accent", 0.12)),
  ("envelope", Ref("led")),
  ("fade", Alpha("led", 0.07)),
];

static METER: Table = &[
  ("low", Ref("ok")),
  ("mid", Ref("led")),
  ("high", Ref("dangerBright")),
  ("hold", Ref("inkMid")),
];

static FOURTRACK: Table = &[
  ("ink", Ref("ink")),
  ("inkSoft", Ref("inkSoft")),
  ("inkFaint", Ref("inkFaint")),
  ("accent", Ref("accent")),
  ("focus", Ref("accentFocus")),
  ("panel", Ref("panel")),
  ("line", Ref("line")),
  ("lineStrong", Ref("lineStrong")),
  ("capTop", Ref("raised")),
  ("capBottom", Mix("raised", "panel", 0.8)),
  ("capDown", Mix("panel", "sunken", 0.2)),
  ("display", Mix("panel", "sunken", 0.6)),
  ("displayText", Ref("inkMid")),
  ("slot", Mix("sunken", "lineStrong", 0.6)),
  ("tray", Mix("panel", "sunken", 0.65)),
  ("face", Ref("raised")),
  ("knobTop", Ref("raised")),
  ("knobBottom", Ref("panel")),
  ("dialA", Ref("line")),
  ("dialB", Ref("panel")),
  ("meterGap", Alpha("surface", 0.7)),
  ("danger", Ref("danger")),
  ("record", Ref("dangerBright")),
  ("ledRedOn", Ref("dangerBright")),
  ("ledRedGlow", Alpha("fourtrack.ledRedOn", 0.45)),
  ("ledRedOff", Mix("fourtrack.ledRedOn", "sunken", 0.85)),
  ("ledAmberOn", Ref("led")),
  ("ledAmberGlow", Alpha("fourtrack.ledAmberOn", 0.45)),
  ("ledAmberOff", Mix("fourtrack.ledAmberOn", "sunken", 0.85)),
  ("ledLimeOn", Ref("ok")),
  ("ledLimeGlow", Alpha("fourtrack.ledLimeOn", 0.45)),
  ("ledLimeOff", Mix("fourtrack.ledLimeOn", "sunken", 0.85)),
  ("ledBlueOn", Ref("accent")),
  ("ledBlueGlow", Alpha("fourtrack.ledBlueOn", 0.45)),
  ("ledBlueOff", Mix("fourtrack.ledBlueOn", "sunken", 0.85)),
  ("tape", Ref("inkFaint")),
  ("label", Ref("surfaceAlt")),
  ("stripe", Ref("accentSoft")),
  ("hub", Ref("raised")),
  ("tapeMarks", Alpha("surface", 0.55)),
];

// the json name of each table, in resolving order. "colors" is the core
static SECTIONS: &[(&str, Table)] = &[
  ("colors", COLORS),
  ("playlist", PLAYLIST),
  ("waveform", WAVEFORM),
  ("meter", METER),
  ("fourtrack", FOURTRACK),
];

// the name a key has in the resolved map
fn full_key(section: &str, key: &str) -> String {
  if section == "colors" { key.to_string() } else { format!("{}.{}", section, key) }
}

fn color(text: &str, what: &str) -> Result<Rgba, String> {
  Rgba::parse(text).ok_or_else(|| format!("{} is not a color: {:?}", what, text))
}

// read the sections that are objects, complaining about the ones that aren't
fn section_object<'a>(
  root: &'a Map<String, Value>,
  section: &str,
) -> Result<Option<&'a Map<String, Value>>, String> {
  match root.get(section) {
    None => Ok(None),
    Some(Value::Object(map)) => Ok(Some(map)),
    Some(_) => Err(format!("{} must be an object", section)),
  }
}

pub fn parse(id: &str, json: &str) -> Result<(ResolvedTheme, Vec<String>), String> {
  let root: Value = serde_json::from_str(json).map_err(|e| format!("not valid JSON: {}", e))?;
  let root = root.as_object().ok_or("a theme must be a JSON object")?;
  let mut warnings = Vec::new();

  let name = match root.get("name") {
    Some(Value::String(name)) => name.clone(),
    _ => return Err("name is missing or not text".to_string()),
  };
  let appearance = match root.get("appearance").and_then(|v| v.as_str()) {
    Some("light") => Appearance::Light,
    Some("dark") => Appearance::Dark,
    _ => return Err("appearance must be \"light\" or \"dark\"".to_string()),
  };

  for field in root.keys() {
    if field != "name" && field != "appearance" && !SECTIONS.iter().any(|(s, _)| s == field) {
      warnings.push(format!("unknown field {}", field));
    }
  }

  let mut colors: BTreeMap<String, String> = BTreeMap::new();
  let mut values: BTreeMap<String, Rgba> = BTreeMap::new();

  for (section, table) in SECTIONS {
    let given = section_object(root, section)?;

    if let Some(given) = given {
      for key in given.keys() {
        if !table.iter().any(|(k, _)| k == key) {
          warnings.push(format!("unknown key {}.{}", section, key));
        }
      }
    }

    for (key, derive) in table.iter() {
      let name = full_key(section, key);
      let lookup = |k: &str| values[k];

      let value = match given.and_then(|g| g.get(*key)) {
        Some(Value::String(text)) => color(text, &name)?,
        Some(other) => return Err(format!("{} is not a color: {}", name, other)),
        None => match derive {
          Required => return Err(format!("missing required color {}", name)),
          Ref(k) => lookup(k),
          Mix(a, b, t) => Rgba::mix(lookup(a), lookup(b), *t),
          Alpha(k, a) => lookup(k).with_alpha(*a),
          Literal(hex) => color(hex, &name)?,
        },
      };

      colors.insert(name.clone(), value.to_hex());
      values.insert(name, value);
    }
  }

  let theme = ResolvedTheme { id: id.to_string(), name, appearance, colors };
  Ok((theme, warnings))
}

const STOCK: &[(&str, &str)] = &[
  ("slate-light", include_str!("../themes/slate-light.json")),
  ("charcoal-light", include_str!("../themes/charcoal-light.json")),
  ("charcoal", include_str!("../themes/charcoal.json")),
  ("slate-dark", include_str!("../themes/slate-dark.json")),
];

// the themes that come with the app. one that fails is skipped, not fatal
pub fn stock() -> Vec<ResolvedTheme> {
  let mut themes = Vec::new();
  for (id, json) in STOCK {
    match parse(id, json) {
      Ok((theme, warnings)) => {
        for warning in warnings {
          println!("theme {}: {}", id, warning);
        }
        themes.push(theme);
      }
      Err(e) => println!("theme {} skipped: {}", id, e),
    }
  }
  themes
}

#[cfg(test)]
mod tests {
  use super::*;

  const CORE_REQUIRED: &str = r##"{
    "name": "Mini", "appearance": "light",
    "colors": {
      "surface": "#ffffff", "panel": "#f0f0f0", "raised": "#fff", "sunken": "#e0e0e0",
      "line": "#dddddd", "lineStrong": "#cccccc", "inkStrong": "#000000", "ink": "#111111",
      "inkSoft": "#666666", "inkFaint": "#999999", "accent": "#0ea5e9", "accentSoft": "#7dd3fc",
      "accentTint": "#f0f9ff", "accentInk": "#0369a1", "led": "#f59e0b", "ledTint": "#fffbeb",
      "ledInk": "#78350f", "danger": "#dc2626", "dangerTint": "#fef2f2"
    }
  }"##;

  fn mini_with(colors_patch: &str) -> String {
    CORE_REQUIRED.replacen("\"colors\": {", &format!("\"colors\": {{ {}", colors_patch), 1)
  }

  fn stock_json(id: &str) -> &'static str {
    STOCK.iter().find(|(i, _)| *i == id).unwrap().1
  }

  #[test]
  fn test_stock_themes_parse_clean() {
    let expected = [
      ("slate-light", Appearance::Light),
      ("charcoal-light", Appearance::Light),
      ("charcoal", Appearance::Dark),
      ("slate-dark", Appearance::Dark),
    ];
    let total: usize = SECTIONS.iter().map(|(_, t)| t.len()).sum();
    for (id, appearance) in expected {
      let (theme, warnings) = parse(id, stock_json(id)).unwrap();
      assert!(warnings.is_empty(), "{}: {:?}", id, warnings);
      assert_eq!(theme.appearance, appearance, "{}", id);
      assert_eq!(theme.colors.len(), total, "{}", id);
      for (section, table) in SECTIONS {
        for (key, _) in table.iter() {
          assert!(theme.colors.contains_key(&full_key(section, key)), "{} {}", id, key);
        }
      }
    }
    let ids: Vec<String> = stock().into_iter().map(|t| t.id).collect();
    assert_eq!(ids, vec!["slate-light", "charcoal-light", "charcoal", "slate-dark"]);
  }

  #[test]
  fn test_slate_light_sets_every_key() {
    let json: Value = serde_json::from_str(stock_json("slate-light")).unwrap();
    for (section, table) in SECTIONS {
      let given = json[section].as_object().unwrap_or_else(|| panic!("{} missing", section));
      for (key, _) in table.iter() {
        assert!(given.contains_key(*key), "slate-light lacks {}.{}", section, key);
      }
      assert_eq!(given.len(), table.len(), "slate-light has extra keys in {}", section);
    }
  }

  #[test]
  fn test_required_only_theme_resolves_everything() {
    let (theme, warnings) = parse("mini", CORE_REQUIRED).unwrap();
    assert!(warnings.is_empty());
    let total: usize = SECTIONS.iter().map(|(_, t)| t.len()).sum();
    assert_eq!(theme.colors.len(), total);
    assert_eq!(theme.colors["raised"], "#ffffff");
    assert_eq!(theme.colors["ok"], "#84cc16");
    assert_eq!(theme.colors["shadow"], "#000000");
    // mix(surface, panel, 0.5): ff/f0 -> f8
    assert_eq!(theme.colors["surfaceAlt"], "#f8f8f8");
    assert_eq!(theme.colors["dangerBright"], "#dc2626");
    assert_eq!(theme.colors["playlist.clip"], "#ffffff");
    assert_eq!(theme.colors["waveform.fill"], "#0ea5e91f");
    // a section key referring to a key of its own section
    assert_eq!(theme.colors["fourtrack.ledRedOn"], "#dc2626");
    assert_eq!(theme.colors["fourtrack.ledRedGlow"], "#dc262673");
  }

  #[test]
  fn test_mix_and_alpha() {
    let black = Rgba::parse("#000000").unwrap();
    let white = Rgba::parse("#ffffff").unwrap();
    assert_eq!(Rgba::mix(black, white, 0.5).to_hex(), "#808080");
    let sky = Rgba::parse("#0ea5e9").unwrap();
    assert_eq!(sky.with_alpha(0.12).to_hex(), "#0ea5e91f");
    // alpha is mixed too
    let clear = Rgba::parse("#ffffff00").unwrap();
    assert_eq!(Rgba::mix(clear, white, 0.5).a, 128);
  }

  #[test]
  fn test_missing_required_key() {
    let json = CORE_REQUIRED.replace("\"ledInk\": \"#78350f\",", "");
    let err = parse("x", &json).unwrap_err();
    assert!(err.contains("ledInk"), "{}", err);
  }

  #[test]
  fn test_bad_colors_name_the_key() {
    for bad in ["#12", "red", "#gggggg", "#12345"] {
      let json = CORE_REQUIRED.replace("\"#0ea5e9\"", &format!("\"{}\"", bad));
      let err = parse("x", &json).unwrap_err();
      assert!(err.contains("accent"), "{}: {}", bad, err);
    }
    let json = CORE_REQUIRED.replace("\"#0ea5e9\"", "12");
    assert!(parse("x", &json).unwrap_err().contains("accent"));
  }

  #[test]
  fn test_bad_theme_shape() {
    assert!(parse("x", "not json").is_err());
    assert!(parse("x", &CORE_REQUIRED.replace("\"light\"", "\"grey\"")).unwrap_err().contains("appearance"));
    assert!(parse("x", &CORE_REQUIRED.replace("\"name\": \"Mini\",", "")).unwrap_err().contains("name"));
    let json = CORE_REQUIRED.replacen("}\n  }", "}, \"playlist\": 3\n  }", 1);
    assert!(parse("x", &json).unwrap_err().contains("playlist"));
    // a bad color in a section names the section key
    let json = CORE_REQUIRED.replacen("}\n  }", "}, \"meter\": {\"low\": \"nope\"}\n  }", 1);
    assert!(parse("x", &json).unwrap_err().contains("meter.low"));
  }

  #[test]
  fn test_unknown_things_warn() {
    let json = CORE_REQUIRED
      .replacen("}\n  }", "}, \"playlist\": {\"bogus\": \"#fff\"}, \"mystery\": {}, \"extra\": 1\n  }", 1);
    let (_, warnings) = parse("x", &json).unwrap();
    assert!(warnings.iter().any(|w| w.contains("playlist.bogus")), "{:?}", warnings);
    assert!(warnings.iter().any(|w| w.contains("mystery")), "{:?}", warnings);
    assert!(warnings.iter().any(|w| w.contains("extra")), "{:?}", warnings);
  }

  #[test]
  fn test_hex_forms() {
    assert_eq!(Rgba::parse("#abc"), Some(Rgba { r: 0xaa, g: 0xbb, b: 0xcc, a: 255 }));
    assert_eq!(Rgba::parse("#ABCDEF80").unwrap().to_hex(), "#abcdef80");
    assert_eq!(Rgba::parse("#ABCDEFFF").unwrap().to_hex(), "#abcdef");
    assert_eq!(Rgba::parse("abcdef"), None);
    let (theme, _) = parse("m", &mini_with("\"accentSoft\": \"#7DD3FCFF\",").replacen("\"accentSoft\": \"#7dd3fc\",", "", 1)).unwrap();
    assert_eq!(theme.colors["accentSoft"], "#7dd3fc");
  }

  #[test]
  fn test_tables_match_readme() {
    let readme = include_str!("../themes/README.md");
    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    for line in readme.lines() {
      if let Some(rest) = line.strip_prefix("## `") {
        found.push((rest.split('`').next().unwrap().to_string(), Vec::new()));
      } else if let Some(rest) = line.strip_prefix("| `") {
        let key = rest.split('`').next().unwrap().to_string();
        found.last_mut().expect("a row before any heading").1.push(key);
      }
    }
    assert_eq!(found.len(), SECTIONS.len());
    for ((name, keys), (section, table)) in found.iter().zip(SECTIONS) {
      let ours: Vec<&str> = table.iter().map(|(k, _)| *k).collect();
      assert_eq!(name, section);
      assert_eq!(keys.iter().map(|k| k.as_str()).collect::<Vec<_>>(), ours, "{}", section);
    }
  }

  #[test]
  fn test_contrast() {
    fn ratio(theme: &ResolvedTheme, a: &str, b: &str) -> f64 {
      let la = Rgba::parse(&theme.colors[a]).unwrap().luminance();
      let lb = Rgba::parse(&theme.colors[b]).unwrap().luminance();
      (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }
    for theme in stock() {
      println!(
        "{:<15} accent/surface {:.2}  inkFaint/surface {:.2}  ink/panel {:.2}  ink/surface {:.2}  inkSoft/surface {:.2}",
        theme.id,
        ratio(&theme, "accent", "surface"),
        ratio(&theme, "inkFaint", "surface"),
        ratio(&theme, "ink", "panel"),
        ratio(&theme, "ink", "surface"),
        ratio(&theme, "inkSoft", "surface"),
      );
      assert!(ratio(&theme, "ink", "surface") >= 4.5, "{} ink", theme.id);
      assert!(ratio(&theme, "inkSoft", "surface") >= 4.5, "{} inkSoft", theme.id);
    }
  }
}
