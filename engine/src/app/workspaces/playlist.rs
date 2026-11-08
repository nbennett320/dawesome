use std::time::{Duration};

// convert a horizontal drop position on the playlist into a start offset
// - drop_x: pixels from the playlist's left edge
// - playlist_width: width of the playlist in pixels
// - max_playlist_dur: time spanned by the full playlist width
// drops outside the playlist are clamped to its start/end
pub fn calc_sample_offset(
  drop_x: f32,
  playlist_width: f32,
  max_playlist_dur: Duration,
) -> Duration {
  if !(playlist_width > 0.) || !drop_x.is_finite() {
    return Duration::ZERO;
  }

  let ratio = (drop_x / playlist_width).clamp(0., 1.);
  let dur = max_playlist_dur.mul_f64(ratio as f64);

  println!(
    "drop_x: {}, playlist width: {}, ratio: {}, max dur: {}ms, offset: {}ms",
    drop_x, playlist_width, ratio, max_playlist_dur.as_millis(), dur.as_millis());

  dur
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_calc_sample_offset() {
    let max = Duration::from_millis(8_000); // 16 beats at 120bpm
    let width = 1_000.;

    assert_eq!(calc_sample_offset(0., width, max).as_millis(), 0);
    assert_eq!(calc_sample_offset(500., width, max).as_millis(), 4_000);
    assert_eq!(calc_sample_offset(250., width, max).as_millis(), 2_000);
    assert_eq!(calc_sample_offset(1_000., width, max).as_millis(), 8_000);
  }

  #[test]
  fn test_calc_sample_offset_clamps_and_rejects_bad_input() {
    let max = Duration::from_millis(8_000);

    // outside the playlist is clamped to its edges
    assert_eq!(calc_sample_offset(-50., 1_000., max), Duration::ZERO);
    assert_eq!(calc_sample_offset(1_500., 1_000., max), max);

    // zero/negative width or non-finite positions don't divide by zero
    assert_eq!(calc_sample_offset(100., 0., max), Duration::ZERO);
    assert_eq!(calc_sample_offset(100., -10., max), Duration::ZERO);
    assert_eq!(calc_sample_offset(f32::NAN, 1_000., max), Duration::ZERO);
  }
}
