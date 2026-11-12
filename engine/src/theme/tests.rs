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
  assert_eq!(base().by_name.len(), 127);
  for (id, appearance) in expected {
    let (theme, warnings) = parse(id, stock_json(id)).unwrap();
    assert!(warnings.is_empty(), "{}: {:?}", id, warnings);
    assert_eq!(theme.appearance, appearance, "{}", id);
    assert_eq!(theme.colors.len(), 127, "{}", id);
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
    serde_json::from_str(include_str!("../../themes/test/slate-light.resolved.json")).unwrap();
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
  let readme = include_str!("../../themes/README.md");
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
