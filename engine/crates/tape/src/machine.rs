// a tape machine: one tape (its transport, so one wobble for everything on
// it) and the tracks recorded on it. as on a real deck, each track has two
// halves:
//
//   record side:   input -> dbx encode -> the core (saturation, squash, eq)
//                  -> what's on the tape (the print)
//   playback side: the print, read as the tape wobbles past the head ->
//                  dropouts -> hiss -> dbx decode -> output
//
// the record side is a RecordSide of its own, so the DAW can run it once
// in the background (print mode, T23) and keep only the playback side
// live, or run both every block (always-live mode). either way the same
// numbers come out: printing then playing is exactly the same as running
// live (the test `printing_then_playing_is_live`)
//
// the machine's settings are what the Transport tab sets (T22); a track's
// drive is its own knob. nothing here allocates once a track is added

use crate::core::TapeCore;
use crate::dbx::DbxEncoder;
use crate::playback::{Playback, PlaybackSettings};
use crate::transport::{TapeTransport, TransportSettings};

// what the machine is set to: the profile's measured values (transport,
// playback) and the Transport tab's knobs on top
#[derive(Clone, Debug, PartialEq)]
pub struct MachineSettings {
  // the tape's motion, as measured (TransportSettings::from_measured)
  pub transport: TransportSettings,
  // hiss, dropouts and the head's base delay, as measured
  pub playback: PlaybackSettings,
  // the Wow & Flutter knob: 0 steady, 1 as measured, 2 twice
  pub wow_flutter: f32,
  // the Hiss knob: 0 none, 1 as measured, 2 twice (+6 db)
  pub hiss: f32,
  // the dbx switch
  pub dbx: bool,
  // the Output knob, after the playback side
  pub output_db: f32,
}

impl Default for MachineSettings {
  fn default() -> Self {
    MachineSettings {
      transport: TransportSettings::default(),
      playback: PlaybackSettings::default(),
      wow_flutter: 1.0,
      hiss: 1.0,
      dbx: true,
      output_db: 0.0,
    }
  }
}

impl MachineSettings {
  // the playback settings with the Hiss knob applied
  fn playback_for(&self, channel_delay: f32) -> PlaybackSettings {
    let mut p = self.playback;
    p.hiss_db = if self.hiss > 0.0 { p.hiss_db + 20.0 * self.hiss.log10() } else { -200.0 };
    p.channel_delay = channel_delay;
    p
  }
}

// ---- the record side

// one track's way onto the tape: dbx encode (when it's on) and the core.
// runs live inside a machine, or alone in a print job
pub struct RecordSide {
  encoder: DbxEncoder,
  core: Box<dyn TapeCore>,
  encoded: Vec<f32>,
}

impl RecordSide {
  // `max_block`: longer calls are split
  pub fn new(core: Box<dyn TapeCore>, rate: f32, max_block: usize) -> RecordSide {
    RecordSide { encoder: DbxEncoder::new(rate), core, encoded: vec![0.0; max_block.max(1)] }
  }

  pub fn reset(&mut self) {
    self.encoder.reset();
    self.core.reset();
  }

  // input to what's on the tape, at a drive (db), with or without dbx
  pub fn process(&mut self, input: &[f32], drive_db: f32, dbx: bool, out: &mut [f32]) {
    let size = self.encoded.len();
    for (inp, o) in input.chunks(size).zip(out.chunks_mut(size)) {
      if dbx {
        let encoded = &mut self.encoded[..inp.len()];
        self.encoder.process(inp, encoded);
        self.core.process(encoded, drive_db, o);
      } else {
        self.core.process(inp, drive_db, o);
      }
    }
  }
}

// ---- the machine

struct Track {
  playback: Playback,
  // present in always-live mode (and while a track waits for its print)
  record: Option<RecordSide>,
  drive_db: f32,
  // the head's timing offset against the first track (azimuth), frames
  channel_delay: f32,
  // what's on the tape this block, when recording live
  printed: Vec<f32>,
}

pub struct TapeMachine {
  settings: MachineSettings,
  rate: f32,
  seed: u64,
  max_block: usize,
  transport: TapeTransport,
  // the tape's speed for the current block, the same for every track
  speed: Vec<f32>,
  frames: usize,
  output_gain: f32,
  tracks: Vec<Track>,
}

impl TapeMachine {
  pub fn new(settings: MachineSettings, rate: f32, seed: u64, max_block: usize) -> TapeMachine {
    let max_block = max_block.max(1);
    let mut transport = TapeTransport::new(settings.transport, rate, seed);
    transport.set_amount(settings.wow_flutter);
    TapeMachine {
      output_gain: 10f32.powf(settings.output_db / 20.0),
      settings,
      rate,
      seed,
      max_block,
      transport,
      speed: vec![1.0; max_block],
      frames: 0,
      tracks: Vec::new(),
    }
  }

  // a track onto the tape: its playback side, and its record side if it
  // runs live (None: its print comes from elsewhere). returns its index.
  // allocates: not on the audio thread
  pub fn add_track(&mut self, record: Option<RecordSide>, channel_delay: f32) -> usize {
    let index = self.tracks.len();
    let seed = self.seed.wrapping_add(1 + index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut playback = Playback::new(self.settings.playback_for(channel_delay), self.rate, seed, self.max_block);
    playback.set_mean_speed(self.transport.mean_speed());
    self.tracks.push(Track { playback, record, drive_db: 0.0, channel_delay, printed: vec![0.0; self.max_block] });
    index
  }

  pub fn tracks(&self) -> usize {
    self.tracks.len()
  }

  pub fn settings(&self) -> &MachineSettings {
    &self.settings
  }

  // the Transport tab's knobs, and the profile behind them. changing the
  // transport's or playback's shapes is slow (tens of microseconds, D54):
  // do that off the audio thread; the knobs alone are quick
  pub fn set_settings(&mut self, settings: MachineSettings) {
    if settings.transport != self.settings.transport {
      self.transport.set_settings(settings.transport);
    }
    self.transport.set_amount(settings.wow_flutter);
    let mean = self.transport.mean_speed();
    for t in self.tracks.iter_mut() {
      t.playback.set_settings(settings.playback_for(t.channel_delay));
      t.playback.set_mean_speed(mean);
    }
    self.output_gain = 10f32.powf(settings.output_db / 20.0);
    self.settings = settings;
  }

  // a track's Drive knob
  pub fn set_drive(&mut self, track: usize, drive_db: f32) {
    self.tracks[track].drive_db = drive_db;
  }

  // swap a track between live (Some) and printed (None). the record side
  // allocated wherever it was made
  pub fn set_record(&mut self, track: usize, record: Option<RecordSide>) -> Option<RecordSide> {
    std::mem::replace(&mut self.tracks[track].record, record)
  }

  // how late the playback side puts the sound, in frames (the head's base
  // delay): print mode stores prints this much early to hide it (O7)
  pub fn latency(&self) -> f64 {
    self.tracks.first().map_or(self.settings.playback.base_delay_ms as f64 * self.rate as f64 / 1000.0, |t| t.playback.latency())
  }

  // the tape's motor: playback stopping and starting with inertia (part E
  // will use this for tape stop)
  pub fn stop(&mut self) {
    self.transport.stop();
  }

  pub fn start(&mut self) {
    self.transport.start();
  }

  pub fn reset(&mut self) {
    self.transport.reset();
    for t in self.tracks.iter_mut() {
      t.playback.reset();
      if let Some(r) = t.record.as_mut() {
        r.reset();
      }
    }
  }

  // move the tape on by `frames` (at most max_block): the speed every
  // track reads with until the next call
  pub fn advance(&mut self, frames: usize) {
    let frames = frames.min(self.max_block);
    self.transport.fill_speed(&mut self.speed[..frames]);
    self.frames = frames;
  }

  // a printed track's playback for the block `advance` set up: the print
  // (as the record side made it) in, the track's output out
  pub fn play(&mut self, track: usize, printed: &[f32], out: &mut [f32]) {
    let n = self.frames.min(printed.len()).min(out.len());
    let t = &mut self.tracks[track];
    t.playback.process(&printed[..n], &self.speed[..n], self.settings.dbx, &mut out[..n]);
    for o in out[..n].iter_mut() {
      *o *= self.output_gain;
    }
  }

  // a live track for the block `advance` set up: recorded and played at
  // once. a track with no record side plays silence (its print isn't in)
  pub fn record_and_play(&mut self, track: usize, input: &[f32], out: &mut [f32]) {
    let n = self.frames.min(input.len()).min(out.len());
    let dbx = self.settings.dbx;
    let t = &mut self.tracks[track];
    let printed = &mut t.printed[..n];
    match t.record.as_mut() {
      Some(r) => r.process(&input[..n], t.drive_db, dbx, printed),
      None => printed.fill(0.0),
    }
    t.playback.process(printed, &self.speed[..n], dbx, &mut out[..n]);
    for o in out[..n].iter_mut() {
      *o *= self.output_gain;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::{SimpleCore, SimpleCoreSettings};

  const RATE: f32 = 48_000.0;
  const BLOCK: usize = 256;

  fn quiet() -> MachineSettings {
    let mut s = MachineSettings { transport: TransportSettings::still(), wow_flutter: 0.0, hiss: 0.0, dbx: false, ..Default::default() };
    s.playback.dropouts_per_minute = 0.0;
    s
  }

  fn core() -> Box<dyn TapeCore> {
    Box::new(SimpleCore::new(SimpleCoreSettings::default(), RATE, BLOCK))
  }

  fn signal(frames: usize) -> Vec<f32> {
    (0..frames).map(|i| ((i as f32 * 0.031).sin() * 0.6 + (i as f32 * 0.0071).sin() * 0.3) * 0.125).collect()
  }

  // a whole signal through a machine, block by block
  fn run_live(machine: &mut TapeMachine, track: usize, input: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0; input.len()];
    for (inp, o) in input.chunks(BLOCK).zip(out.chunks_mut(BLOCK)) {
      machine.advance(inp.len());
      machine.record_and_play(track, inp, o);
    }
    out
  }

  #[test]
  fn printing_then_playing_is_live() {
    // the same settings and seed, one machine live, one from a print made
    // up front: print mode and always-live mode sound the same
    let settings = MachineSettings { wow_flutter: 1.0, hiss: 1.0, dbx: true, ..Default::default() };
    let input = signal(48_000);

    let mut live = TapeMachine::new(settings.clone(), RATE, 7, BLOCK);
    let t = live.add_track(Some(RecordSide::new(core(), RATE, BLOCK)), 0.0);
    live.set_drive(t, 6.0);
    let want = run_live(&mut live, t, &input);

    let mut record = RecordSide::new(core(), RATE, BLOCK);
    let mut printed = vec![0.0; input.len()];
    record.process(&input, 6.0, true, &mut printed);
    let mut machine = TapeMachine::new(settings, RATE, 7, BLOCK);
    let t = machine.add_track(None, 0.0);
    let mut got = vec![0.0; input.len()];
    for (p, o) in printed.chunks(BLOCK).zip(got.chunks_mut(BLOCK)) {
      machine.advance(p.len());
      machine.play(t, p, o);
    }
    assert_eq!(got, want);
  }

  #[test]
  fn still_quiet_tape_is_the_core_delayed() {
    let mut machine = TapeMachine::new(quiet(), RATE, 1, BLOCK);
    let t = machine.add_track(Some(RecordSide::new(core(), RATE, BLOCK)), 0.0);
    let input = signal(24_000);
    let got = run_live(&mut machine, t, &input);
    let mut want = vec![0.0; input.len()];
    let mut c = core();
    c.process(&input, 0.0, &mut want);
    let delay = machine.latency().round() as usize;
    assert!((machine.latency() - delay as f64).abs() < 1e-9, "a whole number of frames");
    let worst = got[delay + 1000..].iter().zip(&want[1000..]).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
    assert!(worst < 1e-3, "off by {worst}");
  }

  #[test]
  fn every_track_wobbles_together() {
    // two tracks, the same input, no hiss or dropouts: one tape, so the
    // same wobble and the same output
    let mut settings = quiet();
    settings.transport = TransportSettings::default();
    settings.wow_flutter = 2.0;
    let mut machine = TapeMachine::new(settings, RATE, 3, BLOCK);
    let a = machine.add_track(Some(RecordSide::new(core(), RATE, BLOCK)), 0.0);
    let b = machine.add_track(Some(RecordSide::new(core(), RATE, BLOCK)), 0.0);
    let input = signal(48_000);
    let (mut oa, mut ob) = (vec![0.0; input.len()], vec![0.0; input.len()]);
    for ((inp, x), y) in input.chunks(BLOCK).zip(oa.chunks_mut(BLOCK)).zip(ob.chunks_mut(BLOCK)) {
      machine.advance(inp.len());
      machine.record_and_play(a, inp, x);
      machine.record_and_play(b, inp, y);
    }
    assert_eq!(oa, ob);
  }

  #[test]
  fn output_knob_and_hiss_knob() {
    let mut machine = TapeMachine::new(quiet(), RATE, 1, BLOCK);
    let t = machine.add_track(Some(RecordSide::new(core(), RATE, BLOCK)), 0.0);
    let input = signal(24_000);
    let plain = run_live(&mut machine, t, &input);
    let mut s = quiet();
    s.output_db = -6.0;
    machine.set_settings(s);
    machine.reset();
    let quieter = run_live(&mut machine, t, &input);
    let ratio = quieter[5000] / plain[5000];
    assert!((ratio - 10f32.powf(-6.0 / 20.0)).abs() < 1e-5);
    // hiss at 0 is silence under silence; at 1 it's there
    let silence = vec![0.0; 24_000];
    let mut s = quiet();
    machine.set_settings(s.clone());
    machine.reset();
    assert!(run_live(&mut machine, t, &silence).iter().all(|&x| x == 0.0));
    s.hiss = 1.0;
    machine.set_settings(s);
    assert!(run_live(&mut machine, t, &silence).iter().any(|&x| x != 0.0));
  }

  #[test]
  fn a_track_without_its_print_plays_silence() {
    let mut machine = TapeMachine::new(quiet(), RATE, 1, BLOCK);
    let t = machine.add_track(None, 0.0);
    assert!(run_live(&mut machine, t, &signal(4096)).iter().all(|&x| x == 0.0));
    let old = machine.set_record(t, Some(RecordSide::new(core(), RATE, BLOCK)));
    assert!(old.is_none());
    assert!(run_live(&mut machine, t, &signal(4096)).iter().any(|&x| x != 0.0));
  }
}
