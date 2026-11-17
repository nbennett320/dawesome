// the network that is the record side of a tape machine: a small gated
// recurrent unit, trained on recordings of the real deck (tools/tape-model)
//
// per sample, per channel, with the drive in db:
//
//   x  = audio * 10^(drive / 20)        what reaches the tape (0 vu is the
//                                       test file's -18 dbfs peak sine)
//   in = [x, drive * drive_scale]
//   r  = sigmoid(Wi_r in + bi_r + Wh_r h + bh_r)
//   z  = sigmoid(Wi_z in + bi_z + Wh_z h + bh_z)
//   n  = tanh(Wi_n in + bi_n + r * (Wh_n h + bh_n))
//   h  = (1 - z) * n + z * h            h starts at zero
//   y  = x + dense . h + dense_b        0 vu at 0 db drive comes back at 1
//
// exactly what model.json's "gates" and "output" say, and what the
// training tools' run_numpy does; the parity test holds them together

use std::fmt;
use std::sync::Arc;

// a trained network's numbers, as plain arrays (row-major)
#[derive(Clone, Debug, PartialEq)]
pub struct GruWeights {
  pub hidden: usize,
  // (3 * hidden) x 2: rows r, z, n; columns audio, drive
  pub w_ih: Vec<f32>,
  // (3 * hidden) x hidden
  pub w_hh: Vec<f32>,
  pub b_ih: Vec<f32>,
  pub b_hh: Vec<f32>,
  // hidden
  pub dense_w: Vec<f32>,
  pub dense_b: f32,
  // the drive input is drive_db times this
  pub drive_scale: f32,
  // what it was trained at; the caller resamples to it
  pub rate: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GruError {
  // which array, how long it is, and how long it should be
  Shape(&'static str, usize, usize),
  NoHidden,
}

impl fmt::Display for GruError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      GruError::Shape(name, got, want) => write!(f, "{name} has {got} numbers, not {want}"),
      GruError::NoHidden => write!(f, "the network has no hidden units"),
    }
  }
}

impl std::error::Error for GruError {}

impl GruWeights {
  pub fn check(&self) -> Result<(), GruError> {
    let h = self.hidden;
    if h == 0 {
      return Err(GruError::NoHidden);
    }
    for (name, len, want) in [
      ("w_ih", self.w_ih.len(), 3 * h * 2),
      ("w_hh", self.w_hh.len(), 3 * h * h),
      ("b_ih", self.b_ih.len(), 3 * h),
      ("b_hh", self.b_hh.len(), 3 * h),
      ("dense_w", self.dense_w.len(), h),
    ] {
      if len != want {
        return Err(GruError::Shape(name, len, want));
      }
    }
    Ok(())
  }
}

// one channel's network: the shared weights, and this channel's state
pub struct Gru {
  weights: Arc<GruWeights>,
  // w_hh by columns (hidden x 3 hidden): each hidden unit's weights into
  // all three gates sit together, so the multiply is independent
  // multiply-adds the compiler can run side by side (by rows it's a chain
  // of dependent adds, about 3x slower)
  w_hh_columns: Vec<f32>,
  h: Vec<f32>,
  // the hidden state's three gates, and the input's, worked out each
  // sample
  gh: Vec<f32>,
  gi: Vec<f32>,
  // the drive's gain, worked out only when the drive changes
  drive_db: f32,
  drive_gain: f32,
}

impl Gru {
  pub fn new(weights: Arc<GruWeights>) -> Result<Gru, GruError> {
    weights.check()?;
    let h = weights.hidden;
    let mut columns = vec![0.0; 3 * h * h];
    for row in 0..3 * h {
      for col in 0..h {
        columns[col * 3 * h + row] = weights.w_hh[row * h + col];
      }
    }
    Ok(Gru {
      weights,
      w_hh_columns: columns,
      h: vec![0.0; h],
      gh: vec![0.0; 3 * h],
      gi: vec![0.0; 3 * h],
      drive_db: 0.0,
      drive_gain: 1.0,
    })
  }

  pub fn weights(&self) -> &Arc<GruWeights> {
    &self.weights
  }

  // back to silence: the state starts at zero, as in training
  pub fn reset(&mut self) {
    self.h.fill(0.0);
  }

  // one sample
  #[inline]
  pub fn step(&mut self, audio: f32, drive_db: f32) -> f32 {
    if drive_db != self.drive_db {
      self.drive_db = drive_db;
      self.drive_gain = 10f32.powf(drive_db / 20.0);
    }
    let w = &*self.weights;
    let n = w.hidden;
    let x = audio * self.drive_gain;
    let d = drive_db * w.drive_scale;

    // the hidden state through w_hh, all three gates at once, a column
    // (one hidden unit's weights) at a time
    self.gh.copy_from_slice(&w.b_hh);
    for (col, &h) in self.h.iter().enumerate() {
      let column = &self.w_hh_columns[col * 3 * n..(col + 1) * 3 * n];
      for (g, &a) in self.gh.iter_mut().zip(column) {
        *g += a * h;
      }
    }

    // the input's share of the gates
    for (row, g) in self.gi.iter_mut().enumerate() {
      *g = w.w_ih[row * 2] * x + w.w_ih[row * 2 + 1] * d + w.b_ih[row];
    }

    // the gates, a whole array at a time (each loop runs side by side)
    let (gi_rz, gi_n) = self.gi.split_at_mut(2 * n);
    let (gh_rz, gh_n) = self.gh.split_at(2 * n);
    for (g, &h) in gi_rz.iter_mut().zip(gh_rz) {
      *g = sigmoid(*g + h);
    }
    let (r, z) = gi_rz.split_at(n);
    for ((c, &r), &h) in gi_n.iter_mut().zip(r).zip(gh_n) {
      *c = tanh(*c + r * h);
    }
    let mut y = w.dense_b;
    for (((h, &z), &c), &dw) in self.h.iter_mut().zip(z).zip(gi_n.iter()).zip(&w.dense_w) {
      *h = (1.0 - z) * c + z * *h;
      y += dw * *h;
    }
    x + y
  }

  // a block, at one drive
  pub fn process(&mut self, audio: &[f32], drive_db: f32, out: &mut [f32]) {
    for (a, o) in audio.iter().zip(out.iter_mut()) {
      *o = self.step(*a, drive_db);
    }
  }
}

#[inline]
fn sigmoid(v: f32) -> f32 {
  1.0 / (1.0 + exp(-v))
}

#[inline]
fn tanh(v: f32) -> f32 {
  // 1 - 2 / (e^2v + 1), clamped where it's 1 to a float's precision
  let v = v.clamp(-10.0, 10.0);
  1.0 - 2.0 / (exp(2.0 * v) + 1.0)
}

// e^v without branches or a library call, so loops of it run side by
// side: 2^k e^r with |r| <= ln2/2, e^r from its series to r^6. within
// about 2e-7 of the real thing (exp_is_close below), so the network still
// matches training within 1e-5
#[inline]
fn exp(v: f32) -> f32 {
  let v = v.clamp(-87.0, 87.0);
  let k = (v * std::f32::consts::LOG2_E).round();
  // ln 2 in two parts (cody-waite), so k ln 2 comes off exactly: in one
  // part it loses ~4e-6 at the far ends
  let r = (v - k * 0.693_145_75) - k * 1.428_606_8e-6;
  let p = 1.0 + r * (1.0 + r * (0.5 + r * (1.0 / 6.0 + r * (1.0 / 24.0 + r * (1.0 / 120.0 + r * (1.0 / 720.0))))));
  p * f32::from_bits(((k as i32 + 127) as u32) << 23)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn tiny(hidden: usize) -> GruWeights {
    GruWeights {
      hidden,
      w_ih: vec![0.1; 6 * hidden],
      w_hh: vec![0.05; 3 * hidden * hidden],
      b_ih: vec![0.0; 3 * hidden],
      b_hh: vec![0.0; 3 * hidden],
      dense_w: vec![0.2; hidden],
      dense_b: 0.0,
      drive_scale: 1.0 / 18.0,
      rate: 48_000,
    }
  }

  #[test]
  fn exp_is_close() {
    let mut worst = 0.0f32;
    let mut v = -80.0f32;
    while v < 80.0 {
      worst = worst.max((exp(v) / v.exp() - 1.0).abs());
      v += 0.0137;
    }
    assert!(worst < 5e-7, "relative error {worst}");
    let mut worst = 0.0f32;
    let mut v = -12.0f32;
    while v < 12.0 {
      worst = worst.max((tanh(v) - v.tanh()).abs());
      v += 0.0011;
    }
    assert!(worst < 5e-7, "tanh error {worst}");
  }

  #[test]
  fn wrong_shapes_are_refused() {
    let mut w = tiny(4);
    w.w_hh.pop();
    assert_eq!(Gru::new(Arc::new(w)).err(), Some(GruError::Shape("w_hh", 47, 48)));
    assert_eq!(Gru::new(Arc::new(tiny(0))).err(), Some(GruError::NoHidden));
  }

  #[test]
  fn reset_starts_again_from_silence() {
    let mut g = Gru::new(Arc::new(tiny(4))).unwrap();
    let first: Vec<f32> = (0..50).map(|i| g.step((i as f32 * 0.3).sin() * 0.5, 0.0)).collect();
    g.reset();
    let again: Vec<f32> = (0..50).map(|i| g.step((i as f32 * 0.3).sin() * 0.5, 0.0)).collect();
    assert_eq!(first, again);
  }

  #[test]
  fn channels_sharing_weights_keep_their_own_state() {
    let w = Arc::new(tiny(4));
    let (mut a, mut b) = (Gru::new(w.clone()).unwrap(), Gru::new(w).unwrap());
    for i in 0..20 {
      a.step(i as f32 * 0.01, 0.0);
    }
    b.step(0.0, 0.0);
    assert_ne!(a.h, b.h);
  }
}
