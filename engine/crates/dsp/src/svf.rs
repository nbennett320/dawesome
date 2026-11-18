// a state-variable filter: one filter that makes every EQ shape (bell,
// shelves, cuts, notch, band-pass) from the same two integrators, mixed
// differently. this is Andrew Simper's (Cytomic) trapezoidal SVF
// ("SvfLinearTrapOptimised2"): it stays stable and quiet when its frequency
// and Q move every sample, which knobs and automation will do. 12 dB/oct
// for the cuts
//
// Coefs is what the shape, frequency, gain and Q work out to; State is one
// channel's memory. response() gives a shape's gain at any frequency, from
// the same coefficients, for drawing the curve (and the tests)

use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
  Bell,
  LowShelf,
  HighShelf,
  LowCut,
  HighCut,
  Notch,
  BandPass,
}

// the highest frequency a filter is set to, as a share of the sample rate
// (just under half: the maths needs it below)
pub const MAX_SHARE: f32 = 0.49;
// what's left in a filter's memory below this is cleared (it would only
// decay into denormals, which are slow on some CPUs)
pub(crate) const TINY: f32 = 1e-20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coefs {
  g: f32,
  k: f32,
  a1: f32,
  a2: f32,
  a3: f32,
  // how much of the input, the band and the low pass make the output
  m0: f32,
  m1: f32,
  m2: f32,
}

impl Coefs {
  // a filter of `shape` at `freq` Hz, `gain` dB (bell and shelves only) and
  // `q`, at `rate` Hz
  pub fn new(shape: Shape, freq: f32, gain: f32, q: f32, rate: f32) -> Coefs {
    let freq = freq.clamp(1., rate * MAX_SHARE);
    let q = q.max(0.01);
    let g = (PI * freq / rate).tan();
    // the bell's and shelves' gain as an amplitude, square-rooted
    let a = 10f32.powf(gain / 40.);
    let (g, k, m0, m1, m2) = match shape {
      Shape::Bell => {
        let k = 1. / (q * a);
        (g, k, 1., k * (a * a - 1.), 0.)
      }
      Shape::LowShelf => {
        let k = 1. / q;
        (g / a.sqrt(), k, 1., k * (a - 1.), a * a - 1.)
      }
      Shape::HighShelf => {
        let k = 1. / q;
        (g * a.sqrt(), k, a * a, k * (1. - a) * a, 1. - a * a)
      }
      Shape::LowCut => {
        let k = 1. / q;
        (g, k, 1., -k, -1.)
      }
      Shape::HighCut => (g, 1. / q, 0., 0., 1.),
      Shape::Notch => {
        let k = 1. / q;
        (g, k, 1., -k, 0.)
      }
      // 0 dB at its peak
      Shape::BandPass => {
        let k = 1. / q;
        (g, k, 0., k, 0.)
      }
    };
    let a1 = 1. / (1. + g * (g + k));
    let a2 = g * a1;
    let a3 = g * a2;
    Coefs { g, k, a1, a2, a3, m0, m1, m2 }
  }

  // just the filter's poles, for `State::outputs` (which gives every pass
  // at once): `g` is tan(pi f / rate), worked out once and shared by
  // filters at the same frequency
  pub fn poles(g: f32, q: f32) -> Coefs {
    let k = 1. / q.max(0.01);
    let a1 = 1. / (1. + g * (g + k));
    let a2 = g * a1;
    let a3 = g * a2;
    Coefs { g, k, a1, a2, a3, m0: 0., m1: 0., m2: 1. }
  }

  // the gain (as an amplitude) these make at `freq` Hz, at `rate` Hz: the
  // analog filter, warped as the trapezoidal one is
  pub fn response(&self, freq: f32, rate: f32) -> f32 {
    let w = (PI * freq.min(rate * 0.4999) / rate).tan() / self.g;
    // s = jw: low pass 1 / (s² + ks + 1), band s / (s² + ks + 1)
    let (dr, di) = (1. - w * w, self.k * w);
    let d = dr * dr + di * di;
    // low = conj(den) / |den|², band = jw · conj(den) / |den|²
    let (lr, li) = (dr / d, -di / d);
    let (br, bi) = (w * di / d, w * dr / d);
    let re = self.m0 + self.m1 * br + self.m2 * lr;
    let im = self.m1 * bi + self.m2 * li;
    (re * re + im * im).sqrt()
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
  ic1: f32,
  ic2: f32,
}

impl State {
  #[inline]
  pub fn run(&mut self, c: &Coefs, x: f32) -> f32 {
    let v3 = x - self.ic2;
    let v1 = c.a1 * self.ic1 + c.a2 * v3;
    let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
    self.ic1 = 2. * v1 - self.ic1;
    self.ic2 = 2. * v2 - self.ic2;
    c.m0 * x + c.m1 * v1 + c.m2 * v2
  }

  // the low, high and band pass of `x` at once (12 dB/oct each, the band
  // 0 dB at its peak). the memory moves the same whichever is listened to,
  // so a filter can glide from one to another without starting anything
  // cold
  #[inline]
  pub fn outputs(&mut self, c: &Coefs, x: f32) -> [f32; 3] {
    let v3 = x - self.ic2;
    let v1 = c.a1 * self.ic1 + c.a2 * v3;
    let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
    self.ic1 = 2. * v1 - self.ic1;
    self.ic2 = 2. * v2 - self.ic2;
    [v2, x - c.k * v1 - v2, c.k * v1]
  }

  pub fn reset(&mut self) {
    *self = State::default();
  }

  // clear what's too small to hear (once a block)
  pub fn flush(&mut self) {
    if self.ic1.abs() < TINY { self.ic1 = 0.; }
    if self.ic2.abs() < TINY { self.ic2 = 0.; }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const RATE: f32 = 48_000.;

  // the gain a filter gives a sine at `freq`, measured
  fn measured(c: &Coefs, freq: f32) -> f32 {
    let mut s = State::default();
    let frames = (RATE * 2.) as usize;
    // its RMS after it's settled, as a sine's amplitude (a sampled peak
    // misses high frequencies' tops)
    let mut sum = 0f64;
    for i in 0..frames {
      let y = s.run(c, (2. * PI * freq * i as f32 / RATE).sin());
      if i >= frames / 2 { sum += (y as f64).powi(2); }
    }
    (2. * sum / (frames / 2) as f64).sqrt() as f32
  }

  fn db(amplitude: f32) -> f32 {
    20. * amplitude.log10()
  }

  #[test]
  fn test_each_shape_does_what_it_says_at_its_frequency() {
    let at = |shape, gain| db(Coefs::new(shape, 1000., gain, 0.707, RATE).response(1000., RATE));
    assert!((at(Shape::Bell, 6.) - 6.).abs() < 0.01);
    assert!((at(Shape::Bell, -12.) + 12.).abs() < 0.01);
    // shelves are half their gain at their frequency
    assert!((at(Shape::LowShelf, 6.) - 3.).abs() < 0.05);
    assert!((at(Shape::HighShelf, 6.) - 3.).abs() < 0.05);
    // a Butterworth cut is 3 dB down at its frequency
    assert!((at(Shape::LowCut, 0.) + 3.).abs() < 0.05);
    assert!((at(Shape::HighCut, 0.) + 3.).abs() < 0.05);
    assert!(at(Shape::Notch, 0.) < -60.);
    assert!(at(Shape::BandPass, 0.).abs() < 0.01);
  }

  #[test]
  fn test_its_outputs_are_its_cuts_and_band_pass() {
    let g = (PI * 1000. / RATE).tan();
    let poles = Coefs::poles(g, 2.);
    let shapes = [Shape::HighCut, Shape::LowCut, Shape::BandPass];
    let mut all = State::default();
    let mut each = [State::default(); 3];
    for i in 0..2000 {
      let x = (i as f32 * 0.37).sin() + if i == 10 { 1. } else { 0. };
      let outs = all.outputs(&poles, x);
      for (n, shape) in shapes.iter().enumerate() {
        let one = each[n].run(&Coefs::new(*shape, 1000., 0., 2., RATE), x);
        assert!((outs[n] - one).abs() < 1e-4, "{:?} at {}: {} vs {}", shape, i, outs[n], one);
      }
    }
  }

  #[test]
  fn test_shelves_and_cuts_reach_their_gain_away_from_their_frequency() {
    let r = |shape, freq| db(Coefs::new(shape, 1000., 6., 0.707, RATE).response(freq, RATE));
    assert!((r(Shape::LowShelf, 20.) - 6.).abs() < 0.1);
    assert!(r(Shape::LowShelf, 15_000.).abs() < 0.1);
    assert!((r(Shape::HighShelf, 15_000.) - 6.).abs() < 0.1);
    // 12 dB an octave: 40 dB two decades down... near enough a few octaves out
    assert!(r(Shape::LowCut, 125.) < -34.);
    assert!(r(Shape::LowCut, 10_000.).abs() < 0.1);
    assert!(r(Shape::HighCut, 8000.) < -34.);
  }

  #[test]
  fn test_what_it_does_to_sound_matches_its_response() {
    let shapes = [Shape::Bell, Shape::LowShelf, Shape::HighShelf, Shape::LowCut, Shape::HighCut, Shape::Notch, Shape::BandPass];
    for shape in shapes {
      let c = Coefs::new(shape, 2000., 9., 2., RATE);
      for freq in [100., 1000., 2000., 3000., 12_000.] {
        let (heard, said) = (db(measured(&c, freq)), db(c.response(freq, RATE)));
        // (a notch's response at its own frequency is too deep to measure)
        if said < -40. { assert!(heard < -30.); continue; }
        // (within a tenth of a dB, or half a dB once it's far down)
        let near = if said < -20. { 0.5 } else { 0.1 };
        assert!((heard - said).abs() < near, "{:?} at {} Hz: heard {} dB, said {} dB", shape, freq, heard, said);
      }
    }
  }

  // ---- the golden fixture the app's curve is checked against
  //
  // the app draws the EQ's curve itself (src/views/native-effect/eq/), with
  // the same maths in TypeScript. fixtures/eq-response.json pins what these
  // filters give, and the app's tests check their copy against it. if this
  // fails because the filters changed on purpose, write the file again with
  // UPDATE_FIXTURES=1 cargo test -p dawesome-dsp, and check the app's tests

  // (shape as eq.rs's list numbers it, freq, gain, q)
  const CASES: [(usize, f32, f32, f32); 14] = [
    (0, 1000., 6., 0.707), (0, 120., -9., 4.), (0, 8000., 12., 0.3),
    (1, 200., 6., 0.707), (1, 80., -12., 1.5),
    (2, 6000., 4., 0.707), (2, 12_000., -8., 2.),
    (3, 60., 0., 0.707), (3, 300., 0., 3.),
    (4, 10_000., 0., 0.707), (4, 2000., 0., 0.5),
    (5, 50., 0., 8.), (6, 1500., 0., 2.), (6, 19_000., 0., 0.7),
  ];
  const FREQS: [f32; 16] = [
    20., 31.5, 50., 80., 125., 200., 315., 500., 800., 1250., 2000., 3150., 5000., 8000., 12_500., 20_000.,
  ];

  fn fixture() -> String {
    let shapes = crate::effects::eq::SHAPES;
    let mut out = format!("{{\n  \"rate\": {},\n  \"freqs\": [{}],\n  \"cases\": [\n", RATE,
      FREQS.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(", "));
    for (i, &(shape, freq, gain, q)) in CASES.iter().enumerate() {
      let c = Coefs::new(shapes[shape], freq, gain, q, RATE);
      let db: Vec<String> = FREQS.iter().map(|&f| format!("{:.4}", db(c.response(f, RATE)).max(-200.))).collect();
      out += &format!(
        "    {{ \"shape\": {}, \"freq\": {}, \"gain\": {}, \"q\": {}, \"db\": [{}] }}{}\n",
        shape, freq, gain, q, db.join(", "), if i + 1 < CASES.len() { "," } else { "" }
      );
    }
    out + "  ]\n}\n"
  }

  // every number in a text, in order
  fn numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
      .filter(|t| !t.is_empty() && *t != "-")
      .map(|t| t.parse().unwrap())
      .collect()
  }

  #[test]
  fn test_the_apps_curve_fixture_is_what_the_filters_give() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/eq-response.json");
    let now = fixture();
    if std::env::var("UPDATE_FIXTURES").is_ok() {
      std::fs::write(&path, &now).unwrap();
    }
    let saved = std::fs::read_to_string(&path).expect("fixtures/eq-response.json (UPDATE_FIXTURES=1 writes it)");
    let (a, b) = (numbers(&now), numbers(&saved));
    assert_eq!(a.len(), b.len(), "the fixture has a different shape: write it again");
    for (x, y) in a.iter().zip(&b) {
      assert!((x - y).abs() < 1e-3, "the filters give {} where the fixture says {}", x, y);
    }
  }
}
