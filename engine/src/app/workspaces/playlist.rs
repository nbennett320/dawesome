use std::time::{Duration};

// convert a position along the playlist into a start offset
// - position: distance from the playlist's start (e.g. pixels or beats)
// - playlist_length: length of the whole playlist, in the same unit
// - max_playlist_dur: time spanned by the whole playlist
// positions outside the playlist are clamped to its start/end
pub fn calc_sample_offset(
  position: f32,
  playlist_length: f32,
  max_playlist_dur: Duration,
) -> Duration {
  if !(playlist_length > 0.) || !position.is_finite() {
    return Duration::ZERO;
  }

  let ratio = (position / playlist_length).clamp(0., 1.);
  let dur = max_playlist_dur.mul_f64(ratio as f64);

  println!(
    "position: {}, playlist length: {}, ratio: {}, max dur: {}ms, offset: {}ms",
    position, playlist_length, ratio, max_playlist_dur.as_millis(), dur.as_millis());

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
