use std::time::{Duration};

pub fn calc_sample_offset(
  drop_x: f32,
  drop_y: f32,
  min_bound_x: f32,
  min_bound_y: f32,
  max_bound_x: f32,
  max_bound_y: f32,
  max_playlist_dur: Duration,
) -> Duration {
  let width = max_bound_x - min_bound_x;
  // let adjusted_min_x = 0. as f32;
  // let adjusted_max_x = width;
  let adjusted_drop_x = drop_x - min_bound_x;

  let ratio = adjusted_drop_x / width;
  let max_dur_us = max_playlist_dur.as_micros() as f32;
  let drop_offset_us = max_dur_us * ratio / 10.;
  
  Duration::from_micros(drop_offset_us.round() as u64)
}

