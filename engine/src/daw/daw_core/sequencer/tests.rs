use super::*;

const BEAT: Duration = Duration::from_millis(500);
const KICK: &str = "assets/assets_66-bd-01.wav";
const SNARE: &str = "assets/assets_66-sd-01.wav";

fn ms(n: u64) -> Duration { Duration::from_millis(n) }

// a sequencer with a kick and a snare row, and a one bar pattern
fn kit() -> (Sequencer, u64, u64, u64) {
  let mut seq = Sequencer::default();
  let kick = seq.add_row(KICK).unwrap();
  let snare = seq.add_row(SNARE).unwrap();
  let pattern = seq.add_pattern(1, 4).unwrap();
  (seq, kick, snare, pattern)
}

// where a pattern's hits start, in steps, in row order
fn hit_starts(seq: &Sequencer, pattern: u64) -> Vec<f64> {
  seq.sample_hits(pattern).into_iter().map(|(_, start)| start).collect()
}

fn note(pitch: u8, start: f64, length: f64) -> PatternNote {
  PatternNote { pitch, start, length, velocity: 0.8 }
}

// a clicked step's note
fn step(start: f64) -> PatternNote {
  PatternNote { pitch: DEFAULT_PITCH, start, length: 1., velocity: STEP_VELOCITY }
}

// a plugin as effects.rs gives it, in slot `slot`
fn synth(slot: u64, name: &str) -> Synth {
  Synth {
    id: slot,
    plugin_id: "vst3:synth".into(),
    path: "/fake/Synth.vst3".into(),
    name: name.into(),
    vendor: "Fake Audio".into(),
    volume: 1.,
    muted: false,
  }
}

#[test]
fn test_steps_are_16th_notes() {
  assert_eq!(steps_in_bars(1, 4), 16);
  assert_eq!(steps_in_bars(2, 3), 24);
  assert_eq!(step_offset(0, BEAT), ms(0));
  assert_eq!(step_offset(1, BEAT), ms(125));
  assert_eq!(step_offset(16, BEAT), ms(2_000));
}

#[test]
fn test_a_new_pattern_is_empty_and_its_bars_long() {
  let (seq, _, _, pattern) = kit();
  let p = seq.pattern(pattern).unwrap();
  assert_eq!((p.length, p.floor), (16, 16));
  assert_eq!(p.name.as_deref(), Some("Sequence 1"));
  assert!(p.notes.is_empty());
  assert_eq!(seq.pattern_len(pattern, BEAT), Some(ms(2_000)));
}

#[test]
fn test_a_pattern_is_1_to_128_bars() {
  let (mut seq, _, _, pattern) = kit();
  assert!(seq.add_pattern(0, 4).is_err());
  assert_eq!(seq.add_pattern(129, 4).unwrap_err(), "a pattern can be 1 to 128 bars long, not 129");
  let long = seq.add_pattern(PATTERN_BARS_MAX, 4).unwrap();
  assert_eq!(seq.pattern(long).unwrap().length, 2_048);

  // the counter sets the length and the floor
  seq.set_pattern_bars(pattern, 3, 4).unwrap();
  let p = seq.pattern(pattern).unwrap();
  assert_eq!((p.length, p.floor), (48, 48));
  assert!(seq.set_pattern_bars(pattern, 0, 4).is_err());
  assert!(seq.set_pattern_bars(pattern, 129, 4).is_err());
  assert!(seq.set_pattern_bars(99, 1, 4).is_err());
}

#[test]
fn test_hits_follow_the_steps() {
  let (mut seq, kick, snare, pattern) = kit();
  for s in [8, 0] { seq.set_step(pattern, kick, s, true).unwrap(); }
  seq.set_step(pattern, snare, 4, true).unwrap();

  assert_eq!(hit_starts(&seq, pattern), vec![0., 8., 4.]);

  // turning a step off takes its hit out
  seq.set_step(pattern, kick, 8, false).unwrap();
  assert_eq!(hit_starts(&seq, pattern), vec![0., 4.]);
}

#[test]
fn test_every_note_of_a_sample_row_is_a_hit() {
  let (mut seq, kick, _, pattern) = kit();
  // whatever its pitch, length or velocity
  seq.set_pattern_notes(pattern, kick, vec![note(72, 2.5, 0.25), note(40, 6., 9.)]).unwrap();
  assert_eq!(hit_starts(&seq, pattern), vec![2.5, 6.]);
}

#[test]
fn test_muted_rows_dont_play() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.set_row_muted(kick, true).unwrap();
  assert_eq!(hit_starts(&seq, pattern), vec![4.]);
}

#[test]
fn test_notes_past_the_end_come_back() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  seq.set_step(pattern, kick, 20, true).unwrap();

  // shortened: the note is kept, but doesn't play
  seq.set_pattern_bars(pattern, 1, 4).unwrap();
  assert!(hit_starts(&seq, pattern).is_empty());
  assert!(seq.set_step(pattern, kick, 20, true).unwrap_err().contains("there's no step 20"));

  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  assert_eq!(hit_starts(&seq, pattern), vec![20.]);
}

#[test]
fn test_a_step_is_a_note_at_the_rows_step_pitch() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_step(pattern, kick, 4, true).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), &[step(4.)]);
  // on again: nothing changes
  seq.set_step(pattern, kick, 4, true).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick).len(), 1);

  // the new note goes in order
  let longer = PatternNote { length: 3., velocity: 0.5, ..step(2.) };
  let higher = note(72, 2., 1.);
  seq.set_pattern_notes(pattern, kick, vec![higher, step(4.)]).unwrap();
  seq.set_step(pattern, kick, 2, true).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), &[step(2.), higher, step(4.)]);

  // off takes out every note on the step at the step pitch, whatever its
  // length or velocity. one at another pitch stays
  seq.set_pattern_notes(pattern, kick, vec![longer, step(2.), higher, step(4.)]).unwrap();
  seq.set_step(pattern, kick, 2, false).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), &[higher, step(4.)]);
  seq.set_step(pattern, kick, 4, false).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), &[higher]);

  // the last note gone: the row isn't in the pattern
  seq.set_pattern_notes(pattern, kick, vec![]).unwrap();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_step(pattern, kick, 0, false).unwrap();
  assert!(seq.pattern(pattern).unwrap().notes.is_empty());

  assert!(seq.set_step(pattern, 99, 0, true).unwrap_err().contains("no row with id 99"));
  assert!(seq.set_step(99, kick, 0, true).is_err());
  assert!(seq.set_step(pattern, kick, 16, true).is_err());
}

#[test]
fn test_a_step_follows_the_rows_step_pitch() {
  let (mut seq, kick, _, pattern) = kit();
  seq.row_mut(kick).unwrap().step_pitch = 36;
  seq.set_step(pattern, kick, 0, true).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), &[PatternNote { pitch: 36, ..step(0.) }]);
  // a note at the old pitch isn't on the step
  seq.set_pattern_notes(pattern, kick, vec![step(0.)]).unwrap();
  seq.set_step(pattern, kick, 0, false).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), &[step(0.)]);
}

#[test]
fn test_removing_a_row_takes_its_notes() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.remove_row(kick).unwrap();

  assert_eq!(seq.rows().len(), 1);
  assert!(!seq.pattern(pattern).unwrap().notes.contains_key(&kick));
  assert!(seq.set_step(pattern, kick, 0, true).is_err());
  assert!(seq.remove_row(kick).is_err());
}

#[test]
fn test_rename_and_duplicate() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_step(pattern, kick, 3, true).unwrap();
  seq.rename_pattern(pattern, "  verse ").unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().name.as_deref(), Some("verse"));
  seq.set_pattern_bars(pattern, 2, 4).unwrap();

  let other = seq.add_pattern(1, 4).unwrap();
  let copy = seq.duplicate_pattern(pattern).unwrap();
  // straight after the original, with its notes and floor
  let order: Vec<u64> = seq.patterns().iter().map(|p| p.id).collect();
  assert_eq!(order, vec![pattern, copy, other]);
  let duplicate = seq.pattern(copy).unwrap();
  assert_eq!(duplicate.name.as_deref(), Some("verse copy"));
  assert_eq!((duplicate.length, duplicate.floor), (32, 32));
  assert!(duplicate.lit(kick, 3));

  // the copy is its own pattern
  seq.set_step(copy, kick, 3, false).unwrap();
  assert!(seq.pattern(pattern).unwrap().lit(kick, 3));

  seq.rename_pattern(pattern, " ").unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().name, None);
}

#[test]
fn test_ids_arent_reused() {
  let (mut seq, kick, _, pattern) = kit();
  seq.remove_row(kick).unwrap();
  seq.remove_pattern(pattern).unwrap();
  assert_ne!(seq.add_row(KICK).unwrap(), kick);
  assert_ne!(seq.add_synth_row(synth(kick, "Synth")), kick);
  assert_ne!(seq.add_pattern(1, 4).unwrap(), pattern);
}

#[test]
fn test_volume_is_kept_in_range() {
  let (mut seq, kick, _, _) = kit();
  seq.set_row_volume(kick, 9.).unwrap();
  assert_eq!(seq.row(kick).unwrap().volume, ROW_VOLUME_MAX);
  seq.set_row_volume(kick, f32::NAN).unwrap();
  assert_eq!(seq.row(kick).unwrap().volume, ROW_VOLUME);
  assert!(seq.set_row_volume(99, 1.).unwrap_err().contains("no row with id 99"));
}

#[test]
fn test_a_missing_sample_is_an_error() {
  let mut seq = Sequencer::default();
  assert!(seq.add_row("assets/nope.wav").is_err());
  assert!(seq.rows().is_empty());
}

#[test]
fn test_save_and_restore() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.set_row_volume(snare, 0.5).unwrap();
  seq.rename_pattern(pattern, "beat").unwrap();

  let mut saved = seq.saved();
  // the snare's sample has gone missing
  *saved.sample_paths_mut().nth(1).unwrap() = "assets/nope.wav".into();

  let mut loaded = Sequencer::default();
  let missing = loaded.restore(saved);
  assert_eq!(missing.len(), 1);
  assert_eq!(missing[0].0, "assets/nope.wav");
  assert_eq!(loaded.rows().len(), 1);
  // its notes went with it
  assert_eq!(hit_starts(&loaded, pattern), vec![0.]);
  assert_eq!(loaded.pattern(pattern).unwrap().notes.keys().copied().collect::<Vec<_>>(), vec![kick]);
  assert_eq!(loaded.pattern(pattern).unwrap().name.as_deref(), Some("beat"));
  // new ids carry on after the loaded ones
  assert!(loaded.add_pattern(1, 4).unwrap() > pattern);
  assert!(loaded.add_row(KICK).unwrap() > snare);
}

#[test]
fn test_a_restored_pattern_is_made_sensible() {
  let (seq, kick, _, _) = kit();
  let mut saved = seq.saved();
  let pattern = |id, length, floor, notes| SavedPattern { id, name: None, length, floor, notes, folder: Some(42) };
  saved.patterns = vec![
    // notes out of order, one that isn't a note, and some of a row that
    // isn't there
    pattern(0, 16, 0, BTreeMap::from([
      (kick, vec![note(64, 4., 1.), note(60, 4., 1.), note(61, f64::NAN, 1.), note(62, 0., 2.)]),
      (77, vec![note(60, 0., 1.)]),
    ])),
    // a floor past its end
    pattern(1, 16, 64, BTreeMap::from([(kick, vec![])])),
    // no steps, and an id used twice: left out
    pattern(2, 0, 0, BTreeMap::new()),
    pattern(1, 32, 32, BTreeMap::new()),
  ];
  let mut loaded = Sequencer::default();
  loaded.restore(saved);

  assert_eq!(loaded.patterns().len(), 2);
  let first = loaded.pattern(0).unwrap();
  assert_eq!(first.notes_of(kick), &[note(62, 0., 2.), note(60, 4., 1.), note(64, 4., 1.)]);
  assert_eq!(first.notes.len(), 1);
  // a floor of none is the length; a folder that isn't there is the top
  assert_eq!((first.floor, first.folder), (16, None));
  let second = loaded.pattern(1).unwrap();
  assert_eq!((second.length, second.floor), (16, 16));
  // a row with no notes isn't in it
  assert!(second.notes.is_empty());
}

#[test]
fn test_folders() {
  let (mut seq, _, _, pattern) = kit();
  let drums = seq.add_folder("  Drums ", None).unwrap();
  let fills = seq.add_folder("Fills", Some(drums)).unwrap();
  assert_eq!(seq.folder(drums).unwrap().name, "Drums");
  assert_eq!(seq.folder(fills).unwrap().parent, Some(drums));

  // names: not blank, and not twice in a folder (case aside)
  assert!(seq.add_folder("  ", None).is_err());
  assert!(seq.add_folder("drums", None).is_err());
  assert!(seq.add_folder("Drums", Some(drums)).is_ok());
  assert!(seq.add_folder("x", Some(99)).is_err());
  assert!(seq.rename_folder(fills, "FILLS").is_ok());
  let other = seq.add_folder("Other", Some(drums)).unwrap();
  assert!(seq.rename_folder(other, "fills").is_err());

  // patterns go in folders, and back to the top
  seq.move_pattern(pattern, Some(fills)).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().folder, Some(fills));
  assert!(seq.move_pattern(pattern, Some(99)).is_err());
  assert!(seq.move_pattern(99, None).is_err());
  assert_eq!(seq.patterns_in_folder(drums), vec![pattern]);

  // a copy goes in the same folder
  let copy = seq.duplicate_pattern(pattern).unwrap();
  assert_eq!(seq.pattern(copy).unwrap().folder, Some(fills));

  // folders move, but not inside themselves
  assert!(seq.move_folder(drums, Some(fills)).is_err());
  assert!(seq.move_folder(drums, Some(drums)).is_err());
  seq.move_folder(fills, None).unwrap();
  assert_eq!(seq.folder(fills).unwrap().parent, None);
  assert!(!seq.folder_within(fills, drums));
}

#[test]
fn test_removing_a_folder_takes_what_is_in_it() {
  let (mut seq, _, _, pattern) = kit();
  let outside = seq.add_pattern(1, 4).unwrap();
  let drums = seq.add_folder("Drums", None).unwrap();
  let fills = seq.add_folder("Fills", Some(drums)).unwrap();
  let keep = seq.add_folder("Keep", None).unwrap();
  seq.move_pattern(pattern, Some(fills)).unwrap();

  assert_eq!(seq.remove_folder(drums).unwrap(), vec![pattern]);
  assert!(seq.folder(drums).is_none() && seq.folder(fills).is_none());
  assert!(seq.folder(keep).is_some());
  assert!(seq.pattern(pattern).is_none());
  assert!(seq.pattern(outside).is_some());
  assert!(seq.remove_folder(drums).is_err());
}

#[test]
fn test_folders_are_saved_and_restored() {
  let (mut seq, _, _, pattern) = kit();
  let drums = seq.add_folder("Drums", None).unwrap();
  let fills = seq.add_folder("Fills", Some(drums)).unwrap();
  seq.move_pattern(pattern, Some(fills)).unwrap();

  let mut loaded = Sequencer::default();
  loaded.restore(seq.saved());
  assert_eq!(loaded.folders(), seq.folders());
  assert_eq!(loaded.pattern(pattern).unwrap().folder, Some(fills));
  // new ids carry on after the loaded ones
  assert!(loaded.add_folder("New", None).unwrap() > fills);
}

#[test]
fn test_a_sequencer_without_folders_is_saved_without_them() {
  let (seq, _, _, _) = kit();
  let json = serde_json::to_value(seq.saved()).unwrap();
  assert!(json.get("folders").is_none());
  assert!(json["patterns"][0].get("folder").is_none());
  assert!(json["patterns"][0].get("notes").is_none());

  let saved: SavedSequencer = serde_json::from_value(json).unwrap();
  let mut loaded = Sequencer::default();
  loaded.restore(saved);
  assert_eq!(loaded.pattern(0).unwrap().folder, None);
}

#[test]
fn test_broken_folders_are_mended_on_load() {
  let folder = |id, parent| PatternFolder { id, name: format!("f{}", id), parent };
  let saved = SavedSequencer {
    rows: vec![],
    patterns: vec![SavedPattern { id: 0, name: None, length: 16, floor: 16, notes: BTreeMap::new(), folder: Some(42) }],
    folders: vec![
      folder(1, Some(2)), folder(2, Some(1)), // a loop
      folder(3, Some(99)),                    // a parent that's gone
      folder(3, None),                        // an id used twice
    ],
  };
  let mut loaded = Sequencer::default();
  loaded.restore(saved);

  assert_eq!(loaded.folders().len(), 3);
  // the loop is broken, and each folder still reaches the top
  assert!(loaded.folders().iter().all(|f| !f.parent.map_or(false, |p| loaded.folder_within(p, f.id))));
  assert_eq!(loaded.folder(3).unwrap().parent, None);
  // the pattern's folder wasn't there: it's at the top
  assert_eq!(loaded.pattern(0).unwrap().folder, None);
}

#[test]
fn test_state_names_match_the_frontend() {
  let (mut seq, kick, _, pattern) = kit();
  let keys = seq.add_synth_row(synth(500, "Keys"));
  seq.set_step(pattern, kick, 2, true).unwrap();
  let json = serde_json::to_value(seq.state(&())).unwrap();
  assert_eq!(json["rows"][0], serde_json::json!({
    "id": kick, "label": null, "volume": 1.0, "pan": 0.0, "muted": false, "stepPitch": 60,
    "instrument": { "kind": "sample", "path": KICK, "samplePitch": 60, "gated": false },
  }));
  assert_eq!(json["rows"][2], serde_json::json!({
    "id": keys, "label": null, "volume": 1.0, "pan": 0.0, "muted": false, "stepPitch": 60,
    "instrument": {
      "kind": "synth", "slot": 500, "pluginId": "vst3:synth", "name": "Keys", "vendor": "Fake Audio",
      "missing": true, "hasEditor": false,
    },
  }));
  assert_eq!(json["patterns"][0], serde_json::json!({
    "id": pattern, "name": "Sequence 1", "length": 16, "floor": 16,
    "notes": { kick.to_string(): [{ "pitch": 60, "start": 2.0, "length": 1.0, "velocity": 1.0 }] },
  }));
  assert!(json.get("synths").is_none());

  let folder = seq.add_folder("Drums", None).unwrap();
  seq.move_pattern(pattern, Some(folder)).unwrap();
  seq.set_pattern_notes(pattern, kick, vec![]).unwrap();
  let json = serde_json::to_value(seq.state(&())).unwrap();
  assert_eq!(json["patterns"][0]["folder"], folder);
  // always given, even with none
  assert_eq!(json["patterns"][0]["notes"], serde_json::json!({}));
  assert_eq!(json["folders"][0], serde_json::json!({ "id": folder, "name": "Drums" }));
}

#[test]
fn test_split_a_pattern_by_row() {
  let (mut seq, kick, snare, pattern) = kit();
  for s in [0, 8] { seq.set_step(pattern, kick, s, true).unwrap(); }
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.rename_pattern(pattern, "verse").unwrap();
  // a note past the end, which isn't split off
  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  seq.set_step(pattern, kick, 20, true).unwrap();
  seq.set_pattern_bars(pattern, 1, 4).unwrap();
  let after = seq.add_pattern(1, 4).unwrap();

  let split = seq.split_pattern(pattern).unwrap();
  assert_eq!(split.len(), 2);
  let (kick_path, kick_name, kick_pattern) = &split[0];
  let (_, _, snare_pattern) = &split[1];
  assert_eq!(kick_path, KICK);
  assert_eq!(kick_name, "assets_66-bd-01");

  // each holds just its row's notes, named after its sample, after the source
  let order: Vec<u64> = seq.patterns().iter().map(|p| p.id).collect();
  assert_eq!(order, vec![pattern, *kick_pattern, *snare_pattern, after]);
  let k = seq.pattern(*kick_pattern).unwrap();
  assert_eq!(k.name.as_deref(), Some("verse (assets_66-bd-01)"));
  assert_eq!((k.length, k.floor), (16, 16));
  assert_eq!(k.notes_of(kick), &[step(0.), step(8.)]);
  assert!(!k.notes.contains_key(&snare));
  assert_eq!(seq.single_sample(*snare_pattern), Some(SNARE));
  assert_eq!(seq.single_sample(pattern), None);

  // the source is unchanged, and splitting it again reuses the same patterns
  assert!(seq.pattern(pattern).unwrap().lit(snare, 4));
  assert_eq!(seq.split_pattern(pattern).unwrap(), split);
  assert_eq!(seq.patterns().len(), 4);
}

#[test]
fn test_a_split_needs_notes_and_no_synth_notes() {
  let (mut seq, kick, _, pattern) = kit();
  assert!(seq.split_pattern(pattern).unwrap_err().contains("no notes to explode"));
  assert!(seq.split_pattern(99).is_err());

  let keys = seq.add_synth_row(synth(500, "Keys"));
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_pattern_notes(pattern, keys, vec![note(60, 4., 1.)]).unwrap();
  seq.rename_pattern(pattern, "verse").unwrap();
  assert_eq!(seq.split_pattern(pattern).unwrap_err(), "pattern verse has notes on synths, which can't be exploded yet");

  // a synth's notes past the end don't play, so they don't count
  seq.set_pattern_notes(pattern, keys, vec![note(60, 40., 1.)]).unwrap();
  assert_eq!(seq.split_pattern(pattern).unwrap().len(), 1);
  // nor is a pattern of synth notes alone a sample's
  let tune = seq.add_pattern(1, 4).unwrap();
  seq.set_pattern_notes(tune, keys, vec![note(60, 4., 1.)]).unwrap();
  assert_eq!(seq.single_sample(tune), None);
}

#[test]
fn test_a_split_is_named_after_a_rows_label() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  // (with the snare too, so the kick's is a new pattern)
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.rename_row(kick, "Kick").unwrap();
  let split = seq.split_pattern(pattern).unwrap();
  assert_eq!(split[0].1, "Kick");
  assert_eq!(seq.pattern(split[0].2).unwrap().name.as_deref(), Some("Sequence 1 (Kick)"));
}

#[test]
fn test_notes_are_tidied_and_sorted() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_pattern_notes(pattern, kick, vec![
    note(64, 4., 2.),
    note(60, 0., 1.5),
    // a pitch MIDI doesn't have, a start before the pattern's, a velocity too loud
    PatternNote { pitch: 200, start: -3., length: 4., velocity: 1.5 },
    // past the end, and across it: kept as they are
    note(62, 40., 2.),
    note(65, 15., 4.),
    // too short, and not long at all
    note(67, 2., 0.001),
    note(67, 3., -1.),
    // not numbers: left out
    note(68, f64::NAN, 1.),
    note(69, 1., f64::INFINITY),
    PatternNote { velocity: f32::NAN, ..note(70, 1., 1.) },
  ]).unwrap();

  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), &[
    note(60, 0., 1.5),
    PatternNote { pitch: 127, start: 0., length: 4., velocity: 1. },
    note(67, 2., MIN_NOTE),
    note(67, 3., MIN_NOTE),
    note(64, 4., 2.),
    note(65, 15., 4.),
    note(62, 40., 2.),
  ]);

  assert!(seq.set_pattern_notes(99, kick, vec![]).is_err());
  assert!(seq.set_pattern_notes(pattern, 99, vec![]).unwrap_err().contains("no row with id 99"));
  let too_many = vec![note(60, 0., 1.); MAX_NOTES + 1];
  assert_eq!(seq.set_pattern_notes(pattern, kick, too_many).unwrap_err(), "a pattern can hold at most 40000 notes for a row");
  // a refused edit changes nothing
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick).len(), 7);
  // no notes: the row isn't in the pattern
  seq.set_pattern_notes(pattern, kick, vec![]).unwrap();
  assert!(seq.pattern(pattern).unwrap().notes.is_empty());
}

#[test]
fn test_notes_start_no_later_than_128_bars_of_32_beats() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_pattern_notes(pattern, kick, vec![note(60, 1e9, 1.)]).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick)[0].start, 16_384.);
}

#[test]
fn test_notes_are_kept_past_a_shortened_patterns_end() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  let notes = vec![note(62, 0., 1.), note(61, 14., 4.), note(60, 20., 2.)];
  seq.set_pattern_notes(pattern, kick, notes.clone()).unwrap();

  let copy = seq.duplicate_pattern(pattern).unwrap();
  assert_eq!(seq.pattern(copy).unwrap().notes, seq.pattern(pattern).unwrap().notes);

  seq.set_pattern_bars(pattern, 1, 4).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), notes);
  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes_of(kick), notes);
}

#[test]
fn test_synth_rows_have_unique_names_and_take_their_notes_when_they_go() {
  let (mut seq, kick, _, pattern) = kit();
  let first = seq.add_synth_row(synth(500, "Synth"));
  let second = seq.add_synth_row(synth(501, "synth"));
  let third = seq.add_synth_row(synth(502, "Synth"));
  let names: Vec<String> = seq.rows().iter().map(Row::name).collect();
  assert_eq!(names, vec!["assets_66-bd-01", "assets_66-sd-01", "Synth", "synth 2", "Synth 3"]);
  // a label isn't a synth's name
  seq.rename_row(third, "Lead").unwrap();
  assert_eq!(seq.row(third).unwrap().name(), "Lead");
  assert_eq!(seq.free_synth_name("Synth"), "Synth 4");

  let other = seq.add_pattern(4, 4).unwrap();
  seq.set_pattern_notes(pattern, first, vec![note(60, 0., 1.)]).unwrap();
  seq.set_pattern_notes(pattern, second, vec![note(62, 0., 1.)]).unwrap();
  seq.set_pattern_notes(other, first, vec![note(64, 0., 1.)]).unwrap();
  seq.set_step(other, kick, 0, true).unwrap();
  seq.remove_row(first).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes.keys().copied().collect::<Vec<_>>(), vec![second]);
  assert_eq!(seq.pattern(other).unwrap().notes.keys().copied().collect::<Vec<_>>(), vec![kick]);
  assert!(seq.remove_row(first).is_err());

  // a synth row's volume and mute are its plugin's
  seq.set_row_volume(second, 9.).unwrap();
  seq.set_row_muted(second, true).unwrap();
  let synths: Vec<(u64, f32, bool)> = seq.synths().iter().map(|s| (s.id, s.volume, s.muted)).collect();
  assert_eq!(synths, vec![(501, ROW_VOLUME_MAX, true), (502, ROW_VOLUME, false)]);
}

#[test]
fn test_synth_notes_go_by_slot_and_sample_rows_have_none() {
  let (mut seq, kick, _, pattern) = kit();
  let keys = seq.add_synth_row(synth(500, "Keys"));
  let bass = seq.add_synth_row(synth(400, "Bass"));
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_pattern_notes(pattern, bass, vec![note(40, 0., 1.)]).unwrap();
  seq.set_pattern_notes(pattern, keys, vec![note(72, 2., 1.)]).unwrap();
  let p = seq.pattern(pattern).unwrap();
  // in row order, by slot
  let notes: Vec<(u64, Vec<PatternNote>)> = seq.synth_notes(p).map(|(slot, notes)| (slot, notes.to_vec())).collect();
  assert_eq!(notes, vec![(500, vec![note(72, 2., 1.)]), (400, vec![note(40, 0., 1.)])]);
  // and a synth row isn't a sample hit
  assert_eq!(hit_starts(&seq, pattern), vec![0.]);
}

#[test]
fn test_synth_rows_are_saved_and_restored() {
  let (mut seq, _, _, pattern) = kit();
  let keys = seq.add_synth_row(synth(500, "Keys"));
  let notes = vec![note(48, 0., 8.), note(72, 3.5, 0.25)];
  seq.set_pattern_notes(pattern, keys, notes.clone()).unwrap();
  seq.set_row_volume(keys, 0.5).unwrap();
  let empty = seq.add_pattern(1, 4).unwrap();

  let json = serde_json::to_value(seq.saved()).unwrap();
  assert_eq!(json["rows"][2], serde_json::json!({
    "id": keys, "volume": 0.5, "muted": false, "stepPitch": 60,
    "instrument": {
      "kind": "synth", "slot": 500, "pluginId": "vst3:synth", "name": "Keys", "vendor": "Fake Audio",
      "path": "/fake/Synth.vst3", "state": "",
    },
  }));
  assert_eq!(json["rows"][0]["instrument"], serde_json::json!({ "kind": "sample", "path": KICK, "samplePitch": 60, "gated": false }));
  assert_eq!(json["patterns"][0]["notes"][keys.to_string()][1], serde_json::json!({ "pitch": 72, "start": 3.5, "length": 0.25, "velocity": 0.8f32 }));
  assert_eq!(json["patterns"][0]["floor"], 16);
  // a pattern without notes is saved without them
  assert!(json["patterns"][1].get("notes").is_none());

  let mut loaded = Sequencer::default();
  loaded.restore(serde_json::from_value(json.clone()).unwrap());
  assert!(loaded.same_as(&seq));
  assert!(loaded.pattern(empty).unwrap().notes.is_empty());

  // given new slots, the notes stay with their row
  let mut saved: SavedSequencer = serde_json::from_value(json).unwrap();
  saved.renumber_slots(|| 900);
  loaded.restore(saved);
  assert_eq!(loaded.row(keys).unwrap().synth().unwrap().slot, 900);
  assert_eq!(loaded.pattern(pattern).unwrap().notes_of(keys), notes);
}

#[test]
fn test_pan_is_kept_in_range() {
  let (mut seq, kick, _, _) = kit();
  seq.set_row_pan(kick, -3.).unwrap();
  assert_eq!(seq.row(kick).unwrap().pan, -1.);
  seq.set_row_pan(kick, f32::NAN).unwrap();
  assert_eq!(seq.row(kick).unwrap().pan, 0.);
  assert!(seq.set_row_pan(99, 0.5).is_err());
}

#[test]
fn test_a_row_is_panned_in_stereo() {
  let (mut seq, kick, _, _) = kit();
  let played = |seq: &Sequencer| -> Vec<f32> { seq.row(kick).unwrap().one_shot().unwrap().source().collect() };
  let centered = played(&seq);
  assert_eq!(seq.row(kick).unwrap().one_shot().unwrap().source().channels(), 2);

  // hard left: the left side as it was, the right silent
  seq.set_row_pan(kick, -1.).unwrap();
  let left = played(&seq);
  assert_eq!(left.len(), centered.len());
  assert!(left.chunks(2).zip(centered.chunks(2)).all(|(l, c)| l[0] == c[0] && l[1] == 0.));
  assert!(centered.iter().any(|s| *s != 0.));

  // half right: the right as it was, the left at half
  seq.set_row_pan(kick, 0.5).unwrap();
  let right = played(&seq);
  assert!(right.chunks(2).zip(centered.chunks(2)).all(|(r, c)| r[1] == c[1] && (r[0] - c[0] * 0.5).abs() < 1e-6));

  // a synth row has no sample to play
  let keys = seq.add_synth_row(synth(500, "Keys"));
  assert!(seq.row(keys).unwrap().one_shot().is_none());
}

#[test]
fn test_rows_are_labelled() {
  let (mut seq, kick, _, _) = kit();
  assert_eq!(seq.row(kick).unwrap().name(), "assets_66-bd-01");
  seq.rename_row(kick, "  Kick  ").unwrap();
  assert_eq!(seq.row(kick).unwrap().label.as_deref(), Some("Kick"));
  assert_eq!(seq.row(kick).unwrap().name(), "Kick");
  // cut to ROW_LABEL_MAX characters
  seq.rename_row(kick, &"é".repeat(60)).unwrap();
  assert_eq!(seq.row(kick).unwrap().label.as_ref().unwrap().chars().count(), ROW_LABEL_MAX);
  // a blank label goes back to the sample's name
  seq.rename_row(kick, "   ").unwrap();
  assert_eq!(seq.row(kick).unwrap().label, None);
  assert!(seq.rename_row(99, "x").is_err());
}

#[test]
fn test_rows_of_both_instruments_are_moved() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  let keys = seq.add_synth_row(synth(500, "Keys"));
  let order = |seq: &Sequencer| seq.rows().iter().map(|r| r.id).collect::<Vec<_>>();

  seq.move_row(keys, 0).unwrap();
  assert_eq!(order(&seq), vec![keys, kick, snare]);
  seq.move_row(keys, 1).unwrap();
  assert_eq!(order(&seq), vec![kick, keys, snare]);
  // past the end: the bottom
  seq.move_row(kick, 99).unwrap();
  assert_eq!(order(&seq), vec![keys, snare, kick]);
  // notes go with the row
  assert!(seq.pattern(pattern).unwrap().lit(kick, 0));
  assert!(seq.move_row(99, 0).is_err());
}

#[test]
fn test_a_replaced_sample_keeps_the_row() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.rename_row(kick, "Boom").unwrap();
  seq.set_row_pan(kick, 0.25).unwrap();
  seq.set_row_muted(kick, true).unwrap();

  seq.replace_row_sample(kick, SNARE, RowSound::load(SNARE).unwrap()).unwrap();
  let row = seq.row(kick).unwrap();
  assert_eq!((row.sample().unwrap().path.as_str(), row.label.as_deref(), row.pan, row.muted), (SNARE, Some("Boom"), 0.25, true));
  assert_eq!(seq.rows()[0].id, kick);
  assert!(seq.pattern(pattern).unwrap().lit(kick, 0));
  assert_eq!(row.one_shot().unwrap().duration(), RowSound::load(SNARE).unwrap().duration);

  // a synth row has no sample to replace
  let keys = seq.add_synth_row(synth(500, "Keys"));
  let refused = seq.replace_row_sample(keys, SNARE, RowSound::load(SNARE).unwrap()).unwrap_err();
  assert!(refused.contains("plays a synth"), "{}", refused);
}

#[test]
fn test_labels_and_pan_are_saved_and_restored() {
  let (mut seq, kick, snare, _) = kit();
  seq.rename_row(kick, "Kick").unwrap();
  seq.set_row_pan(snare, 0.3).unwrap();

  let json = serde_json::to_value(seq.saved()).unwrap();
  assert_eq!(json["rows"][0]["label"], "Kick");
  // left out when there's none, or it's centered
  assert!(json["rows"][0].get("pan").is_none());
  assert!(json["rows"][1].get("label").is_none());

  let mut loaded = Sequencer::default();
  assert!(loaded.restore(seq.saved()).is_empty());
  assert!(loaded.same_as(&seq));
  assert_eq!(loaded.row(snare).unwrap().pan, 0.3);

  // a row saved with only what it must have
  let bare: SavedRow = serde_json::from_value(serde_json::json!({
    "id": 0, "instrument": { "kind": "sample", "path": KICK },
  })).unwrap();
  assert_eq!((bare.label, bare.volume, bare.pan, bare.muted, bare.step_pitch), (None, ROW_VOLUME, 0., false, DEFAULT_PITCH));
  assert_eq!(bare.instrument, SavedInstrument::Sample(SavedSample { path: KICK.into(), sample_pitch: DEFAULT_PITCH, gated: false }));
}

#[test]
fn test_new_patterns_take_the_lowest_free_number() {
  let mut seq = Sequencer::default();
  let names = |seq: &Sequencer| seq.patterns().iter().map(|p| p.label()).collect::<Vec<_>>();
  let first = seq.add_pattern(1, 4).unwrap();
  let second = seq.add_pattern(1, 4).unwrap();
  assert_eq!(names(&seq), vec!["Sequence 1", "Sequence 2"]);

  // a gap is filled first
  seq.remove_pattern(first).unwrap();
  seq.add_pattern(1, 4).unwrap();
  assert_eq!(names(&seq).last().unwrap(), "Sequence 1");

  // a pattern renamed off its number frees it; one renamed onto a number
  // takes it (case aside)
  seq.rename_pattern(second, "verse").unwrap();
  let fill = seq.add_pattern(1, 4).unwrap();
  assert_eq!(seq.pattern(fill).unwrap().label(), "Sequence 2");
  seq.rename_pattern(fill, "sequence 3").unwrap();
  assert_eq!(seq.free_pattern_name(), "Sequence 2");
  seq.rename_pattern(second, "Sequence 2").unwrap();
  assert_eq!(seq.free_pattern_name(), "Sequence 4");
}

#[test]
fn test_unnamed_patterns_are_shown_by_their_number() {
  let mut seq = Sequencer::default();
  let unnamed = |id| SavedPattern { id, name: None, length: 16, floor: 16, notes: BTreeMap::new(), folder: None };
  seq.restore(SavedSequencer { patterns: vec![unnamed(1), unnamed(2)], ..Default::default() });
  assert_eq!(seq.pattern(1).unwrap().label(), "Sequence 2");
  assert_eq!(seq.pattern(2).unwrap().label(), "Sequence 3");

  // a new pattern doesn't take a number an old unnamed one is shown with
  let a = seq.add_pattern(1, 4).unwrap();
  let b = seq.add_pattern(1, 4).unwrap();
  assert_eq!(seq.pattern(a).unwrap().label(), "Sequence 1");
  assert_eq!(seq.pattern(b).unwrap().label(), "Sequence 4");

  // and a pattern renamed blank goes back to its number
  seq.rename_pattern(a, " ").unwrap();
  assert_eq!(seq.pattern(a).unwrap().label(), format!("Sequence {}", a + 1));
}

#[test]
fn test_a_copy_shares_and_an_edit_copies_only_what_it_changes() {
  let (mut seq, kick, snare, pattern) = kit();
  let other = seq.add_pattern(1, 4).unwrap();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.set_step(other, kick, 0, true).unwrap();
  let before = seq.clone();
  assert!(seq.same_as(&before));

  seq.set_step(pattern, kick, 8, true).unwrap();
  assert!(!seq.same_as(&before));
  // the other pattern, the snare's notes and the rows are still shared
  assert!(Arc::ptr_eq(&seq.patterns()[1], &before.patterns()[1]));
  let (now, was) = (seq.pattern(pattern).unwrap(), before.pattern(pattern).unwrap());
  assert!(Arc::ptr_eq(&now.notes[&snare], &was.notes[&snare]));
  assert!(!Arc::ptr_eq(&now.notes[&kick], &was.notes[&kick]));
  assert!(Arc::ptr_eq(&seq.rows, &before.rows));

  // a step that's already so, or an edit refused, copies nothing
  let before = seq.clone();
  seq.set_step(pattern, kick, 8, true).unwrap();
  assert!(seq.set_row_volume(99, 0.5).is_err());
  assert!(Arc::ptr_eq(&seq.patterns()[0], &before.patterns()[0]));
  assert!(Arc::ptr_eq(&seq.rows, &before.rows));

  // nor does repointing samples that haven't moved
  assert!(!seq.repoint_rows(&|_| None));
  assert!(Arc::ptr_eq(&seq.rows, &before.rows));
  // and the same content in new lists is still the same
  seq.set_row_volume(kick, 1.).unwrap();
  assert!(!Arc::ptr_eq(&seq.rows, &before.rows));
  assert!(seq.same_as(&before));
}
