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

fn sample(state: &daw::InnerState, track: u32, beat: f32) -> u64 {
  add_clip(state, KICK.to_string(), track, Some(beat)).unwrap().id
}

// a one bar pattern with a kick on its first step, and a clip of it
fn pattern_clip(state: &daw::InnerState, track: u32, beat: f64) -> (u64, u64) {
  let pattern = {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    let row = sequencer.add_row(KICK).unwrap();
    let pattern = sequencer.add_pattern(1, 4).unwrap();
    sequencer.set_step(pattern, row, 0, true).unwrap();
    pattern
  };
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let at = audiograph.beat_interval().mul_f64(beat);
  let clip = audiograph.add_pattern_node::<daw::timing::Note>(pattern, at, track, None).unwrap();
  (pattern, clip)
}

fn node(state: &daw::InnerState, id: u64) -> daw::AudioNode {
  state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().clone()
}

#[test]
fn test_paste_puts_the_earliest_clip_on_the_beat_and_keeps_the_rest_apart() {
  let state = state();
  // on tracks 0 and 2, three and a half beats apart, and a pattern clip
  let (a, b) = (sample(&state, 0, 2.), sample(&state, 2, 5.5));
  let (pattern, c) = pattern_clip(&state, 1, 4.);
  change_clip_settings(&state, b, daw::ClipSettings { pitch: 3., pan: 0.5, ..Default::default() }, None).unwrap();
  crate::commands::clip::set_windows(&state, &[crate::commands::clip::WindowEdit {
    id: b, window: crate::commands::clip::ClipWindow { start: 0.02, end: Some(0.09) },
  }]).unwrap();
  let b_beat = node_beat(&state, b).unwrap();
  state.history.lock().unwrap().clear();

  assert_eq!(copy(&state, &[b, a, c]), 3);
  let before = saved(&state);
  let pasted = paste(&state, 32., None).unwrap();
  assert_eq!(steps(&state), 1);
  assert_eq!(pasted.left_out, 0);

  // the earliest on beat 32, on the tracks they were copied from
  let clips = &pasted.clips.clips;
  assert_eq!(clips.len(), 2);
  assert_eq!((clips[0].start_beat, clips[0].track_number), (32., 0));
  assert!((clips[1].start_beat - (32. + b_beat - 2.)).abs() < 1e-5);
  assert_eq!(clips[1].track_number, 2);
  let patterns = &pasted.clips.pattern_clips;
  assert_eq!((patterns[0].start_beat, patterns[0].track_number, patterns[0].pattern_id), (34., 1, pattern));
  // new clips, with the trim and settings of the ones they were copied from
  assert!(![a, b, c].contains(&clips[1].id));
  assert_eq!(clips[1].info.settings, node(&state, b).settings());
  assert_eq!(clips[1].info.window, node(&state, b).window().map(crate::commands::clip::ClipWindow::of_sample));
  assert_eq!(clips[0].path, KICK);

  // pasted again, a second set lands in the same place
  let again = paste(&state, 32., None).unwrap();
  assert_eq!(again.clips.clips[0].start_beat, 32.);
  assert_ne!(again.clips.clips[0].id, clips[0].id);
  assert_eq!(state.playlist.audiograph.lock().unwrap().nodes.len(), 6);

  // each paste is one step
  assert!(history::undo(&state).unwrap().is_some());
  let undone = history::undo(&state).unwrap().unwrap();
  assert_eq!(undone.clips, Some(vec![]));
  assert_eq!(saved(&state), before);
  let redone = history::redo(&state).unwrap().unwrap();
  assert_eq!(redone.clips.unwrap().len(), 3);
}

#[test]
fn test_paste_on_a_track_moves_every_clip_by_as_many_tracks() {
  let state = state();
  let (a, b) = (sample(&state, 1, 2.), sample(&state, 3, 3.));
  copy(&state, &[a, b]);

  // the topmost copied clip (track 1) on track 4: the other two below it
  let pasted = paste(&state, 16.25, Some(4)).unwrap();
  let clips = &pasted.clips.clips;
  assert_eq!((clips[0].start_beat, clips[0].track_number), (16.25, 4));
  assert_eq!((clips[1].start_beat, clips[1].track_number), (17.25, 6));
  // and above where it was copied from
  let up = paste(&state, 8., Some(0)).unwrap();
  assert_eq!(up.clips.clips.iter().map(|clip| clip.track_number).collect::<Vec<_>>(), vec![0, 2]);
}

#[test]
fn test_a_paste_past_the_end_pastes_nothing() {
  let state = state();
  let (a, b) = (sample(&state, 0, 2.), sample(&state, 1, 10.));
  copy(&state, &[a, b]);
  state.history.lock().unwrap().clear();
  let before = saved(&state);

  // the second clip would start on beat 514, past bar 128
  let refused = paste(&state, 506., None).unwrap_err();
  assert!(refused.contains("past the end of the playlist"), "{}", refused);
  assert_eq!(saved(&state), before);
  assert_eq!(steps(&state), 0);

  // the last beat itself is still the playlist
  assert!(paste(&state, 504., None).is_ok());
  assert!(paste(&state, f64::NAN, None).is_err());
  assert!(paste(&state, -1., None).is_err());
}

#[test]
fn test_a_pasted_clip_is_its_own_clip_of_the_same_sound() {
  let state = state();
  let original = sample(&state, 0, 2.);
  copy(&state, &[original]);
  let pasted = paste(&state, 8., None).unwrap().clips.clips[0].id;
  let heard = |id: u64| -> Vec<f32> {
    let clip = node(&state, id);
    clip.voice(48_000, clip.start_offset.as_nanos() as i128, 0).collect()
  };
  let sound = heard(original);
  assert!(sound.iter().any(|&sample| sample != 0.));
  assert!(heard(pasted) == sound);

  // trimmed, pitched and turned down, the pasted clip changes nothing
  // about the one it was copied from
  crate::commands::clip::set_windows(&state, &[crate::commands::clip::WindowEdit {
    id: pasted, window: crate::commands::clip::ClipWindow { start: 0.03, end: Some(0.06) },
  }]).unwrap();
  change_clip_settings(&state, pasted, daw::ClipSettings { pitch: -5., gain_db: -9., ..Default::default() }, None).unwrap();
  assert!(heard(pasted) != sound);
  assert!(heard(original) == sound);
  assert_eq!(node(&state, original).window(), None);
  assert_eq!(node(&state, original).settings(), daw::ClipSettings::default());
  // and the same the other way round
  let trimmed = heard(pasted);
  change_clip_settings(&state, original, daw::ClipSettings { pitch: 7., ..Default::default() }, None).unwrap();
  assert!(heard(pasted) == trimmed);
}

#[test]
fn test_copy_and_paste_write_no_file() {
  let dir = std::env::temp_dir().join(format!("dawesome-clipboard-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  let take = dir.join("take.wav");
  std::fs::copy(KICK, &take).unwrap();
  let listing = || {
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap()
      .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
      .collect();
    names.sort();
    names
  };

  let state = state();
  let id = add_clip(&state, take.to_string_lossy().into_owned(), 0, Some(1.)).unwrap().id;
  copy(&state, &[id]);
  paste(&state, 4., None).unwrap();
  cut(&state, &[id]).unwrap();
  paste(&state, 9., Some(2)).unwrap();
  let first = state.playlist.audiograph.lock().unwrap().nodes[0].id;
  repeat(&state, &[first]).unwrap();
  assert_eq!(listing(), vec!["take.wav".to_string()]);
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_cutting_takes_the_clips_out_and_leaves_the_rest() {
  let state = state();
  let (a, b, c) = (sample(&state, 0, 2.), sample(&state, 1, 4.), sample(&state, 0, 9.));
  state.history.lock().unwrap().clear();
  let before = saved(&state);

  assert_eq!(cut(&state, &[a, b]).unwrap(), 2);
  assert_eq!(steps(&state), 1);
  // gone, with nothing else moved
  {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    assert_eq!(audiograph.nodes.len(), 1);
    assert_eq!(audiograph.nodes[0].id, c);
  }
  assert_eq!(node_beat(&state, c), Ok(9.));

  // and they can be pasted
  let pasted = paste(&state, 20., None).unwrap();
  assert_eq!(pasted.clips.clips.iter().map(|clip| clip.start_beat).collect::<Vec<_>>(), vec![20., 22.]);
  history::undo(&state).unwrap();
  // one undo brings both back
  assert!(history::undo(&state).unwrap().is_some());
  assert_eq!(saved(&state), before);

  // with nothing to cut or copy, the clipboard keeps what it had
  assert_eq!(cut(&state, &[]).unwrap(), 2);
  assert_eq!(copy(&state, &[]), 2);
  assert_eq!(copy(&state, &[9_999]), 2);
  // a clip that isn't there: nothing is cut
  let had = steps(&state);
  assert!(cut(&state, &[c, 9_999]).is_err());
  assert_eq!(steps(&state), had);
  assert_eq!(state.clipboard.lock().unwrap().len(), 2);
}

#[test]
fn test_a_clip_given_twice_is_copied_once() {
  let state = state();
  let a = sample(&state, 0, 2.);
  assert_eq!(copy(&state, &[a, a]), 1);
  assert_eq!(repeat(&state, &[a, a]).unwrap().clips.clips.len(), 1);
}

#[test]
fn test_a_paste_stops_at_the_end_of_a_three_four_playlist() {
  let state = state();
  let a = sample(&state, 0, 2.);
  copy(&state, &[a]);
  crate::commands::transport::set_time_signature(&state, daw::timing::TimeSignature { numerator: 3, denominator: 4 });

  // 128 bars of three beats: beat 384 is the last
  assert!(paste(&state, 384., None).is_ok());
  assert!(paste(&state, 385., None).unwrap_err().contains(PAST_THE_END));
}

#[test]
fn test_the_last_beat_is_still_the_playlist_at_any_tempo() {
  let state = state();
  let (a, b) = (sample(&state, 0, 2.), sample(&state, 1, 5.));
  for tempo in [97., 133.3, 61.7] {
    crate::commands::transport::change_tempo(&state, tempo, None).unwrap();
    copy(&state, &[a, b]);
    // the second clip lands on beat 512, the last
    assert!(paste(&state, 509., None).is_ok(), "at {}bpm", tempo);
    assert!(paste(&state, 509.01, None).is_err(), "at {}bpm", tempo);
  }
}

#[test]
fn test_a_clip_of_a_pattern_thats_gone_is_left_out() {
  let state = state();
  let a = sample(&state, 0, 2.);
  let (pattern, c) = pattern_clip(&state, 1, 3.);
  copy(&state, &[a, c]);
  crate::commands::sequencer::remove_pattern_for_tests(&state, pattern);

  // (left out even where it would have landed past the end: the paste
  // isn't refused for a clip that isn't pasted)
  assert_eq!(paste(&state, 255.5, None).unwrap().left_out, 1);
  let pasted = paste(&state, 8., None).unwrap();
  assert_eq!(pasted.left_out, 1);
  assert_eq!(pasted.clips.clips.len(), 1);
  assert!(pasted.clips.pattern_clips.is_empty());
  assert_eq!(pasted.clips.clips[0].start_beat, 8.);
}

#[test]
fn test_repeat_starts_exactly_where_the_span_ends() {
  let state = state();
  // off the grid: from beat 2 to wherever the second clip ends
  state.playlist.ui.lock().unwrap().snap_enabled = false;
  let (a, b) = (sample(&state, 0, 2.), sample(&state, 2, 5.));
  let (_, c) = pattern_clip(&state, 1, 3.);
  state.history.lock().unwrap().clear();
  let before = saved(&state);
  let (first, last) = (node(&state, a), node(&state, b));
  let span = last.start_offset + last.duration() - first.start_offset;

  // the pattern clip ends on beat 7, after the second sample clip: the
  // span is from beat 2 to beat 7
  let pattern_end = Duration::from_millis(3_500);
  assert!(pattern_end > last.start_offset + last.duration());
  let repeated = repeat(&state, &[b, c, a]).unwrap();
  assert_eq!(steps(&state), 1);
  let clips = &repeated.clips.clips;
  let by_track = |track: u32| node(&state, clips.iter().find(|clip| clip.track_number == track).unwrap().id);
  // to the nanosecond, on the same tracks, the same distance apart
  assert_eq!(by_track(0).start_offset, pattern_end);
  assert_eq!(by_track(2).start_offset - by_track(0).start_offset, last.start_offset - first.start_offset);
  assert_eq!((repeated.clips.pattern_clips[0].start_beat, repeated.clips.pattern_clips[0].track_number), (8., 1));
  assert!(span < pattern_end);

  // without the pattern clip, the span ends with the last sample clip,
  // wherever that is
  history::undo(&state).unwrap();
  assert_eq!(saved(&state), before);
  let repeated = repeat(&state, &[a, b]).unwrap();
  let first_copy = node(&state, repeated.clips.clips.iter().map(|clip| clip.id).min().unwrap());
  assert_eq!(first_copy.start_offset, first.start_offset + span);

  // the clipboard is as it was: empty
  assert_eq!(state.clipboard.lock().unwrap().len(), 0);
  assert!(repeat(&state, &[]).unwrap().clips.clips.is_empty());
  assert!(repeat(&state, &[9_999]).is_err());
}

#[test]
fn test_a_repeat_past_the_end_repeats_nothing() {
  let state = state();
  let (a, b) = (sample(&state, 0, 260.), sample(&state, 1, 500.));
  state.history.lock().unwrap().clear();
  let before = saved(&state);

  // from beat 260 to past beat 500: the second duplicate would be past bar 128
  let refused = repeat(&state, &[a, b]).unwrap_err();
  assert!(refused.contains("past the end of the playlist"), "{}", refused);
  assert_eq!(saved(&state), before);
  assert_eq!(steps(&state), 0);
}

#[test]
fn test_the_halves_of_a_cut_clip_stay_together_when_pasted() {
  let state = state();
  state.playlist.ui.lock().unwrap().snap_enabled = false;
  let id = sample(&state, 0, 2.337);
  let changed = crate::commands::clip::split(&state, &[id], 2.4).unwrap();
  let right = changed.clips[1].id;
  let (left_node, right_node) = (node(&state, id), node(&state, right));
  assert_eq!(left_node.origin(), right_node.origin());

  copy(&state, &[id, right]);
  let pasted = paste(&state, 40.123, None).unwrap();
  let (a, b) = (node(&state, pasted.clips.clips[0].id), node(&state, pasted.clips.clips[1].id));
  // the second still starts on the very nanosecond the first ends on
  assert_eq!(a.origin(), b.origin());
  assert_eq!(a.start_offset + a.duration(), b.start_offset);

  // and repeated
  let repeated = repeat(&state, &[id, right]).unwrap();
  let (c, d) = (node(&state, repeated.clips.clips[0].id), node(&state, repeated.clips.clips[1].id));
  assert_eq!(c.origin(), d.origin());
  assert_eq!(c.start_offset, right_node.start_offset + right_node.duration());
}

#[test]
fn test_a_looping_clip_and_its_cut_halves_are_pasted_and_repeated_as_they_play() {
  let state = state();
  // round its one bar pattern two and a half times, from beat 3, then cut
  // on its second time round (step 20.5)
  let (pattern, c) = pattern_clip(&state, 1, 3.);
  crate::commands::clip::set_windows(&state, &[crate::commands::clip::WindowEdit {
    id: c, window: crate::commands::clip::ClipWindow { start: 0., end: Some(40.) },
  }]).unwrap();
  let right = crate::commands::clip::split(&state, &[c], 3. + 20.5 / 4.).unwrap().pattern_clips[1].id;
  let clip = |id: u64| state.playlist.audiograph.lock().unwrap().get_pattern_node_by_id(id).unwrap().clone();
  let beat = state.playlist.audiograph.lock().unwrap().beat_interval();
  assert_eq!(((clip(c).start, clip(c).end()), (clip(right).start, clip(right).end())), ((0., 20.5), (20.5, 40.)));

  // pasted, each keeps its own part of its pattern (past the pattern's end
  // too), and the second still starts on the nanosecond the first ends on
  copy(&state, &[c, right]);
  state.playlist.sequencer.lock().unwrap().set_pattern_bars(pattern, 2, 4).unwrap();
  let pasted = paste(&state, 40., None).unwrap();
  let (a, b) = (clip(pasted.clips.pattern_clips[0].id), clip(pasted.clips.pattern_clips[1].id));
  assert_eq!(((a.start, a.end()), (b.start, b.end())), ((0., 20.5), (20.5, 40.)));
  assert_eq!(a.origin(beat), b.origin(beat));
  assert_eq!(a.start_offset + a.duration(beat), b.start_offset);

  // repeated, the copies start where the clip ends: its own end, not its
  // pattern's
  let repeated = repeat(&state, &[c, right]).unwrap();
  let first = repeated.clips.pattern_clips.iter().map(|copy| clip(copy.id)).min_by_key(|copy| copy.start_offset).unwrap();
  assert_eq!(first.start_offset, clip(right).start_offset + clip(right).duration(beat));
  assert_eq!(first.start_offset, beat.mul_f64(3. + 10.));
}

#[test]
fn test_pasted_at_another_tempo_clips_keep_their_beats() {
  let state = state();
  let (a, b) = (sample(&state, 0, 2.), sample(&state, 1, 6.));
  copy(&state, &[a, b]);
  crate::commands::transport::change_tempo(&state, 90., None).unwrap();

  let pasted = paste(&state, 16., None).unwrap();
  let beats: Vec<f32> = pasted.clips.clips.iter().map(|clip| clip.start_beat).collect();
  assert!((beats[0] - 16.).abs() < 1e-5 && (beats[1] - 20.).abs() < 1e-5, "{:?}", beats);
}

#[test]
fn test_a_new_project_empties_the_clipboard() {
  let state = state();
  let a = sample(&state, 0, 2.);
  assert_eq!(copy(&state, &[a]), 1);
  project::apply(&state, &project::Project::default(), None);
  assert_eq!(state.clipboard.lock().unwrap().len(), 0);
  assert_eq!(paste(&state, 4., None).unwrap(), Placed::default());
}
