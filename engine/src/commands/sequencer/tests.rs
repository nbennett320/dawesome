use super::*;
use std::collections::BTreeMap;
use std::time::Duration;

const KICK: &str = "assets/assets_66-bd-01.wav";

#[test]
fn test_a_pattern_follows_the_time_signature() {
  let state = daw::InnerState::default();
  state.playlist.time_signature.lock().unwrap().numerator = 3;
  let added = add_pattern_of(&state, 2).unwrap();
  assert_eq!(added.sequencer.patterns[0].length, 24);
  assert_eq!(added.sequencer.patterns[0].id, added.id);
}

#[test]
fn test_pattern_clips_snap_and_report_their_beat() {
  let state = daw::InnerState::default();
  let pattern = add_pattern_of(&state, 1).unwrap().id;

  // default snap is one beat
  let placed = add_pattern_clip_to(&state, pattern, 1, Some(2.4)).unwrap();
  assert_eq!(placed.start_beat, 2.);
  assert_eq!(
    state.playlist.audiograph.lock().unwrap().placement(placed.id),
    Some((Duration::from_millis(1_000), 1)));

  assert!(add_pattern_clip_to(&state, 99, 0, None).is_err());
}

#[test]
fn test_removing_a_pattern_removes_its_clips() {
  let state = daw::InnerState::default();
  let pattern = add_pattern_of(&state, 1).unwrap().id;
  let other = add_pattern_of(&state, 1).unwrap().id;
  let a = add_pattern_clip_to(&state, pattern, 0, Some(0.)).unwrap().id;
  add_pattern_clip_to(&state, other, 0, Some(4.)).unwrap();

  let removed = remove_pattern_of(&state, pattern).unwrap();
  assert_eq!(removed.removed_clips, vec![a]);
  assert_eq!(removed.sequencer.patterns.len(), 1);
  assert_eq!(state.playlist.audiograph.lock().unwrap().pattern_nodes.len(), 1);
  assert!(remove_pattern_of(&state, pattern).is_err());
}

#[test]
fn test_a_folder_in_use_isnt_removed() {
  let state = daw::InnerState::default();
  let placed = add_pattern_of(&state, 1).unwrap().id;
  let unplaced = add_pattern_of(&state, 1).unwrap().id;
  let folder = add_pattern_folder_to(&state, "Drums", None).unwrap().id;
  {
    let mut seq = state.playlist.sequencer.lock().unwrap();
    seq.move_pattern(placed, Some(folder)).unwrap();
    seq.move_pattern(unplaced, Some(folder)).unwrap();
  }
  add_pattern_clip_to(&state, placed, 0, Some(0.)).unwrap();

  let refused = remove_pattern_folder_of(&state, folder).unwrap_err();
  assert!(refused.contains("1 of its patterns is"), "{}", refused);
  assert_eq!(state.playlist.sequencer.lock().unwrap().patterns().len(), 2);

  // with its clip gone, the folder and both patterns go
  state.playlist.audiograph.lock().unwrap().remove_pattern_nodes_of(placed);
  let seq = remove_pattern_folder_of(&state, folder).unwrap();
  assert!(seq.patterns.is_empty() && seq.folders.is_empty());
}

#[test]
fn test_adding_a_sample_row_returns_it_and_the_sequencer() {
  let state = daw::InnerState::default();
  add_pattern_of(&state, 1).unwrap();
  let added = add_sample(&state, KICK).unwrap();
  assert_eq!(added.sequencer.rows.len(), 1);
  assert_eq!(added.sequencer.rows[0].id, added.row_id);
  assert_eq!(added.sequencer.rows[0].sample_path(), Some(KICK));
  let json = serde_json::to_value(&added).unwrap();
  assert_eq!(json["rowId"], added.row_id);
  assert!(json["sequencer"]["patterns"].is_array());
  assert!(add_sample(&state, "assets/nope.wav").is_err());
}

#[test]
fn test_a_pattern_needs_steps_to_play() {
  let state = Arc::new(daw::InnerState::default());
  assert!(daw::start_pattern(&state, 7, 0.).is_err());
  assert!(!state.playlist.playing());
}

#[test]
fn test_a_pattern_starts_from_its_marker_kept_inside_it() {
  let state = daw::InnerState::default();
  // one bar of 4/4 at 120bpm: 16 steps, 2s
  let pattern = add_pattern_of(&state, 1).unwrap().id;
  let from = |step: f64| {
    *state.playlist.pattern_start.lock().unwrap() = step;
    daw::pattern_start(&state, pattern).as_secs_f64() * 1000.
  };
  assert_eq!(from(0.), 0.);
  assert_eq!(from(4.), 500.);
  assert_eq!(from(2.5), 312.5);
  // past the end (the pattern was made shorter): from its start
  assert_eq!(from(16.), 0.);
  assert_eq!(from(-3.), 0.);
  // another tempo: the same step
  daw::retime(&state, 60.).unwrap();
  assert_eq!(from(4.), 1000.);
}

#[test]
fn test_a_seek_moves_only_the_pattern_playing() {
  let state = Arc::new(daw::InnerState::default());
  let pattern = add_pattern_of(&state, 1).unwrap().id;
  let other = add_pattern_of(&state, 1).unwrap().id;

  // nothing playing: nothing starts, and the marker's left to the frontend
  assert_eq!(daw::seek_pattern(&state, pattern, 8.), Ok(()));
  assert!(!state.playlist.playing());
  assert_eq!(*state.playlist.pattern_start.lock().unwrap(), 0.);

  // another pattern playing: it plays on as it was
  *state.playlist.mode.lock().unwrap() = daw::PlayMode::Pattern(other);
  state.playlist.set_playing(true);
  assert_eq!(daw::seek_pattern(&state, pattern, 8.), Ok(()));
  assert_eq!(state.playlist.playing_pattern(), Some(other));
  assert_eq!(*state.playlist.pattern_start.lock().unwrap(), 0.);
}

// a pattern that's played `ago`, from `from_step`
fn fake_pattern_run(state: &daw::InnerState, pattern: u64, from_step: f64, ago: Duration) {
  *state.playlist.mode.lock().unwrap() = daw::PlayMode::Pattern(pattern);
  *state.playlist.pattern_start.lock().unwrap() = from_step;
  *state.playlist.started_time.lock().unwrap() = Some(std::time::Instant::now() - ago);
  state.playlist.set_playing(true);
}

#[test]
fn test_a_tempo_change_carries_a_pattern_on_from_its_step() {
  let state = daw::InnerState::default();
  // one bar of 4/4 at 120bpm: 16 steps, 2s, 125ms a step
  let pattern = add_pattern_of(&state, 1).unwrap().id;
  let close = |a: f64, b: f64| (a - b).abs() < 0.5;

  // from step 4 (0.5s), 1.25s ago: step 14
  fake_pattern_run(&state, pattern, 4., Duration::from_millis(1_250));
  assert!(close(daw::pattern_step(&state, pattern).unwrap(), 14.));
  assert_eq!(daw::retime(&state, 60.), Ok(daw::Retimed::Pattern(pattern)));
  assert!(!state.playlist.playing());
  let step = *state.playlist.pattern_start.lock().unwrap();
  assert!(close(step, 14.), "step {}", step);

  // round the pattern's end: 0.5s + 3s is 1.5s into the second pass, step 12
  daw::retime(&state, 120.).unwrap();
  fake_pattern_run(&state, pattern, 4., Duration::from_secs(3));
  daw::retime(&state, 90.).unwrap();
  let step = *state.playlist.pattern_start.lock().unwrap();
  assert!(close(step, 12.), "step {}", step);
}

fn change_tempo_mid_pattern() -> (Arc<daw::InnerState>, u64) {
  let state = Arc::new(daw::InnerState::default());
  let pattern = add_pattern_of(&state, 1).unwrap().id;
  fake_pattern_run(&state, pattern, 4., Duration::from_secs(1));

  let (done_tx, done_rx) = std::sync::mpsc::channel();
  let changing = state.clone();
  std::thread::spawn(move || done_tx.send(daw::set_playlist_tempo(&changing, 90.)));
  let changed = done_rx.recv_timeout(Duration::from_secs(5)).expect("the tempo change hung");
  assert_eq!(changed, Ok(90.));
  (state, pattern)
}

#[test]
fn test_a_tempo_change_carries_a_playing_pattern_on_from_its_step() {
  let (state, _) = change_tempo_mid_pattern();
  let step = *state.playlist.pattern_start.lock().unwrap();
  assert!((step - 12.).abs() < 0.5, "step {}", step);
  daw::stop_playlist(&state);
}

// plays through the real output: run with `cargo test -- --ignored`
#[test]
#[ignore = "needs an audio output device"]
fn test_a_tempo_change_carries_on_playing_a_pattern() {
  let (state, pattern) = change_tempo_mid_pattern();
  assert_eq!(state.playlist.playing_pattern(), Some(pattern));
  daw::stop_playlist(&state);
}

#[test]
fn test_pattern_length_in_the_transport_math() {
  let state = daw::InnerState::default();
  let pattern = add_pattern_of(&state, 2).unwrap().id;
  // two bars of 4/4 at 120bpm
  assert_eq!(daw::pattern_len(&state, pattern), Some(Duration::from_millis(4_000)));
}

const SNARE: &str = "assets/assets_66-sd-01.wav";

// the project as it would be saved: what undo and redo have to get back to
fn saved(state: &daw::InnerState) -> crate::project::Project {
  crate::project::capture(state, None)
}

fn steps(state: &daw::InnerState) -> usize {
  state.history.lock().unwrap().len()
}

// a sequencer with two rows, a pattern with steps in a folder, and a clip
// of the pattern: (state, kick row, snare row, pattern, folder, clip). its
// plugins are fakes (see effects.rs)
fn drums() -> (Arc<daw::InnerState>, u64, u64, u64, u64, u64) {
  let state = crate::effects::testing::rig().state;
  let kick = add_sample(&state, KICK).unwrap().row_id;
  let snare = add_sample(&state, SNARE).unwrap().row_id;
  let pattern = add_pattern_of(&state, 1).unwrap().id;
  set_step(&state, pattern, kick, 0, true, None).unwrap();
  set_step(&state, pattern, snare, 4, true, None).unwrap();
  let folder = add_pattern_folder_to(&state, "Drums", None).unwrap().id;
  edit(&state, None, |seq| seq.move_pattern(pattern, Some(folder))).unwrap();
  let clip = add_pattern_clip_to(&state, pattern, 1, Some(4.)).unwrap().id;
  (state, kick, snare, pattern, folder, clip)
}

// a synth row (the fake one) and a one bar pattern, in `state`
fn synth_and_pattern(state: &daw::InnerState) -> (u64, u64) {
  let synth = crate::commands::synth::add(state, &crate::effects::testing::synth().id).unwrap().row_id;
  let pattern = add_pattern_of(state, 1).unwrap().id;
  (synth, pattern)
}

#[test]
fn test_every_sequencer_edit_can_be_undone_and_redone() {
  let (state, kick, snare, pattern, folder, _) = drums();
  let (synth, notes) = synth_and_pattern(&state);
  let s: &daw::InnerState = &state;

  // each edit is made on top of the ones before it
  let edits: Vec<(&str, Box<dyn Fn() -> Result<(), String> + '_>)> = vec![
    ("add a row", Box::new(|| add_sample(s, KICK).map(|_| ()))),
    ("row volume", Box::new(|| edit(s, None, |seq| seq.set_row_volume(kick, 0.5)).map(|_| ()))),
    ("mute a row", Box::new(|| edit(s, None, |seq| seq.set_row_muted(snare, true)).map(|_| ()))),
    ("pan a row", Box::new(|| edit(s, None, |seq| seq.set_row_pan(kick, -0.5)).map(|_| ()))),
    ("a row's step pitch", Box::new(|| edit(s, None, |seq| seq.set_row_step_pitch(kick, 62)).map(|_| ()))),
    ("a row's sample pitch", Box::new(|| edit_row(s, None, snare, |seq| seq.set_row_sample_pitch(snare, 48)).map(|_| ()))),
    ("gate a row", Box::new(|| edit_row(s, None, kick, |seq| seq.set_row_gated(kick, true)).map(|_| ()))),
    ("label a row", Box::new(|| edit(s, None, |seq| seq.rename_row(snare, "Snare")).map(|_| ()))),
    ("move a row", Box::new(|| edit(s, None, |seq| seq.move_row(snare, 0)).map(|_| ()))),
    ("replace a row's sample", Box::new(|| replace_sample(s, kick, SNARE).map(|_| ()))),
    ("light a step", Box::new(|| set_step(s, pattern, kick, 8, true, None).map(|_| ()))),
    ("clear a step", Box::new(|| set_step(s, pattern, kick, 0, false, None).map(|_| ()))),
    ("rename a pattern", Box::new(|| edit(s, None, |seq| seq.rename_pattern(pattern, "verse")).map(|_| ()))),
    ("set the notes", Box::new(|| set_notes(s, notes, synth, vec![note(60, 1.), note(64, 2.)], None).map(|_| ()))),
    ("set a sample row's notes", Box::new(|| set_notes(s, pattern, snare, vec![note(48, 2.5)], None).map(|_| ()))),
    ("a long pattern", Box::new(|| edit(s, None, |seq| seq.set_pattern_bars(notes, 128, 4)).map(|_| ()))),
    ("synth volume", Box::new(|| edit(s, None, |seq| seq.set_row_volume(synth, 0.5)).map(|_| ()))),
    ("mute a synth", Box::new(|| edit(s, None, |seq| seq.set_row_muted(synth, true)).map(|_| ()))),
    ("pattern length", Box::new(|| edit(s, None, |seq| seq.set_pattern_bars(pattern, 2, 4)).map(|_| ()))),
    ("add a pattern", Box::new(|| add_pattern_of(s, 4).map(|_| ()))),
    ("duplicate a pattern", Box::new(|| duplicate_pattern_of(s, pattern).map(|_| ()))),
    ("add a folder", Box::new(|| add_pattern_folder_to(s, "Fills", Some(folder)).map(|_| ()))),
    ("add another folder", Box::new(|| add_pattern_folder_to(s, "Keys", None).map(|_| ()))),
    ("rename a folder", Box::new(|| edit(s, None, |seq| seq.rename_folder(folder, "Kit")).map(|_| ()))),
    // (the folder just added, out to the top)
    ("move a folder", Box::new(|| edit(s, None, |seq| seq.move_folder(folder + 1, None)).map(|_| ()))),
    ("move a pattern", Box::new(|| edit(s, None, |seq| seq.move_pattern(pattern, None)).map(|_| ()))),
    ("remove a folder", Box::new(|| remove_pattern_folder_of(s, folder).map(|_| ()))),
    // with its notes in every pattern
    ("remove a row", Box::new(|| edit(s, None, |seq| seq.remove_row(snare)).map(|_| ()))),
    // with its clip in the playlist
    ("remove a pattern", Box::new(|| remove_pattern_of(s, pattern).map(|_| ()))),
    // with its notes and its plugin
    ("remove a synth row", Box::new(|| edit(s, None, |seq| seq.remove_row(synth)).map(|_| ()))),
  ];

  let start = saved(&state);
  let had = steps(&state);
  for (what, change) in &edits {
    let count = steps(&state);
    let before = saved(&state);
    change().unwrap_or_else(|e| panic!("could not {}: {}", what, e));
    let after = saved(&state);
    assert_ne!(before, after, "{} changed nothing", what);
    assert_eq!(steps(&state), count + 1, "{} is one step", what);

    assert!(history::undo(&state).unwrap().is_some());
    assert_eq!(saved(&state), before, "{} undone", what);
    assert!(history::redo(&state).unwrap().is_some());
    assert_eq!(saved(&state), after, "{} redone", what);
  }

  // and all the way back
  for _ in &edits { history::undo(&state).unwrap(); }
  assert_eq!(steps(&state), had);
  assert_eq!(saved(&state), start);
}

fn note(pitch: u8, start: f64) -> daw::PatternNote {
  daw::PatternNote { pitch, start, length: 1., velocity: 0.7 }
}

#[test]
fn test_a_drag_of_notes_is_one_step() {
  let (state, _, _, _, _, _) = drums();
  let (synth, pattern) = synth_and_pattern(&state);
  let before = saved(&state);
  let had = steps(&state);

  // a note drawn, then dragged along, without letting go of the mouse
  for n in 0..10 {
    let change = set_notes(&state, pattern, synth, vec![note(60, 0.), note(62, n as f64 * 0.5)], Some(31)).unwrap();
    let notes = &change.pattern.notes[&synth];
    assert_eq!(notes.iter().map(|note| note.start).fold(0., f64::max), n as f64 * 0.5);
  }
  assert_eq!(steps(&state), had + 1);
  let after = saved(&state);

  history::undo(&state).unwrap();
  assert_eq!(saved(&state), before);
  assert!(state.playlist.sequencer.lock().unwrap().pattern(pattern).unwrap().notes.is_empty());
  history::redo(&state).unwrap();
  assert_eq!(saved(&state), after);

  // the next drag is a step of its own; one that changes nothing isn't one
  set_notes(&state, pattern, synth, vec![note(60, 0.)], Some(32)).unwrap();
  set_notes(&state, pattern, synth, vec![note(60, 0.)], Some(33)).unwrap();
  assert_eq!(steps(&state), had + 2);
  assert!(set_notes(&state, 99, synth, vec![], None).is_err());
  assert_eq!(steps(&state), had + 2);
}

#[test]
fn test_placing_a_pattern_is_one_step() {

  let (state, _, _, pattern, _, _) = drums();
  let before = saved(&state);
  let had = steps(&state);

  let placed = add_pattern_clip_to(&state, pattern, 3, Some(12.)).unwrap();
  assert_eq!(steps(&state), had + 1);
  let after = saved(&state);
  assert_ne!(before, after);

  // undone, the clip is gone and its pattern stays. redone, it's the
  // same clip again
  assert_eq!(history::undo(&state).unwrap().unwrap().clips, Some(vec![]));
  assert_eq!(saved(&state), before);
  assert!(state.playlist.sequencer.lock().unwrap().pattern(pattern).is_some());
  assert_eq!(history::redo(&state).unwrap().unwrap().clips, Some(vec![placed.id]));
  assert_eq!(saved(&state), after);
}

#[test]
fn test_an_edit_that_fails_is_no_step() {
  let (state, kick, _, pattern, _, _) = drums();
  let had = steps(&state);
  assert!(set_step(&state, pattern, 99, 0, true, None).is_err());
  assert!(set_step(&state, pattern, kick, 999, true, None).is_err());
  assert!(remove_pattern_of(&state, 99).is_err());
  assert!(add_sample(&state, "assets/nope.wav").is_err());
  // nor is one that changes nothing: the step was lit already
  set_step(&state, pattern, kick, 0, true, None).unwrap();
  assert_eq!(steps(&state), had);
}

#[test]
fn test_a_stroke_of_steps_is_one_step() {
  let (state, kick, _, pattern, _, _) = drums();
  let before = saved(&state);
  let had = steps(&state);

  // eight steps painted without letting go of the mouse
  for step in 1..=8 {
    set_step(&state, pattern, kick, step, true, Some(21)).unwrap();
  }
  assert_eq!(steps(&state), had + 1);
  let after = saved(&state);

  history::undo(&state).unwrap();
  assert_eq!(saved(&state), before);
  history::redo(&state).unwrap();
  assert_eq!(saved(&state), after);

  // the next stroke, and a knob's drag, are steps of their own
  set_step(&state, pattern, kick, 12, true, Some(22)).unwrap();
  for i in 1..=20 {
    edit(&state, Some(23), |seq| seq.set_row_volume(kick, 1. - 0.02 * i as f32)).unwrap();
  }
  assert_eq!(steps(&state), had + 3);
  history::undo(&state).unwrap();
  assert_eq!(state.playlist.sequencer.lock().unwrap().row(kick).unwrap().volume, 1.);
}

#[test]
fn test_a_removed_row_and_pattern_come_back_as_themselves() {
  let (state, _, snare, pattern, _, clip) = drums();

  edit(&state, None, |seq| seq.remove_row(snare)).unwrap();
  history::undo(&state).unwrap();
  {
    let seq = state.playlist.sequencer.lock().unwrap();
    assert_eq!(seq.row(snare).unwrap().sample().unwrap().path, SNARE);
    assert!(seq.pattern(pattern).unwrap().lit(snare, 4));
  }

  let removed = remove_pattern_of(&state, pattern).unwrap();
  assert_eq!(removed.removed_clips, vec![clip]);
  // its clip comes back with it, on its beat and track, and is selected
  let applied = history::undo(&state).unwrap().unwrap();
  assert_eq!(applied.clips, Some(vec![clip]));
  assert!(state.playlist.sequencer.lock().unwrap().pattern(pattern).is_some());
  assert_eq!(
    state.playlist.audiograph.lock().unwrap().placement(clip),
    Some((Duration::from_millis(2_000), 1)));

  // what's added after an undo never takes the id of something that could
  // still come back by redo
  history::undo(&state).unwrap();
  history::redo(&state).unwrap();
  edit(&state, None, |seq| seq.remove_row(snare)).unwrap();
  history::undo(&state).unwrap();
  let rows = add_sample(&state, KICK).unwrap().sequencer.rows;
  let ids: std::collections::BTreeSet<u64> = rows.iter().map(|row| row.id).collect();
  assert_eq!(ids.len(), rows.len());
  let added = add_pattern_of(&state, 1).unwrap();
  assert_ne!(added.id, pattern);
}

#[test]
fn test_an_undone_pattern_plays_as_it_did() {
  let (state, kick, _, pattern, _, _) = drums();
  // what the pattern's clip sounds like
  let render = || {
    let sounds = {
      let mut audiograph = state.playlist.audiograph.lock().unwrap();
      audiograph.init();
      let sounds = audiograph.run_sounds(Duration::from_secs(5), false, false);
      audiograph.pause();
      sounds
    };
    let (engine, mut renderer) = daw::Engine::new(44_100);
    let voices = sounds
      .into_iter()
      .map(|(delay, source, route)| engine.boxed_voice(source, Some(1), engine.frames_in(delay), route))
      .collect();
    engine.start_run(1, state.playlist.mixer.master(), voices).unwrap();

    let mut out = vec![0f32; 44_100 * 5 * 2];
    renderer.render(&mut out, std::time::Instant::now(), Duration::ZERO);
    out
  };
  let before = render();
  assert!(before.iter().any(|&sample| sample != 0.));

  set_step(&state, pattern, kick, 2, true, None).unwrap();
  assert!(render() != before);
  history::undo(&state).unwrap();
  assert!(render() == before);
}

#[test]
fn test_exploding_is_one_step() {
  let (state, _, _, pattern, _, clip) = drums();
  let other = add_pattern_clip_to(&state, pattern, 2, Some(8.)).unwrap().id;
  let before = saved(&state);
  let had = steps(&state);

  let exploded = explode(&state, &[clip, other]).unwrap();
  assert_eq!(exploded.removed, vec![clip, other]);
  assert_eq!(steps(&state), had + 1);
  let after = saved(&state);
  // it made a pattern per sample, and named tracks after them
  assert!(after.sequencer.patterns.len() > before.sequencer.patterns.len());
  assert_ne!(before.mixer, after.mixer);

  // undone: the clips are pattern clips again, the patterns it made are
  // gone, and so are the names
  let applied = history::undo(&state).unwrap().unwrap();
  assert_eq!(applied.clips, Some(vec![clip, other]));
  assert_eq!(saved(&state), before);

  let made: Vec<u64> = exploded.clips.iter().map(|c| c.id).collect();
  let applied = history::redo(&state).unwrap().unwrap();
  assert_eq!(applied.clips, Some(made));
  assert_eq!(saved(&state), after);
}

// (the cases of src/editing/paint.spec.ts, for paintSpans and paintBars,
// which must agree)
#[test]
fn test_the_paint_layout() {
  // end to end over the drag, the last cut short
  assert_eq!(paint_spans(4., 12., 4., 0.25), vec![(4., 8.), (8., 12.)]);
  assert_eq!(paint_spans(0., 10., 4., 0.25), vec![(0., 4.), (4., 8.), (8., 10.)]);
  assert_eq!(paint_spans(2., 3., 4., 0.25), vec![(2., 3.)]);
  // the same dragged backwards
  assert_eq!(paint_spans(10., 0., 4., 0.25), paint_spans(0., 10., 4., 0.25));
  // a click, or a drag shorter than a clip can be: one whole clip
  assert_eq!(paint_spans(6., 6., 4., 0.25), vec![(6., 10.)]);
  assert_eq!(paint_spans(6., 6.1, 4., 0.25), vec![(6., 10.)]);
  // an end piece too short to be a clip is left off
  assert_eq!(paint_spans(0., 8.1, 4., 0.25), vec![(0., 4.), (4., 8.)]);
  // nothing before the playlist's start, and nothing of no length
  assert_eq!(paint_spans(-2., 4., 4., 0.25), vec![(0., 4.)]);
  assert!(paint_spans(0., 8., 0., 0.25).is_empty());

  assert_eq!(paint_bars(0., 4., 4., 64), 1);
  assert_eq!(paint_bars(0., 4.5, 4., 64), 2);
  assert_eq!(paint_bars(8., 0., 4., 64), 2);
  assert_eq!(paint_bars(0., 9., 3., 64), 3);
  assert_eq!(paint_bars(3., 3., 4., 64), 1);
  assert_eq!(paint_bars(0., 1_000., 4., 64), 64);
}

// the pattern clips on a track, as (start beat, window), in order
fn clips_on(state: &daw::InnerState, track: u32) -> Vec<(f32, Option<crate::commands::clip::ClipWindow>)> {
  let audiograph = state.playlist.audiograph.lock().unwrap();
  let interval = audiograph.beat_interval();
  let mut clips: Vec<_> = audiograph.pattern_nodes.iter()
    .filter(|node| node.track_number == track)
    .map(|node| LoadedPatternClip::of(node, interval))
    .map(|clip| (clip.start_beat, clip.window))
    .collect();
  clips.sort_by(|a, b| a.0.total_cmp(&b.0));
  clips
}

#[test]
fn test_painting_lays_clips_end_to_end() {
  let (state, _, _, pattern, _, _) = drums();
  let had = steps(&state);

  // a one bar pattern (four beats), painted over ten
  let painted = paint(&state, Some(pattern), 2, 4., 14.).unwrap();
  assert_eq!(painted.pattern_id, pattern);
  assert_eq!(painted.clips.len(), 3);
  assert!(painted.clips.iter().all(|clip| clip.pattern_id == pattern && clip.track_number == 2));
  let starts: Vec<f32> = painted.clips.iter().map(|clip| clip.start_beat).collect();
  assert_eq!(starts, vec![4., 8., 12.]);
  // the last is trimmed to the drag's end: two beats, eight steps
  assert_eq!(painted.clips[0].window, None);
  assert_eq!(painted.clips[1].window, None);
  assert_eq!(painted.clips[2].window, Some(crate::commands::clip::ClipWindow { start: 0., end: Some(8.) }));
  assert_eq!(clips_on(&state, 2), painted.clips.iter().map(|c| (c.start_beat, c.window)).collect::<Vec<_>>());
  // no new pattern, and one step
  assert_eq!(painted.sequencer.patterns.len(), 1);
  assert_eq!(steps(&state), had + 1);
}

#[test]
fn test_painting_with_no_pattern_makes_one() {
  let (state, _, _, pattern, _, _) = drums();
  let painted = paint(&state, None, 0, 2., 8.).unwrap();

  // six beats needs two bars: a pattern of them, played to the end of the
  // drag
  assert_ne!(painted.pattern_id, pattern);
  let made = painted.sequencer.patterns.iter().find(|p| p.id == painted.pattern_id).unwrap();
  assert_eq!((made.name.as_deref(), made.length, made.floor), (Some("Sequence 2"), 32, 32));
  assert_eq!(painted.clips.len(), 1);
  assert_eq!(painted.clips[0].start_beat, 2.);
  assert_eq!(painted.clips[0].window, Some(crate::commands::clip::ClipWindow { start: 0., end: Some(24.) }));

  // a click: a bar, and the clip is all of it
  let clicked = paint(&state, None, 0, 20., 20.).unwrap();
  let made = clicked.sequencer.patterns.iter().find(|p| p.id == clicked.pattern_id).unwrap();
  assert_eq!(made.length, 16);
  assert_eq!(clicked.clips[0].window, None);
}

#[test]
fn test_painting_below_the_last_track_makes_one() {
  let (state, _, _, pattern, _, _) = drums();
  let count = state.playlist.audiograph.lock().unwrap().track_count();
  let painted = paint(&state, Some(pattern), count, 0., 4.).unwrap();
  assert_eq!(painted.track_count, count + 1);
  assert!(painted.mixer.tracks.iter().any(|t| t.track_number == count));
}

#[test]
fn test_a_paint_is_one_step() {
  let (state, _, _, pattern, _, _) = drums();
  for given in [Some(pattern), None] {
    let before = saved(&state);
    let had = steps(&state);

    let painted = paint(&state, given, 3, 0., 10.).unwrap();
    assert_eq!(steps(&state), had + 1);
    let after = saved(&state);
    assert_ne!(before, after);

    // undone, every clip goes (and the pattern it made); redone, they're back
    history::undo(&state).unwrap();
    assert_eq!(saved(&state), before);
    let applied = history::redo(&state).unwrap().unwrap();
    let ids: Vec<u64> = painted.clips.iter().map(|clip| clip.id).collect();
    assert_eq!(applied.clips, Some(ids));
    assert_eq!(saved(&state), after);
  }
}

#[test]
fn test_painting_an_unknown_pattern_changes_nothing() {
  let (state, _, _, _, _, _) = drums();
  let before = saved(&state);
  let had = steps(&state);
  assert!(paint(&state, Some(99), 0, 0., 8.).is_err());
  assert!(paint(&state, None, 0, f64::NAN, 8.).is_err());
  assert_eq!(saved(&state), before);
  assert_eq!(steps(&state), had);
}

#[test]
fn test_explode_skips_sample_clips() {
  let state = daw::InnerState::default();
  let pattern = add_pattern_of(&state, 1).unwrap().id;
  let row = add_sample(&state, KICK).unwrap().row_id;
  state.playlist.sequencer.lock().unwrap().set_step(pattern, row, 4, true).unwrap();
  let clip = add_pattern_clip_to(&state, pattern, 2, Some(4.)).unwrap().id;
  let sample = state.playlist.audiograph.lock().unwrap()
    .construct_and_add_node(KICK.to_string(), Duration::ZERO, 0).unwrap();

  let exploded = explode(&state, &[sample, clip]).unwrap();
  assert_eq!(exploded.removed, vec![clip]);
  assert_eq!(exploded.clips.len(), 1);
  // where the clip was. track 0 holds only kicks already, so it joins it
  assert_eq!(exploded.clips[0].start_beat, 4.);
  assert_eq!(exploded.clips[0].track_number, 0);
  // a one-row pattern is its own split, so no new pattern is needed
  assert_eq!(exploded.clips[0].pattern_id, pattern);
  assert_eq!(exploded.sequencer.patterns.len(), 1);
  let named = exploded.mixer.tracks.iter().find(|t| t.track_number == 0).unwrap();
  assert_eq!(named.name.as_deref(), Some("assets_66-bd-01"));
}

#[test]
fn test_replacing_a_sample_keeps_the_row() {
  let (state, kick, _, pattern, _, _) = drums();
  edit(&state, None, |seq| seq.rename_row(kick, "Boom")).unwrap();
  edit(&state, None, |seq| seq.set_row_volume(kick, 0.5)).unwrap();
  let before = state.playlist.sequencer.lock().unwrap().pattern(pattern).unwrap().clone();

  let row = replace_sample(&state, kick, SNARE).unwrap().row;
  assert_eq!(row.id, kick);
  assert_eq!(row.sample_path(), Some(SNARE));
  assert_eq!(row.label.as_deref(), Some("Boom"));
  assert_eq!(row.volume, 0.5);
  assert_eq!(state.playlist.sequencer.lock().unwrap().pattern(pattern).unwrap(), &before);

  // a file that isn't a sample leaves it as it was, and is no step
  let count = steps(&state);
  assert!(replace_sample(&state, kick, "assets/nope.wav").is_err());
  assert_eq!(steps(&state), count);

  // and a synth row has no sample to replace
  let (synth, _) = synth_and_pattern(&state);
  assert!(replace_sample(&state, synth, SNARE).unwrap_err().contains("plays a synth"));
}

// every row's and every pattern's rev, by id
fn revs(state: &daw::InnerState) -> (BTreeMap<u64, u64>, BTreeMap<u64, u64>) {
  let seq = state.playlist.sequencer.lock().unwrap();
  (
    seq.rows().iter().map(|r| (r.id, r.rev.get())).collect(),
    seq.patterns().iter().map(|p| (p.id, p.rev.get())).collect(),
  )
}

// the ids whose rev isn't what it was
fn changed(before: &BTreeMap<u64, u64>, after: &BTreeMap<u64, u64>) -> Vec<u64> {
  after.iter().filter(|(id, rev)| before.get(id) != Some(rev)).map(|(id, _)| *id).collect()
}

#[test]
fn test_a_row_edit_replies_with_the_row_and_undo_puts_its_rev_back() {
  let (state, kick, snare, _, _, _) = drums();
  let s: &daw::InnerState = &state;
  // as the commands make them
  let edits: Vec<(&str, Box<dyn Fn() -> Result<daw::RowChange, String> + '_>)> = vec![
    ("rename_row", Box::new(|| edit_row(s, None, snare, |seq| seq.rename_row(snare, "Snare")))),
    ("set_row_volume", Box::new(|| edit_row(s, Some(7), kick, |seq| seq.set_row_volume(kick, 0.5)))),
    ("set_row_pan", Box::new(|| edit_row(s, Some(8), kick, |seq| seq.set_row_pan(kick, -0.5)))),
    ("set_row_muted", Box::new(|| edit_row(s, None, snare, |seq| seq.set_row_muted(snare, true)))),
    ("replace_row_sample", Box::new(|| replace_sample(s, kick, SNARE))),
    ("set_row_sample_pitch", Box::new(|| edit_row(s, None, kick, |seq| seq.set_row_sample_pitch(kick, 72)))),
    ("set_row_gated", Box::new(|| edit_row(s, None, snare, |seq| seq.set_row_gated(snare, true)))),
  ];

  for (what, edit) in &edits {
    let before = revs(s);
    let change = edit().unwrap();
    let after = revs(s);
    assert_eq!(changed(&before.0, &after.0), vec![change.row.id], "{} changes its row", what);
    assert_eq!(after.1, before.1, "{} changes no pattern", what);
    // the row as the whole sequencer has it, at its new rev
    assert_eq!(change.row.rev.get(), after.0[&change.row.id], "{}", what);
    assert_eq!(sequencer_state(s).rows.iter().find(|r| r.id == change.row.id), Some(&change.row), "{}", what);

    history::undo(&state).unwrap();
    assert_eq!(revs(s), before, "{} undone", what);
    history::redo(&state).unwrap();
    assert_eq!(revs(s), after, "{} redone", what);
  }

  let json = serde_json::to_value(edit_row(s, None, kick, |seq| seq.set_row_muted(kick, true)).unwrap()).unwrap();
  assert_eq!(json.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["row"]);
  assert_eq!(json["row"]["id"], kick);
  assert_eq!(json["row"]["rev"], revs(s).0[&kick]);
  assert!(edit_row(s, None, 99, |seq| seq.set_row_muted(99, true)).is_err());
}

#[test]
fn test_a_pattern_edit_replies_with_the_pattern_and_undo_puts_its_rev_back() {
  let (state, kick, snare, pattern, _, _) = drums();
  let other = add_pattern_of(&state, 1).unwrap().id;
  set_step(&state, other, kick, 0, true, None).unwrap();
  let s: &daw::InnerState = &state;
  let edits: Vec<(&str, Box<dyn Fn() -> Result<daw::PatternChange, String> + '_>)> = vec![
    ("set_pattern_step", Box::new(|| set_step(s, pattern, kick, 8, true, Some(3)))),
    ("set_pattern_notes", Box::new(|| set_notes(s, pattern, snare, vec![note(48, 2.5)], Some(4)))),
    ("set_pattern_bars", Box::new(|| edit_pattern(s, None, pattern, |seq| seq.set_pattern_bars(pattern, 2, 4)))),
  ];

  for (what, edit) in &edits {
    let before = revs(s);
    let change = edit().unwrap();
    let after = revs(s);
    assert_eq!(change.pattern.id, pattern);
    assert_eq!(changed(&before.1, &after.1), vec![pattern], "{} changes its pattern", what);
    assert_eq!(after.0, before.0, "{} changes no row", what);
    assert_eq!(change.pattern.rev.get(), after.1[&pattern], "{}", what);
    assert_eq!(sequencer_state(s).pattern(pattern), Some(&change.pattern), "{}", what);

    history::undo(&state).unwrap();
    assert_eq!(revs(s), before, "{} undone", what);
    history::redo(&state).unwrap();
    assert_eq!(revs(s), after, "{} redone", what);
  }

  // the reply is the one pattern, and is all the frontend needs of it
  let json = serde_json::to_value(set_step(s, pattern, kick, 12, true, None).unwrap()).unwrap();
  assert_eq!(json.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["pattern"]);
  assert_eq!(json["pattern"]["id"], pattern);
  assert_eq!(json["pattern"]["rev"], revs(s).1[&pattern]);
  assert_eq!(json["pattern"]["notes"][kick.to_string()][2]["start"], 12.);
  assert!(set_step(s, 99, kick, 0, true, None).is_err());
}

#[test]
fn test_moving_a_row_changes_no_rev() {
  let (state, _, snare, _, _, _) = drums();
  let before = revs(&state);
  edit(&state, None, |seq| seq.move_row(snare, 0)).unwrap();
  assert_eq!(revs(&state), before);
  history::undo(&state).unwrap();
  assert_eq!(revs(&state), before);
}

#[test]
fn test_a_rows_step_pitch_moves_its_notes_where_all_are_at_the_old_one() {
  let (state, kick, snare, pattern, _, _) = drums();
  let s: &daw::InnerState = &state;
  // steps (one of them past the pattern's end): moved with it
  edit(s, None, |seq| seq.set_pattern_bars(pattern, 2, 4)).unwrap();
  set_step(s, pattern, kick, 20, true, None).unwrap();
  edit(s, None, |seq| seq.set_pattern_bars(pattern, 1, 4)).unwrap();
  // a chord on the step pitch and another: left as it is
  let chords = add_pattern_of(s, 1).unwrap().id;
  set_notes(s, chords, kick, vec![note(60, 0.), note(64, 0.)], None).unwrap();
  // only the snare's notes: not the kick's to move
  let snares = add_pattern_of(s, 1).unwrap().id;
  set_step(s, snares, snare, 2, true, None).unwrap();

  let before = revs(s);
  let had = steps(s);
  let reply = edit(s, None, |seq| seq.set_row_step_pitch(kick, 48)).unwrap();
  let after = revs(s);
  assert_eq!(steps(s), had + 1);
  assert_eq!(changed(&before.0, &after.0), vec![kick]);
  assert_eq!(changed(&before.1, &after.1), vec![pattern]);
  let kick_notes = |p: u64| -> Vec<(u8, f64)> {
    reply.pattern(p).unwrap().notes[&kick].iter().map(|n| (n.pitch, n.start)).collect()
  };
  assert_eq!(kick_notes(pattern), vec![(48, 0.), (48, 20.)]);
  assert_eq!(kick_notes(chords), vec![(60, 0.), (64, 0.)]);
  assert_eq!(reply.rows.iter().find(|r| r.id == kick).unwrap().step_pitch, 48);
  // a step clicked now is at it
  set_step(s, chords, kick, 4, true, None).unwrap();
  assert!(sequencer_state(s).pattern(chords).unwrap().notes[&kick].iter().any(|n| (n.pitch, n.start) == (48, 4.)));
  history::undo(&state).unwrap();

  // one step to undo, which puts back the revs too
  history::undo(&state).unwrap();
  assert_eq!(revs(s), before);
  assert_eq!(sequencer_state(s).pattern(pattern).unwrap().notes[&kick][1].pitch, 60);

  // the pitch it has already: no step
  edit(s, None, |seq| seq.set_row_step_pitch(kick, 60)).unwrap();
  assert_eq!(steps(s), had);
  assert_eq!(edit(s, None, |seq| seq.set_row_step_pitch(kick, 128)).unwrap_err(), "there's no note 128");
  assert!(edit(s, None, |seq| seq.set_row_step_pitch(99, 60)).unwrap_err().contains("no row with id 99"));
  assert_eq!(steps(s), had);
}

#[test]
fn test_a_sample_rows_pitch_and_gate_change_no_notes() {
  let (state, kick, _, pattern, _, _) = drums();
  let s: &daw::InnerState = &state;
  let notes = sequencer_state(s).pattern(pattern).unwrap().notes.clone();
  let sample = |change: daw::RowChange| match change.row.instrument {
    daw::InstrumentState::Sample(sample) => (sample.sample_pitch, sample.gated),
    daw::InstrumentState::Synth(_) => panic!("a sample row"),
  };

  assert_eq!(sample(edit_row(s, None, kick, |seq| seq.set_row_sample_pitch(kick, 72)).unwrap()), (72, false));
  assert_eq!(sample(edit_row(s, None, kick, |seq| seq.set_row_gated(kick, true)).unwrap()), (72, true));
  assert_eq!(sequencer_state(s).pattern(pattern).unwrap().notes, notes);

  // refused: a note MIDI hasn't, and a synth row
  let had = steps(s);
  assert_eq!(edit_row(s, None, kick, |seq| seq.set_row_sample_pitch(kick, 128)).unwrap_err(), "there's no note 128");
  let (synth, _) = synth_and_pattern(&state);
  let had_synth = steps(s);
  assert!(had_synth == had + 2);
  assert!(edit_row(s, None, synth, |seq| seq.set_row_sample_pitch(synth, 60)).unwrap_err().contains("plays a synth"));
  assert!(edit_row(s, None, synth, |seq| seq.set_row_gated(synth, true)).unwrap_err().contains("plays a synth"));
  assert_eq!(steps(s), had_synth);
}
