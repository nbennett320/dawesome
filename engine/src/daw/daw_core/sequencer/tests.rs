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
  let pattern = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  (seq, kick, snare, pattern)
}

fn hit_times(seq: &Sequencer, pattern: u64) -> Vec<(u64, Duration)> {
  seq.hits(pattern, BEAT).into_iter().map(|(row, at)| (row.id, at)).collect()
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
  assert_eq!(p.length, 16);
  assert_eq!(p.name.as_deref(), Some("Loop 1"));
  assert!(p.steps.is_empty());
  assert_eq!(seq.pattern_len(pattern, BEAT), Some(ms(2_000)));
}

#[test]
fn test_bars_must_be_a_choice() {
  let (mut seq, _, _, pattern) = kit();
  assert!(seq.add_pattern(3, 4, PatternKind::Drum).is_err());
  assert!(seq.set_pattern_bars(pattern, 0, 4).is_err());
  seq.set_pattern_bars(pattern, 4, 4).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().length, 64);
}

#[test]
fn test_hits_follow_the_lit_steps() {
  let (mut seq, kick, snare, pattern) = kit();
  for step in [0, 8] { seq.set_step(pattern, kick, step, true).unwrap(); }
  seq.set_step(pattern, snare, 4, true).unwrap();

  assert_eq!(hit_times(&seq, pattern), vec![(kick, ms(0)), (kick, ms(1_000)), (snare, ms(500))]);

  // clearing a step takes its hit out
  seq.set_step(pattern, kick, 8, false).unwrap();
  assert_eq!(hit_times(&seq, pattern), vec![(kick, ms(0)), (snare, ms(500))]);
}

#[test]
fn test_muted_rows_dont_play() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.set_row_muted(kick, true).unwrap();
  assert_eq!(hit_times(&seq, pattern), vec![(snare, ms(500))]);
}

#[test]
fn test_steps_past_the_end_come_back() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  seq.set_step(pattern, kick, 20, true).unwrap();

  // shortened: the step is kept, but doesn't play
  seq.set_pattern_bars(pattern, 1, 4).unwrap();
  assert!(hit_times(&seq, pattern).is_empty());
  assert!(seq.set_step(pattern, kick, 20, true).is_err());

  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  assert_eq!(hit_times(&seq, pattern), vec![(kick, ms(2_500))]);
}

#[test]
fn test_removing_a_row_takes_its_steps() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.remove_row(kick).unwrap();

  assert_eq!(seq.rows().len(), 1);
  assert!(!seq.pattern(pattern).unwrap().steps.contains_key(&kick));
  assert!(seq.set_step(pattern, kick, 0, true).is_err());
}

#[test]
fn test_rename_and_duplicate() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_step(pattern, kick, 3, true).unwrap();
  seq.rename_pattern(pattern, "  verse ").unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().name.as_deref(), Some("verse"));

  let other = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  let copy = seq.duplicate_pattern(pattern).unwrap();
  // straight after the original, with its steps
  let order: Vec<u64> = seq.patterns().iter().map(|p| p.id).collect();
  assert_eq!(order, vec![pattern, copy, other]);
  assert_eq!(seq.pattern(copy).unwrap().name.as_deref(), Some("verse copy"));
  assert!(seq.pattern(copy).unwrap().lit(kick, 3));

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
  assert_ne!(seq.add_pattern(1, 4, PatternKind::Drum).unwrap(), pattern);
}

#[test]
fn test_volume_is_kept_in_range() {
  let (mut seq, kick, _, _) = kit();
  seq.set_row_volume(kick, 9.).unwrap();
  assert_eq!(seq.row(kick).unwrap().volume, ROW_VOLUME_MAX);
  seq.set_row_volume(kick, f32::NAN).unwrap();
  assert_eq!(seq.row(kick).unwrap().volume, ROW_VOLUME);
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
  saved.rows[1].path = "assets/nope.wav".into();

  let mut loaded = Sequencer::default();
  let missing = loaded.restore(saved);
  assert_eq!(missing.len(), 1);
  assert_eq!(missing[0].0, "assets/nope.wav");
  assert_eq!(loaded.rows().len(), 1);
  // its steps went with it
  assert_eq!(hit_times(&loaded, pattern), vec![(kick, ms(0))]);
  assert_eq!(loaded.pattern(pattern).unwrap().name.as_deref(), Some("beat"));
  // new ids carry on after the loaded ones
  assert!(loaded.add_pattern(1, 4, PatternKind::Drum).unwrap() > pattern);
  assert!(loaded.add_row(KICK).unwrap() > snare);
}

#[test]
fn test_folders() {
  let (mut seq, _, _, pattern) = kit();
  let drums = seq.add_folder("  Drums ", None, PatternKind::Drum).unwrap();
  let fills = seq.add_folder("Fills", Some(drums), PatternKind::Drum).unwrap();
  assert_eq!(seq.folder(drums).unwrap().name, "Drums");
  assert_eq!(seq.folder(fills).unwrap().parent, Some(drums));

  // names: not blank, and not twice in a folder (case aside)
  assert!(seq.add_folder("  ", None, PatternKind::Drum).is_err());
  assert!(seq.add_folder("drums", None, PatternKind::Drum).is_err());
  assert!(seq.add_folder("Drums", Some(drums), PatternKind::Drum).is_ok());
  assert!(seq.add_folder("x", Some(99), PatternKind::Drum).is_err());
  assert!(seq.rename_folder(fills, "FILLS").is_ok());
  let other = seq.add_folder("Other", Some(drums), PatternKind::Drum).unwrap();
  assert!(seq.rename_folder(other, "fills").is_err());

  // patterns go in folders, and back to the top
  seq.move_pattern(pattern, Some(fills)).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().folder, Some(fills));
  assert!(seq.move_pattern(pattern, Some(99)).is_err());
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
  let outside = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  let drums = seq.add_folder("Drums", None, PatternKind::Drum).unwrap();
  let fills = seq.add_folder("Fills", Some(drums), PatternKind::Drum).unwrap();
  let keep = seq.add_folder("Keep", None, PatternKind::Drum).unwrap();
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
  let drums = seq.add_folder("Drums", None, PatternKind::Drum).unwrap();
  let fills = seq.add_folder("Fills", Some(drums), PatternKind::Drum).unwrap();
  seq.move_pattern(pattern, Some(fills)).unwrap();

  let mut loaded = Sequencer::default();
  loaded.restore(seq.saved());
  assert_eq!(loaded.folders(), seq.folders());
  assert_eq!(loaded.pattern(pattern).unwrap().folder, Some(fills));
  // new ids carry on after the loaded ones
  assert!(loaded.add_folder("New", None, PatternKind::Drum).unwrap() > fills);
}

#[test]
fn test_a_project_without_folders_still_loads() {
  let json = serde_json::json!({
    "rows": [],
    "patterns": [{ "id": 0, "name": null, "length": 16, "steps": {} }],
  });
  let saved: SavedSequencer = serde_json::from_value(json).unwrap();
  assert!(saved.folders.is_empty());
  let mut loaded = Sequencer::default();
  loaded.restore(saved);
  assert_eq!(loaded.pattern(0).unwrap().folder, None);

  // and one without folders is saved as before
  let json = serde_json::to_value(loaded.saved()).unwrap();
  assert!(json.get("folders").is_none());
  assert!(json["patterns"][0].get("folder").is_none());
}

#[test]
fn test_broken_folders_are_mended_on_load() {
  let folder = |id, parent| PatternFolder { id, name: format!("f{}", id), kind: PatternKind::Drum, parent };
  let pattern = Pattern { id: 0, name: None, kind: PatternKind::Drum, length: 16, steps: BTreeMap::new(), folder: Some(42), notes: BTreeMap::new() };
  let saved = SavedSequencer {
    rows: vec![],
    synths: vec![],
    patterns: vec![pattern],
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
  seq.set_step(pattern, kick, 2, true).unwrap();
  let json = serde_json::to_value(seq.state(&())).unwrap();
  assert_eq!(json["rows"][0]["path"], KICK);
  assert_eq!(json["rows"][0]["muted"], false);
  assert_eq!(json["patterns"][0]["length"], 16);
  assert_eq!(json["patterns"][0]["steps"][kick.to_string()], serde_json::json!([2]));
  let folder = seq.add_folder("Drums", None, PatternKind::Drum).unwrap();
  seq.move_pattern(pattern, Some(folder)).unwrap();
  let json = serde_json::to_value(seq.state(&())).unwrap();
  assert_eq!(json["patterns"][0]["folder"], folder);
  assert_eq!(json["folders"][0], serde_json::json!({ "id": folder, "name": "Drums", "kind": "drum" }));
  assert_eq!(json["patterns"][0]["kind"], "drum");
  assert_eq!(json["patterns"][0]["notes"], serde_json::json!({}));
  assert_eq!(json["synths"], serde_json::json!([]));
}

#[test]
fn test_split_a_pattern_by_row() {
  let (mut seq, kick, snare, pattern) = kit();
  for step in [0, 8] { seq.set_step(pattern, kick, step, true).unwrap(); }
  seq.set_step(pattern, snare, 4, true).unwrap();
  seq.rename_pattern(pattern, "verse").unwrap();
  let after = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();

  let split = seq.split_pattern(pattern).unwrap();
  assert_eq!(split.len(), 2);
  let (kick_path, kick_name, kick_pattern) = &split[0];
  let (_, _, snare_pattern) = &split[1];
  assert_eq!(kick_path, KICK);
  assert_eq!(kick_name, "assets_66-bd-01");

  // each holds just its row's steps, named after its sample, after the source
  let order: Vec<u64> = seq.patterns().iter().map(|p| p.id).collect();
  assert_eq!(order, vec![pattern, *kick_pattern, *snare_pattern, after]);
  let k = seq.pattern(*kick_pattern).unwrap();
  assert_eq!(k.name.as_deref(), Some("verse (assets_66-bd-01)"));
  assert_eq!(k.length, 16);
  assert!(k.lit(kick, 0) && k.lit(kick, 8) && !k.lit(snare, 4));
  assert_eq!(seq.single_sample(*snare_pattern), Some(SNARE));
  assert_eq!(seq.single_sample(pattern), None);

  // the source is unchanged, and splitting it again reuses the same patterns
  assert!(seq.pattern(pattern).unwrap().lit(snare, 4));
  assert_eq!(seq.split_pattern(pattern).unwrap(), split);
  assert_eq!(seq.patterns().len(), 4);
}

fn note(pitch: u8, start: f64, length: f64) -> PatternNote {
  PatternNote { pitch, start, length, velocity: 0.8 }
}

fn synth(id: u64, name: &str) -> Synth {
  Synth {
    id,
    plugin_id: "vst3:synth".into(),
    path: "/fake/Synth.vst3".into(),
    name: name.into(),
    vendor: "Fake Audio".into(),
    volume: 1.,
    muted: false,
  }
}

// a sequencer with a synth (id 50) and a one bar note pattern
fn notes_kit() -> (Sequencer, u64, u64) {
  let mut seq = Sequencer::default();
  seq.add_synth(synth(50, "Synth")).unwrap();
  let pattern = seq.add_pattern(1, 4, PatternKind::Note).unwrap();
  (seq, 50, pattern)
}

#[test]
fn test_a_note_pattern_is_1_to_64_bars_and_a_drum_pattern_1_2_or_4() {
  let mut seq = Sequencer::default();
  assert!(seq.add_pattern(3, 4, PatternKind::Drum).is_err());
  let notes = seq.add_pattern(3, 4, PatternKind::Note).unwrap();
  assert_eq!(seq.pattern(notes).unwrap().length, 48);
  seq.set_pattern_bars(notes, 64, 4).unwrap();
  assert!(seq.set_pattern_bars(notes, 65, 4).is_err());
  assert!(seq.set_pattern_bars(notes, 0, 4).is_err());
  let drums = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  assert!(seq.set_pattern_bars(drums, 3, 4).is_err());
}

#[test]
fn test_notes_are_tidied_and_kept_in_the_order_given() {
  let (mut seq, s, pattern) = notes_kit();
  seq.set_pattern_notes(pattern, s, vec![
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

  let notes = &seq.pattern(pattern).unwrap().notes[&s];
  assert_eq!(notes, &vec![
    note(64, 4., 2.),
    note(60, 0., 1.5),
    PatternNote { pitch: 127, start: 0., length: 4., velocity: 1. },
    note(62, 40., 2.),
    note(65, 15., 4.),
    note(67, 2., MIN_NOTE),
    note(67, 3., MIN_NOTE),
  ]);

  assert!(seq.set_pattern_notes(99, s, vec![]).is_err());
  assert!(seq.set_pattern_notes(pattern, 99, vec![]).is_err());
  let too_many = vec![note(60, 0., 1.); MAX_NOTES + 1];
  assert!(seq.set_pattern_notes(pattern, s, too_many).unwrap_err().contains("at most"));
  // a refused edit changes nothing
  assert_eq!(seq.pattern(pattern).unwrap().notes[&s].len(), 7);
  // no notes: the synth isn't in the pattern
  seq.set_pattern_notes(pattern, s, vec![]).unwrap();
  assert!(seq.pattern(pattern).unwrap().notes.is_empty());
}

#[test]
fn test_a_pattern_is_drums_or_notes_never_both() {
  let (mut seq, s, notes) = notes_kit();
  let kick = seq.add_row(KICK).unwrap();
  let drums = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  assert!(seq.set_pattern_notes(drums, s, vec![note(60, 0., 1.)]).is_err());
  assert!(seq.set_step(notes, kick, 0, true).is_err());
  seq.set_pattern_notes(notes, s, vec![note(60, 0., 1.)]).unwrap();
  assert!(seq.split_pattern(notes).unwrap_err().contains("note pattern"));
  assert!(seq.hit_steps(notes).is_empty());

  // and folders hold one kind
  let drum_folder = seq.add_folder("Drums", None, PatternKind::Drum).unwrap();
  let note_folder = seq.add_folder("Keys", None, PatternKind::Note).unwrap();
  assert!(seq.move_pattern(notes, Some(drum_folder)).is_err());
  seq.move_pattern(notes, Some(note_folder)).unwrap();
  assert!(seq.add_folder("Fills", Some(note_folder), PatternKind::Drum).is_err());
  assert!(seq.move_folder(drum_folder, Some(note_folder)).is_err());
  let json = serde_json::to_value(seq.state(&())).unwrap();
  assert_eq!(json["folders"][1]["kind"], "note");
  assert_eq!(json["patterns"][0]["kind"], "note");
  assert_eq!(json["patterns"][0]["steps"], serde_json::json!({}));
  assert_eq!(json["patterns"][0]["notes"][s.to_string()][0]["pitch"], 60);
}

#[test]
fn test_notes_are_kept_past_a_shortened_patterns_end() {
  let (mut seq, s, pattern) = notes_kit();
  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  let notes = vec![note(60, 20., 2.), note(61, 14., 4.), note(62, 0., 1.)];
  seq.set_pattern_notes(pattern, s, notes.clone()).unwrap();

  let copy = seq.duplicate_pattern(pattern).unwrap();
  assert_eq!(seq.pattern(copy).unwrap().notes, seq.pattern(pattern).unwrap().notes);

  seq.set_pattern_bars(pattern, 1, 4).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes[&s], notes);
  seq.set_pattern_bars(pattern, 2, 4).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes[&s], notes);
}

#[test]
fn test_synths_have_unique_names_and_take_their_notes_when_they_go() {
  let (mut seq, s, pattern) = notes_kit();
  seq.add_synth(synth(51, "synth")).unwrap();
  seq.add_synth(synth(52, "Synth")).unwrap();
  assert!(seq.add_synth(synth(52, "Other")).is_err());
  let names: Vec<&str> = seq.synths().iter().map(|s| s.name.as_str()).collect();
  assert_eq!(names, vec!["Synth", "synth 2", "Synth 3"]);

  let other = seq.add_pattern(4, 4, PatternKind::Note).unwrap();
  seq.set_pattern_notes(pattern, s, vec![note(60, 0., 1.)]).unwrap();
  seq.set_pattern_notes(pattern, 51, vec![note(62, 0., 1.)]).unwrap();
  seq.set_pattern_notes(other, s, vec![note(64, 0., 1.)]).unwrap();
  seq.remove_synth(s).unwrap();
  assert_eq!(seq.pattern(pattern).unwrap().notes.keys().copied().collect::<Vec<_>>(), vec![51]);
  assert!(seq.pattern(other).unwrap().notes.is_empty());
  assert!(seq.remove_synth(s).is_err());

  seq.set_synth_volume(51, 9.).unwrap();
  seq.set_synth_muted(51, true).unwrap();
  assert_eq!(seq.synth(51).map(|s| (s.volume, s.muted)), Some((ROW_VOLUME_MAX, true)));
  assert!(seq.set_synth_volume(99, 1.).is_err());
}

#[test]
fn test_synths_and_notes_are_saved_and_restored() {
  let (mut seq, s, pattern) = notes_kit();
  let notes = vec![note(72, 3.5, 0.25), note(48, 0., 8.)];
  seq.set_pattern_notes(pattern, s, notes.clone()).unwrap();
  seq.set_synth_volume(s, 0.5).unwrap();
  let empty = seq.add_pattern(1, 4, PatternKind::Note).unwrap();

  let json = serde_json::to_value(seq.saved()).unwrap();
  assert_eq!(json["synths"][0]["name"], "Synth");
  assert_eq!(json["synths"][0]["volume"], 0.5);
  assert_eq!(json["patterns"][0]["kind"], "note");
  assert_eq!(json["patterns"][0]["notes"][s.to_string()][0], serde_json::json!({ "pitch": 72, "start": 3.5, "length": 0.25, "velocity": 0.8f32 }));
  // a pattern without notes is saved without them
  assert!(json["patterns"][1].get("notes").is_none());

  let mut loaded = Sequencer::default();
  loaded.restore(serde_json::from_value(json.clone()).unwrap());
  assert_eq!(loaded.pattern(pattern).unwrap().notes[&s], notes);
  assert!(loaded.pattern(empty).unwrap().notes.is_empty());
  assert_eq!(loaded.synth(s).unwrap().volume, 0.5);

  // renumbered, the notes follow their synth; notes for a synth that
  // isn't there are left out
  let mut saved: SavedSequencer = serde_json::from_value(json).unwrap();
  saved.patterns[0].notes.insert(77, vec![note(60, 0., 1.)]);
  saved.renumber_synths(|| 900);
  loaded.restore(saved);
  assert_eq!(loaded.synths()[0].id, 900);
  assert_eq!(loaded.pattern(pattern).unwrap().notes.keys().copied().collect::<Vec<_>>(), vec![900]);
}

#[test]
fn test_a_drum_only_project_is_saved_as_before() {
  let (mut seq, kick, _, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  seq.add_folder("Drums", None, PatternKind::Drum).unwrap();
  let json = serde_json::to_value(seq.saved()).unwrap();
  assert!(json.get("synths").is_none());
  assert!(json["patterns"][0].get("kind").is_none());
  assert!(json["patterns"][0].get("notes").is_none());
  assert!(json["folders"][0].get("kind").is_none());
}

#[test]
fn test_split_needs_steps() {

  let (mut seq, _, _, pattern) = kit();
  assert!(seq.split_pattern(pattern).is_err());
  assert!(seq.split_pattern(99).is_err());
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
  assert_eq!(seq.pattern(split[0].2).unwrap().name.as_deref(), Some("Loop 1 (Kick)"));
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
  let centerd: Vec<f32> = seq.row(kick).unwrap().source().collect();
  assert_eq!(seq.row(kick).unwrap().source().channels(), 2);

  // hard left: the left side as it was, the right silent
  seq.set_row_pan(kick, -1.).unwrap();
  let left: Vec<f32> = seq.row(kick).unwrap().source().collect();
  assert_eq!(left.len(), centerd.len());
  assert!(left.chunks(2).zip(centerd.chunks(2)).all(|(l, c)| l[0] == c[0] && l[1] == 0.));
  assert!(centerd.iter().any(|s| *s != 0.));

  // half right: the right as it was, the left at half
  seq.set_row_pan(kick, 0.5).unwrap();
  let right: Vec<f32> = seq.row(kick).unwrap().source().collect();
  assert!(right.chunks(2).zip(centerd.chunks(2)).all(|(r, c)| r[1] == c[1] && (r[0] - c[0] * 0.5).abs() < 1e-6));
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
fn test_rows_are_moved() {
  let (mut seq, kick, snare, pattern) = kit();
  seq.set_step(pattern, kick, 0, true).unwrap();
  let hat = seq.add_row(KICK).unwrap();
  let order = |seq: &Sequencer| seq.rows().iter().map(|r| r.id).collect::<Vec<_>>();

  seq.move_row(hat, 0).unwrap();
  assert_eq!(order(&seq), vec![hat, kick, snare]);
  seq.move_row(hat, 1).unwrap();
  assert_eq!(order(&seq), vec![kick, hat, snare]);
  // past the end: the bottom
  seq.move_row(kick, 99).unwrap();
  assert_eq!(order(&seq), vec![hat, snare, kick]);
  // steps go with the row
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

  seq.replace_row_sample(kick, SNARE, RowSample::load(SNARE).unwrap()).unwrap();
  let row = seq.row(kick).unwrap();
  assert_eq!((row.path.as_str(), row.label.as_deref(), row.pan, row.muted), (SNARE, Some("Boom"), 0.25, true));
  assert_eq!(seq.rows()[0].id, kick);
  assert!(seq.pattern(pattern).unwrap().lit(kick, 0));
  assert_eq!(row.duration(), RowSample::load(SNARE).unwrap().duration);
}

#[test]
fn test_labels_and_pan_are_saved_and_restored() {
  let (mut seq, kick, snare, _) = kit();
  seq.rename_row(kick, "Kick").unwrap();
  seq.set_row_pan(snare, 0.3).unwrap();

  let json = serde_json::to_value(seq.saved()).unwrap();
  assert_eq!(json["rows"][0]["label"], "Kick");
  // left out when there's none, or it's centerd: older projects look the same
  assert!(json["rows"][0].get("pan").is_none());
  assert!(json["rows"][1].get("label").is_none());

  let mut loaded = Sequencer::default();
  assert!(loaded.restore(seq.saved()).is_empty());
  assert!(loaded.same_as(&seq));
  assert_eq!(loaded.row(snare).unwrap().pan, 0.3);

  // a project from before labels and pan
  let old: SavedRow = serde_json::from_value(serde_json::json!({ "id": 0, "path": KICK, "volume": 1.0 })).unwrap();
  assert_eq!((old.label, old.pan), (None, 0.));
}

#[test]
fn test_new_patterns_take_the_lowest_free_number_of_their_kind() {
  let mut seq = Sequencer::default();
  let names = |seq: &Sequencer| seq.patterns().iter().map(Pattern::label).collect::<Vec<_>>();
  let loop1 = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  // each kind counts on its own
  seq.add_pattern(1, 4, PatternKind::Note).unwrap();
  let loop2 = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  seq.add_pattern(1, 4, PatternKind::Note).unwrap();
  assert_eq!(names(&seq), vec!["Loop 1", "Sequence 1", "Loop 2", "Sequence 2"]);

  // a gap is filled first
  seq.remove_pattern(loop1).unwrap();
  seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  assert_eq!(names(&seq).last().unwrap(), "Loop 1");

  // a pattern renamed off its number frees it; one renamed onto a number
  // takes it (case aside)
  seq.rename_pattern(loop2, "verse").unwrap();
  let fill = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  assert_eq!(seq.pattern(fill).unwrap().label(), "Loop 2");
  seq.rename_pattern(fill, "loop 3").unwrap();
  assert_eq!(seq.free_pattern_name(PatternKind::Drum), "Loop 2");
  seq.rename_pattern(loop2, "Loop 2").unwrap();
  assert_eq!(seq.free_pattern_name(PatternKind::Drum), "Loop 4");
}

#[test]
fn test_unnamed_patterns_are_shown_by_their_kind_and_number() {
  let mut seq = Sequencer::default();
  let saved = SavedSequencer {
    patterns: vec![
      Pattern { id: 1, name: None, kind: PatternKind::Drum, length: 16, steps: BTreeMap::new(), folder: None, notes: BTreeMap::new() },
      Pattern { id: 2, name: None, kind: PatternKind::Note, length: 16, steps: BTreeMap::new(), folder: None, notes: BTreeMap::new() },
    ],
    ..Default::default()
  };
  seq.restore(saved);
  assert_eq!(seq.pattern(1).unwrap().label(), "Loop 2");
  assert_eq!(seq.pattern(2).unwrap().label(), "Sequence 3");

  // a new pattern doesn't take a number an old unnamed one is shown with
  let a = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  let b = seq.add_pattern(1, 4, PatternKind::Drum).unwrap();
  assert_eq!(seq.pattern(a).unwrap().label(), "Loop 1");
  assert_eq!(seq.pattern(b).unwrap().label(), "Loop 3");

  // and a pattern renamed blank goes back to its number
  seq.rename_pattern(a, " ").unwrap();
  assert_eq!(seq.pattern(a).unwrap().label(), format!("Loop {}", a + 1));
}
