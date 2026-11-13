use super::super::testing::*;
use super::*;
use crate::commands::sequencer::edit;
use crate::commands::synth as synths;
use std::sync::atomic::Ordering::SeqCst;

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 1e-5
}

fn note(pitch: u8, start: f64, length: f64) -> daw::PatternNote {
  daw::PatternNote { pitch, start, length, velocity: 1. }
}

// a synth row (its master copy at `gain`), below a sample row so its id
// isn't its slot, and a pattern with a note for it. (its slot, the pattern)
fn synth_with_notes(rig: &Rig, gain: f32) -> (u64, u64) {
  rig.state.playlist.sequencer.lock().unwrap().add_row("assets/assets_66-bd-01.wav").unwrap();
  let row = synths::add(&rig.state, &synth().id).unwrap().row_id;
  let slot = rig.slot_of(row);
  rig.turn(slot, gain);
  let pattern = rig.state.playlist.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
  crate::commands::sequencer::set_notes(&rig.state, pattern, row, vec![note(60, 0., 4.)], None).unwrap();
  (slot, pattern)
}

// the synth row whose plugin is in `slot`
fn row_of(rig: &Rig, slot: u64) -> u64 {
  let sequencer = rig.state.playlist.sequencer.lock().unwrap();
  sequencer.rows().iter().find(|row| row.synth().map_or(false, |synth| synth.slot == slot)).expect("a synth row").id
}

fn clip(rig: &Rig, pattern: u64, track: u32) -> u64 {
  crate::commands::sequencer::add_pattern_clip_for_tests(&rig.state, pattern, track, Some(0.))
}

fn steps(rig: &Rig) -> usize {
  rig.state.history.lock().unwrap().len()
}

// what a copy plays once a change of gain (ramped across a block) has
// been made: the second time round
fn heard(rig: &mut Rig, synth: u64, track: Option<u32>) -> Vec<f32> {
  rig.hear_notes(synth, track, &[(60, 0, 300)], 400);
  rig.hear_notes(synth, track, &[(60, 0, 300)], 400)
}

#[test]
fn test_a_synth_is_an_instrument_with_a_unique_name_and_its_master_copy_on_the_master() {
  let mut rig = rig();
  let a = synths::add(&rig.state, &synth().id).unwrap();
  let b = synths::add(&rig.state, &synth().id).unwrap();
  let synths: Vec<&daw::SynthState> = b.sequencer.rows.iter().filter_map(|row| row.synth()).collect();
  let names: Vec<&str> = synths.iter().map(|s| s.name.as_str()).collect();
  assert_eq!(names, vec!["Synth", "Synth 2"]);
  assert!(!synths[0].missing);
  assert_eq!((b.sequencer.rows[1].id, b.row_id), (1, 1));
  let json = serde_json::to_value(&b.sequencer.rows[1]).unwrap();
  assert_eq!(json["instrument"]["pluginId"], synth().id);
  assert_eq!(json["instrument"]["hasEditor"], false);
  assert_eq!(json["volume"], 1.);

  // its slot reaches its plugin's name and settings
  let (a_slot, b_slot) = (rig.slot_of(a.row_id), rig.slot_of(b.row_id));
  assert_eq!(synths[1].slot, b_slot);
  assert_eq!(rig.effects().slot_name(b_slot).as_deref(), Some("Synth 2"));
  rig.turn(a_slot, 0.3);
  assert!(close(rig.gain_of(a_slot), 0.3));
  // its master copy plays notes straight to the master; no track has one
  assert!(close(rig.hear_notes(a_slot, None, &[(60, 0, 300)], 400)[100], 0.3));
  assert!(rig.effects().copy_ids().get(&(a_slot, Some(0))).is_none());
  // racks hold effects only
  assert!(rig.effects().racks().is_empty());

  let err = synths::add(&rig.state, &gain().id).unwrap_err();
  assert!(err.contains("isn't an instrument"), "{}", err);
  assert!(synths::add(&rig.state, &gone_synth().id).is_err());
  assert_eq!(steps(&rig), 2);
}

#[test]
fn test_a_copy_appears_when_a_clip_lands_and_its_notes_play_through_its_track() {
  let mut rig = rig();
  let (synth, pattern) = synth_with_notes(&rig, 0.4);
  // no clip: no copy
  assert_eq!(rig.loader.alive.load(SeqCst), 1);
  clip(&rig, pattern, 2);
  assert_eq!(rig.loader.alive.load(SeqCst), 2);
  // with the master copy's settings, on the track's bus
  assert_eq!(rig.copy_gain(synth, Some(2)), Some(0.4));
  assert!(close(rig.hear_notes(synth, Some(2), &[(60, 0, 300)], 400)[100], 0.4));
  // and through the track's effects, after the copy
  rig.effects().add(Channel::Track(2), &gain().id, None).unwrap();
  assert!(close(heard(&mut rig, synth, Some(2))[100], 0.4 * START_GAIN));

  // a clip of a pattern without its notes needs none
  let empty = rig.state.playlist.sequencer.lock().unwrap().add_pattern(1, 4).unwrap();
  clip(&rig, empty, 5);
  assert!(rig.effects().copy_ids().get(&(synth, Some(5))).is_none());
}

#[test]
fn test_two_tracks_playing_a_synth_each_hear_only_their_own_notes() {
  let mut rig = rig();
  let (synth, pattern) = synth_with_notes(&rig, 0.5);
  clip(&rig, pattern, 0);
  clip(&rig, pattern, 1);
  let copies = rig.effects().copy_ids();
  let (a, b) = (copies[&(synth, Some(0))], copies[&(synth, Some(1))]);
  assert!(a != b && a != synth && b != synth);
  rig.state.playlist.mixer.set_volume_db(1, -6.);

  // notes for track 0's copy: heard on track 0 only (track 1's fader
  // makes the difference)
  rig.run += 1;
  let run = rig.run;
  rig.engine.start_run(run, rig.state.playlist.mixer.master(), vec![]).unwrap();
  let mut slice = vec![
    daw::SliceNote { on: Duration::ZERO, off: Duration::from_millis(2), synth, route: Some(rig.state.playlist.mixer.track_route(0)), pitch: 60, velocity: 1. },
  ];
  daw::play_slice_notes(&rig.engine, run, Duration::ZERO, slice.clone(), &copies).unwrap();
  let mut out = vec![0f32; 400 * 2];
  rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
  assert!(close(out[20], 0.5));

  // and track 1's on track 1, through its fader
  rig.run += 1;
  let run = rig.run;
  rig.engine.start_run(run, rig.state.playlist.mixer.master(), vec![]).unwrap();
  slice[0].route = Some(rig.state.playlist.mixer.track_route(1));
  daw::play_slice_notes(&rig.engine, run, Duration::ZERO, slice, &copies).unwrap();
  rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
  assert!((out[20] / 0.5 - 0.501).abs() < 0.01, "{}", out[20]);
}

#[test]
fn test_the_master_copys_settings_reach_the_copies() {
  let rig = rig();
  let (synth, pattern) = synth_with_notes(&rig, 0.4);
  clip(&rig, pattern, 0);
  clip(&rig, pattern, 3);
  rig.turn(synth, 0.7);
  assert_eq!(rig.copy_gain(synth, Some(0)), Some(0.4));

  // not while its interface isn't showing (but always before playing)
  rig.effects().push_synth_states(false);
  assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.4));
  synths::before_playing(&rig.state);
  assert_eq!(rig.copy_gain(synth, Some(0)), Some(0.7));
  assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.7));

  // while it's showing, about once a second; once more as it's closed
  rig.effects().inner.lock().unwrap().synths.get_mut(&synth).unwrap().pushed_at = None;
  rig.loader.editors_open.store(true, SeqCst);
  rig.turn(synth, 0.2);
  rig.effects().push_synth_states(false);
  assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.2));
  rig.turn(synth, 0.1);
  rig.effects().push_synth_states(false);
  assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.2));
  rig.effects().inner.lock().unwrap().synths.get_mut(&synth).unwrap().pushed_at = None;
  rig.loader.editors_open.store(false, SeqCst);
  rig.effects().push_synth_states(false);
  assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.1));
  rig.turn(synth, 0.6);
  rig.effects().inner.lock().unwrap().synths.get_mut(&synth).unwrap().pushed_at = None;
  rig.effects().push_synth_states(false);
  assert_eq!(rig.copy_gain(synth, Some(3)), Some(0.1));

  // and a copy made now starts with them
  clip(&rig, pattern, 6);
  assert_eq!(rig.copy_gain(synth, Some(6)), Some(0.6));
}

#[test]
fn test_volume_and_mute_reach_every_copy() {
  let mut rig = rig();
  let (synth, pattern) = synth_with_notes(&rig, 0.5);
  clip(&rig, pattern, 1);
  let had = steps(&rig);

  let row = row_of(&rig, synth);

  // a drag of the volume is one step
  for v in [0.9, 0.7, 0.5] { edit(&rig.state, Some(5), |seq| seq.set_row_volume(row, v)).unwrap(); }
  assert_eq!(steps(&rig), had + 1);
  assert!(close(heard(&mut rig, synth, Some(1))[100], 0.25));
  assert!(close(heard(&mut rig, synth, None)[100], 0.25));

  let seq = edit(&rig.state, None, |seq| seq.set_row_muted(row, true)).unwrap();
  assert!(seq.rows.iter().find(|r| r.id == row).unwrap().muted);
  assert!(heard(&mut rig, synth, Some(1)).iter().all(|&s| s == 0.));
  assert!(heard(&mut rig, synth, None).iter().all(|&s| s == 0.));

  crate::history::undo(&rig.state).unwrap();
  assert!(close(heard(&mut rig, synth, Some(1))[100], 0.25));
  crate::history::undo(&rig.state).unwrap();
  assert!(close(heard(&mut rig, synth, Some(1))[100], 0.5));
  assert!(edit(&rig.state, None, |seq| seq.set_row_volume(999, 1.)).is_err());
}

#[test]
fn test_removing_a_synth_and_undoing_it_brings_back_its_notes_and_settings() {
  let mut rig = rig();
  let (synth, pattern) = synth_with_notes(&rig, 0.35);
  clip(&rig, pattern, 2);
  let at = WindowPosition { x: 10., y: 20. };
  rig.effects().keep_window_position(synth, at);
  let heard = Arc::new(Mutex::new(Vec::new()));
  let h = heard.clone();
  rig.effects().on_unload(move |id| h.lock().unwrap().push(id));
  let had = steps(&rig);
  let row = row_of(&rig, synth);

  let seq = edit(&rig.state, None, |seq| seq.remove_row(row)).unwrap();
  assert!(seq.rows.iter().all(|r| r.synth().is_none()));
  assert!(seq.pattern(pattern).unwrap().notes.is_empty());
  assert_eq!(steps(&rig), had + 1);
  // its master copy and its copy are torn down
  assert_eq!(rig.loader.alive.load(SeqCst), 0);
  assert!(heard.lock().unwrap().contains(&synth));
  assert!(rig.effects().slot_name(synth).is_none());

  crate::history::undo(&rig.state).unwrap();
  let seq = synths::sequencer_state(&rig.state);
  assert_eq!(seq.rows[1].synth().unwrap().slot, synth);
  assert_eq!(*seq.pattern(pattern).unwrap().notes[&row], vec![note(60, 0., 4.)]);
  assert!(close(rig.gain_of(synth), 0.35));
  assert_eq!(rig.copy_gain(synth, Some(2)), Some(0.35));
  assert_eq!(rig.effects().window_position(synth), Some(at));
  assert!(close(rig.hear_notes(synth, Some(2), &[(60, 0, 300)], 400)[100], 0.35));

  // redone and undone again: the same
  crate::history::redo(&rig.state).unwrap();
  assert_eq!(rig.loader.alive.load(SeqCst), 0);
  crate::history::undo(&rig.state).unwrap();
  assert!(close(rig.gain_of(synth), 0.35));
}

#[test]
fn test_adding_a_synth_on_a_track_is_one_step() {
  let rig = rig();
  let before = crate::project::capture(&rig.state, None);
  let had = steps(&rig);
  let added = synths::add_on_track(&rig.state, &synth().id, 4, Some(8.)).unwrap();
  assert_eq!(steps(&rig), had + 1);

  let pattern = added.project.sequencer.pattern(added.pattern_id).unwrap();
  assert_eq!((pattern.name.as_deref(), pattern.length, pattern.floor), (Some("Sequence 1"), 16, 16));
  assert!(pattern.notes.is_empty());
  assert_eq!(added.project.sequencer.rows[0].id, added.row_id);
  let clip = &added.project.pattern_clips[0];
  assert_eq!((clip.pattern_id, clip.track_number, clip.start_beat), (added.pattern_id, 4, 8.));
  let json = serde_json::to_value(&added).unwrap();
  assert!(json.get("rowId").is_some() && json.get("patternId").is_some() && json["project"].get("pattern_clips").is_none());
  let after = crate::project::capture(&rig.state, None);

  // its first note makes the track's copy
  let slot = rig.slot_of(added.row_id);
  crate::commands::sequencer::set_notes(&rig.state, added.pattern_id, added.row_id, vec![note(60, 0., 1.)], None).unwrap();
  assert!(rig.effects().copy_ids().contains_key(&(slot, Some(4))));
  crate::history::undo(&rig.state).unwrap();

  crate::history::undo(&rig.state).unwrap();
  assert_eq!(crate::project::capture(&rig.state, None), before);
  assert_eq!(rig.loader.alive.load(SeqCst), 0);
  crate::history::redo(&rig.state).unwrap();
  assert_eq!(crate::project::capture(&rig.state, None), after);
  assert!(synths::add_on_track(&rig.state, &gain().id, 0, None).is_err());
  assert_eq!(steps(&rig), had + 1);
}

#[test]
fn test_a_key_plays_a_synths_master_copy_until_its_let_go() {
  let mut rig = rig();
  let (synth, pattern) = synth_with_notes(&rig, 0.25);
  clip(&rig, pattern, 1);
  rig.state.playlist.mixer.set_mute(1, true);
  let engine = rig.engine.clone();

  // by its row
  let row = row_of(&rig, synth);
  synths::play_row_note_on(&rig.state, &engine, row, 60, 0.9).unwrap();
  let mut out = vec![0f32; 100 * 2];
  rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
  assert!(out.iter().all(|&s| close(s, 0.25)));

  synths::stop_row_note_on(&rig.state, &engine, row, 60).unwrap();
  rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
  assert!(out.iter().all(|&s| s == 0.));
  assert!(synths::play_row_note_on(&rig.state, &engine, row, 128, 0.9).is_err());
  assert!(synths::play_row_note_on(&rig.state, &engine, 999, 60, 0.9).unwrap_err().contains("no row with id 999"));

  // a sample row is silent, for now
  let kick = rig.state.playlist.sequencer.lock().unwrap().rows()[0].id;
  synths::play_row_note_on(&rig.state, &engine, kick, 60, 0.9).unwrap();
  rig.renderer.render(&mut out, Instant::now(), Duration::ZERO);
  assert!(out.iter().all(|&s| s == 0.));
  synths::stop_row_note_on(&rig.state, &engine, kick, 60).unwrap();
}

#[test]
fn test_a_copy_no_track_needs_is_kept_a_while_then_let_go() {
  let rig = rig();
  let (synth, pattern) = synth_with_notes(&rig, 0.5);
  let placed = clip(&rig, pattern, 2);
  let copy = rig.effects().copy_ids()[&(synth, Some(2))];
  let loads = rig.loader.loads.load(SeqCst);

  // moved away and back: the same copy, not loaded again
  rig.state.playlist.audiograph.lock().unwrap().pattern_nodes.iter_mut().find(|n| n.id == placed).unwrap().track_number = 3;
  synths::follow(&rig.state);
  assert!(rig.effects().copy_ids().get(&(synth, Some(2))).is_none());
  assert_eq!(rig.loader.alive.load(SeqCst), 3);
  rig.state.playlist.audiograph.lock().unwrap().pattern_nodes.iter_mut().find(|n| n.id == placed).unwrap().track_number = 2;
  synths::follow(&rig.state);
  assert_eq!(rig.effects().copy_ids()[&(synth, Some(2))], copy);
  assert_eq!(rig.loader.loads.load(SeqCst), loads + 1);

  // the one on track 3 goes once it's been spare long enough
  rig.effects().drop_copies_spare_for(Duration::ZERO);
  assert_eq!(rig.loader.alive.load(SeqCst), 2);
  assert_eq!(rig.effects().copy_ids()[&(synth, Some(2))], copy);
}
