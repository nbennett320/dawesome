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
  let adjusted_max_x = width;
  let adjusted_drop_x = drop_x - min_bound_x;
  println!("drop_x: {}, min_bound_x: {}, max_bound_x: {}, adjusted_drop_x: {}", drop_x, min_bound_x, max_bound_x, adjusted_drop_x);

  let ratio = adjusted_drop_x / max_bound_x;
  println!("offset ratio: {}, max dur: {}ms, ", ratio, max_playlist_dur.as_millis());
  let max_dur_us = max_playlist_dur.as_micros() as f64;
  let drop_offset_us = max_dur_us * ratio as f64;
  
  let dur = Duration::from_micros(drop_offset_us.round() as u64);

  println!("drop_offsetoffset dur: {}ms", dur.as_millis());

  dur
}

