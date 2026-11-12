// "Join": making one clip of several on a track
//
// sample clips that are neighbouring parts of one sample, still where a cut
// left them, are put back together, and nothing is written. any other
// sample clips are written out together as one new file (see bounce.rs),
// from the earliest start to the latest end, with silence in the gaps and
// overlaps added together
//
// pattern clips make a new pattern holding their steps and notes one after
// another, and are replaced by one clip of it (or, for the halves of a cut
// clip, by one clip of the pattern they were cut from)
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::time::Duration;
use serde::Serialize;

use crate::bounce;
use crate::clip_commands::{changed_clips, ChangedClips};
use crate::daw;
use crate::history;
use crate::project_files;

pub const ONE_TRACK: &str = "Join needs the clips to be on one track.";
pub const ONE_KIND: &str = "Join needs clips of one kind: sample clips, or pattern clips.";
pub const SAME_GRID: &str =
  "Join needs the pattern clips to sit on the same sixteenth-note grid as the first one.";

// what a join made
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Joined {
  // the joined clip, which is the first of the clips, changed
  #[serde(flatten)]
  pub clips: ChangedClips,
  // the other clips, which are gone
  pub removed: Vec<u64>,
  // the sequencer, when joining pattern clips made a pattern
  pub sequencer: Option<daw::SequencerState>,
}

// join clips into one (see the top of the file). the clips have to be on
// one track and of one kind. the joined clip is the earliest of them, with
// its id. one step to undo
// (async: writing sample clips out as one file can take a while)
#[tauri::command(async)]
pub fn join_clips(state: daw::TState<'_>, ids: Vec<u64>) -> Result<Joined, String> {
  join(&state, &ids, daw::open_engine_rate())
}

// - rate: the engine's rate, for a file that has to be written. None when
//   no output is open: the first clip's sample's own rate is used
pub(crate) fn join(state: &daw::InnerState, ids: &[u64], rate: Option<u32>) -> Result<Joined, String> {
  let mut ids = ids.to_vec();
  ids.sort_unstable();
  ids.dedup();
  if ids.len() < 2 {
    return Err("Join needs two or more clips.".into());
  }

  // which kind they are, in the order they start
  let (samples, patterns) = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    if let Some(missing) = ids.iter().find(|&&id| audiograph.placement(id).is_none()) {
      return Err(format!("no node with id {}", missing));
    }
    let tracks: HashSet<u32> = ids.iter().filter_map(|&id| audiograph.placement(id)).map(|(_, track)| track).collect();
    if tracks.len() > 1 { return Err(ONE_TRACK.into()); }

    let mut samples: Vec<daw::AudioNode> = ids.iter().filter_map(|&id| audiograph.get_node_by_id(id).cloned()).collect();
    let mut patterns: Vec<daw::PatternNode> = ids.iter().filter_map(|&id| audiograph.get_pattern_node_by_id(id).cloned()).collect();
    samples.sort_by_key(|node| (node.start_offset, node.id));
    patterns.sort_by_key(|node| (node.start_offset, node.id));
    (samples, patterns)
  };

  match (samples.is_empty(), patterns.is_empty()) {
    (false, true) => join_samples(state, samples, rate),
    (true, false) => join_patterns(state, patterns),
    _ => Err(ONE_KIND.into()),
  }
}

// whether sample clips, in the order they start, are neighbouring parts of
// one sample that still meet: played one after another, they're what one
// clip of the lot would play
fn samples_line_up(clips: &[daw::AudioNode]) -> bool {
  clips.windows(2).all(|pair| {
    let (left, right) = (&pair[0], &pair[1]);
    let (a, b) = (left.settings(), right.settings());
    left.sample_path() == right.sample_path()
      && (a.gain_db, a.pan, a.pitch) == (b.gain_db, b.pan, b.pitch)
      // no fade where they meet
      && a.decay_ms == 0. && b.attack_ms == 0.
      // the second starts in the sample where the first ends, and in the
      // playlist on the nanosecond it ends on
      && left.window().and_then(|window| window.end) == Some(right.window_start())
      && left.origin() == right.origin()
  })
}

fn join_samples(state: &daw::InnerState, clips: Vec<daw::AudioNode>, rate: Option<u32>) -> Result<Joined, String> {
  let ids: Vec<u64> = clips.iter().map(|node| node.id).collect();
  let (first, rest) = (ids[0], ids[1..].to_vec());

  if samples_line_up(&clips) {
    // one clip again: from where the first starts to where the last ends,
    // with the first's fade in and the last's fade out
    let last = clips.last().expect("two or more clips");
    let (start, end) = (clips[0].window_start(), last.window().and_then(|window| window.end));
    let decay_ms = last.settings().decay_ms;

    history::clip_edit(state, &ids, None, || {
      let mut audiograph = state.playlist.audiograph.lock().unwrap();
      if let Some(gone) = ids.iter().find(|&&id| audiograph.get_node_by_id(id).is_none()) {
        return Err(format!("no node with id {}", gone));
      }
      for &id in &rest { audiograph.remove_node(id)?; }
      let node = audiograph.get_mut_node_by_id(first).expect("a clip that was just found");
      node.restore_window(start, end);
      node.set_settings(daw::ClipSettings { decay_ms, ..node.settings() });
      Ok(())
    }, |_| vec![])?;

    return Ok(Joined { clips: changed_clips(state, &[first]), removed: rest, sequencer: None });
  }

  // written out as one file
  let dir = bounce::clips_dir(state)?;
  let _writing = bounce::writing();
  let rate = rate.unwrap_or_else(|| clips[0].sample_rate());
  let frame = |at: Duration| daw::run_frame(at.as_nanos() as i128, rate);
  let starts = frame(clips[0].start_offset);
  let mut mixed: Vec<f32> = Vec::new();
  for clip in &clips {
    // each on the frame the engine starts it on, as it's heard in a run
    // from the playlist's start (unclipped, until they're added together)
    let at = (frame(clip.start_offset) - starts).max(0) as usize * 2;
    let samples: Vec<f32> = clip.voice(rate, clip.start_offset.as_nanos() as i128, 0).collect();
    if mixed.len() < at + samples.len() { mixed.resize(at + samples.len(), 0.); }
    for (sample, into) in samples.iter().zip(mixed[at..].iter_mut()) { *into += sample; }
  }
  for sample in mixed.iter_mut() { *sample = sample.clamp(-1., 1.); }
  if mixed.is_empty() {
    return Err("Join needs clips that play something.".into());
  }

  let name = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    bounce::source_name(&audiograph, first, "join").ok_or_else(|| format!("no node with id {}", first))?
  };
  let path = project_files::numbered_path(&dir, &name, "join", &HashSet::new());
  bounce::write_wav(&path, rate, &mixed)?;

  let swapped = daw::AudioNode::new(first, path.to_string_lossy().into_owned(), Duration::ZERO, 0)
    .and_then(|mut node| history::clip_edit(state, &ids, None, || {
      let mut audiograph = state.playlist.audiograph.lock().unwrap();
      // a clip removed or changed while the file was being written: the
      // file isn't what the clips play any more, and nothing changes
      if !clips.iter().all(|clip| bounce::unchanged(&audiograph, clip.id, &history::ClipSnap::Sample(clip.clone()))) {
        return Err(bounce::CHANGED_MEANWHILE.into());
      }
      let (start, track) = audiograph.placement(first).expect("a clip that was just found");
      node.start_offset = start;
      node.track_number = track;
      for &id in &ids { audiograph.remove_node(id)?; }
      audiograph.add_node(node);
      Ok(())
    }, |_| vec![]));
  if let Err(e) = swapped {
    let _ = std::fs::remove_file(&path);
    return Err(e);
  }

  Ok(Joined { clips: changed_clips(state, &[first]), removed: rest, sequencer: None })
}

// whether pattern clips, in the order they start, are neighbouring parts
// of one pattern that still meet
fn patterns_line_up(clips: &[daw::PatternNode], beat: Duration) -> bool {
  clips.windows(2).all(|pair| {
    let (left, right) = (&pair[0], &pair[1]);
    left.pattern_id == right.pattern_id
      && left.window.and_then(|window| window.end).is_some()
      && left.window.and_then(|window| window.end) == right.window.map(|window| window.start)
      && left.origin(beat) == right.origin(beat)
  })
}

// a whole number, if `steps` is one but for rounding
fn whole(steps: f64) -> Option<u32> {
  let rounded = steps.round();
  if steps >= -1e-6 && (steps - rounded).abs() < 1e-6 { Some(rounded.max(0.) as u32) } else { None }
}

// what a pattern is called: its name, or its number
fn label(pattern: &daw::Pattern) -> String {
  pattern.name.clone().unwrap_or_else(|| (pattern.id + 1).to_string())
}

// the joined patterns' names, in order: "a + b", or past three of them,
// "a + 3 more"
pub fn joined_name(names: &[String]) -> String {
  match names {
    [] => String::new(),
    [first, rest @ ..] if rest.len() >= 3 => format!("{} + {} more", first, rest.len()),
    _ => names.join(" + "),
  }
}

fn join_patterns(state: &daw::InnerState, clips: Vec<daw::PatternNode>) -> Result<Joined, String> {
  let ids: Vec<u64> = clips.iter().map(|node| node.id).collect();
  let (first, rest) = (ids[0], ids[1..].to_vec());
  let before = history::picture(state, &ids);

  let joined = (|| -> Result<Option<()>, String> {
    // the graph is locked before the sequencer, as everywhere
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let beat = audiograph.beat_interval();
    // a step's length, in seconds (not rounded to a whole nanosecond: over
    // a few bars that would add up to more than "on the grid" allows)
    let step = beat.as_secs_f64() / daw::STEPS_PER_BEAT as f64;
    if !(step > 0.) { return Err("there's no tempo to join at".into()); }
    if let Some(gone) = ids.iter().find(|&&id| audiograph.get_pattern_node_by_id(id).is_none()) {
      return Err(format!("no node with id {}", gone));
    }

    // the halves of a cut clip: one clip of their pattern again
    if patterns_line_up(&clips, beat) {
      let window = daw::PatternWindow {
        start: clips[0].window.map_or(0., |window| window.start),
        end: clips.last().and_then(|clip| clip.window).and_then(|window| window.end),
      };
      for &id in &rest { audiograph.remove_node(id)?; }
      let node = audiograph.pattern_nodes.iter_mut().find(|n| n.id == first).expect("a clip that was just found");
      node.window = if window.start == 0. && window.end.is_none() { None } else { Some(window) };
      return Ok(None);
    }

    let mut sequencer = audiograph.sequencer.lock().unwrap();
    let beats_per_bar = state.playlist.time_signature.lock().unwrap().numerator;
    let mut steps: BTreeMap<u64, BTreeSet<u32>> = BTreeMap::new();
    let mut notes: BTreeMap<u64, Vec<daw::PatternNote>> = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    let mut names = Vec::new();
    // how many steps the clips span, from the first's start
    let mut span = 0f64;

    for clip in &clips {
      let pattern = sequencer.pattern(clip.pattern_id)
        .ok_or_else(|| format!("no pattern with id {}", clip.pattern_id))?;
      kinds.insert(pattern.kind);
      if kinds.len() > 1 { return Err("Drum patterns and note patterns can't be joined".into()); }
      names.push(label(pattern));
      let (from, to) = clip.span(pattern.length);
      // where its steps go in the new pattern: it has to start a whole
      // number of steps after the first clip, on a step of its own pattern
      let after = (clip.start_offset - clips[0].start_offset).as_secs_f64() / step;
      let (at, from_step) = whole(after).zip(whole(from)).ok_or(SAME_GRID)?;

      for (row, lit) in &pattern.steps {
        for &step in lit.iter().filter(|&&step| clip.plays(step, pattern.length)) {
          steps.entry(*row).or_default().insert(at + step - from_step);
        }
      }
      // and its notes, as the clip plays them: those that start in it, cut
      // at its end
      for (synth, synth_notes) in &pattern.notes {
        for note in synth_notes.iter().filter(|note| note.start >= from && note.start < to) {
          notes.entry(*synth).or_default().push(daw::PatternNote {
            start: note.start - from + at as f64,
            length: note.end().min(to) - note.start,
            ..*note
          });
        }
      }
      span = span.max(after + (to - from));
    }

    // as long as the clips' span, up to the next bar line
    let bar = daw::steps_in_bars(1, beats_per_bar);
    let length = ((span - 1e-6) / bar as f64).ceil().max(1.) as u32 * bar;
    let made = sequencer.add_pattern_after(clips[0].pattern_id, joined_name(&names), length, steps, notes)?;

    drop(sequencer);

    for &id in &rest { audiograph.remove_node(id)?; }
    let node = audiograph.pattern_nodes.iter_mut().find(|n| n.id == first).expect("a clip that was just found");
    node.pattern_id = made;
    // exactly as long as the clips were, so nothing after it moves
    node.window = if span < length as f64 - 1e-6 {
      Some(daw::PatternWindow { start: 0., end: Some((span * 1e6).round() / 1e6) })
    } else {
      None
    };
    Ok(Some(()))
  })();

  // (recorded whatever happened: on a refusal nothing changed, and that's
  // no step)
  history::record_between(state, None, before, history::picture(state, &ids));
  let sequencer = joined?.map(|_| crate::synth_commands::sequencer_state(state));
  Ok(Joined { clips: changed_clips(state, &[first]), removed: rest, sequencer })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::{Path, PathBuf};
  use std::sync::Arc;
  use std::time::Instant;
  use crate::clip_commands::{add_clip, change_clip_settings, ClipWindow};
  use crate::project;

  const KICK: &str = "assets/assets_66-bd-01.wav";
  const SNARE: &str = "assets/assets_66-sd-01.wav";
  const STEP: f32 = 1. / 32_768.;

  fn saved_project(name: &str) -> (Arc<daw::InnerState>, PathBuf) {
    let dir = std::env::temp_dir().join(format!("dawesome-join-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(dir).unwrap();
    let state = Arc::new(daw::InnerState::default());
    state.project.lock().unwrap().reset(Some(dir.join("song.dawesome")));
    (state, dir)
  }

  fn noise_wav(dir: &Path, name: &str, rate: u32, frames: u32, loud: f32) -> String {
    let path = dir.join(name);
    let spec = hound::WavSpec { channels: 2, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    let mut seed = name.len() as u32 + 3;
    for frame in 0..frames {
      for channel in 0..2u32 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let noise = ((seed >> 16) % 4_000) as f32 - 2_000.;
        let tone = (frame as f32 * (0.03 + channel as f32 * 0.01)).sin() * 8_000.;
        writer.write_sample(((tone + noise) * loud) as i16).unwrap();
      }
    }
    writer.finalize().unwrap();
    path.to_string_lossy().into_owned()
  }

  // what's heard of the playlist from its start, on an engine at `rate`
  fn play(state: &daw::InnerState, rate: u32, total: Duration) -> Vec<f32> {
    let mut graph = state.playlist.audiograph.lock().unwrap();
    let (engine, mut renderer) = daw::Engine::new(rate);
    let beat = graph.beat_interval();
    graph.init();
    let mut voices = Vec::new();
    let mut at = Duration::ZERO;
    while at < total {
      graph.set_current_offset(Some(at));
      for (delay, source, route) in graph.run_sounds_at(at, rate, beat, false, at.is_zero()) {
        voices.push(engine.boxed_voice(source, Some(1), engine.frames_in(at + delay), route));
      }
      at += beat;
    }
    graph.pause();
    engine.start_run(1, graph.mixer.master(), voices).unwrap();
    let mut out = vec![0f32; engine.frames_in(total) as usize * 2];
    renderer.render(&mut out, Instant::now(), Duration::ZERO);
    out
  }

  fn worst(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0., f32::max)
  }

  fn node(state: &daw::InnerState, id: u64) -> daw::AudioNode {
    state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().clone()
  }

  fn saved(state: &daw::InnerState) -> project::Project {
    project::capture(state, None)
  }

  fn steps(state: &daw::InnerState) -> usize {
    state.history.lock().unwrap().len()
  }

  #[test]
  fn test_the_halves_of_a_cut_are_put_back_together_and_nothing_is_written() {
    let rate = 48_000;
    let total = Duration::from_secs(3);
    // an unsaved project: there's nowhere a file could be written
    let state = Arc::new(daw::InnerState::default());
    state.playlist.ui.lock().unwrap().snap_enabled = false;
    let id = add_clip(&state, KICK.to_string(), 1, Some(1.337)).unwrap().id;
    change_clip_settings(&state, id, daw::ClipSettings {
      pitch: -5., gain_db: -3., pan: 0.2, attack_ms: 10., decay_ms: 20.,
    }, None).unwrap();
    let whole = node(&state, id);
    let original = play(&state, rate, total);
    let before = saved(&state);

    // cut in three, then joined
    let right = crate::clip_commands::split(&state, &[id], 1.45).unwrap().clips[1].id;
    let last = crate::clip_commands::split(&state, &[right], 1.6).unwrap().clips[1].id;
    state.history.lock().unwrap().clear();
    let joined = join(&state, &[last, id, right], Some(rate)).unwrap();

    assert_eq!(joined.clips.clips.len(), 1);
    assert_eq!(joined.clips.clips[0].id, id);
    assert_eq!(joined.removed, vec![right, last]);
    assert_eq!(joined.sequencer, None);
    assert_eq!(steps(&state), 1);
    // the clip it was, fades and all, playing exactly what it did
    let after = node(&state, id);
    assert_eq!(after.window(), None);
    assert_eq!((after.start_offset, after.settings()), (whole.start_offset, whole.settings()));
    assert_eq!(saved(&state), before);
    assert!(play(&state, rate, total) == original);

    // one undo: three clips again
    let undone = history::undo(&state).unwrap().unwrap();
    assert_eq!(undone.clips.unwrap().len(), 3);
    assert_eq!(state.playlist.audiograph.lock().unwrap().nodes.len(), 3);
    assert!(play(&state, rate, total) == original);
  }

  #[test]
  fn test_halves_that_no_longer_line_up_are_not_healed() {
    let state = Arc::new(daw::InnerState::default());
    let cut = |beat: f32| {
      let id = add_clip(&state, KICK.to_string(), 0, Some(beat)).unwrap().id;
      let right = crate::clip_commands::split(&state, &[id], beat as f64 + 0.1).unwrap().clips[1].id;
      (id, right)
    };
    let refused = |ids: &[u64]| join(&state, ids, Some(44_100)).unwrap_err();

    // one moved, one with another pitch, one with a fade where they meet,
    // and the same two halves in the other order in time: each would have
    // to be written out, and the project isn't saved
    let (a, b) = cut(2.);
    state.playlist.audiograph.lock().unwrap().move_node(b, Duration::from_millis(1_100), 0).unwrap();
    assert_eq!(refused(&[a, b]), bounce::NEEDS_SAVING);
    let (a, b) = cut(8.);
    change_clip_settings(&state, b, daw::ClipSettings { pitch: 1., ..Default::default() }, None).unwrap();
    assert_eq!(refused(&[a, b]), bounce::NEEDS_SAVING);
    let (a, b) = cut(16.);
    change_clip_settings(&state, a, daw::ClipSettings { decay_ms: 5., ..Default::default() }, None).unwrap();
    assert_eq!(refused(&[a, b]), bounce::NEEDS_SAVING);
    // two whole clips of the same file end to end aren't parts of one clip
    let a = add_clip(&state, KICK.to_string(), 0, Some(24.)).unwrap().id;
    let b = add_clip(&state, KICK.to_string(), 0, Some(25.)).unwrap().id;
    assert_eq!(refused(&[a, b]), bounce::NEEDS_SAVING);
  }

  #[test]
  fn test_other_sample_clips_are_written_out_as_one_file() {
    let rate = 48_000;
    let total = Duration::from_secs(5);
    let (state, dir) = saved_project("bounce");
    let one = noise_wav(&dir, "First.wav", 44_100, 30_000, 1.);
    let two = noise_wav(&dir, "second.wav", 48_000, 40_000, 1.);
    // a beat's gap between the first two, and the third over the second
    let a = add_clip(&state, one, 2, Some(1.25)).unwrap().id;
    let b = add_clip(&state, two.clone(), 2, Some(3.75)).unwrap().id;
    let c = add_clip(&state, two, 2, Some(4.5)).unwrap().id;
    change_clip_settings(&state, a, daw::ClipSettings { pitch: -4., pan: -0.5, attack_ms: 30., ..Default::default() }, None).unwrap();
    change_clip_settings(&state, b, daw::ClipSettings { gain_db: -9., ..Default::default() }, None).unwrap();
    change_clip_settings(&state, c, daw::ClipSettings { gain_db: -9., pan: 0.5, decay_ms: 100., ..Default::default() }, None).unwrap();
    crate::clip_commands::set_windows(&state, &[crate::clip_commands::WindowEdit {
      id: c, window: ClipWindow { start: 0.1, end: Some(0.6) },
    }]).unwrap();
    let (start, track) = (node(&state, a).start_offset, 2);
    let end = [a, b, c].iter().map(|&id| node(&state, id)).map(|n| n.start_offset + n.duration()).max().unwrap();
    let original = play(&state, rate, total);
    state.history.lock().unwrap().clear();
    let before = project::capture(&state, Some(&dir));

    let joined = join(&state, &[c, a, b], Some(rate)).unwrap();
    // one clip, the first one, from the earliest start, as it comes
    assert_eq!(joined.removed, vec![b, c]);
    let after = node(&state, a);
    assert_eq!(state.playlist.audiograph.lock().unwrap().nodes.len(), 1);
    assert_eq!((after.start_offset, after.track_number), (start, track));
    assert_eq!((after.settings(), after.window()), (daw::ClipSettings::default(), None));
    // named after the first clip's file
    let file = dir.join("clips").join("First_join_1.wav");
    assert_eq!(after.sample_path(), file.to_string_lossy());
    // to the latest end, give or take a frame
    let frames = (hound::WavReader::open(&file).unwrap().len() / 2) as i64;
    let span = daw::run_frame(end.as_nanos() as i128, rate) - daw::run_frame(start.as_nanos() as i128, rate);
    assert!((frames - span).abs() <= 1, "{} frames for a span of {}", frames, span);

    // it plays what the three did, gap, overlap and all
    let copy = play(&state, rate, total);
    let off = worst(&copy, &original);
    assert!(off <= STEP, "off by {} steps", off / STEP);
    // (and that wasn't silence, before or in the overlap)
    let at = |beat: f64| (beat * 0.5 * rate as f64) as usize * 2;
    assert!(original[at(1.5)..at(2.)].iter().any(|&s| s != 0.));
    assert!(original[at(3.3)..at(3.7)].iter().all(|&s| s == 0.));
    assert!(original[at(4.6)..at(4.9)].iter().any(|&s| s != 0.));

    // one step: undone, the three clips are back and the file stays
    assert_eq!(steps(&state), 1);
    assert!(history::undo(&state).unwrap().is_some());
    assert_eq!(project::capture(&state, Some(&dir)), before);
    assert!(play(&state, rate, total) == original);
    assert!(file.is_file());
    // redone, no second file
    history::redo(&state).unwrap();
    assert_eq!(std::fs::read_dir(dir.join("clips")).unwrap().count(), 1);
    // a second join is numbered on
    let d = add_clip(&state, KICK.to_string(), 2, Some(20.)).unwrap().id;
    let again = join(&state, &[a, d], Some(rate)).unwrap();
    assert!(again.clips.clips[0].path.ends_with("First_join_2.wav"), "{}", again.clips.clips[0].path);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_clips_too_loud_together_are_cut_off_at_full_scale() {
    let rate = 44_100;
    let (state, dir) = saved_project("loud");
    let path = noise_wav(&dir, "loud.wav", rate, 20_000, 2.5);
    let a = add_clip(&state, path.clone(), 0, Some(0.)).unwrap().id;
    let b = add_clip(&state, path, 0, Some(0.)).unwrap().id;
    join(&state, &[a, b], Some(rate)).unwrap();
    let mut reader = hound::WavReader::open(dir.join("clips").join("loud_join_1.wav")).unwrap();
    let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();
    assert!(samples.iter().all(|s| s.abs() <= 1.));
    assert!(samples.iter().filter(|s| s.abs() == 1.).count() > 100);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_what_join_refuses() {
    let (state, dir) = saved_project("refusals");
    let a = add_clip(&state, KICK.to_string(), 0, Some(0.)).unwrap().id;
    let b = add_clip(&state, KICK.to_string(), 1, Some(2.)).unwrap().id;
    let c = add_clip(&state, KICK.to_string(), 0, Some(4.)).unwrap().id;
    let pattern = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let row = sequencer.add_row(KICK).unwrap();
      let pattern = sequencer.add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
      sequencer.set_step(pattern, row, 0, true).unwrap();
      pattern
    };
    let p = state.playlist.audiograph.lock().unwrap()
      .add_pattern_node::<daw::timing::Note>(pattern, Duration::from_secs(8), 0, None).unwrap();
    state.history.lock().unwrap().clear();
    let before = project::capture(&state, Some(&dir));

    // on two tracks, of two kinds, too few, or not there
    assert_eq!(join(&state, &[a, b], Some(44_100)).unwrap_err(), ONE_TRACK);
    assert_eq!(join(&state, &[a, c, p], Some(44_100)).unwrap_err(), ONE_KIND);
    assert!(join(&state, &[a], Some(44_100)).is_err());
    assert!(join(&state, &[a, a], Some(44_100)).is_err());
    assert!(join(&state, &[a, 9_999], Some(44_100)).is_err());
    assert!(join(&state, &[], Some(44_100)).is_err());
    assert_eq!(project::capture(&state, Some(&dir)), before);
    assert_eq!(steps(&state), 0);
    assert!(!dir.join("clips").exists());
    let _ = std::fs::remove_dir_all(&dir);
  }

  // rows of kick and snare, and two one bar patterns: "a" with the kick on
  // steps 0 and 8, and an unnamed one with the snare on 4 and 12
  fn two_patterns(state: &daw::InnerState) -> (u64, u64, u64, u64) {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    let kick = sequencer.add_row(KICK).unwrap();
    let snare = sequencer.add_row(SNARE).unwrap();
    let a = sequencer.add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
    let b = sequencer.add_pattern(1, 4, daw::PatternKind::Drum).unwrap();
    sequencer.rename_pattern(a, "a").unwrap();
    for (pattern, row, step) in [(a, kick, 0), (a, kick, 8), (b, snare, 4), (b, snare, 12), (b, kick, 15)] {
      sequencer.set_step(pattern, row, step, true).unwrap();
    }
    (kick, snare, a, b)
  }

  fn pattern_clip(state: &daw::InnerState, pattern: u64, beat: f64) -> u64 {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let at = audiograph.beat_interval().mul_f64(beat);
    audiograph.add_pattern_node::<daw::timing::Note>(pattern, at, 1, None).unwrap()
  }

  #[test]
  fn test_pattern_clips_become_one_clip_of_a_new_pattern() {
    let rate = 44_100;
    let total = Duration::from_secs(8);
    let state = Arc::new(daw::InnerState::default());
    let (kick, snare, a, b) = two_patterns(&state);
    // a bar of "a" on bar 2, a bar's gap, then the other on bar 4
    let (first, second) = (pattern_clip(&state, a, 4.), pattern_clip(&state, b, 12.));
    let original = play(&state, rate, total);
    let patterns_before = crate::synth_commands::sequencer_state(&state);
    state.history.lock().unwrap().clear();
    let before = saved(&state);

    let joined = join(&state, &[second, first], Some(rate)).unwrap();
    assert_eq!(steps(&state), 1);
    assert_eq!(joined.removed, vec![second]);
    assert!(joined.clips.clips.is_empty());
    let clip = &joined.clips.pattern_clips[0];
    assert_eq!((clip.id, clip.start_beat, clip.track_number), (first, 4., 1));

    // a new three bar pattern, named after both ("2" is the second's number),
    // straight after the first: the two patterns are as they were
    let sequencer = joined.sequencer.expect("a new pattern");
    assert_eq!(sequencer.patterns.len(), 3);
    assert_eq!(sequencer.patterns[0], patterns_before.patterns[0]);
    assert_eq!(sequencer.patterns[2], patterns_before.patterns[1]);
    let made = &sequencer.patterns[1];
    assert_eq!(made.id, clip.pattern_id);
    assert_eq!((made.name.as_deref(), made.length), (Some("a + 2"), 48));
    // the first's steps, an empty bar, then the second's
    assert_eq!(made.steps[&kick], BTreeSet::from([0, 8, 47]));
    assert_eq!(made.steps[&snare], BTreeSet::from([36, 44]));
    // three bars is the whole pattern: no window
    assert_eq!(clip.window, None);

    // every hit where it was
    assert!(play(&state, rate, total) == original);

    // undone: the pattern is gone and both clips are back
    let undone = history::undo(&state).unwrap().unwrap();
    assert_eq!(undone.clips, Some(vec![first, second]));
    assert_eq!(saved(&state), before);
    assert_eq!(crate::synth_commands::sequencer_state(&state), patterns_before);
    assert!(play(&state, rate, total) == original);
    history::redo(&state).unwrap();
    assert!(play(&state, rate, total) == original);
  }

  #[test]
  fn test_joined_pattern_clips_keep_their_span_and_merge_where_they_overlap() {
    let rate = 44_100;
    let total = Duration::from_secs(6);
    let state = Arc::new(daw::InnerState::default());
    let (kick, snare, a, b) = two_patterns(&state);
    // the second starts half way through the first, and is trimmed to end
    // two steps early: 16 + 8 - 2 steps in all
    let (first, second) = (pattern_clip(&state, a, 2.), pattern_clip(&state, b, 4.));
    crate::clip_commands::set_windows(&state, &[crate::clip_commands::WindowEdit {
      id: second, window: ClipWindow { start: 0., end: Some(14.) },
    }]).unwrap();
    let original = play(&state, rate, total);

    let joined = join(&state, &[first, second], Some(rate)).unwrap();
    let made = joined.sequencer.unwrap().patterns.into_iter().find(|p| p.id == joined.clips.pattern_clips[0].pattern_id).unwrap();
    // 22 steps, so a two bar pattern, and a clip trimmed to the 22
    assert_eq!(made.length, 32);
    assert_eq!(joined.clips.pattern_clips[0].window, Some(ClipWindow { start: 0., end: Some(22.) }));
    // the kick's two steps, and the snare's from step 8 on (the second
    // clip's kick on step 15 was trimmed off)
    assert_eq!(made.steps[&kick], BTreeSet::from([0, 8]));
    assert_eq!(made.steps[&snare], BTreeSet::from([12, 20]));
    assert!(play(&state, rate, total) == original);
    assert_eq!(state.playlist.audiograph.lock().unwrap().end(), Duration::from_millis(1_000 + 2_750));
  }

  #[test]
  fn test_joined_pattern_clips_keep_the_notes_they_play() {
    let state = Arc::new(daw::InnerState::default());
    let note = |pitch, start, length| daw::PatternNote { pitch, start, length, velocity: 0.5 };
    let (a, b, s) = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      sequencer.add_synth(daw::Synth {
        id: 40, plugin_id: "vst3:synth".into(), path: String::new(), name: "Synth".into(),
        vendor: String::new(), volume: 1., muted: false,
      }).unwrap();
      let a = sequencer.add_pattern(1, 4, daw::PatternKind::Note).unwrap();
      let b = sequencer.add_pattern(1, 4, daw::PatternKind::Note).unwrap();
      sequencer.set_pattern_notes(a, 40, vec![note(60, 2., 4.), note(61, 13., 3.)]).unwrap();
      sequencer.set_pattern_notes(b, 40, vec![note(62, 0.5, 1.)]).unwrap();
      (a, b, 40)
    };
    let first = pattern_clip(&state, a, 4.);
    let second = pattern_clip(&state, b, 8.);
    // the first plays up to step 14: its second note is cut there
    state.playlist.audiograph.lock().unwrap().set_pattern_window(first, 0., Some(14.)).unwrap();

    let joined = join(&state, &[first, second], Some(44_100)).unwrap();
    let sequencer = joined.sequencer.expect("a new pattern");
    let made = sequencer.patterns.iter().find(|p| p.id == joined.clips.pattern_clips[0].pattern_id).unwrap();
    assert_eq!(made.kind, daw::PatternKind::Note);
    assert_eq!(made.notes[&s], vec![note(60, 2., 4.), note(61, 13., 1.), note(62, 16.5, 1.)]);

    // a drum pattern's clip and a note pattern's aren't joined
    let (_, _, drums, _) = two_patterns(&state);
    let third = pattern_clip(&state, drums, 16.);
    let fourth = pattern_clip(&state, a, 20.);
    assert!(join(&state, &[third, fourth], Some(44_100)).unwrap_err().contains("can't be joined"));
  }

  #[test]
  fn test_the_halves_of_a_cut_pattern_clip_are_one_clip_of_its_pattern_again() {

    let state = Arc::new(daw::InnerState::default());
    let (_, _, a, _) = two_patterns(&state);
    let clip = pattern_clip(&state, a, 4.);
    let before = saved(&state);
    let patterns = crate::synth_commands::sequencer_state(&state);
    // cut between steps, and again
    let right = crate::clip_commands::split(&state, &[clip], 5.375).unwrap().pattern_clips[1].id;
    let last = crate::clip_commands::split(&state, &[right], 7.).unwrap().pattern_clips[1].id;

    let joined = join(&state, &[clip, right, last], None).unwrap();
    assert_eq!(joined.sequencer, None);
    assert_eq!(joined.removed, vec![right, last]);
    assert_eq!(joined.clips.pattern_clips[0].window, None);
    assert_eq!(joined.clips.pattern_clips[0].pattern_id, a);
    // no new pattern
    assert_eq!(crate::synth_commands::sequencer_state(&state), patterns);
    assert_eq!(saved(&state), before);
  }

  #[test]
  fn test_pattern_clips_off_the_grid_are_not_joined() {
    let state = Arc::new(daw::InnerState::default());
    let (_, _, a, b) = two_patterns(&state);
    // a tenth of a step late
    let (first, second) = (pattern_clip(&state, a, 4.), pattern_clip(&state, b, 8.025));
    state.history.lock().unwrap().clear();
    let before = saved(&state);
    assert_eq!(join(&state, &[first, second], None).unwrap_err(), SAME_GRID);
    assert_eq!(saved(&state), before);
    assert_eq!(steps(&state), 0);
    assert_eq!(state.playlist.sequencer.lock().unwrap().patterns().len(), 2);

    // a clip cut between two steps starts off its own pattern's grid
    let third = pattern_clip(&state, b, 16.);
    let right = crate::clip_commands::split(&state, &[third], 17.125).unwrap().pattern_clips[1].id;
    let fourth = pattern_clip(&state, a, 24.125);
    assert_eq!(join(&state, &[right, fourth], None).unwrap_err(), SAME_GRID);
  }

  #[test]
  fn test_pattern_clips_on_the_beat_are_on_the_grid_at_any_tempo() {
    // (tempos whose beats aren't a whole number of nanoseconds, far enough
    // into the playlist for a rounded step to have drifted)
    for tempo in [90., 97., 110., 140., 150., 61.3] {
      let state = Arc::new(daw::InnerState::default());
      state.playlist.audiograph.lock().unwrap().set_tempo(tempo);
      let (_, _, a, b) = two_patterns(&state);
      let (first, second) = (pattern_clip(&state, a, 4.), pattern_clip(&state, b, 244.));
      let joined = join(&state, &[first, second], None);
      assert!(joined.is_ok(), "at {}bpm: {:?}", tempo, joined.err());
      // sixty bars and one more: 61 in all
      let sequencer = joined.unwrap().sequencer.unwrap();
      assert_eq!(sequencer.patterns.iter().map(|p| p.length).max(), Some(61 * 16), "at {}bpm", tempo);
    }
  }

  #[test]
  fn test_a_join_that_cant_be_written_changes_nothing() {
    let (state, dir) = saved_project("fails");
    let one = noise_wav(&dir, "one.wav", 44_100, 4_410, 1.);
    let a = add_clip(&state, one.clone(), 0, Some(0.)).unwrap().id;
    let b = add_clip(&state, one, 0, Some(2.)).unwrap().id;
    state.history.lock().unwrap().clear();
    let before = project::capture(&state, Some(&dir));
    // something in the way of the file being written
    std::fs::create_dir_all(dir.join("clips").join(".one_join_1.wav.part").join("in the way")).unwrap();

    let failed = join(&state, &[a, b], Some(44_100)).unwrap_err();
    assert!(failed.contains("one_join_1.wav"), "{}", failed);
    assert_eq!(project::capture(&state, Some(&dir)), before);
    assert_eq!(steps(&state), 0);
    assert!(!dir.join("clips").join("one_join_1.wav").exists());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_joined_names() {
    let names = |list: &[&str]| joined_name(&list.iter().map(|name| name.to_string()).collect::<Vec<_>>());
    assert_eq!(names(&["a", "b"]), "a + b");
    assert_eq!(names(&["verse", "2", "fill"]), "verse + 2 + fill");
    assert_eq!(names(&["verse", "2", "fill", "4"]), "verse + 3 more");
    assert_eq!(names(&["verse", "verse", "verse", "verse", "verse"]), "verse + 4 more");
  }
}
