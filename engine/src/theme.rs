// color themes: the stock ones are embedded, and a theme file may give just
// the required colors, with the rest taken from themes/base.json
//
// the format and every key are described in themes/README.md. base.json lists
// every key and what it is when a theme leaves it out
use std::collections::BTreeMap;
use std::sync::OnceLock;

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

// a value: a color, a reference to another key, or a function of those
#[derive(Debug, Clone, PartialEq)]
enum Expr {
  Color(Rgba),
  Ref(String),
  Mix(Box<Expr>, Box<Expr>, f64),
  Alpha(Box<Expr>, f64),
  Required,
}

// a tiny recursive-descent reader over the text of one value
struct Reader {
  chars: Vec<char>,
  pos: usize,
}

impl Reader {
  fn skip_space(&mut self) {
    while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
      self.pos += 1;
    }
  }

  // the next non-space character, if it's `want`, is consumed
  fn eat(&mut self, want: char) -> bool {
    self.skip_space();
    let found = self.chars.get(self.pos) == Some(&want);
    if found {
      self.pos += 1;
    }
    found
  }

  fn expect(&mut self, want: char) -> Result<(), String> {
    if self.eat(want) { Ok(()) } else { Err(format!("expected {:?}", want)) }
  }

  fn take_while(&mut self, keep: impl Fn(char) -> bool) -> String {
    let start = self.pos;
    while self.chars.get(self.pos).is_some_and(|c| keep(*c)) {
      self.pos += 1;
    }
    self.chars[start..self.pos].iter().collect()
  }

  fn number(&mut self) -> Result<f64, String> {
    self.skip_space();
    let text = self.take_while(|c| c.is_ascii_digit() || "+-.eE".contains(c));
    text.parse().map_err(|_| format!("{:?} is not a number", text))
  }

  fn expr(&mut self) -> Result<Expr, String> {
    self.skip_space();
    if self.chars.get(self.pos) == Some(&'#') {
      self.pos += 1;
      let hex = self.take_while(|c| c.is_ascii_alphanumeric());
      let text = format!("#{}", hex);
      return Rgba::parse(&text).map(Expr::Color).ok_or(format!("{:?} is not a color", text));
    }

    let word = self.take_while(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    if word.is_empty() {
      return Err("expected a color, a key or a function".to_string());
    }
    if !self.eat('(') {
      return Ok(if word == "required" { Expr::Required } else { Expr::Ref(word) });
    }

    let expr = match word.as_str() {
      "mix" => {
        let a = self.expr()?;
        self.expect(',')?;
        let b = self.expr()?;
        self.expect(',')?;
        let t = self.number()?;
        if !(0.0..=1.0).contains(&t) {
          return Err(format!("mix weight {} is outside 0 to 1", t));
        }
        Expr::Mix(Box::new(a), Box::new(b), t)
      }
      "alpha" => {
        let k = self.expr()?;
        self.expect(',')?;
        let a = self.number()?;
        if !(0.0..=1.0).contains(&a) {
          return Err(format!("alpha {} is outside 0 to 1", a));
        }
        Expr::Alpha(Box::new(k), a)
      }
      other => return Err(format!("unknown function {}", other)),
    };
    self.expect(')')?;
    Ok(expr)
  }
}

fn parse_expr(text: &str) -> Result<Expr, String> {
  let mut reader = Reader { chars: text.chars().collect(), pos: 0 };
  let expr = reader.expr()?;
  reader.skip_space();
  if reader.pos < reader.chars.len() {
    return Err(format!("unexpected {:?}", reader.chars[reader.pos..].iter().collect::<String>()));
  }
  Ok(expr)
}

// base.json, read once: the sections in it, and each key's default
struct Base {
  // section name -> (key, expression)
  sections: BTreeMap<String, Vec<(String, String)>>,
  // the same by resolved name: "surface", "playlist.clip"
  by_name: BTreeMap<String, String>,
}

// the name a key has in the resolved map
fn full_key(section: &str, key: &str) -> String {
  if section == "colors" { key.to_string() } else { format!("{}.{}", section, key) }
}

fn base() -> &'static Base {
  static BASE: OnceLock<Base> = OnceLock::new();
  BASE.get_or_init(|| {
    let root: Value = serde_json::from_str(include_str!("../themes/base.json")).expect("base.json is JSON");
    let mut base = Base { sections: BTreeMap::new(), by_name: BTreeMap::new() };
    for (section, keys) in root.as_object().expect("base.json is an object") {
      let mut list = Vec::new();
      for (key, text) in keys.as_object().expect("base sections are objects") {
        let text = text.as_str().expect("base values are text").to_string();
        base.by_name.insert(full_key(section, key), text.clone());
        list.push((key.clone(), text));
      }
      base.sections.insert(section.clone(), list);
    }
    base
  })
}

// works out keys on demand, remembering each
struct Resolver<'a> {
  // the theme's own values by resolved name
  given: BTreeMap<String, String>,
  done: BTreeMap<String, Rgba>,
  // the keys being worked out now, to spot a loop
  active: Vec<&'a str>,
}

impl<'a> Resolver<'a> {
  fn resolve(&mut self, name: &'a str) -> Result<Rgba, String> {
    if let Some(value) = self.done.get(name) {
      return Ok(*value);
    }
    if let Some(at) = self.active.iter().position(|k| *k == name) {
      let mut cycle = self.active[at..].to_vec();
      cycle.push(name);
      return Err(format!("{} is part of a loop: {}", name, cycle.join(" -> ")));
    }

    let own = self.given.get(name);
    let text = match own {
      Some(text) => text.as_str(),
      None => base().by_name.get(name).ok_or(format!("{} is not a key", name))?.as_str(),
    };
    let expr = parse_expr(text).map_err(|e| format!("{}: {}", name, e))?;
    if expr == Expr::Required && own.is_none() {
      return Err(format!("missing required color {}", name));
    }

    self.active.push(name);
    let value = self.eval(&expr, name);
    self.active.pop();
    let value = value?;
    self.done.insert(name.to_string(), value);
    Ok(value)
  }

  // `owner` is the key the expression belongs to, for the messages
  fn eval(&mut self, expr: &Expr, owner: &str) -> Result<Rgba, String> {
    match expr {
      Expr::Color(c) => Ok(*c),
      Expr::Ref(other) => {
        let (other, _) = base().by_name.get_key_value(other.as_str()).ok_or_else(|| {
          format!("{} refers to {}, which is not a key", owner, other)
        })?;
        self.resolve(other)
      }
      Expr::Mix(a, b, t) => Ok(Rgba::mix(self.eval(a, owner)?, self.eval(b, owner)?, *t)),
      Expr::Alpha(k, a) => Ok(self.eval(k, owner)?.with_alpha(*a)),
      Expr::Required => Err(format!("{}: required is only for base.json", owner)),
    }
  }
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
  let base = base();

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
    if field != "name" && field != "appearance" && !base.sections.contains_key(field) {
      warnings.push(format!("unknown field {}", field));
    }
  }

  let mut given = BTreeMap::new();
  for (section, keys) in &base.sections {
    let Some(own) = section_object(root, section)? else { continue };
    for (key, value) in own {
      let name = full_key(section, key);
      if !keys.iter().any(|(k, _)| k == key) {
        warnings.push(format!("unknown key {}", name));
        continue;
      }
      match value {
        Value::String(text) => given.insert(name, text.clone()),
        other => return Err(format!("{} is not a color: {}", name, other)),
      };
    }
  }

  let mut resolver = Resolver { given, done: BTreeMap::new(), active: Vec::new() };
  let mut colors = BTreeMap::new();
  for name in base.by_name.keys() {
    colors.insert(name.clone(), resolver.resolve(name)?.to_hex());
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
  use std::collections::BTreeSet;

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

  // the mini theme with one more key in a section
  fn mini_set(section: &str, key: &str, value: &str) -> String {
    if section == "colors" {
      return mini_with(&format!("\"{}\": \"{}\",", key, value));
    }
    let extra = format!("}}, \"{}\": {{\"{}\": \"{}\"}}\n  }}", section, key, value);
    CORE_REQUIRED.replacen("}\n  }", &extra, 1)
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
    assert_eq!(base().by_name.len(), 119);
    for (id, appearance) in expected {
      let (theme, warnings) = parse(id, stock_json(id)).unwrap();
      assert!(warnings.is_empty(), "{}: {:?}", id, warnings);
      assert_eq!(theme.appearance, appearance, "{}", id);
      assert_eq!(theme.colors.len(), 119, "{}", id);
      for name in base().by_name.keys() {
        assert!(theme.colors.contains_key(name), "{} {}", id, name);
      }
    }
    let ids: Vec<String> = stock().into_iter().map(|t| t.id).collect();
    assert_eq!(ids, vec!["slate-light", "charcoal-light", "charcoal", "slate-dark"]);
  }

  #[test]
  fn test_slate_light_matches_snapshot() {
    let snapshot: BTreeMap<String, String> =
      serde_json::from_str(include_str!("../themes/test/slate-light.resolved.json")).unwrap();
    let (theme, _) = parse("slate-light", stock_json("slate-light")).unwrap();
    assert_eq!(theme.colors, snapshot);
  }

  #[test]
  fn test_base_resolves_with_only_required_keys() {
    let (theme, warnings) = parse("mini", CORE_REQUIRED).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(theme.colors.len(), base().by_name.len());
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
  fn test_nested_expression() {
    // alpha(accent, 0.5) is 0ea5e980, mixed halfway to black (alpha ff)
    let json = mini_set("playlist", "clip", "mix(alpha(accent, 0.5), #000, 0.5)");
    let (theme, _) = parse("x", &json).unwrap();
    assert_eq!(theme.colors["playlist.clip"], "#075375c0");
  }

  #[test]
  fn test_theme_value_can_be_an_expression() {
    let json = mini_set("playlist", "clip", "mix(panel, raised, 0.5)");
    let (theme, _) = parse("x", &json).unwrap();
    // f0 and ff -> f8
    assert_eq!(theme.colors["playlist.clip"], "#f8f8f8");
    // a plain key reference, and a section key reference
    let json = mini_set("meter", "low", "playlist.barLine");
    assert_eq!(parse("x", &json).unwrap().0.colors["meter.low"], "#cccccc");
  }

  #[test]
  fn test_whitespace_variants() {
    let tight = parse("x", &mini_set("playlist", "clip", "mix(panel,raised,0.5)")).unwrap().0;
    for spaced in ["  mix( panel , raised , 0.5 )  ", "mix (panel,raised,0.5)", "\tmix(\npanel, raised,\n0.5)"] {
      let json = mini_set("playlist", "clip", &spaced.replace('\n', "\\n").replace('\t', "\\t"));
      let theme = parse("x", &json).unwrap().0;
      assert_eq!(theme.colors["playlist.clip"], tight.colors["playlist.clip"], "{:?}", spaced);
    }
  }

  #[test]
  fn test_malformed_expressions_name_the_key() {
    for bad in ["mix(a,b)", "mix(panel, raised, x)", "alpha(accent, 2)", "alpha(accent, -0.1)", "mix(panel, raised, 1.5)", "foo(", "foo(a)",
                "mix(panel, raised, 0.5", "panel raised", "", "#12", "mix(panel, raised, 0.5))", "required"] {
      let err = parse("x", &mini_set("playlist", "clip", bad)).unwrap_err();
      assert!(err.contains("playlist.clip"), "{:?}: {}", bad, err);
    }
  }

  #[test]
  fn test_unknown_reference_is_an_error() {
    let err = parse("x", &mini_set("playlist", "clip", "nothere")).unwrap_err();
    assert!(err.contains("playlist.clip") && err.contains("nothere"), "{}", err);
    let err = parse("x", &mini_set("playlist", "clip", "mix(panel, other.thing, 0.5)")).unwrap_err();
    assert!(err.contains("other.thing"), "{}", err);
  }

  #[test]
  fn test_cycle_is_an_error() {
    let json = mini_with("\"surface\": \"panel\", \"panel\": \"surface\",")
      .replacen("\"surface\": \"#ffffff\", \"panel\": \"#f0f0f0\",", "", 1);
    let err = parse("x", &json).unwrap_err();
    assert!(err.contains("loop") && err.contains("surface") && err.contains("panel"), "{}", err);
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
  fn test_readme_matches_base() {
    let readme = include_str!("../themes/README.md");
    let mut in_core = false;
    let mut in_sections = false;
    let mut core = BTreeSet::new();
    let mut sections = BTreeSet::new();
    for line in readme.lines() {
      if line.starts_with("## ") {
        in_core = line.starts_with("## The core keys");
        in_sections = line == "## Sections";
      } else if in_core && line.starts_with("| `") {
        core.insert(line[3..].split('`').next().unwrap().to_string());
      } else if in_sections && line.starts_with("- **`") {
        sections.insert(line[5..].split('`').next().unwrap().to_string());
      }
    }
    // colors is the core table, so it isn't listed under Sections
    sections.insert("colors".to_string());

    let ours: BTreeSet<String> = base().sections["colors"].iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(core, ours);
    assert_eq!(sections, base().sections.keys().cloned().collect::<BTreeSet<_>>());
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
