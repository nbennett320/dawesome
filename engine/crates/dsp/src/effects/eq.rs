// the EQ: up to 8 bands, each a bell, shelf, cut, notch or band-pass (one
// SVF each, svf.rs), then an output gain. stereo-linked: both sides get the
// same filters
//
// a band starts off. switching one on or off fades it in or out over FADE,
// so it doesn't click

use std::f32::consts::PI;

use crate::effect::{Effect, EffectInfo, Entry, Io, Values};
use crate::param::{ParamDesc, Taper, Unit};
use crate::svf::{Coefs, Shape, State};
use crate::transport::Transport;

pub const BANDS: usize = 8;
// each band's parameters, in this order, from band * PER_BAND
pub const PER_BAND: usize = 5;
pub const ON: usize = 0;
pub const SHAPE: usize = 1;
pub const FREQ: usize = 2;
pub const GAIN: usize = 3;
pub const Q: usize = 4;
// after the bands
pub const OUTPUT: usize = BANDS * PER_BAND;

// as the shape parameter lists them
pub const SHAPES: [Shape; 7] =
  [Shape::Bell, Shape::LowShelf, Shape::HighShelf, Shape::LowCut, Shape::HighCut, Shape::Notch, Shape::BandPass];
const SHAPE_NAMES: &[&str] = &["Bell", "Low shelf", "High shelf", "Low cut", "High cut", "Notch", "Band-pass"];

// how long a band takes to fade in or out
const FADE: f32 = 0.010;

// band n's parameters (from 1), starting at `freq` Hz
macro_rules! band {
  ($n:literal, $freq:literal) => {
    [
      ParamDesc::toggle(concat!("band", $n, ".on"), "On", concat!("Band ", $n), false),
      ParamDesc::choice(concat!("band", $n, ".shape"), "Shape", concat!("Band ", $n), SHAPE_NAMES, 0),
      ParamDesc::continuous(concat!("band", $n, ".freq"), "Freq", concat!("Band ", $n), Unit::Hz, (20., 20_000.), $freq, Taper::Log),
      ParamDesc::continuous(concat!("band", $n, ".gain"), "Gain", concat!("Band ", $n), Unit::Db, (-24., 24.), 0., Taper::Linear),
      ParamDesc::continuous(concat!("band", $n, ".q"), "Q", concat!("Band ", $n), Unit::None, (0.1, 18.), 0.707, Taper::Log),
    ]
  };
}

const B: [[ParamDesc; PER_BAND]; BANDS] = [
  band!(1, 50.),
  band!(2, 120.),
  band!(3, 300.),
  band!(4, 700.),
  band!(5, 1500.),
  band!(6, 3500.),
  band!(7, 8000.),
  band!(8, 14000.),
];

// (a flat list: ids are places in it. only ever add to the end)
pub static PARAMS: [ParamDesc; BANDS * PER_BAND + 1] = [
  B[0][0], B[0][1], B[0][2], B[0][3], B[0][4],
  B[1][0], B[1][1], B[1][2], B[1][3], B[1][4],
  B[2][0], B[2][1], B[2][2], B[2][3], B[2][4],
  B[3][0], B[3][1], B[3][2], B[3][3], B[3][4],
  B[4][0], B[4][1], B[4][2], B[4][3], B[4][4],
  B[5][0], B[5][1], B[5][2], B[5][3], B[5][4],
  B[6][0], B[6][1], B[6][2], B[6][3], B[6][4],
  B[7][0], B[7][1], B[7][2], B[7][3], B[7][4],
  ParamDesc::continuous("output", "Output", "Output", Unit::Db, (-24., 24.), 0., Taper::Linear),
];

pub static ENTRY: Entry = Entry {
  info: EffectInfo { id: "eq", name: "EQ", category: "EQ", version: 1 },
  params: &PARAMS,
  make,
};

fn make(rate: f32) -> Box<dyn Effect> {
  Box::new(Eq::new(rate))
}

// band `band`'s parameter `which` (ON, SHAPE, ...)
pub fn id(band: usize, which: usize) -> usize {
  band * PER_BAND + which
}

// a band's settings, as its filter is made from them
#[derive(Clone, Copy, PartialEq)]
struct Setting {
  shape: Shape,
  freq: f32,
  gain: f32,
  q: f32,
}

impl Setting {
  fn of(params: &Values, band: usize) -> Setting {
    Setting {
      shape: SHAPES[params.choice(id(band, SHAPE)).min(SHAPES.len() - 1)],
      freq: params.get(id(band, FREQ)),
      gain: params.get(id(band, GAIN)),
      q: params.get(id(band, Q)),
    }
  }

  // how long it takes to fall by a factor of e, in seconds, after its input
  // stops (a generous guess from its poles: Q over pi times the frequency
  // for a resonant one, the slow pole of an overdamped one)
  fn tau(&self) -> f32 {
    let q = match self.shape {
      // a bell rings like a filter of its Q times its gain
      Shape::Bell => self.q * 10f32.powf(self.gain / 40.),
      _ => self.q,
    };
    // (shelves move their poles by up to the square root of their gain: half
    // the frequency covers the most they can)
    q.max(1. / (2. * q)) / (PI * self.freq * 0.5)
  }

  // what it can boost by, in dB
  fn boost(&self) -> f32 {
    match self.shape {
      Shape::Bell | Shape::LowShelf | Shape::HighShelf => self.gain.max(0.),
      _ => 0.,
    }
  }
}

#[derive(Clone, Copy)]
struct Band {
  // what its filter is made from now (none until it's first heard)
  setting: Option<Setting>,
  coefs: Coefs,
  sides: [State; 2],
  // how far it's faded in: 0 is off, 1 on
  mix: f32,
}

impl Band {
  fn new() -> Band {
    Band {
      setting: None,
      coefs: Coefs::new(Shape::Bell, 1000., 0., 1., 48_000.),
      sides: [State::default(); 2],
      mix: 0.,
    }
  }
}

pub struct Eq {
  rate: f32,
  bands: [Band; BANDS],
  // it's run since it was made or reset: bands fade when they're switched
  // (before, each starts as it's set, with nothing to fade from)
  started: bool,
}

impl Eq {
  pub fn new(rate: f32) -> Eq {
    Eq { rate, bands: [Band::new(); BANDS], started: false }
  }
}

impl Effect for Eq {
  fn set_sample_rate(&mut self, rate: f32) {
    self.rate = rate;
    for band in &mut self.bands { band.setting = None; }
  }

  fn reset(&mut self) {
    for band in &mut self.bands {
      band.sides = [State::default(); 2];
    }
    self.started = false;
  }

  fn process(&mut self, io: Io, params: &Values, _: &Transport) {
    let Io { input, output, .. } = io;
    let [out_l, out_r] = output;
    out_l.copy_from_slice(input[0]);
    out_r.copy_from_slice(input[1]);
    let fade_step = 1. / (FADE * self.rate);
    let starting = !std::mem::replace(&mut self.started, true);

    for (i, band) in self.bands.iter_mut().enumerate() {
      let on = params.on(id(i, ON));
      if !on && band.mix == 0. { continue; }
      let setting = Setting::of(params, i);
      if band.setting != Some(setting) {
        band.coefs = Coefs::new(setting.shape, setting.freq, setting.gain, setting.q, self.rate);
        band.setting = Some(setting);
      }
      let target = if on { 1. } else { 0. };
      if starting { band.mix = target; }
      let mut mix = band.mix;
      for (side, samples) in [&mut *out_l, &mut *out_r].into_iter().enumerate() {
        let state = &mut band.sides[side];
        mix = band.mix;
        if mix == target {
          for x in samples.iter_mut() { *x = state.run(&band.coefs, *x); }
        } else {
          for x in samples.iter_mut() {
            let y = state.run(&band.coefs, *x);
            *x += mix * (y - *x);
            mix = if target > mix { (mix + fade_step).min(1.) } else { (mix - fade_step).max(0.) };
          }
        }
        state.flush();
      }
      band.mix = mix;
      // faded out: it starts afresh when it's next switched on
      if mix == 0. { band.sides = [State::default(); 2]; }
    }

    let output_gain = 10f32.powf(params.get(OUTPUT) / 20.);
    if output_gain != 1. {
      for x in out_l.iter_mut().chain(out_r.iter_mut()) { *x *= output_gain; }
    }
  }

  fn tail_frames(&self, params: &Values, rate: f32) -> Option<u64> {
    // bands in a row ring one after another, so their times add. each must
    // fall by a millionth (about 14 times tau), and by whatever all the
    // boosts (and the output) lift it by on top
    // (in one pass: this runs on the audio thread, which mustn't allocate)
    let (mut taus, mut longest, mut boost) = (0f32, 0f32, params.get(OUTPUT).max(0.));
    for i in (0..BANDS).filter(|&i| params.on(id(i, ON)) || self.bands[i].mix > 0.) {
      let setting = Setting::of(params, i);
      let tau = setting.tau();
      taus += tau;
      longest = longest.max(tau);
      boost += setting.boost();
    }
    // (dB to factors of e)
    let seconds = 14. * taus + longest * boost / 8.686 + FADE;
    Some((seconds * rate).ceil() as u64)
  }
}

// the EQ's gain, as an amplitude, at `freq` Hz with these values (every band
// that's on, and the output): what the app's curve draws
pub fn response(params: &Values, freq: f32, rate: f32) -> f32 {
  let bands: f32 = (0..BANDS)
    .filter(|&i| params.on(id(i, ON)))
    .map(|i| {
      let s = Setting::of(params, i);
      Coefs::new(s.shape, s.freq, s.gain, s.q, rate).response(freq, rate)
    })
    .product();
  bands * 10f32.powf(params.get(OUTPUT) / 20.)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::runner::{Runner, Store};

  const RATE: f32 = 48_000.;

  // a sine at `freq` through the EQ: its level in dB once it's settled
  fn heard(store: &std::sync::Arc<Store>, freq: f32) -> f32 {
    let mut runner = Runner::new(store.clone(), RATE);
    let transport = Transport { sample_rate: RATE as u32, ..Transport::default() };
    let frames = RATE as usize;
    let input: Vec<f32> = (0..frames).map(|i| (2. * PI * freq * i as f32 / RATE).sin()).collect();
    let (mut l, mut r) = (vec![0.; frames], vec![0.; frames]);
    for at in (0..frames).step_by(512) {
      let end = (at + 512).min(frames);
      runner.process([&input[at..end], &input[at..end]], [&mut l[at..end], &mut r[at..end]], &transport);
    }
    // its RMS once settled, as a sine's amplitude
    let settled = &l[frames / 2..];
    let rms = (2. * settled.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / settled.len() as f64).sqrt();
    20. * rms.log10() as f32
  }

  #[test]
  fn test_it_starts_flat() {
    let store = Store::new(&ENTRY);
    for freq in [50., 1000., 10_000.] {
      assert!(heard(&store, freq).abs() < 0.01);
    }
  }

  #[test]
  fn test_bands_that_are_on_shape_the_sound_and_off_ones_dont() {
    let store = Store::new(&ENTRY);
    store.set_plain(id(2, ON), 1.);
    store.set_plain(id(2, FREQ), 1000.);
    store.set_plain(id(2, GAIN), 6.);
    // band 5 is set too, but left off
    store.set_plain(id(5, FREQ), 1000.);
    store.set_plain(id(5, GAIN), 12.);
    assert!((heard(&store, 1000.) - 6.).abs() < 0.1);
    assert!(heard(&store, 100.).abs() < 0.2);
  }

  #[test]
  fn test_the_output_gain_applies_to_everything() {
    let store = Store::new(&ENTRY);
    store.set_plain(OUTPUT, -6.);
    assert!((heard(&store, 440.) + 6.).abs() < 0.01);
  }

  #[test]
  fn test_its_response_is_what_it_does() {
    let store = Store::new(&ENTRY);
    for (band, shape, freq, gain) in [(0, 3, 80., 0.), (1, 0, 400., -8.), (4, 0, 2500., 5.), (7, 2, 9000., 4.)] {
      store.set_plain(id(band, ON), 1.);
      store.set_plain(id(band, SHAPE), shape as f32);
      store.set_plain(id(band, FREQ), freq);
      store.set_plain(id(band, GAIN), gain);
    }
    let plains = store.plains();
    for freq in [60., 200., 400., 1000., 2500., 6000., 12_000.] {
      let said = 20. * response(&Values(&plains), freq, RATE).log10();
      let was = heard(&store, freq);
      assert!((was - said).abs() < 0.1, "at {} Hz: heard {} dB, said {} dB", freq, was, said);
    }
  }
}
