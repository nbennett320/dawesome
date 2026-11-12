use super::*;
use rodio::buffer::SamplesBuffer;

#[test]
fn test_soft_limit() {
  // quiet and moderate levels pass unchanged
  assert_eq!(soft_limit(0.5), 0.5);
  assert_eq!(soft_limit(-0.9), -0.9);
  // louder ones are rounded off below 1, keeping their sign and order
  assert!(soft_limit(1.2) < 1. && soft_limit(1.2) > 0.9);
  assert!(soft_limit(4.) > soft_limit(1.2));
  assert!(soft_limit(100.) <= 1.);
  assert_eq!(soft_limit(-1.2), -soft_limit(1.2));
  // smooth at the knee
  assert!((soft_limit(0.9001) - 0.9001).abs() < 1e-4);
}

// `frames` frames of 1.0 at 1kHz
fn ones(channels: u16, frames: usize) -> SamplesBuffer<f32> {
  SamplesBuffer::new(channels, 1000, vec![1.; frames * channels as usize])
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 1e-3
}

#[test]
fn test_db_to_gain() {
  assert_eq!(db_to_gain(0.), 1.);
  assert!(close(db_to_gain(-20.), 0.1));
  assert!(close(db_to_gain(6.), 1.995));
  // silent at the bottom, and never louder than the top
  assert_eq!(db_to_gain(MIN_DB), 0.);
  assert_eq!(db_to_gain(f32::NAN), 0.);
  assert_eq!(db_to_gain(40.), db_to_gain(MAX_DB));
}

#[test]
fn test_atomic_f32() {
  let peak = AtomicF32::new(0.);
  peak.fetch_max(0.5);
  peak.fetch_max(0.25);
  peak.fetch_max(-3.);
  assert_eq!(peak.load(), 0.5);
  assert_eq!(peak.take(), 0.5);
  assert_eq!(peak.load(), 0.);
}

#[test]
fn test_unchanged_at_the_defaults() {
  let mixer = Mixer::default();
  let source = SamplesBuffer::new(2, 1000, vec![0.1, 0.9, -0.2, 0.8]);
  let out: Vec<f32> = mixer.master_strip(mixer.track_strip(0, source)).collect();

  assert_eq!(out, vec![0.1, 0.9, -0.2, 0.8]);
}

#[test]
fn test_volume_pan_and_mute_start_straight_away() {
  let mixer = Mixer::default();
  mixer.set_volume_db(0, -20.);
  mixer.set_pan(1, 0.5);
  mixer.set_mute(2, true);

  let first = |track: u32| -> Vec<f32> { mixer.track_strip(track, ones(2, 4)).take(2).collect() };
  let quiet = first(0);
  assert!(close(quiet[0], 0.1) && close(quiet[1], 0.1));
  assert_eq!(first(1), vec![0.5, 1.]);
  assert_eq!(first(2), vec![0., 0.]);

  // mono only gets the volume
  let mono: Vec<f32> = mixer.track_strip(1, ones(1, 2)).collect();
  assert_eq!(mono, vec![1., 1.]);
}

#[test]
fn test_solo_silences_the_other_tracks() {
  let mixer = Mixer::default();
  mixer.set_solo(1, true);
  // soloing twice still counts once
  mixer.set_solo(1, true);

  let level = |track: u32| -> f32 { mixer.track_strip(track, ones(2, 1)).next().unwrap() };
  assert_eq!(level(0), 0.);
  assert_eq!(level(1), 1.);

  mixer.set_solo(1, false);
  assert_eq!(level(0), 1.);
}

#[test]
fn test_changes_are_heard_while_playing_without_a_click() {
  let mixer = Mixer::default();
  // 1s of audio at 1kHz
  let mut strip = mixer.track_strip(0, ones(2, 1000));

  let start: Vec<f32> = strip.by_ref().take(200).collect();
  assert!(start.iter().all(|&s| s == 1.));

  mixer.set_mute(0, true);
  let rest: Vec<f32> = strip.collect();

  // it fades out over a few samples rather than jumping to silence
  let lefts: Vec<f32> = rest.iter().step_by(2).copied().collect();
  let first_change = lefts.iter().position(|&s| s < 1.).unwrap();
  assert!(first_change <= CONTROL_FRAMES as usize);
  assert!(lefts[first_change] > 0.5);
  assert!(lefts.windows(2).all(|w| w[1] <= w[0]));
  assert!(*lefts.last().unwrap() < 1e-3);
}

#[test]
fn test_master_volume() {
  let mixer = Mixer::default();
  mixer.set_master_volume_db(-20.);
  let out: Vec<f32> = mixer.master_strip(ones(2, 1)).collect();
  assert!(close(out[0], 0.1));
}

#[test]
fn test_meters_record_the_loudest_sample() {
  let mixer = Mixer::default();
  mixer.set_volume_db(0, -20.);
  let source = SamplesBuffer::new(1, 1000, vec![0.5, -1., 0.25]);
  let _: Vec<f32> = mixer.master_strip(mixer.track_strip(0, source)).collect();

  let levels = mixer.take_levels();
  assert_eq!(levels.tracks.len(), 1);
  assert!(close(levels.tracks[0].1, 0.1));
  assert!(close(levels.master, 0.1));
  // reading the meters starts them again
  assert_eq!(mixer.take_levels().master, 0.);
}

#[test]
fn test_settings_and_apply() {
  let mixer = Mixer::default();
  mixer.set_volume_db(1, -6.);
  mixer.set_solo(1, true);
  mixer.set_master_volume_db(3.);

  let settings = mixer.settings(3);
  assert_eq!(settings.tracks.len(), 3);
  assert_eq!(settings.tracks[1].settings, ChannelSettings { volume_db: -6., pan: 0., mute: false, solo: true });

  // a project with only track 2 changed: track 1 goes back to its defaults
  let other = Mixer::default();
  other.set_pan(2, -1.);
  mixer.apply(&other.settings(0));
  let applied = mixer.settings(3);
  assert_eq!(applied.tracks[1].settings, ChannelSettings::default());
  assert_eq!(applied.tracks[2].settings.pan, -1.);
  assert_eq!(applied.master.volume_db, 0.);
  assert_eq!(mixer.solo_count.load(Ordering::Relaxed), 0);
}

#[test]
fn test_track_names() {
  let mixer = Mixer::default();
  assert_eq!(mixer.set_name(5, "  Drums  "), Some("Drums".into()));
  assert_eq!(mixer.set_name(1, &"x".repeat(100)).unwrap().len(), MAX_NAME_CHARS);

  // a named track is listed even before it's used
  let settings = mixer.settings(2);
  assert_eq!(settings.tracks.len(), 3);
  assert_eq!(settings.tracks[2].name.as_deref(), Some("Drums"));

  // resetting the mix keeps names, a project replaces them
  mixer.reset();
  assert_eq!(mixer.settings(0).tracks[2].name.as_deref(), Some("Drums"));
  mixer.apply(&MixerSettings::default());
  assert!(mixer.settings(0).tracks.iter().all(|t| t.name.is_none()));

  mixer.set_name(5, "Bass");
  assert_eq!(mixer.set_name(5, "   "), None);
  assert!(mixer.settings(0).tracks.iter().all(|t| t.name.is_none()));
}

#[test]
fn test_values_kept_in_range() {
  let mixer = Mixer::default();
  assert_eq!(mixer.set_volume_db(0, 40.), MAX_DB);
  assert_eq!(mixer.set_volume_db(0, -500.), MIN_DB);
  assert_eq!(mixer.set_volume_db(0, f32::NAN), 0.);
  assert_eq!(mixer.set_pan(0, 3.), 1.);
}

#[test]
fn test_serde_names_match_the_frontend() {
  let settings = MixerSettings {
    tracks: vec![
      TrackMix { track_number: 0, name: None, settings: ChannelSettings::default() },
      TrackMix { track_number: 1, name: Some("Drums".into()), settings: ChannelSettings::default() },
    ],
    master: ChannelSettings::default(),
  };
  assert_eq!(serde_json::to_value(&settings).unwrap(), serde_json::json!({
    "tracks": [
      { "trackNumber": 0, "volumeDb": 0., "pan": 0., "mute": false, "solo": false },
      { "trackNumber": 1, "name": "Drums", "volumeDb": 0., "pan": 0., "mute": false, "solo": false },
    ],
    "master": { "volumeDb": 0., "pan": 0., "mute": false, "solo": false },
  }));
}
