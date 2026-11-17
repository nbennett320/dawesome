// the tape on the audio thread: one tape machine (crates/tape), and the
// tracks on it. a stereo track takes two of the machine's tracks, its left
// and right sides, like two of a 4-track's tracks. each track on tape is
// either
//
// - live (always-live mode, T23): recorded and played every block, or
// - printed (print mode, the default): its record side was worked out
//   ahead, in the background (daw/tape/print.rs), and during a run its
//   print plays through the playback side, read a head's delay ahead so it
//   lands on time (W4). without a print yet, or with the transport stopped,
//   the track is heard direct, as a 4-track's input is before it's on tape
//   (W2, D64). between the two it crossfades
//
// it's built whole off the audio thread (adding a track allocates) and
// sent over; knobs, drives and prints after that are small messages, so
// turning a knob doesn't restart the tape's wobble. with no tape, the
// engine runs exactly as before (T14)
use std::sync::Arc;

use tape::{MachineSettings, TapeMachine};

use super::processor;

// how long a printed track takes to cross between its print and direct
const CROSSFADE_SECONDS: f32 = 0.05;

// a track's print: what its record side put on the tape, in song time at
// the engine's rate. only the stretches with sound are kept (a minute of
// stereo is 23 MB): anywhere else is silence
pub struct Print {
  pub rate: u32,
  // in song order, not overlapping
  pub regions: Vec<Region>,
}

pub struct Region {
  // the song frame its first sample is at
  pub start: i64,
  pub sides: [Vec<f32>; 2],
}

impl Print {
  // one side of the print from song frame `from`, into `out` (silence
  // where there's none)
  pub fn read(&self, side: usize, from: i64, out: &mut [f32]) {
    out.fill(0.0);
    let to = from + out.len() as i64;
    // (few regions: the first that could overlap, by binary search)
    let first = self.regions.partition_point(|r| r.start + r.sides[side].len() as i64 <= from);
    for region in &self.regions[first..] {
      if region.start >= to {
        break;
      }
      let samples = &region.sides[side];
      let a = from.max(region.start);
      let b = to.min(region.start + samples.len() as i64);
      if a < b {
        out[(a - from) as usize..(b - from) as usize].copy_from_slice(&samples[(a - region.start) as usize..(b - region.start) as usize]);
      }
    }
  }

  // how much memory it holds, in bytes
  pub fn bytes(&self) -> usize {
    self.regions.iter().map(|r| (r.sides[0].len() + r.sides[1].len()) * 4).sum()
  }
}

struct OnTape {
  number: u32,
  // its left and right tracks on the machine
  sides: [usize; 2],
  // recorded live every block (always-live mode)
  live: bool,
  // printed: its print, if it's in
  print: Option<Arc<Print>>,
  // the print is out of date: heard until the track has crossed back to
  // direct, then let go
  letting_go: bool,
  // how much of the track is its print (0: direct, 1: the print)
  mix: f32,
  // the song frame the next block should start at, if the tape is primed
  // for it: anything else is a seek or a loop, and it's primed again
  next: Option<i64>,
}

pub struct EngineTape {
  pub machine: TapeMachine,
  // where on a track the tape sits: before its first effect (true), as on
  // a Portastudio, or after its whole chain
  pub before_effects: bool,
  tracks: Vec<OnTape>,
  rate: u32,
  // how far ahead a print is read: the playback side's delay, in frames
  ahead: usize,
  // how much `mix` moves a frame
  step: f32,
  // what comes off the tape this block, and the print it's read from
  scratch: [Vec<f32>; 2],
  print_scratch: Vec<f32>,
  // prints let go of, to be sent off the audio thread to be freed
  pub dropped: Vec<Arc<Print>>,
}

// what a track on tape does this block
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Use {
  // recorded and played (always-live mode)
  Live([usize; 2]),
  // its print plays, crossed with what's direct
  Printed(usize),
}

impl EngineTape {
  // `tracks`: (track number, its left and right tracks on the machine,
  // whether it's recorded live)
  pub fn new(machine: TapeMachine, before_effects: bool, rate: u32, tracks: Vec<(u32, [usize; 2], bool)>) -> Box<EngineTape> {
    let ahead = machine.latency().round().max(0.0) as usize;
    let tracks = tracks.into_iter().map(|(number, sides, live)| OnTape { number, sides, live, print: None, letting_go: false, mix: 0.0, next: None }).collect::<Vec<_>>();
    let room = tracks.len() + 4;
    Box::new(EngineTape {
      machine,
      before_effects,
      tracks,
      rate,
      ahead,
      step: 1.0 / (CROSSFADE_SECONDS * rate.max(1) as f32),
      scratch: [vec![0.0; processor::MAX_BLOCK], vec![0.0; processor::MAX_BLOCK]],
      print_scratch: vec![0.0; processor::MAX_BLOCK + ahead],
      dropped: Vec::with_capacity(room),
    })
  }

  // the tape's knobs (no allocation)
  pub fn set_settings(&mut self, settings: MachineSettings) {
    self.machine.set_settings(settings);
  }

  // a track's drive (no allocation)
  pub fn set_drive(&mut self, track_number: u32, drive_db: f32) {
    if let Some(t) = self.tracks.iter().find(|t| t.number == track_number) {
      self.machine.set_drive(t.sides[0], drive_db);
      self.machine.set_drive(t.sides[1], drive_db);
    }
  }

  // a printed track's print, or None when it's out of date: the old one
  // is let go (to be freed off the audio thread) once the track has
  // crossed back to direct. one at another rate is no print at all
  pub fn set_print(&mut self, track_number: u32, print: Option<Arc<Print>>) {
    let rate = self.rate;
    let found = self.tracks.iter().position(|t| t.number == track_number && !t.live);
    let print = match (found, print) {
      (Some(i), Some(p)) if p.rate == rate => Some((i, p)),
      (_, Some(p)) => {
        self.drop_print(p);
        None
      }
      _ => None,
    };
    match (found, print) {
      (_, Some((i, new))) => {
        let t = &mut self.tracks[i];
        let old = t.print.replace(new);
        t.letting_go = false;
        t.next = None;
        if let Some(old) = old { self.drop_print(old); }
      }
      // kept, heard, until the crossfade to direct is done (see run)
      (Some(i), None) => {
        let t = &mut self.tracks[i];
        t.letting_go = t.print.is_some();
      }
      (None, None) => {}
    }
  }

  // a print is let go of: kept to be freed off the audio thread (the
  // renderer empties `dropped` every block, so there's always room)
  fn drop_print(&mut self, print: Arc<Print>) {
    if self.dropped.len() < self.dropped.capacity() {
      self.dropped.push(print);
    }
  }

  // what a track does this block, if it's on tape and the tape has
  // anything to do with it. a printed track with no print, or with the
  // transport stopped, and done crossing back, is left alone: direct
  pub fn uses(&self, track_number: u32, playing: bool) -> Option<Use> {
    let i = self.tracks.iter().position(|t| t.number == track_number)?;
    let t = &self.tracks[i];
    if t.live {
      return Some(Use::Live(t.sides));
    }
    let wanted = playing && t.print.is_some() && !t.letting_go;
    (wanted || t.mix > 0.0).then_some(Use::Printed(i))
  }

  // a track's stereo block onto the tape and off it again (live), or its
  // print crossed in (printed), in place. `advance` must have moved the
  // tape on for this block first. `position`: the song frame the block
  // starts at; `playing`: whether a run is
  pub fn run(&mut self, what: Use, position: i64, playing: bool, left: &mut [f32], right: &mut [f32]) {
    match what {
      Use::Live(sides) => {
        for (side, buffer) in [left, right].into_iter().enumerate() {
          let frames = buffer.len();
          let out = &mut self.scratch[side][..frames];
          self.machine.record_and_play(sides[side], buffer, out);
          buffer.copy_from_slice(out);
        }
      }
      Use::Printed(i) => self.run_printed(i, position, playing, left, right),
    }
  }

  fn run_printed(&mut self, i: usize, position: i64, playing: bool, left: &mut [f32], right: &mut [f32]) {
    let frames = left.len();
    let ahead = self.ahead;
    let t = &mut self.tracks[i];
    let target = if playing && t.print.is_some() && !t.letting_go { 1.0 } else { 0.0 };
    let Some(print) = t.print.clone() else {
      t.mix = 0.0;
      return;
    };

    // a new place on the print (a run starting, a seek, a loop): start the
    // playback side again from there, primed with what's just ahead
    if playing && t.next != Some(position) {
      for side in 0..2 {
        let primer = &mut self.print_scratch[..ahead];
        print.read(side, position, primer);
        self.machine.prime(t.sides[side], primer);
      }
    }
    t.next = playing.then_some(position + frames as i64);

    // the print, from a head's delay ahead, through the playback side
    for side in 0..2 {
      let read = &mut self.print_scratch[..frames];
      if playing {
        print.read(side, position + ahead as i64, read);
      } else {
        read.fill(0.0);
      }
      let out = &mut self.scratch[side][..frames];
      self.machine.play(t.sides[side], read, out);
    }

    // crossed with what's direct
    let mut mix = t.mix;
    for k in 0..frames {
      mix = if target > mix { (mix + self.step).min(target) } else { (mix - self.step).max(target) };
      left[k] = left[k] * (1.0 - mix) + self.scratch[0][k] * mix;
      right[k] = right[k] * (1.0 - mix) + self.scratch[1][k] * mix;
    }
    t.mix = mix;

    // back to direct after being let go: the print goes
    if t.letting_go && mix == 0.0 {
      t.letting_go = false;
      t.next = None;
      if let Some(old) = t.print.take() { self.drop_print(old); }
    }
  }
}
