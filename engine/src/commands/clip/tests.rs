use super::*;
use std::sync::Arc;
use crate::commands::clip::{add_clip, change_clip_settings};
use crate::commands::playlist::node_beat;
use crate::project;

const KICK: &str = "assets/assets_66-bd-01.wav";

fn state() -> Arc<daw::InnerState> {
  Arc::new(daw::InnerState::default())
}

fn saved(state: &daw::InnerState) -> project::Project {
  project::capture(state, None)
}

fn steps(state: &daw::InnerState) -> usize {
  state.history.lock().unwrap().len()
}

fn edit(id: u64, start: f64, end: Option<f64>) -> WindowEdit {
  WindowEdit { id, window: ClipWindow { start, end } }
}

// a sample clip a few hundred milliseconds long on beat 4, and a clip of
// a one bar pattern on beat 8: (state, sample clip, pattern clip)
fn clips() -> (Arc<daw::InnerState>, u64, u64) {
  let state = state();
  let sample = add_clip(&state, KICK.to_string(), 0, Some(4.)).unwrap().id;
  let pattern = {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    let row = sequencer.add_row(KICK).unwrap();
    let pattern = sequencer.add_pattern(1, 4).unwrap();
    sequencer.set_step(pattern, row, 0, true).unwrap();
    sequencer.set_step(pattern, row, 10, true).unwrap();
    pattern
  };
  let clip = state.playlist.audiograph.lock().unwrap()
    .add_pattern_node::<daw::timing::Note>(pattern, Duration::from_secs(4), 1, None).unwrap();
  state.history.lock().unwrap().clear();
  (state, sample, clip)
}

fn sample_length(state: &daw::InnerState, id: u64) -> f64 {
  state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().sample_duration().as_secs_f64()
}

#[test]
fn test_trimming_clips_of_both_kinds_is_one_step() {
  let (state, sample, pattern) = clips();
  let length = sample_length(&state, sample);
  assert!(length > 0.1, "the kick is too short for this test");
  let before = saved(&state);

  let changed = set_windows(&state, &[edit(sample, 0.02, Some(0.08)), edit(pattern, 4., Some(12.))]).unwrap();
  assert_eq!(steps(&state), 1);
  let after = saved(&state);
  assert_ne!(before, after);

  // what comes back is each clip as it is now
  assert_eq!(changed.clips.len(), 1);
  assert_eq!(changed.clips[0].id, sample);
  assert_eq!(changed.clips[0].info.window, Some(ClipWindow { start: 0.02, end: Some(0.08) }));
  assert!((changed.clips[0].info.played_duration - 0.06).abs() < 1e-6);
  // 20ms later than beat 4, at 120bpm
  assert!((changed.clips[0].start_beat - 4.04).abs() < 1e-5);
  assert_eq!(changed.pattern_clips.len(), 1);
  assert_eq!(changed.pattern_clips[0].window, Some(ClipWindow { start: 4., end: Some(12.) }));
  // a step later for each of the four steps hidden: a beat
  assert_eq!(changed.pattern_clips[0].start_beat, 9.);

  assert!(history::undo(&state).unwrap().is_some());
  assert_eq!(saved(&state), before);
  assert_eq!(node_beat(&state, sample), Ok(4.));
  assert_eq!(state.playlist.audiograph.lock().unwrap().get_node_by_id(sample).unwrap().window(), None);
  assert!(history::redo(&state).unwrap().is_some());
  assert_eq!(saved(&state), after);

  // a window that's the whole source again is no window
  let whole = set_windows(&state, &[edit(sample, 0., None), edit(pattern, 0., Some(16.))]).unwrap();
  assert_eq!(whole.clips[0].info.window, None);
  assert_eq!(whole.pattern_clips[0].window, None);
  assert_eq!(saved(&state), before);
}

#[test]
fn test_trimming_is_all_or_nothing() {
  let (state, sample, _) = clips();
  let before = saved(&state);

  assert!(set_windows(&state, &[edit(sample, 0.02, None), edit(9_999, 0., None)]).is_err());
  assert_eq!(saved(&state), before);
  assert_eq!(steps(&state), 0);

  // a trim that changes nothing is no step
  set_windows(&state, &[edit(sample, 0., None)]).unwrap();
  assert_eq!(steps(&state), 0);
  // numbers that make no sense are the source's ends
  set_windows(&state, &[edit(sample, f64::NAN, Some(f64::INFINITY))]).unwrap();
  set_windows(&state, &[edit(sample, -4., Some(-1.))]).unwrap();
  assert_eq!(saved(&state), before);
  assert_eq!(steps(&state), 0);
  // and an end before the start leaves the least a clip can be
  set_windows(&state, &[edit(sample, 0.05, Some(0.01))]).unwrap();
  let node = state.playlist.audiograph.lock().unwrap().get_node_by_id(sample).unwrap().clone();
  assert_eq!(node.window().unwrap().start, Duration::from_millis(50));
  assert_eq!(node.duration(), daw::MIN_CLIP);
}

#[test]
fn test_cutting_a_selection_is_one_step() {
  let (state, sample, pattern) = clips();
  // a second sample clip, which the cut isn't inside
  let elsewhere = add_clip(&state, KICK.to_string(), 2, Some(20.)).unwrap().id;
  // (the pattern clip is moved under the cut: onto beat 3)
  state.playlist.audiograph.lock().unwrap().move_node(pattern, Duration::from_millis(1_500), 1).unwrap();
  change_clip_settings(&state, sample, daw::ClipSettings { attack_ms: 20., decay_ms: 30., ..Default::default() }, None).unwrap();
  state.history.lock().unwrap().clear();
  let before = saved(&state);

  // beat 4.1: 50ms into the sample clip, and 4.4 steps into the pattern clip
  let changed = split(&state, &[sample, pattern, elsewhere], 4.1).unwrap();
  assert_eq!(steps(&state), 1);
  let after = saved(&state);

  // both halves of each clip that was cut, and nothing of the one that wasn't
  assert_eq!(changed.clips.len(), 2);
  assert_eq!(changed.pattern_clips.len(), 2);
  let (left, right) = (&changed.clips[0], &changed.clips[1]);
  assert_eq!(left.id, sample);
  assert!(right.id != sample && right.id != pattern && right.id != elsewhere);
  assert_eq!(left.start_beat, 4.);
  assert!((right.start_beat - 4.1).abs() < 1e-5);
  assert_eq!(left.info.window, Some(ClipWindow { start: 0., end: right.info.window.map(|w| w.start) }));
  assert!((right.info.window.unwrap().start - 0.05).abs() < 1e-8);
  assert_eq!(right.info.window.unwrap().end, None);
  assert_eq!((left.path.as_str(), right.track_number), (KICK, 0));
  // the fade in stays with the first half, the fade out goes with the second
  assert_eq!((left.info.settings.attack_ms, left.info.settings.decay_ms), (20., 0.));
  assert_eq!((right.info.settings.attack_ms, right.info.settings.decay_ms), (0., 30.));
  let (first, second) = (&changed.pattern_clips[0], &changed.pattern_clips[1]);
  assert_eq!(first.id, pattern);
  assert_eq!(first.window, Some(ClipWindow { start: 0., end: Some(4.4) }));
  assert_eq!(second.window, Some(ClipWindow { start: 4.4, end: None }));
  assert_eq!((first.start_beat, second.track_number), (3., 1));
  assert!((second.start_beat - 4.1).abs() < 1e-5);

  // one undo puts every clip back as it was, and takes the new ones away
  let undone = history::undo(&state).unwrap().unwrap();
  assert_eq!(undone.clips, Some(vec![sample, pattern]));
  assert_eq!(saved(&state), before);
  {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    assert_eq!((audiograph.nodes.len(), audiograph.pattern_nodes.len()), (2, 1));
  }
  // redone, the halves are the same clips again
  let redone = history::redo(&state).unwrap().unwrap();
  assert_eq!(redone.clips.unwrap().len(), 4);
  assert_eq!(saved(&state), after);
  assert_eq!(changed_clips(&state, &[right.id]).clips[0], *right);
}

#[test]
fn test_stretching_clips_is_one_step() {
  let (state, sample, pattern) = clips();
  let other = add_clip(&state, KICK.to_string(), 2, Some(20.)).unwrap().id;
  change_clip_settings(&state, other, daw::ClipSettings { pitch: 5., gain_db: -6., ..Default::default() }, None).unwrap();
  state.history.lock().unwrap().clear();
  let before = saved(&state);
  let node = |id: u64| state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().clone();
  let (was, other_was) = (node(sample), node(other));
  let ends = |clip: &daw::AudioNode| clip.start_offset + clip.duration();

  // one to twice its length from its start, the other to half its length
  // (an octave above where it was) from its end
  let edits = [
    StretchEdit { id: sample, pitch: -12., keep: Keep::Start },
    StretchEdit { id: other, pitch: 17., keep: Keep::End },
  ];
  let changed = stretch(&state, &edits).unwrap();
  assert_eq!(steps(&state), 1);
  assert_eq!(changed.clips.len(), 2);
  assert_eq!(changed.clips[0].info.settings.pitch, -12.);
  let after = saved(&state);

  // twice as long, starting where it did
  assert_eq!(node(sample).start_offset, was.start_offset);
  assert!(node(sample).duration().abs_diff(was.duration() * 2) < Duration::from_micros(1));
  // half as long, ending where it did, with its other settings as they were
  assert!(node(other).duration().abs_diff(other_was.duration() / 2) < Duration::from_micros(1));
  assert!(ends(&node(other)).abs_diff(ends(&other_was)) < Duration::from_nanos(2));
  assert!(node(other).start_offset > other_was.start_offset);
  assert_eq!(node(other).settings().gain_db, -6.);
  assert!(changed.clips[1].start_beat > 20.);

  assert!(history::undo(&state).unwrap().is_some());
  assert_eq!(saved(&state), before);
  assert_eq!(node(other).start_offset, other_was.start_offset);
  assert!(history::redo(&state).unwrap().is_some());
  assert_eq!(saved(&state), after);

  // a pitch out of range is kept in it
  let far = stretch(&state, &[StretchEdit { id: sample, pitch: -90., keep: Keep::Start }]).unwrap();
  assert_eq!(far.clips[0].info.settings.pitch, -24.);
  // a pattern clip can't be stretched, and then nothing is
  let had = (saved(&state), steps(&state));
  let bad = [StretchEdit { id: other, pitch: 0., keep: Keep::Start }, StretchEdit { id: pattern, pitch: 3., keep: Keep::Start }];
  assert!(stretch(&state, &bad).is_err());
  assert_eq!((saved(&state), steps(&state)), had);
  // the same pitch again is no step
  stretch(&state, &[StretchEdit { id: sample, pitch: -24., keep: Keep::Start }]).unwrap();
  assert_eq!(steps(&state), had.1);
}

#[test]
fn test_a_clip_stretched_from_its_end_at_the_playlists_start_stays_in_the_playlist() {
  let state = state();
  let id = add_clip(&state, KICK.to_string(), 0, Some(0.)).unwrap().id;
  // four times as long, from its end: its start can't go before beat 0
  let was = state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().duration();
  let changed = stretch(&state, &[StretchEdit { id, pitch: -24., keep: Keep::End }]).unwrap();
  assert_eq!(changed.clips[0].start_beat, 0.);
  assert_eq!(changed.clips[0].info.settings.pitch, -24.);
  // so it's its end that moved: it's four times as long from beat 0
  assert!((changed.clips[0].info.played_duration - was.as_secs_f32() * 4.).abs() < 1e-4);
}

#[test]
fn test_a_cut_lands_where_it_is_asked_for_whatever_the_snap_setting() {
  // "Split at playhead" cuts where the playhead is, and the Cut tool has
  // done its own snapping: the engine doesn't snap a cut
  let (state, sample, pattern) = clips();
  state.playlist.audiograph.lock().unwrap().move_node(pattern, Duration::from_millis(1_500), 1).unwrap();
  assert!(state.playlist.ui.lock().unwrap().snap_enabled);

  let changed = split(&state, &[sample, pattern], 4.0537).unwrap();
  assert_eq!((changed.clips.len(), changed.pattern_clips.len()), (2, 2));
  assert!((changed.clips[1].start_beat - 4.0537).abs() < 1e-5);
  assert!((changed.pattern_clips[1].start_beat - 4.0537).abs() < 1e-5);
  // the pattern clip started on beat 3: 4.2148 steps in
  assert!((changed.pattern_clips[1].window.unwrap().start - 4.2148).abs() < 1e-6);
  // and both halves still play the pattern
  assert_eq!(changed.pattern_clips[0].pattern_id, changed.pattern_clips[1].pattern_id);
}

// (run by hand, in a release build: `cargo test --release five_hundred -- --ignored --nocapture`)
#[test]
#[ignore = "a timing test"]
fn test_edits_are_quick_with_five_hundred_clips() {
  use std::time::Instant;
  let state = state();
  state.playlist.ui.lock().unwrap().snap_enabled = false;
  let ids: Vec<u64> = (0..500)
    .map(|n| add_clip(&state, KICK.to_string(), n % 10, Some((n / 10) as f32 * 4. + (n % 10) as f32 * 0.25)).unwrap().id)
    .collect();
  state.history.lock().unwrap().clear();
  let limit = Duration::from_millis(50);
  let mut slowest = Duration::ZERO;
  let mut timed = |what: &str, run: &mut dyn FnMut()| {
    let started = Instant::now();
    run();
    let took = started.elapsed();
    println!("{:>28}: {:?}", what, took);
    assert!(took < limit, "{} took {:?}", what, took);
    slowest = slowest.max(took);
  };

  let one = ids[250];
  timed("trim one clip", &mut || { set_windows(&state, &[edit(one, 0.02, Some(0.1))]).unwrap(); });
  timed("cut one clip", &mut || { split(&state, &[one], 100.1).unwrap(); });
  timed("stretch one clip", &mut || { stretch(&state, &[StretchEdit { id: ids[251], pitch: -5., keep: Keep::Start }]).unwrap(); });
  timed("move one clip", &mut || { crate::commands::clip::move_clip(&state, ids[252], 3, Some(7.3)).unwrap(); });
  timed("trim fifty clips", &mut || {
    let edits: Vec<WindowEdit> = ids[..50].iter().map(|&id| edit(id, 0.01, Some(0.12))).collect();
    set_windows(&state, &edits).unwrap();
  });
  timed("copy fifty clips", &mut || { crate::clipboard::copy(&state, &ids[50..100]); });
  timed("paste fifty clips", &mut || { crate::clipboard::paste(&state, 210., None).unwrap(); });
  timed("repeat fifty clips", &mut || { crate::clipboard::repeat(&state, &ids[100..150]).unwrap(); });
  timed("delete fifty clips", &mut || { crate::commands::clip::remove_clips(&state, &ids[150..200]).unwrap(); });
  let state_arc = state.clone();
  timed("undo", &mut || { history::undo(&state_arc).unwrap(); });
  timed("the project after an undo", &mut || { crate::commands::history::project_view(&state); });
  timed("redo", &mut || { history::redo(&state_arc).unwrap(); });
  println!("{:>28}: {:?}", "slowest", slowest);
}

#[test]
fn test_a_cut_that_cuts_nothing_is_no_step() {
  let (state, sample, pattern) = clips();
  let before = saved(&state);

  // beat 30 is inside neither clip, and beat 4 is the sample clip's edge
  for beat in [30., 4., 4.001] {
    assert_eq!(split(&state, &[sample, pattern], beat).unwrap(), ChangedClips::default());
  }
  assert_eq!(saved(&state), before);
  assert_eq!(steps(&state), 0);

  assert!(split(&state, &[sample, 9_999], 4.1).is_err());
  assert!(split(&state, &[sample], f64::NAN).is_err());
  assert!(split(&state, &[sample], -1.).is_err());
  assert_eq!(saved(&state), before);
}

#[test]
fn test_add_and_move_snap_to_the_snap_setting() {
  let state = daw::InnerState::default();
  let sample = daw::METRONOME_TICK_PATH.to_string();
  let ms = |id: u64| state.playlist.audiograph.lock().unwrap()
    .get_node_by_id(id).unwrap().start_offset.as_millis();

  // default snap is one beat (500ms at 120bpm)
  let id = add_node_at_offset(&state, sample.clone(), 0, Duration::from_millis(700), true).unwrap();
  assert_eq!(ms(id), 500);
  let id2 = add_node_at_offset(&state, sample.clone(), 0, Duration::from_millis(800), true).unwrap();
  assert_eq!(ms(id2), 1_000);

  // snap disabled keeps the exact position
  let id3 = add_node_at_offset(&state, sample.clone(), 0, Duration::from_millis(700), false).unwrap();
  assert_eq!(ms(id3), 700);

  // a different snap setting is respected (sixteenth notes = 125ms)
  state.playlist.ui.lock().unwrap().snap_subdivision = daw::timing::Note::SixteenthNote;
  let id4 = add_node_at_offset(&state, sample, 0, Duration::from_millis(700), true).unwrap();
  assert_eq!(ms(id4), 750);
}

#[test]
fn test_clip_settings() {
  let state = daw::InnerState::default();
  let sample = daw::METRONOME_TICK_PATH.to_string();
  let id = add_node_at_offset(&state, sample, 0, Duration::ZERO, false).unwrap();

  let info = get_clip_of(&state, id).unwrap();
  assert_eq!(info.settings, daw::ClipSettings::default());
  assert_eq!(info.sample_duration, info.played_duration);

  // an octave up halves the played length, and out of range values are kept in range
  let info = set_clip_settings_of(&state, id, daw::ClipSettings {
    pitch: 12.,
    gain_db: 99.,
    ..Default::default()
  }).unwrap();
  assert_eq!(info.settings.pitch, 12.);
  assert_eq!(info.settings.gain_db, 12.);
  assert!((info.played_duration - info.sample_duration / 2.).abs() < 1e-4);
  assert_eq!(get_clip_of(&state, id).unwrap(), info);

  // the frontend's names deserialize
  let settings: daw::ClipSettings = serde_json::from_str(
    r#"{"gainDb":-6,"pan":0.5,"pitch":0,"attackMs":10,"decayMs":20}"#).unwrap();
  assert_eq!(settings.pan, 0.5);

  assert!(get_clip_of(&state, 9999).is_err());
}

#[test]
fn test_snapped_nodes_report_their_beat() {
  let state = daw::InnerState::default();
  let sample = daw::METRONOME_TICK_PATH.to_string();

  // dropped at beat 1.4 snaps to beat 1 (default snap is one beat)
  let id = add_node_at_offset(&state, sample, 0, beat_to_offset(&state, Some(1.4)), true).unwrap();
  assert_eq!(node_beat(&state, id).unwrap(), 1.);

  // moved to beat 3.6 snaps to beat 4
  move_audiograph_node_to(&state, id, 1, beat_to_offset(&state, Some(3.6))).unwrap();
  assert_eq!(node_beat(&state, id).unwrap(), 4.);

  // offset_to_beat undoes beat_to_offset when nothing snaps
  let offset = beat_to_offset(&state, Some(2.46));
  assert!((offset_to_beat(&state, offset) - 2.46).abs() < 0.001);

  assert!(node_beat(&state, 9_999).is_err());
}

#[test]
fn test_move_a_group_of_nodes() {
  let state = daw::InnerState::default();
  let sample = daw::METRONOME_TICK_PATH.to_string();
  // 120bpm, one-beat snap. b is off the grid, a quarter beat after a
  let a = add_node_at_offset(&state, sample.clone(), 0, beat_to_offset(&state, Some(2.)), true).unwrap();
  let b = add_node_at_offset(&state, sample.clone(), 1, beat_to_offset(&state, Some(2.25)), false).unwrap();
  let c = add_node_at_offset(&state, sample, 3, beat_to_offset(&state, Some(9.)), true).unwrap();
  let beat = |id: u64| node_beat(&state, id).unwrap();

  // a dropped at beat 4.4 on track 1: snaps to 4, b keeps its quarter
  // beat gap, and everything moves down a track
  let moved = move_audiograph_nodes_to(&state, &[a, b], a, 4.4, 1).unwrap();
  assert_eq!(moved, vec![
    MovedNode { id: a, start_beat: 4., track_number: 1 },
    MovedNode { id: b, start_beat: 4.25, track_number: 2 },
  ]);
  // c wasn't in the group
  assert_eq!(beat(c), 9.);

  // well past the first four bars
  move_audiograph_nodes_to(&state, &[a, c], c, 236., 3).unwrap();
  assert_eq!((beat(a), beat(c)), (231., 236.));

  // dragged past the end of the playlist (512 beats): c stops at the end
  move_audiograph_nodes_to(&state, &[a, c], c, 600., 3).unwrap();
  assert_eq!((beat(a), beat(c)), (507., 512.));

  // with snap off the grabbed clip lands exactly where it's dropped
  state.playlist.ui.lock().unwrap().snap_enabled = false;
  move_audiograph_nodes_to(&state, &[a, b], b, 1.5, 2).unwrap();
  assert!((beat(b) - 1.5).abs() < 0.001);
}
