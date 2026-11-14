use std::vec::{Vec};

// a sample's waveform: its level at evenly spaced points, about 2 points to
// each millisecond of sound. each point is the loudest sample (of any
// channel) in its stretch of the sound, keeping its sign, from 0 (full scale
// one way) through 0.5 (silence) to 1 (full scale the other way). it isn't
// stretched to fill the clip: a quiet recording's waveform looks quiet
// - samples: interleaved samples for all channels
// - sample_rate: the sample file's own rate
// returns None if the samples are empty or too short to draw
pub fn calc_waveform_from_samples(
  samples: &[i16],
  channels: u16,
  sample_rate: u32,
) -> Option<Vec<f32>> {
  if samples.is_empty() || channels == 0 || sample_rate == 0 {
    return None;
  }

  let dur_ms = samples.len() as f32 / sample_rate as f32 * 1_000. / channels as f32;
  // at least one point, so very short samples don't divide by zero
  let points = (dur_ms.round() as usize * 2).max(1);
  if points >= samples.len() { return None; }

  let stretch = samples.len() / points;
  Some(samples
    .chunks(stretch)
    .take(points)
    .map(|stretch| {
      let peak = stretch.iter().copied().max_by_key(|&s| (s as i32).abs()).unwrap_or(0);
      level(peak)
    })
    .collect())
}

// a sample's level in a waveform: 0.5 is silence, 0 and 1 full scale
fn level(sample: i16) -> f32 {
  (0.5 + sample as f32 / 65_536.).clamp(0., 1.)
}

// a waveform's levels as sent to the frontend: each level (0..1) as a
// little-endian u16 (0..65535), in base64. a fraction of the size of the
// levels as JSON numbers (see decodeLevels in src/util/waveform.ts)
pub fn encode_levels(levels: &[f32]) -> String {
  let bytes: Vec<u8> = levels
    .iter()
    .flat_map(|level| ((level.clamp(0., 1.) * 65_535.).round() as u16).to_le_bytes())
    .collect();

  base64::encode(bytes)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_encode_levels() {
    // 0, 65535, 32768 as little-endian u16s. src/util/waveform.spec.ts
    // decodes the same string
    assert_eq!(encode_levels(&[0., 1., 0.5]), "AAD//wCA");
    // kept within 0..1
    assert_eq!(encode_levels(&[-1., 2.]), encode_levels(&[0., 1.]));
    assert_eq!(encode_levels(&[]), "");
  }

  #[test]
  fn test_levels_are_not_stretched_to_fill_the_clip() {
    // a tenth of full scale stays a tenth, however quiet the sample is
    let quiet: Vec<i16> = (0..48_000).map(|i| if i % 4 < 2 { 3_277 } else { -3_277 }).collect();
    let levels = calc_waveform_from_samples(&quiet, 1, 48_000).unwrap();
    assert!(levels.iter().all(|&level| (level - 0.5).abs() > 0.04 && (level - 0.5).abs() < 0.06), "{:?}", &levels[..4]);
    // full scale reaches the edges
    let loud: Vec<i16> = (0..48_000).map(|i| if i % 48 < 24 { i16::MAX } else { i16::MIN }).collect();
    let levels = calc_waveform_from_samples(&loud, 1, 48_000).unwrap();
    assert!(levels.iter().any(|&level| level > 0.99) && levels.iter().any(|&level| level < 0.01));
  }

  #[test]
  fn test_points_follow_the_sample_rate() {
    // a second of stereo sound gets 2000 points, whatever its rate
    let noise = |n: usize| (0..n).map(|i| ((i * 7919) % 2001) as i16 - 1000).collect::<Vec<_>>();
    assert_eq!(calc_waveform_from_samples(&noise(44_100 * 2), 2, 44_100).unwrap().len(), 2_000);
    assert_eq!(calc_waveform_from_samples(&noise(48_000 * 2), 2, 48_000).unwrap().len(), 2_000);
  }
}
