// a built-in effect's parameters: each described once, as a ParamDesc in a
// static slice. everything else is drawn from that list: the app's controls,
// how the value is shown and typed in, what a project saves, and (later)
// what automation can move
//
// a parameter has three forms
// - plain: what it means (Hz, dB, a choice's index). effects see these,
//   and projects save them, by `key`
// - normalized: 0 to 1, as the app and VST3 pass them around. the taper
//   decides how the range is spread over it (a frequency, logarithmically)
// - text: what the app shows ("2.50 kHz", "+3.0 dB", "Low shelf")
//
// a parameter's id is its place in its effect's list. so the list only
// ever grows at the end: never reorder it or take one out, or saved
// automation (and the app's ids) would point at the wrong parameter.
// projects save by `key`, which mustn't change either

// what a parameter's plain value is in, for showing it
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
  None,
  Hz,
  Db,
  Percent,
  Ms,
  Beats,
}

impl Unit {
  pub fn label(self) -> &'static str {
    match self {
      Unit::None => "",
      Unit::Hz => "Hz",
      Unit::Db => "dB",
      Unit::Percent => "%",
      Unit::Ms => "ms",
      Unit::Beats => "beats",
    }
  }
}

// how a continuous range is spread over 0 to 1
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taper {
  Linear,
  // evenly by ratio (each octave the same width): for frequencies, times
  // and Q. its range must be above 0
  Log,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
  // any value in its range
  Continuous,
  // off (0) or on (1)
  Toggle,
  // one of these, by index (0, 1, ...)
  Choice(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamDesc {
  // what projects save it as, e.g. "band3.freq". never changes
  pub key: &'static str,
  pub name: &'static str,
  // the controls it's shown with ("Band 3"): the generated panel lays
  // groups out together, in the order they first appear
  pub group: &'static str,
  pub unit: Unit,
  pub min: f32,
  pub max: f32,
  pub default: f32,
  pub taper: Taper,
  pub kind: Kind,
  // moving it bends the pitch of what the effect holds, on purpose (a
  // delay's time, gliding like tape). the conformance suite checks it
  // stays clean as it moves, not that it's free of sudden steps: a pitch
  // sweep is all steps. set with .bending_pitch()
  pub bends_pitch: bool,
}

impl ParamDesc {
  pub const fn continuous(
    key: &'static str,
    name: &'static str,
    group: &'static str,
    unit: Unit,
    (min, max): (f32, f32),
    default: f32,
    taper: Taper,
  ) -> ParamDesc {
    ParamDesc { key, name, group, unit, min, max, default, taper, kind: Kind::Continuous, bends_pitch: false }
  }

  pub const fn toggle(key: &'static str, name: &'static str, group: &'static str, on: bool) -> ParamDesc {
    ParamDesc {
      key,
      name,
      group,
      unit: Unit::None,
      min: 0.,
      max: 1.,
      default: if on { 1. } else { 0. },
      taper: Taper::Linear,
      kind: Kind::Toggle,
      bends_pitch: false,
    }
  }

  pub const fn choice(
    key: &'static str,
    name: &'static str,
    group: &'static str,
    labels: &'static [&'static str],
    default: usize,
  ) -> ParamDesc {
    ParamDesc {
      key,
      name,
      group,
      unit: Unit::None,
      min: 0.,
      max: (labels.len() - 1) as f32,
      default: default as f32,
      taper: Taper::Linear,
      kind: Kind::Choice(labels),
      bends_pitch: false,
    }
  }

  // the same, marked as bending pitch when it moves (see bends_pitch)
  pub const fn bending_pitch(self) -> ParamDesc {
    ParamDesc { bends_pitch: true, ..self }
  }

  // whether it glides to a new value (continuous ones) or jumps (switches
  // and choices: there's nothing in between)
  pub fn smoothed(&self) -> bool {
    self.kind == Kind::Continuous
  }

  // as VST3 counts them: 0 for continuous, else the number of steps
  pub fn step_count(&self) -> i32 {
    match self.kind {
      Kind::Continuous => 0,
      Kind::Toggle => 1,
      Kind::Choice(labels) => labels.len() as i32 - 1,
    }
  }

  // a plain value made one it can have: in range, and whole for steps
  pub fn clamp(&self, plain: f32) -> f32 {
    let plain = if plain.is_nan() { self.default } else { plain.clamp(self.min, self.max) };
    match self.kind {
      Kind::Continuous => plain,
      Kind::Toggle | Kind::Choice(_) => plain.round(),
    }
  }

  pub fn normalize(&self, plain: f32) -> f32 {
    let plain = self.clamp(plain);
    if self.max <= self.min { return 0.; }
    let n = match self.taper {
      Taper::Linear => (plain - self.min) / (self.max - self.min),
      Taper::Log => (plain / self.min).ln() / (self.max / self.min).ln(),
    };
    n.clamp(0., 1.)
  }

  pub fn plain(&self, normalized: f32) -> f32 {
    let n = if normalized.is_nan() { self.normalize(self.default) } else { normalized.clamp(0., 1.) };
    let plain = match self.taper {
      Taper::Linear => self.min + n * (self.max - self.min),
      Taper::Log => self.min * (self.max / self.min).powf(n),
    };
    self.clamp(plain)
  }

  pub fn format(&self, plain: f32) -> String {
    let plain = self.clamp(plain);
    match self.kind {
      Kind::Toggle => return if plain >= 0.5 { "On" } else { "Off" }.to_string(),
      Kind::Choice(labels) => return labels[plain as usize].to_string(),
      Kind::Continuous => {}
    }
    match self.unit {
      Unit::Hz if plain >= 1000. => format!("{:.2} kHz", plain / 1000.),
      Unit::Hz if plain >= 100. => format!("{:.0} Hz", plain),
      Unit::Hz => format!("{:.1} Hz", plain),
      // (no "-0.0")
      Unit::Db if plain.abs() < 0.05 => "0.0 dB".to_string(),
      Unit::Db => format!("{:+.1} dB", plain),
      Unit::Percent => format!("{:.0}%", plain),
      Unit::Ms => format!("{:.0} ms", plain),
      Unit::Beats => format!("{:.2} beats", plain),
      Unit::None => format!("{:.2}", plain),
    }
  }

  // what someone typed, as a plain value: "2.5k", "2.5 kHz", "-3", "on",
  // a choice's label. none if it can't be read
  pub fn parse(&self, text: &str) -> Option<f32> {
    let text = text.trim().to_lowercase();
    match self.kind {
      Kind::Toggle => {
        return match text.as_str() {
          "on" | "1" | "yes" | "true" => Some(1.),
          "off" | "0" | "no" | "false" => Some(0.),
          _ => None,
        }
      }
      Kind::Choice(labels) => {
        return labels.iter().position(|l| l.to_lowercase() == text).map(|i| i as f32);
      }
      Kind::Continuous => {}
    }
    let unit = self.unit.label().to_lowercase();
    let mut number = text.as_str();
    let mut scale = 1.;
    if self.unit == Unit::Hz {
      if let Some(n) = number.strip_suffix("khz").or_else(|| number.strip_suffix('k')) {
        number = n;
        scale = 1000.;
      }
    }
    if !unit.is_empty() {
      number = number.strip_suffix(unit.as_str()).unwrap_or(number);
    }
    let value: f32 = number.trim().parse().ok()?;
    if !value.is_finite() { return None; }
    Some(self.clamp(value * scale))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const FREQ: ParamDesc = ParamDesc::continuous("f", "Freq", "", Unit::Hz, (20., 20_000.), 1000., Taper::Log);
  const GAIN: ParamDesc = ParamDesc::continuous("g", "Gain", "", Unit::Db, (-24., 24.), 0., Taper::Linear);
  const SHAPE: ParamDesc = ParamDesc::choice("s", "Shape", "", &["Bell", "Notch", "Low cut"], 0);
  const ON: ParamDesc = ParamDesc::toggle("o", "On", "", false);

  #[test]
  fn test_a_log_range_spreads_octaves_evenly() {
    assert!((FREQ.normalize(20.) - 0.).abs() < 1e-6);
    assert!((FREQ.normalize(20_000.) - 1.).abs() < 1e-6);
    // 632 Hz is the geometric middle of 20 Hz and 20 kHz
    assert!((FREQ.plain(0.5) - 632.46).abs() < 0.1);
  }

  #[test]
  fn test_steps_snap_to_whole_values() {
    assert_eq!(SHAPE.plain(0.4), 1.);
    assert_eq!(SHAPE.plain(0.8), 2.);
    assert_eq!(SHAPE.normalize(1.), 0.5);
    assert_eq!(SHAPE.step_count(), 2);
    assert_eq!(ON.plain(0.7), 1.);
    assert_eq!(ON.step_count(), 1);
  }

  #[test]
  fn test_values_are_shown_in_their_units() {
    assert_eq!(FREQ.format(2500.), "2.50 kHz");
    assert_eq!(FREQ.format(440.), "440 Hz");
    assert_eq!(FREQ.format(50.), "50.0 Hz");
    assert_eq!(GAIN.format(3.), "+3.0 dB");
    assert_eq!(GAIN.format(-0.01), "0.0 dB");
    assert_eq!(SHAPE.format(2.), "Low cut");
    assert_eq!(ON.format(1.), "On");
  }

  #[test]
  fn test_typed_values_are_read_in_their_units() {
    assert_eq!(FREQ.parse("2.5k"), Some(2500.));
    assert_eq!(FREQ.parse("2.5 kHz"), Some(2500.));
    assert_eq!(FREQ.parse("440hz"), Some(440.));
    // out of range is brought in
    assert_eq!(FREQ.parse("5"), Some(20.));
    assert_eq!(GAIN.parse("-3 dB"), Some(-3.));
    assert_eq!(SHAPE.parse("notch"), Some(1.));
    assert_eq!(ON.parse("off"), Some(0.));
    assert_eq!(GAIN.parse("loud"), None);
    assert_eq!(GAIN.parse("NaN"), None);
  }

  #[test]
  fn test_nonsense_values_become_ones_it_can_have() {
    assert_eq!(GAIN.clamp(f32::NAN), 0.);
    assert_eq!(GAIN.plain(f32::NAN), 0.);
    assert_eq!(GAIN.clamp(100.), 24.);
  }
}
