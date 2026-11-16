// what an effect is told about the moment it's playing (the engine's
// plugins get the same: plugin::Transport is this)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transport {
  // whether a run is playing (chains are processed while stopped too)
  pub playing: bool,
  // beats a minute
  pub tempo: f64,
  // the time signature
  pub beats_per_bar: u16,
  pub beat_unit: u16,
  // where the block starts in the song, in frames. while stopped it stays
  // where the last run got to
  pub position: i64,
  pub sample_rate: u32,
}

impl Default for Transport {
  fn default() -> Self {
    Transport {
      playing: false,
      tempo: 120.,
      beats_per_bar: 4,
      beat_unit: 4,
      position: 0,
      sample_rate: 44_100,
    }
  }
}
