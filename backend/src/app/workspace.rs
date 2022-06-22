#[derive(Clone, Copy)]
pub struct WorkspaceViewport {
  pub width: Option<f32>,
  pub height: Option<f32>,
  pub min_bound_x: Option<f32>,
  pub min_bound_y: Option<f32>,
  pub max_bound_x: Option<f32>,
  pub max_bound_y: Option<f32>,
}

impl WorkspaceViewport {
  pub fn new() -> Self {
    WorkspaceViewport {
      width: None, 
      height: None,
      min_bound_x: None,
      min_bound_y: None,
      max_bound_x: None,
      max_bound_y: None,
    }
  }

  pub fn set_bounding_box(
    &mut self,
    min_bound_x: f32,
    min_bound_y: f32,
    max_bound_x: f32,
    max_bound_y: f32,
  ) {
    let width = max_bound_x -  min_bound_x;
    let height = max_bound_y -  min_bound_y;

    self.width = Some(width);
    self.height = Some(height);
    self.min_bound_x = Some(min_bound_x);
    self.min_bound_y = Some(min_bound_y);
    self.max_bound_x = Some(max_bound_x);
    self.max_bound_y = Some(max_bound_y);
  }
}
