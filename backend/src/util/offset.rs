pub fn calc_playlist_sample_offset(
  drop_x: f32,
  drop_y: f32,
  min_bound_x: f32,
  min_bound_y: f32,
  max_bound_x: f32,
  max_bound_y: f32,
  max_sample_offset: u64,
) -> u64 {
  let ratio: f32;
  let res: u64;

  if drop_x <= min_bound_x {
    res = 0;
  } else {
    let drop_x0 = drop_x - min_bound_x;
    let drop_y0 = drop_y - min_bound_y;
    let (min_bound_x0, min_bound_y0) = (0., 0.);
    let max_bound_x0 = max_bound_x - min_bound_x;
    let max_bound_y0 = max_bound_y - min_bound_y;
    ratio = (drop_x0 / max_bound_x0) as f32;
    res = (((max_sample_offset as f32) * ratio) as f32).round() as u64;
  }

  res
}
