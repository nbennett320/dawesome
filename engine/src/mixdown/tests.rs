use super::*;
use crate::test_support::play;
use std::time::Instant;
use crate::commands::clip::add_clip;

const KICK: &str = "assets/assets_66-bd-01.wav";
const LONG: &str = "assets/slide_guitar.wav";

fn mixdown(state: &daw::InnerState, rate: u32) -> Vec<f32> {
  let mut samples = Vec::new();
  render(snapshot(state, rate).unwrap(), rate, &mut |chunk| { samples.extend_from_slice(chunk); Ok(()) },
    &Progress::default(), &AtomicBool::new(false)).unwrap();
  samples
}

fn peak(samples: &[f32]) -> f32 {
  samples.iter().fold(0., |m, s| m.max(s.abs()))
}

fn worst(a: &[f32], b: &[f32]) -> f32 {
  a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0., f32::max)
}

// a pattern of kicks and a long sample, on track `track`, from `beat`
fn pattern_clip(state: &daw::InnerState, track: u32, at: Duration) -> u64 {
  let pattern = {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    let kick = sequencer.add_row(KICK).unwrap();
    let long = sequencer.add_row(LONG).unwrap();
    let pattern = sequencer.add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
    for (row, step) in [(kick, 0), (kick, 6), (long, 15)] {
      sequencer.set_step(pattern, row, step, true).unwrap();
    }
    sequencer.set_row_volume(long, 0.3).unwrap();
    pattern
  };
  state.playlist.audiograph.lock().unwrap()
    .add_pattern_node::<daw::timing::Note>(pattern, at, track, None).unwrap()
}

#[test]
fn test_a_mixdown_is_what_playback_plays() {
  for (tempo, rate) in [(120., 48_000), (87.3, 44_100), (90., 96_000)] {
    let state = daw::InnerState::default();
    state.playlist.audiograph.lock().unwrap().set_tempo(tempo);
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    add_clip(&state, LONG.into(), 1, Some(1.37)).unwrap();
    pattern_clip(&state, 2, Duration::from_nanos(1_234_567_891));
    state.playlist.mixer.set_volume_db(1, -6.);
    state.playlist.mixer.set_pan(2, 0.4);
    state.playlist.mixer.set_master_volume_db(-3.);

    let mixed = mixdown(&state, rate);
    let played = play(&state, rate, Duration::from_secs_f64(mixed.len() as f64 / 2. / rate as f64));
    assert_eq!(mixed.len(), played.len());
    assert!(peak(&mixed) > 0.1);
    // (the same voices, summed in another order: as near as f32 gets)
    let off = worst(&mixed, &played);
    assert!(off < 1e-5, "off by {} at {} bpm, {}Hz", off, tempo, rate);
  }
}

#[test]
fn test_mute_and_solo_are_heard_as_set() {
  let state = daw::InnerState::default();
  add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
  add_clip(&state, LONG.into(), 1, Some(0.)).unwrap();
  let both = mixdown(&state, 48_000);

  state.playlist.mixer.set_mute(1, true);
  let kick_only = mixdown(&state, 48_000);
  state.playlist.mixer.set_mute(1, false);
  state.playlist.mixer.set_solo(0, true);
  let soloed = mixdown(&state, 48_000);
  assert_eq!(kick_only, soloed);
  assert_ne!(kick_only, both);

  // the kick is shorter than the long sample: past it, there's nothing
  let kick_len = {
    let graph = state.playlist.audiograph.lock().unwrap();
    daw::run_frame(graph.nodes.iter().find(|n| n.track_number == 0).unwrap().duration().as_nanos() as i128, 48_000) as usize
  };
  assert_eq!(peak(&soloed[(kick_len + 48) * 2..]), 0.);
  assert!(peak(&both[(kick_len + 48) * 2..]) > 0.01);
}

#[test]
fn test_the_metronome_is_never_mixed_down() {
  let state = daw::InnerState::default();
  add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
  state.playlist.mixer.set_mute(0, true);
  state.metronome_enabled.store(true, Ordering::SeqCst);
  let mixed = mixdown(&state, 48_000);
  assert!(!mixed.is_empty());
  assert_eq!(peak(&mixed), 0.);
}

#[test]
fn test_the_master_fader_turns_it_down() {
  let state = daw::InnerState::default();
  add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
  // (quiet to start with, so the limiter isn't in play)
  state.playlist.mixer.set_volume_db(0, -12.);
  let full = mixdown(&state, 48_000);
  state.playlist.mixer.set_master_volume_db(-6.);
  let down = mixdown(&state, 48_000);
  let ratio = peak(&down) / peak(&full);
  assert!((ratio - daw::db_to_gain(-6.)).abs() < 0.01, "{}", ratio);
}

#[test]
fn test_it_runs_to_the_last_clips_end_and_a_ringing_hit_is_kept() {
  let state = daw::InnerState::default();
  // a pattern whose last hit, the long sample, rings well past its end
  pattern_clip(&state, 0, Duration::ZERO);
  let snap = snapshot(&state, 48_000).unwrap();
  let end = daw::run_frame(snap.end().as_nanos() as i128, 48_000) as usize;
  let mixed = mixdown(&state, 48_000);
  let frames = mixed.len() / 2;
  assert!(frames > end + 48_000, "{} frames for a song {} long", frames, end);
  // ending on its last sound, not on silence
  assert!(peak(&mixed[mixed.len() - 2..]) > 0.);

  // a clip that ends in silence ends at its end
  let state = daw::InnerState::default();
  add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
  let end = daw::run_frame(snapshot(&state, 48_000).unwrap().end().as_nanos() as i128, 48_000) as usize;
  assert!(mixdown(&state, 48_000).len() / 2 <= end + 1);
}

#[test]
fn test_an_empty_playlist_is_refused() {
  let state = daw::InnerState::default();
  assert_eq!(snapshot(&state, 48_000).err().unwrap(), NOTHING_TO_MIX);
}

#[test]
fn test_changes_after_it_starts_dont_reach_it_and_it_doesnt_reach_the_meters() {
  let state = daw::InnerState::default();
  add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
  let before = mixdown(&state, 48_000);
  state.playlist.mixer.take_levels();

  let snap = snapshot(&state, 48_000).unwrap();
  state.playlist.mixer.set_mute(0, true);
  state.playlist.audiograph.lock().unwrap().set_tempo(60.);
  let mut samples = Vec::new();
  render(snap, 48_000, &mut |chunk| { samples.extend_from_slice(chunk); Ok(()) },
    &Progress::default(), &AtomicBool::new(false)).unwrap();
  assert_eq!(samples, before);

  let levels = state.playlist.mixer.take_levels();
  assert_eq!(levels.master, 0.);
  assert!(levels.tracks.iter().all(|&(_, peak)| peak == 0.));
  // and the live song isn't left running
  assert!(!state.playlist.audiograph.lock().unwrap().running);
}

#[test]
fn test_a_cancel_stops_it_and_progress_is_reported() {
  let state = daw::InnerState::default();
  add_clip(&state, LONG.into(), 0, Some(0.)).unwrap();
  let progress = Progress::default();
  let cancel = AtomicBool::new(false);
  let mut chunks = 0;
  let result = render(snapshot(&state, 48_000).unwrap(), 48_000, &mut |_| {
    chunks += 1;
    if chunks == 3 { cancel.store(true, Ordering::Relaxed); }
    Ok(())
  }, &progress, &cancel);
  assert_eq!(result.err().unwrap(), CANCELLED);
  assert!(progress.fraction() > 0. && progress.fraction() < 1.);

  let progress = Progress::default();
  render(snapshot(&state, 48_000).unwrap(), 48_000, &mut |_| Ok(()), &progress, &AtomicBool::new(false)).unwrap();
  assert_eq!(progress.fraction(), 1.);
}

#[test]
fn test_more_hits_than_the_engine_holds_at_once_are_all_heard() {
  // 1100 kicks a fifth of a beat apart: more than MAX_VOICES over the
  // song (the scheduler only ever has a beat's worth waiting), though
  // only a few sound at once
  let state = daw::InnerState::default();
  state.playlist.ui.lock().unwrap().snap_enabled = false;
  let count = 1100;
  for n in 0..count {
    add_clip(&state, KICK.into(), 0, Some(n as f32 * 0.2)).unwrap();
  }
  let mixed = mixdown(&state, 48_000);
  let starts: Vec<usize> = state.playlist.audiograph.lock().unwrap().nodes.iter()
    .map(|node| daw::run_frame(node.start_offset.as_nanos() as i128, 48_000) as usize)
    .collect();
  assert_eq!(starts.len(), count);
  // each kick is heard where it starts: the mix jumps there
  let silent = starts.iter().filter(|&&at| peak(&mixed[at * 2..(at + 100) * 2]) < 0.01).count();
  assert_eq!(silent, 0);
}

#[test]
fn test_effects_are_mixed_down_through_second_instances() {
  use crate::daw::Channel;
  use crate::effects::testing::*;
  use std::sync::atomic::Ordering::SeqCst;

  let rig = rig();
  let state = &rig.state;
  add_clip(state, KICK.into(), 0, Some(0.)).unwrap();
  add_clip(state, LONG.into(), 1, Some(0.5)).unwrap();
  // under the limiter's knee, so the gains multiply exactly
  state.playlist.mixer.set_master_volume_db(-12.);
  let dry = mixdown(state, 48_000);

  let effects = &state.playlist.effects;
  let a = effects.add(Channel::Track(0), &gain().id, None).unwrap();
  let m = effects.add(Channel::Master, &gain().id, None).unwrap();
  let off = effects.add(Channel::Track(1), &gain().id, None).unwrap();
  rig.turn(a, 0.25);
  effects.set_bypassed(off, true).unwrap();
  let loads = rig.loader.loads.load(SeqCst);
  let wet = mixdown(state, 48_000);

  // the bypassed one isn't loaded again; the others are, and are gone after
  assert_eq!(rig.loader.loads.load(SeqCst), loads + 2);
  assert_eq!(rig.loader.alive.load(SeqCst), 3);
  assert_eq!(rig.loader.rates.lock().unwrap()[loads..], [48_000, 48_000]);
  // the live ones are as they were
  assert!((rig.gain_of(a) - 0.25).abs() < 1e-9 && (rig.gain_of(m) - START_GAIN).abs() < 1e-9);

  // the kick through both, the long sample through the master's only
  let kick_only = {
    let state = daw::InnerState::default();
    add_clip(&state, KICK.into(), 0, Some(0.)).unwrap();
    state.playlist.mixer.set_master_volume_db(-12.);
    mixdown(&state, 48_000)
  };
  let n = dry.len().min(wet.len()).min(kick_only.len());
  let expected: Vec<f32> = (0..n).map(|i| {
    let long = dry[i] - kick_only[i];
    (kick_only[i] * 0.25 + long) * START_GAIN
  }).collect();
  assert!(peak(&wet[..n]) > 0.01);
  let off_by = worst(&wet[..n], &expected);
  assert!(off_by < 1e-5, "off by {}", off_by);
}

// the frames that sound (left side of interleaved stereo), as runs of (from, to)
fn sounding(samples: &[f32]) -> Vec<(usize, usize)> {
  let mut runs: Vec<(usize, usize)> = Vec::new();
  for (i, pair) in samples.chunks(2).enumerate() {
    if pair[0] == 0. { continue; }
    match runs.last_mut() {
      Some(run) if run.1 == i => run.1 = i + 1,
      _ => runs.push((i, i + 1)),
    }
  }
  runs
}

// a rig with the fake synth (its master copy playing at 0.25), and a one
// bar note pattern of two notes for it: one from step 3 to 6, one from 14
// to the end. (rig, synth, pattern)
fn synth_and_notes() -> (crate::effects::testing::Rig, u64, u64) {
  use crate::effects::testing::*;
  let rig = rig();
  let synth = crate::commands::synth::add(&rig.state, &synth().id).unwrap().synth_id;
  rig.turn(synth, 0.25);
  let pattern = {
    let mut sequencer = rig.state.playlist.sequencer.lock().unwrap();
    let pattern = sequencer.add_pattern(1, 4, daw::PatternKind::Note).unwrap();
    let note = |pitch, start, length| daw::PatternNote { pitch, start, length, velocity: 1. };
    sequencer.set_pattern_notes(pattern, synth, vec![note(62, 14., 4.), note(60, 3., 3.)]).unwrap();
    pattern
  };
  (rig, synth, pattern)
}

// a clip of a pattern on a track, at `at`, with the synths following it
fn note_clip(state: &daw::InnerState, pattern: u64, track: u32, at: Duration) -> u64 {
  let clip = state.playlist.audiograph.lock().unwrap()
    .add_pattern_node::<daw::timing::Note>(pattern, at, track, None).unwrap();
  crate::commands::synth::follow(state);
  clip
}

// the playlist as the scheduler plays it, a beat at a time, from `from`,
// `total` long, on the rig's engine (whose chains are the live plugins)
fn play_on_rig(rig: &mut crate::effects::testing::Rig, run: u64, from: Duration, total: Duration) -> Vec<f32> {
  let engine = rig.engine.clone();
  let rate = engine.sample_rate();
  let copies = rig.effects().copy_ids();
  {
    let mut graph = rig.state.playlist.audiograph.lock().unwrap();
    let beat = graph.beat_interval();
    graph.init();
    engine.start_run(run, graph.mixer.master(), vec![]).unwrap();
    let mut at = Duration::ZERO;
    while at < total {
      graph.set_current_offset(Some(from + at));
      let voices = graph.run_sounds_at(at, rate, beat, false, at.is_zero()).into_iter()
        .map(|(delay, source, route)| engine.boxed_voice(source, Some(run), engine.frames_in(at + delay), route))
        .collect();
      engine.play(voices).unwrap();
      daw::play_slice_notes(&engine, run, at, graph.run_notes(beat, None), &copies).unwrap();
      at += beat;
    }
    graph.pause();
  }
  let mut out = vec![0f32; engine.frames_in(total) as usize * 2];
  rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
  out
}

#[test]
fn test_a_pattern_clips_notes_play_on_its_tracks_copy_and_are_mixed_down() {
  let (mut rig, _, pattern) = synth_and_notes();
  let state = rig.state.clone();
  // on beat 1 (half a second in at 120 bpm), cut at step 15. a step is
  // 6000 frames at 48kHz
  let clip = note_clip(&state, pattern, 2, Duration::from_millis(500));
  state.playlist.audiograph.lock().unwrap().set_pattern_window(clip, 0., Some(15.)).unwrap();

  // the first crosses a beat (a slice), the second is cut at the clip's end
  let expected = vec![(24_000 + 18_000, 24_000 + 36_000), (24_000 + 84_000, 24_000 + 90_000)];
  let played = play_on_rig(&mut rig, 100, Duration::ZERO, Duration::from_secs(3));
  assert_eq!(sounding(&played), expected);
  assert_eq!(played[(24_000 + 20_000) * 2], 0.25);

  // from the middle of the first: it isn't started late, and nothing is
  // left hanging
  let played = play_on_rig(&mut rig, 101, Duration::from_secs(1), Duration::from_secs(2));
  assert_eq!(sounding(&played), vec![(108_000 - 48_000, 114_000 - 48_000)]);

  // the mixdown hears the same, through a second instance of the synth
  let mixed = mixdown(&state, 48_000);
  assert_eq!(sounding(&mixed), expected);
  // and on another track, through a copy made for that track
  state.playlist.audiograph.lock().unwrap().pattern_nodes[0].track_number = 3;
  assert_eq!(sounding(&mixdown(&state, 48_000)), expected);
}

#[test]
fn test_a_synth_on_two_tracks_is_mixed_down_through_each_tracks_fader() {
  let (rig, synth, pattern) = synth_and_notes();
  let state = rig.state.clone();
  // the other track's clip plays other notes: from step 8 to 10
  let other = {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    let other = sequencer.add_pattern(1, 4, daw::PatternKind::Note).unwrap();
    sequencer.set_pattern_notes(other, synth, vec![daw::PatternNote { pitch: 70, start: 8., length: 2., velocity: 1. }]).unwrap();
    other
  };
  note_clip(&state, pattern, 1, Duration::ZERO);
  note_clip(&state, other, 3, Duration::ZERO);
  state.playlist.mixer.set_volume_db(3, -6.);

  let mixed = mixdown(&state, 48_000);
  assert_eq!(sounding(&mixed), vec![(18_000, 36_000), (48_000, 60_000), (84_000, 96_000)]);
  let ratio = mixed[50_000 * 2] / mixed[20_000 * 2];
  assert!((ratio - 0.501).abs() < 0.01, "{}", ratio);
  // a copy each, and the master's: none of them the live ones
  assert_eq!(rig.loader.alive.load(std::sync::atomic::Ordering::SeqCst), 3);

  // muted, it isn't heard on either; quieter, on both
  crate::commands::synth::set_muted(&state, synth, true).unwrap();
  assert!(sounding(&mixdown(&state, 48_000)).is_empty());
  crate::commands::synth::set_muted(&state, synth, false).unwrap();
  crate::commands::synth::set_volume(&state, synth, 0.5, None).unwrap();
  let quieter = mixdown(&state, 48_000);
  assert!((quieter[20_000 * 2] - mixed[20_000 * 2] * 0.5).abs() < 1e-6);
  assert!((quieter[50_000 * 2] - mixed[50_000 * 2] * 0.5).abs() < 1e-6);
}

#[test]
fn test_a_synth_added_just_now_is_mixed_down_once_it_has_set_itself_up() {
  use crate::effects::testing::*;
  // synths that set themselves up a moment after they're loaded, as a
  // sampler loads its sounds: until then they're silent and their settings
  // say they're empty, and a copy given those stays empty
  let rig = rig();
  let settle = Duration::from_millis(150);
  *rig.loader.settles_in.lock().unwrap() = settle;
  let state = rig.state.clone();
  let note = |pitch, start, length| daw::PatternNote { pitch, start, length, velocity: 1. };
  let pattern = state.playlist.sequencer.lock().unwrap().add_pattern(1, 4, daw::PatternKind::Note).unwrap();
  note_clip(&state, pattern, 2, Duration::ZERO);

  // mixed down as soon as it's added, without waiting for it: silent
  let first = crate::commands::synth::add(&state, &synth().id).unwrap().synth_id;
  crate::commands::sequencer::set_notes(&state, pattern, first, vec![note(60, 3., 3.), note(62, 14., 4.)], None).unwrap();
  assert!(sounding(&mixdown(&state, 48_000)).is_empty());

  // with the wait, the same, straight after another is added in its place:
  // the notes are there, from the song's start
  rig.effects().use_settle(settle);
  crate::commands::synth::remove(&state, first).unwrap();
  let second = crate::commands::synth::add(&state, &synth().id).unwrap().synth_id;
  crate::commands::sequencer::set_notes(&state, pattern, second, vec![note(60, 3., 3.), note(62, 14., 4.)], None).unwrap();
  let mixed = mixdown(&state, 48_000);
  assert_eq!(sounding(&mixed), vec![(18_000, 36_000), (84_000, 96_000)]);
  assert_eq!(mixed[20_000 * 2], START_GAIN);
}

#[test]
fn test_a_note_pattern_on_its_own_plays_on_the_master_copies() {
  let (mut rig, _, pattern) = synth_and_notes();
  let engine = rig.engine.clone();
  // a clip on a track too: its copy doesn't play the pattern on its own
  note_clip(&rig.state, pattern, 2, Duration::from_secs(60));
  let copies = rig.effects().copy_ids();
  assert_eq!(copies.len(), 2);
  // two times round, as the scheduler loops it: back to its start each bar
  {
    let mut graph = rig.state.playlist.audiograph.lock().unwrap();
    let beat = graph.beat_interval();
    let bar = beat * 4;
    engine.start_run(200, graph.mixer.master(), vec![]).unwrap();
    let mut at = Duration::ZERO;
    while at < bar * 2 {
      graph.set_current_offset(Some(if at >= bar { at - bar } else { at }));
      let notes = graph.pattern_notes(pattern, beat);
      assert!(notes.iter().all(|n| n.route.is_none()));
      daw::play_slice_notes(&engine, 200, at, notes, &copies).unwrap();
      at += beat;
    }
  }
  // (the track's strip is muted: what's heard is the master copy's)
  rig.state.playlist.mixer.set_mute(2, true);
  let mut heard = vec![0f32; 4 * 48_000 * 2];
  rig.renderer.render(&mut heard, Instant::now(), Duration::ZERO);
  assert_eq!(sounding(&heard), vec![(18_000, 36_000), (84_000, 96_000), (114_000, 132_000), (180_000, 192_000)]);
}

#[test]
fn test_a_mixdown_fails_if_an_effect_cant_be_loaded_again() {

  use crate::daw::Channel;
  use crate::effects::testing::*;
  let rig = rig();
  add_clip(&rig.state, KICK.into(), 0, Some(0.)).unwrap();
  rig.state.playlist.effects.add(Channel::Track(0), &gain().id, None).unwrap();
  rig.loader.broken.store(true, std::sync::atomic::Ordering::SeqCst);
  let err = snapshot(&rig.state, 48_000).err().unwrap();
  assert!(err.starts_with("Gain couldn't be loaded for the mixdown"), "{}", err);
}
