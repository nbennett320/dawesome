// scrubbing (part S in context/tape-project/02-plan.md): the song played at
// the speed the playhead's dragged, either way, as a tape rocked over the
// heads. normal playback can't run backwards or jump (plugins stream
// forward), so a scrub reads a proxy instead: a cheap render of the whole
// song kept in memory (daw/scrub/proxy.rs), with crates/tape's Scrubber
use std::sync::Arc;

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

// a scrub on the audio thread: the proxy it reads, and the tape over it.
// made off the audio thread, and freed there too (through the garbage)
pub struct ScrubSource {
  proxy: Arc<Proxy>,
  scrubber: ::tape::Scrubber,
}

impl ScrubSource {
  pub fn new(proxy: Arc<Proxy>, out_rate: u32, at: f64, drive: ::tape::Drive) -> ScrubSource {
    let scrubber = ::tape::Scrubber::new(proxy.rate as f64, out_rate as f64, at, drive);
    ScrubSource { proxy, scrubber }
  }

  // the drag's place, or the wind's speed, whichever it follows
  pub fn drive(&mut self, value: f64) {
    let drive = match self.scrubber.drive() {
      ::tape::Drive::Follow(_) => ::tape::Drive::Follow(value),
      ::tape::Drive::Wind(_) => ::tape::Drive::Wind(value),
    };
    self.scrubber.set_drive(drive);
  }

  pub fn add_to(&mut self, out: &mut [f32]) {
    self.scrubber.add_to(&self.proxy.samples, out);
  }

  pub fn at(&self) -> f64 {
    self.scrubber.at()
  }

  pub fn end(&mut self) {
    self.scrubber.end();
  }

  pub fn done(&self) -> bool {
    self.scrubber.done()
  }
}
