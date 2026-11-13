use super::*;
use std::path::Path;
use std::sync::Arc;
use serde_json::json;

use crate::project::{capture, load_from, save_to, Project};

// what a project's JSON has once it's merged: its sequencer and pattern
// clips, read as this app reads them (compared as values, 0 and 0.0 would
// differ)
fn merged(mut project: Value) -> (daw::SavedSequencer, Vec<ProjectPatternClip>) {
  merge_pattern_kinds(&mut project).unwrap();
  assert_eq!(project["version"], 4);
  assert!(project.get("sequencer").is_none());
  (
    serde_json::from_value(project["patternSequencer"].clone()).unwrap(),
    serde_json::from_value(project["patternClips"].clone()).unwrap(),
  )
}

fn parsed(json: &str) -> Value {
  serde_json::from_str(json).unwrap()
}

// context/pattern-merge-project/04-api.md, section 6.1
#[test]
fn test_the_worked_example() {
  let before = parsed(r#"{
    "version": 3,
    "sequencer": {
      "rows": [
        { "id": 0, "path": "samples/kick.wav", "volume": 1.0, "muted": false },
        { "id": 2, "path": "samples/snare.wav", "label": "Snare", "volume": 0.8, "pan": -0.5, "muted": true }
      ],
      "synths": [
        { "id": 7, "pluginId": "vst3:0123456789abcdef0123456789abcdef", "name": "DecentSampler",
          "vendor": "Decent Samples", "path": "/Library/Audio/Plug-Ins/VST3/DecentSampler.vst3",
          "volume": 0.5, "muted": false, "state": "AAEC" }
      ],
      "patterns": [
        { "id": 0, "name": "Beat", "length": 16, "steps": { "0": [0, 4, 8, 12], "2": [4, 12, 20], "5": [1] } },
        { "id": 1, "name": null, "length": 32, "steps": {}, "folder": 0,
          "notes": { "7": [ { "pitch": 72, "start": 0.0, "length": 1.0, "velocity": 1.0 } ] } },
        { "id": 2, "name": null, "kind": "note", "length": 16, "steps": { "0": [3] }, "folder": 1,
          "notes": { "7": [
            { "pitch": 64, "start": 4.0, "length": 2.0, "velocity": 0.8 },
            { "pitch": 60, "start": 0.0, "length": 4.0, "velocity": 1.0 } ] } },
        { "id": 3, "name": "Pad", "kind": "note", "length": 16, "steps": {}, "folder": 0 }
      ],
      "folders": [
        { "id": 0, "name": "Drums" },
        { "id": 1, "name": "drums", "kind": "note" },
        { "id": 2, "name": "Keys", "kind": "note", "parent": 1 }
      ]
    },
    "patternClips": [
      { "patternId": 0, "trackNumber": 0, "startBeat": 0.0 },
      { "patternId": 0, "trackNumber": 0, "startBeat": 4.0, "window": { "start": 4.0, "end": 12.0 } },
      { "patternId": 2, "trackNumber": 1, "startBeat": 8.0, "window": { "start": 2.0, "end": null } },
      { "patternId": 0, "trackNumber": 2, "startBeat": 16.0, "window": { "start": 0.0, "end": 24.0 } },
      { "patternId": 9, "trackNumber": 2, "startBeat": 0.0 }
    ]
  }"#);
  let after = parsed(r#"{
    "version": 4,
    "patternSequencer": {
      "rows": [
        { "id": 0, "label": null, "volume": 1.0, "pan": 0.0, "muted": false, "stepPitch": 60,
          "instrument": { "kind": "sample", "path": "samples/kick.wav", "samplePitch": 60, "gated": false } },
        { "id": 1, "label": "Snare", "volume": 0.8, "pan": -0.5, "muted": true, "stepPitch": 60,
          "instrument": { "kind": "sample", "path": "samples/snare.wav", "samplePitch": 60, "gated": false } },
        { "id": 2, "label": null, "volume": 0.5, "pan": 0.0, "muted": false, "stepPitch": 60,
          "instrument": { "kind": "synth", "slot": 7, "pluginId": "vst3:0123456789abcdef0123456789abcdef",
            "name": "DecentSampler", "vendor": "Decent Samples",
            "path": "/Library/Audio/Plug-Ins/VST3/DecentSampler.vst3", "state": "AAEC", "window": null } }
      ],
      "patterns": [
        { "id": 0, "name": "Beat", "length": 16, "floor": 16, "folder": null, "notes": {
          "0": [
            { "pitch": 60, "start": 0.0, "length": 1.0, "velocity": 1.0 },
            { "pitch": 60, "start": 4.0, "length": 1.0, "velocity": 1.0 },
            { "pitch": 60, "start": 8.0, "length": 1.0, "velocity": 1.0 },
            { "pitch": 60, "start": 12.0, "length": 1.0, "velocity": 1.0 } ],
          "1": [
            { "pitch": 60, "start": 4.0, "length": 1.0, "velocity": 1.0 },
            { "pitch": 60, "start": 12.0, "length": 1.0, "velocity": 1.0 },
            { "pitch": 60, "start": 20.0, "length": 1.0, "velocity": 1.0 } ] } },
        { "id": 1, "name": "Loop 2", "length": 32, "floor": 32, "folder": 0, "notes": {} },
        { "id": 2, "name": "Sequence 3", "length": 16, "floor": 16, "folder": 1, "notes": {
          "2": [
            { "pitch": 60, "start": 0.0, "length": 4.0, "velocity": 1.0 },
            { "pitch": 64, "start": 4.0, "length": 2.0, "velocity": 0.8 } ] } },
        { "id": 3, "name": "Pad", "length": 16, "floor": 16, "folder": null, "notes": {} }
      ],
      "folders": [
        { "id": 0, "name": "Drums", "parent": null },
        { "id": 1, "name": "drums 2", "parent": null },
        { "id": 2, "name": "Keys", "parent": 1 }
      ]
    },
    "patternClips": [
      { "patternId": 0, "trackNumber": 0, "startBeat": 0.0, "start": 0.0, "length": 16.0 },
      { "patternId": 0, "trackNumber": 0, "startBeat": 4.0, "start": 4.0, "length": 8.0 },
      { "patternId": 2, "trackNumber": 1, "startBeat": 8.0, "start": 2.0, "length": 14.0 },
      { "patternId": 0, "trackNumber": 2, "startBeat": 16.0, "start": 0.0, "length": 16.0 }
    ]
  }"#);

  let expected: daw::SavedSequencer = serde_json::from_value(after["patternSequencer"].clone()).unwrap();
  let expected_clips: Vec<ProjectPatternClip> = serde_json::from_value(after["patternClips"].clone()).unwrap();
  assert_eq!(merged(before), (expected, expected_clips));
}

#[test]
fn test_only_projects_older_than_version_4_with_patterns_are_merged() {
  let unchanged = |json: Value| {
    let mut merged = json.clone();
    merge_pattern_kinds(&mut merged).unwrap();
    assert_eq!(merged, json);
  };
  // version 4 already
  unchanged(json!({ "version": 4, "patternSequencer": { "rows": [] }, "sequencer": { "rows": "anything" } }));
  // nothing to merge
  unchanged(json!({ "version": 3, "clips": [] }));
  unchanged(json!({}));
  // (a version that isn't a number is refused when it's read)
  unchanged(json!({ "version": "3", "sequencer": {} }));

  // no version is version 1; clips alone are merged (and left out, with no
  // pattern to play)
  let mut old = json!({ "sequencer": { "rows": [] } });
  merge_pattern_kinds(&mut old).unwrap();
  assert_eq!(old["version"], 4);
  assert_eq!(merged(json!({ "version": 2, "patternClips": [{ "patternId": 0, "trackNumber": 0, "startBeat": 0 }] })),
    (daw::SavedSequencer::default(), vec![]));
}

#[test]
fn test_what_the_old_app_refused_is_refused() {
  let refused = |sequencer: Value| {
    let json = json!({ "version": 3, "sequencer": sequencer }).to_string();
    let error = Project::from_json(&json).unwrap_err();
    assert!(error.starts_with("not a valid project file: "), "{}", error);
  };
  let pattern = |extra: Value| {
    let mut pattern = json!({ "id": 0, "name": null, "length": 16, "steps": {} });
    pattern.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    json!({ "patterns": [pattern] })
  };
  refused(pattern(json!({ "steps": { "0": [1.5] } })));
  refused(pattern(json!({ "steps": { "0": [-1] } })));
  refused(pattern(json!({ "kind": "chord" })));
  refused(json!({ "patterns": [{ "id": 0, "name": null, "length": 16 }] }));
  refused(json!({ "rows": [{ "path": "kick.wav" }] }));
  refused(json!({ "rows": [{ "id": "zero", "path": "kick.wav" }] }));
  refused(json!({ "folders": [{ "id": 0, "name": "Kit", "kind": "both" }] }));
  refused(json!({ "synths": [{ "id": 3, "name": "Synth" }] }));
}

#[test]
fn test_rows_and_synths_used_twice_are_left_out_and_the_rest_numbered() {
  let (sequencer, _) = merged(json!({
    "version": 3,
    "sequencer": {
      "rows": [{ "id": 5, "path": "a.wav" }, { "id": 5, "path": "b.wav" }, { "id": 1, "path": "c.wav" }],
      "synths": [
        { "id": 5, "pluginId": "vst3:x", "name": "X" },
        { "id": 5, "pluginId": "vst3:y", "name": "Y" },
      ],
      "patterns": [
        { "id": 0, "name": "Steps", "length": 16, "steps": { "5": [0], "1": [3, 3, 2] } },
        { "id": 1, "name": "Notes", "kind": "note", "length": 16, "steps": {},
          "notes": { "5": [{ "pitch": 50, "start": 1, "length": 1, "velocity": 1 }] } },
      ],
    },
  }));
  let rows: Vec<(u64, Option<&str>)> = sequencer.rows.iter().map(|row| (row.id, row.instrument.sample_path())).collect();
  assert_eq!(rows, vec![(0, Some("a.wav")), (1, Some("c.wav")), (2, None)]);
  assert_eq!(sequencer.rows[2].instrument.synth().unwrap().name, "X");
  // a drum pattern's steps go to the sample rows, deduplicated and in
  // order; a note pattern's notes to the synth's row
  let starts = |pattern: &daw::SavedPattern, row: u64| -> Vec<f64> { pattern.notes[&row].iter().map(|n| n.start).collect() };
  assert_eq!(starts(&sequencer.patterns[0], 0), vec![0.]);
  assert_eq!(starts(&sequencer.patterns[0], 1), vec![2., 3.]);
  assert_eq!(sequencer.patterns[1].notes.keys().copied().collect::<Vec<_>>(), vec![2]);
}

#[test]
fn test_folders_are_mended_as_the_old_app_did_and_clashes_numbered() {
  let folder = |id: u64, name: &str, kind: &str, parent: Option<u64>| json!({ "id": id, "name": name, "kind": kind, "parent": parent });
  let (sequencer, _) = merged(json!({
    "version": 3,
    "sequencer": {
      "patterns": [
        // in a folder of its own kind, of the other kind, and one that's gone
        { "id": 0, "name": "a", "length": 16, "steps": {}, "folder": 0 },
        { "id": 1, "name": "b", "length": 16, "steps": {}, "folder": 1 },
        { "id": 2, "name": "c", "kind": "note", "length": 16, "steps": {}, "folder": 99 },
      ],
      "folders": [
        folder(0, "Kit", "drum", None),
        folder(1, "kit", "note", None),
        folder(2, "Kit 2", "note", None),
        // a drum folder in a note folder, and one in a folder that's gone
        folder(3, "Lost", "drum", Some(1)),
        folder(4, "Gone", "drum", Some(99)),
        // parents that loop: the first is moved to the top, then the second is fine
        folder(7, "Up", "note", Some(8)),
        folder(8, "Down", "note", Some(7)),
        // an id used twice
        folder(8, "Again", "note", None),
        // a clash with the numbers taken, renamed ones counting
        folder(10, "X", "drum", None),
        folder(11, "X 2", "drum", None),
        folder(12, "x", "note", None),
        folder(13, "X", "note", None),
        // the same name in another folder isn't a clash
        folder(14, "Kit", "note", Some(7)),
      ],
    },
  }));
  let folders: Vec<(u64, &str, Option<u64>)> = sequencer.folders.iter().map(|f| (f.id, f.name.as_str(), f.parent)).collect();
  assert_eq!(folders, vec![
    (0, "Kit", None), (1, "kit 3", None), (2, "Kit 2", None),
    (3, "Lost", None), (4, "Gone", None),
    (7, "Up", None), (8, "Down", Some(7)),
    (10, "X", None), (11, "X 2", None), (12, "x 3", None), (13, "X 4", None),
    (14, "Kit", Some(7)),
  ]);
  let in_folders: Vec<Option<u64>> = sequencer.patterns.iter().map(|p| p.folder).collect();
  assert_eq!(in_folders, vec![Some(0), None, None]);
}

#[test]
fn test_a_clips_part_is_frozen_as_the_old_app_played_it() {
  let clip = |window: Value| json!({ "patternId": 0, "trackNumber": 0, "startBeat": 1, "window": window });
  let (_, clips) = merged(json!({
    "version": 3,
    "sequencer": { "patterns": [{ "id": 0, "name": "a", "length": 16, "steps": {} }] },
    "patternClips": [
      clip(Value::Null),
      clip(json!({ "start": -4, "end": 8 })),
      clip(json!({ "start": 6, "end": 3 })),
      clip(json!({ "start": 15.5, "end": null })),
      // past the pattern's end: a step long, which plays nothing
      clip(json!({ "start": 20, "end": 30 })),
    ],
  }));
  let parts: Vec<(f64, f64)> = clips.iter().map(|c| (c.start, c.length)).collect();
  assert_eq!(parts, vec![(0., 16.), (0., 8.), (6., 10.), (15.5, 1.), (20., 1.)]);
  assert!(clips.iter().all(|c| c.start_beat == 1.));
}

// the project tests/fixtures/v3-patterns.dawesome, written by the app at
// version 3 (commit 3cad962): five sample and synth rows, two kinds of
// pattern in two folder trees that clash, unnamed patterns, trimmed clips
const FIXTURE: &str = "tests/fixtures/v3-patterns.dawesome";

fn step_notes(starts: &[u32]) -> Arc<Vec<daw::PatternNote>> {
  Arc::new(starts.iter().map(|&start| step_note(start)).collect())
}

fn notes(list: &[(u8, f64, f64, f32)]) -> Arc<Vec<daw::PatternNote>> {
  Arc::new(list.iter().map(|&(pitch, start, length, velocity)| daw::PatternNote { pitch, start, length, velocity }).collect())
}

#[test]
fn test_the_v3_fixture_opens_as_the_old_app_showed_it() {
  // (fake plugins: the fixture's synths aren't among them)
  let state = crate::effects::testing::rig().state;
  let loaded = load_from(&state, Path::new(FIXTURE)).unwrap();
  assert!(loaded.missing.is_empty() && loaded.dropped.is_empty());
  // the two synths' plugin isn't installed: kept, missing
  assert_eq!(loaded.missing_effects.iter().map(|m| (m.track, m.name.as_str())).collect::<Vec<_>>(),
    vec![(None, "Synth 1"), (None, "Synth 2")]);
  assert_eq!((loaded.settings.tempo, loaded.clips.len()), (97., 1));

  let samples = std::fs::canonicalize("tests/fixtures/samples").unwrap();
  let sample = |name: &str, label: Option<&str>, volume: f32, pan: f32, muted: bool, id: u64| daw::RowState {
    id, label: label.map(str::to_string), volume, pan, muted, step_pitch: 60,
    instrument: daw::InstrumentState::Sample(daw::SampleState {
      path: samples.join(name).to_string_lossy().into_owned(), sample_pitch: 60, gated: false,
    }),
  };
  let slots: Vec<u64> = loaded.sequencer.rows.iter().filter_map(|row| row.synth()).map(|synth| synth.slot).collect();
  assert!(slots.len() == 2 && slots[0] != slots[1]);
  let synth = |name: &str, slot: u64, id: u64| daw::RowState {
    id, label: None, volume: 1., pan: 0., muted: false, step_pitch: 60,
    instrument: daw::InstrumentState::Synth(daw::SynthState {
      slot, plugin_id: "vst3:00000000000000000000000000BADBAD".into(), name: name.into(), vendor: "Nobody".into(),
      missing: true, has_editor: false,
    }),
  };
  assert_eq!(loaded.sequencer.rows, vec![
    sample("assets_66-bd-01.wav", Some("Kick"), 1., 0., false, 0),
    sample("assets_66-sd-01.wav", None, 1., 0., false, 1),
    sample("assets_66-hh-01-or.wav", None, 0.5, -0.4, true, 2),
    synth("Synth 1", slots[0], 3),
    synth("Synth 2", slots[1], 4),
  ]);

  let pattern = |id: u64, name: &str, length: u32, folder: Option<u64>, notes: Vec<(u64, Arc<Vec<daw::PatternNote>>)>| daw::PatternState {
    id, name: Some(name.into()), length, floor: length, notes: notes.into_iter().collect(), folder,
  };
  let all_evens: Vec<u32> = (0..16).map(|n| n * 2).collect();
  assert_eq!(loaded.sequencer.patterns, vec![
    pattern(0, "Loop 1", 16, Some(0), vec![(0, step_notes(&[0, 4, 8, 12])), (1, step_notes(&[4, 12]))]),
    // unnamed, in no folder
    pattern(1, "Loop 2", 16, None, vec![(0, step_notes(&[0, 6, 10])), (2, step_notes(&[2, 6, 10, 14]))]),
    pattern(2, "Fill", 32, Some(2), vec![
      (0, step_notes(&[0, 16])), (1, step_notes(&[24, 26, 28, 30])), (2, step_notes(&all_evens)),
    ]),
    pattern(3, "Bass", 32, Some(1), vec![(3, notes(&[(36, 0., 2., 0.8), (43, 8., 1.5, 0.6), (41, 16.25, 3., 0.8)]))]),
    pattern(4, "Sequence 5", 64, Some(3), vec![
      (3, notes(&[(60, 0., 16., 0.8)])),
      (4, notes(&[(64, 0., 16., 0.8), (67, 32., 8., 1.)])),
    ]),
  ]);

  // the note folder "Verse" beside the drum folder "Verse" is numbered
  let folder = |id: u64, name: &str, parent: Option<u64>| daw::FolderState { id, name: name.into(), parent };
  assert_eq!(loaded.sequencer.folders, vec![
    folder(0, "Verse", None), folder(1, "Verse 2", None), folder(2, "Fills", Some(0)), folder(3, "Pads", Some(1)),
  ]);

  // the clips, in the file's order, each playing the part it played
  let window = |start: f64, end: Option<f64>| Some(daw::PatternWindow { start, end });
  let clips: Vec<(u64, u32, f32, Option<daw::PatternWindow>)> = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let windows = audiograph.pattern_nodes.iter().map(|node| node.window);
    loaded.pattern_clips.iter().zip(windows).map(|(c, window)| (c.pattern_id, c.track_number, c.start_beat, window)).collect()
  };
  assert_eq!(clips, vec![
    (0, 0, 0., None),
    (3, 2, 0., window(8., None)),
    (0, 0, 4., window(4., Some(12.))),
    (1, 1, 8., None),
    (2, 1, 12., window(8., None)),
    (4, 3, 16., None),
  ]);
}

#[test]
fn test_the_v3_fixture_saves_as_version_4_and_opens_the_same() {
  let dir = std::env::temp_dir().join(format!("dawesome-merge-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  let file = dir.join("v4.dawesome");

  let state = crate::effects::testing::rig().state;
  load_from(&state, Path::new(FIXTURE)).unwrap();
  save_to(&state, &file).unwrap();

  // under a key of its own: an app that only reads version 3 finds no
  // sequencer it can't read, and stops at the version
  let json: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
  assert_eq!(json["version"], 4);
  assert!(json.get("sequencer").is_none());
  assert_eq!(json["patternSequencer"]["rows"].as_array().unwrap().len(), 5);
  assert_eq!(json["patternClips"][1], json!({ "patternId": 3, "trackNumber": 2, "startBeat": 0.0, "start": 8.0, "length": 24.0 }));
  assert!(json["patternClips"].as_array().unwrap().iter().all(|clip| clip.get("window").is_none()));

  // opened again, it's the same project
  let again = crate::effects::testing::rig().state;
  load_from(&again, &file).unwrap();
  let mut first = capture(&state, Some(&dir));
  let mut second = capture(&again, Some(&dir));
  for project in [&mut first, &mut second] {
    project.sequencer.renumber_slots({ let mut n = 0; move || { n += 1; n } });
  }
  assert_eq!(first, second);

  let _ = std::fs::remove_dir_all(&dir);
}
