pub trait UI {
  fn new() -> Self;
  fn vp_width(&self) -> Option<f32>;
  fn vp_height(&self) -> Option<f32>;
}
