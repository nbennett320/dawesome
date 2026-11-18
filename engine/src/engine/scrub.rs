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

// the warble's seed: any, the same each time
const WARBLE_SEED: u64 = 0x5c2b_0b;

// how many frames of the warble are worked out at a time
const WARBLE_BLOCK: usize = 256;

// a scrub on the audio thread: the proxy it reads, the tape over it, and
// (with the tape on) the tape's transport, whose wow and flutter waver the
// scrub as they do playback (S4: about 8 ns a frame). made off the audio
// thread, and freed off it too (through the garbage)
pub struct ScrubSource {
  proxy: Arc<Proxy>,
  scrubber: ::tape::Scrubber,
  warble: Option<Box<::tape::TapeTransport>>,
  speeds: [f32; WARBLE_BLOCK],
}

impl ScrubSource {
  // - warble: the tape's transport settings and its Wow & Flutter amount,
  //   with the tape on
  pub fn new(proxy: Arc<Proxy>, out_rate: u32, at: f64, drive: ::tape::Drive, warble: Option<(::tape::TransportSettings, f32)>) -> ScrubSource {
    let scrubber = ::tape::Scrubber::new(proxy.rate as f64, out_rate as f64, at, drive);
    let warble = warble.map(|(settings, amount)| {
      let mut transport = ::tape::TapeTransport::new(settings, out_rate as f32, WARBLE_SEED);
      transport.set_amount(amount);
      Box::new(transport)
    });
    ScrubSource { proxy, scrubber, warble, speeds: [1.0; WARBLE_BLOCK] }
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
    match self.warble.as_mut() {
      None => self.scrubber.add_to(&self.proxy.samples, out),
      Some(transport) => {
        for block in out.chunks_mut(WARBLE_BLOCK * 2) {
          let speeds = &mut self.speeds[..block.len() / 2];
          transport.fill_speed(speeds);
          self.scrubber.add_warbled(&self.proxy.samples, block, speeds);
        }
      }
    }
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
