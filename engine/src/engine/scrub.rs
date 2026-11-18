// scrubbing (part S in context/tape-project/02-plan.md): the song played at
// the speed the playhead's dragged, either way, as a tape rocked over the
// heads. normal playback can't run backwards or jump (plugins stream
// forward), so a scrub reads a proxy instead: a cheap render of the whole
// song kept in memory (daw/scrub/proxy.rs)

// the scrub proxy: the song's clips and pattern samples through each
// track's fader, pan, mute and solo, summed to mono, from the song's start
#[derive(Debug, PartialEq)]
pub struct Proxy {
  pub rate: u32,
  pub samples: Box<[i16]>,
}

impl Proxy {
  pub fn bytes(&self) -> usize {
    self.samples.len() * std::mem::size_of::<i16>()
  }

  pub fn seconds(&self) -> f64 {
    self.samples.len() as f64 / self.rate.max(1) as f64
  }
}
