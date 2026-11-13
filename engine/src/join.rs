// "Join": making one clip of several on a track
//
// sample clips that are neighbouring parts of one sample, still where a cut
// left them, are put back together, and nothing is written. any other
// sample clips are written out together as one new file (see bounce.rs),
// from the earliest start to the latest end, with silence in the gaps and
// overlaps added together
//
// pattern clips make a new pattern holding their notes one after another,
// each row's together, and are replaced by one clip of it (or, for the
// halves of a cut clip, by one clip of the pattern they were cut from)
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;
use serde::Serialize;

use crate::bounce;
use crate::commands::clip::{changed_clips, ChangedClips};
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
      && left.end() == right.start
      && left.origin(beat) == right.origin(beat)
  })
}

// a whole number, if `steps` is one but for rounding
fn whole(steps: f64) -> Option<u32> {
  let rounded = steps.round();
  if steps >= -1e-6 && (steps - rounded).abs() < 1e-6 { Some(rounded.max(0.) as u32) } else { None }
}

// what a pattern is called (see Pattern::label)
fn label(pattern: &daw::Pattern) -> String {
  pattern.label()
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
      let (start, end) = (clips[0].start, clips.last().expect("two or more clips").end());
      for &id in &rest { audiograph.remove_node(id)?; }
      let node = audiograph.pattern_nodes.iter_mut().find(|n| n.id == first).expect("a clip that was just found");
      node.set_span(start, end - start);
      return Ok(None);
    }

    let mut sequencer = audiograph.sequencer.lock().unwrap();
    let beats_per_bar = state.playlist.time_signature.lock().unwrap().numerator;
    let mut notes: BTreeMap<u64, Vec<daw::PatternNote>> = BTreeMap::new();
    let mut names = Vec::new();
    // how many steps the clips span, from the first's start
    let mut span = 0f64;

    for clip in &clips {
      let pattern = sequencer.pattern(clip.pattern_id)
        .ok_or_else(|| format!("no pattern with id {}", clip.pattern_id))?;
      names.push(label(pattern));
      // where its notes go in the new pattern: it has to start a whole
      // number of steps after the first clip, on a step of its own pattern
      let after = (clip.start_offset - clips[0].start_offset).as_secs_f64() / step;
      let (at, from_step) = whole(after).zip(whole(clip.start)).ok_or(SAME_GRID)?;

      // its notes as the clip plays them: each time through its pattern,
      // cut at the end of each and at the clip's end, each row's with the
      // other clips' notes of that row
      for (&row, row_notes) in &pattern.notes {
        for pass in clip.passes(pattern.length) {
          for note in pass.notes(row_notes) {
            let (starts, ends) = pass.plays(note);
            notes.entry(row).or_default().push(daw::PatternNote {
              start: starts - from_step as f64 + at as f64,
              length: ends - starts,
              ..*note
            });
          }
        }
      }
      span = span.max(after + clip.length);
    }
    // (a long clip of a short pattern can hold more than a pattern can)
    if notes.values().any(|row| row.len() > daw::MAX_NOTES) {
      return Err(format!("a pattern can hold at most {} notes for a row", daw::MAX_NOTES));
    }

    // as long as the clips' span, up to the next bar line
    let bar = daw::steps_in_bars(1, beats_per_bar);
    let bars = ((span - 1e-6) / bar as f64).ceil().max(1.) as u32;
    if bars > daw::PATTERN_BARS_MAX {
      return Err(format!("Join needs the pattern clips to span no more than {} bars.", daw::PATTERN_BARS_MAX));
    }
    let length = bars * bar;
    let made = sequencer.add_pattern_after(clips[0].pattern_id, joined_name(&names), length, notes)?;

    drop(sequencer);

    for &id in &rest { audiograph.remove_node(id)?; }
    let node = audiograph.pattern_nodes.iter_mut().find(|n| n.id == first).expect("a clip that was just found");
    node.pattern_id = made;
    // exactly as long as the clips were, so nothing after it moves
    node.set_span(0., span);
    Ok(Some(()))
  })();

  // (recorded whatever happened: on a refusal nothing changed, and that's
  // no step)
  history::record_between(state, None, before, history::picture(state, &ids));
  let sequencer = joined?.map(|_| crate::commands::synth::sequencer_state(state));
  Ok(Joined { clips: changed_clips(state, &[first]), removed: rest, sequencer })
}

#[cfg(test)]
mod tests;
