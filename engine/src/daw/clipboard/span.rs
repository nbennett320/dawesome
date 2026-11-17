// a time selection's span in the playlist: from one beat to another, on a
// run of tracks. copying, cutting, clearing, pasting and repeating it, as the
// playlist's keys do while there's a time selection. a clip across one of
// its ends is cut there, so only the part inside it is copied or cleared,
// and a paste or a repeat replaces whatever was in the span it lands on.
// each change is one step to undo
use std::collections::HashSet;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use super::{place, playlist_end, snap, start_of, track_of, Copied, PAST_THE_END};
use crate::daw::actions::clip::{changed_clips, ChangedClips};
use crate::daw::audiograph::AudioGraph;
use crate::daw::history::{self, ClipSnap};
use crate::daw;

// beats `start` to `end`, on tracks `top` to `bottom` (both included)
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Span {
  pub start: f64,
  pub end: f64,
  pub top: u32,
  pub bottom: u32,
}

impl Span {
  fn check(&self) -> Result<(), String> {
    if !self.start.is_finite() || !self.end.is_finite() || self.start < 0. || self.end <= self.start {
      return Err(format!("beats {} to {} aren't a span of the playlist", self.start, self.end));
    }
    if self.top > self.bottom {
      return Err(format!("tracks {} to {} aren't a run of tracks", self.top, self.bottom));
    }
    Ok(())
  }

  fn has_track(&self, track: u32) -> bool {
    track >= self.top && track <= self.bottom
  }
}

// what was copied, as a span: how long it is (in time, and in beats as it
// was), and how many tracks it covers from which
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CopiedSpan {
  pub length: Duration,
  pub beats: f64,
  pub top: u32,
  pub tracks: u32,
}

// what a span edit did: the clips it changed or made, the ones it took away,
// how many clips of patterns that are gone it left out, how many it copied
// (a cut), and the span a paste or a repeat filled
#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpanEdit {
  #[serde(flatten)]
  pub clips: ChangedClips,
  pub removed: Vec<u64>,
  pub left_out: usize,
  pub copied: usize,
  pub span: Option<Span>,
}

// how near an end a clip's edge can be and still count as on it
const ON_EDGE: Duration = Duration::from_millis(1);

fn end_in(audiograph: &AudioGraph<'static>, id: u64) -> Option<Duration> {
  snap(audiograph, id).map(|clip| super::end_of(audiograph, &clip))
}

// the clips on the span's tracks that are across any of it in time
fn clips_across(audiograph: &AudioGraph<'static>, span: &Span, from: Duration, to: Duration) -> Vec<u64> {
  let interval = audiograph.beat_interval();
  let samples = audiograph.nodes.iter()
    .filter(|node| span.has_track(node.track_number))
    .filter(|node| node.start_offset < to && node.start_offset + node.duration() > from)
    .map(|node| node.id);
  let patterns = audiograph.pattern_nodes.iter()
    .filter(|node| span.has_track(node.track_number))
    .filter(|node| node.start_offset < to && node.start_offset + node.duration(interval) > from)
    .map(|node| node.id);
  samples.chain(patterns).collect()
}

// cut clips at both ends of `from`..`to`, so the parts inside are clips of
// their own (as AudioGraph::split_node cuts). a clip too little over an end
// to be cut there stays whole. returns the clips now inside, and the halves
// the cuts made
fn cut_ends(
  audiograph: &mut AudioGraph<'static>, ids: &[u64], from: Duration, to: Duration,
) -> Result<(Vec<u64>, Vec<u64>), String> {
  let mut clips = ids.to_vec();
  let mut made = Vec::new();
  for at in [from, to] {
    for id in clips.clone() {
      if let Some(half) = audiograph.split_node(id, at)? {
        clips.push(half);
        made.push(half);
      }
    }
  }

  let inside = clips.into_iter().filter(|&id| {
    let start = audiograph.placement(id).map(|(start, _)| start);
    let end = end_in(audiograph, id);
    matches!((start, end), (Some(start), Some(end)) if start + ON_EDGE >= from && end <= to + ON_EDGE)
  }).collect();
  Ok((inside, made))
}

// take what's in the span out: clips across its ends are cut there, and
// the parts inside removed. returns what was removed, and the halves made
fn clear_in(
  audiograph: &mut AudioGraph<'static>, ids: &[u64], from: Duration, to: Duration,
) -> Result<(Vec<u64>, Vec<u64>), String> {
  let (inside, made) = cut_ends(audiograph, ids, from, to)?;
  for &id in &inside {
    audiograph.remove_node(id)?;
  }
  Ok((inside, made))
}

// copies of the parts of clips inside the span, cut at its ends, leaving
// the playlist exactly as it was
fn pieces(audiograph: &mut AudioGraph<'static>, span: &Span, from: Duration, to: Duration) -> Result<Vec<ClipSnap>, String> {
  let ids = clips_across(audiograph, span, from, to);
  let before: Vec<ClipSnap> = ids.iter().filter_map(|&id| snap(audiograph, id)).collect();
  let there: HashSet<u64> = audiograph.nodes.iter().map(|node| node.id)
    .chain(audiograph.pattern_nodes.iter().map(|node| node.id))
    .collect();

  let cut = cut_ends(audiograph, &ids, from, to);
  let copies = match &cut {
    Ok((inside, _)) => inside.iter().filter_map(|&id| snap(audiograph, id)).collect(),
    Err(_) => Vec::new(),
  };

  // put it all back: the halves go, and the clips that were cut are as
  // they were (as an undo puts them back)
  let made: Vec<u64> = audiograph.nodes.iter().map(|node| node.id)
    .chain(audiograph.pattern_nodes.iter().map(|node| node.id))
    .filter(|id| !there.contains(id))
    .collect();
  for id in made {
    let _ = audiograph.remove_node(id);
  }
  for clip in before {
    match clip {
      ClipSnap::Sample(node) => {
        let _ = audiograph.remove_node(node.id);
        audiograph.add_node(node);
      }
      ClipSnap::Pattern(node) => {
        let _ = audiograph.remove_node(node.id);
        audiograph.add_pattern_node_with_id(node);
      }
    }
  }

  cut?;
  Ok(copies)
}

fn times(audiograph: &AudioGraph<'static>, span: &Span) -> (Duration, Duration) {
  let interval = audiograph.beat_interval();
  (interval.mul_f64(span.start), interval.mul_f64(span.end))
}

// copy what's in the span to the clipboard, as a span: a paste puts it at
// the same distances from where it lands, and replaces as much as it
// covers. returns how many clips were copied
pub(crate) fn copy_span(state: &daw::Session, span: Span) -> Result<usize, String> {
  span.check()?;
  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  let (from, to) = times(&audiograph, &span);
  let copies = pieces(&mut audiograph, &span, from, to)?;

  let interval = audiograph.beat_interval().as_secs_f64();
  let mut clipboard = state.clipboard.lock().unwrap();
  clipboard.tempo = audiograph.tempo();
  clipboard.clips = copies.into_iter().map(|clip| {
    let after = start_of(&clip).saturating_sub(from);
    Copied { beats_after: if interval > 0. { after.as_secs_f64() / interval } else { 0. }, after, clip }
  }).collect();
  clipboard.span = Some(CopiedSpan {
    length: to - from,
    beats: span.end - span.start,
    top: span.top,
    tracks: span.bottom - span.top + 1,
  });
  Ok(clipboard.len())
}

// take what's in the span out, as one step: the clips after it stay where
// they are
pub(crate) fn clear_span(state: &daw::Session, span: Span) -> Result<SpanEdit, String> {
  span.check()?;
  let ids = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let (from, to) = times(&audiograph, &span);
    clips_across(&audiograph, &span, from, to)
  };
  if ids.is_empty() { return Ok(SpanEdit::default()); }

  let (removed, made) = history::clip_edit(state, &ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let (from, to) = times(&audiograph, &span);
    clear_in(&mut audiograph, &ids, from, to)
  }, |(_, made)| made.clone())?;

  let touched: Vec<u64> = ids.iter().chain(&made).copied().collect();
  Ok(SpanEdit { clips: changed_clips(state, &touched), removed, ..Default::default() })
}

// copy what's in the span, then take it out
pub(crate) fn cut_span(state: &daw::Session, span: Span) -> Result<SpanEdit, String> {
  let copied = copy_span(state, span)?;
  Ok(SpanEdit { copied, ..clear_span(state, span)? })
}

// lay copies over a span, replacing what's in it on its tracks, as one
// step. `copies` are (clip, how long after the span's start, track)
fn lay(
  state: &daw::Session, target: Span, length: Option<Duration>, copies: Vec<(ClipSnap, Duration, u32)>, what: &str,
) -> Result<SpanEdit, String> {
  target.check()?;
  let ids = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let (from, to) = times(&audiograph, &target);
    let to = length.map_or(to, |length| from + length);
    // (checked before anything's cleared: nothing changes unless it all fits)
    let end = playlist_end(state, &audiograph) + Duration::from_micros(1);
    if from > end || copies.iter().any(|(_, after, _)| from + *after > end) {
      return Err(format!("Nothing was {}: a clip {}.", what, PAST_THE_END));
    }
    clips_across(&audiograph, &target, from, to)
  };

  let (removed, cut, made, left_out) = history::clip_edit(state, &ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let (from, to) = times(&audiograph, &target);
    let to = length.map_or(to, |length| from + length);
    let (removed, cut) = clear_in(&mut audiograph, &ids, from, to)?;
    let copies = copies.into_iter().map(|(clip, after, track)| (clip, from + after, track)).collect();
    let (made, left_out) = place(state, &mut audiograph, copies, what)?;
    Ok((removed, cut, made, left_out))
  }, |(_, cut, made, _)| cut.iter().chain(made).copied().collect())?;

  let touched: Vec<u64> = ids.iter().chain(&cut).chain(&made).copied().collect();
  Ok(SpanEdit { clips: changed_clips(state, &touched), removed, left_out, copied: 0, span: Some(target) })
}

// paste the copied span with its start on `beat` and its top track on
// `track`, replacing what's there
pub(crate) fn paste_span(state: &daw::Session, beat: f64, track: u32) -> Result<SpanEdit, String> {
  if !beat.is_finite() || beat < 0. {
    return Err(format!("{} isn't a beat of the playlist", beat));
  }
  let (copied, tempo, span) = {
    let clipboard = state.clipboard.lock().unwrap();
    (clipboard.clips.clone(), clipboard.tempo, clipboard.span)
  };
  let span = span.ok_or("the clipboard holds clips, not a span")?;

  // at the tempo it was copied at, the span is exactly as long as it was
  // (so the halves of a cut clip still meet). at another, as many beats
  let (same_tempo, interval) = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    (audiograph.tempo() == tempo, audiograph.beat_interval())
  };
  let target = Span { start: beat, end: beat + span.beats, top: track, bottom: track + span.tracks - 1 };
  let copies = copied.into_iter().map(|copied| {
    let after = if same_tempo { copied.after } else { interval.mul_f64(copied.beats_after) };
    let on = track + (track_of(&copied.clip) - span.top);
    (copied.clip, after, on)
  }).collect();
  lay(state, target, same_tempo.then_some(span.length), copies, "pasted")
}

// place a copy of what's in the span straight after it, replacing what's
// there
pub(crate) fn repeat_span(state: &daw::Session, span: Span) -> Result<SpanEdit, String> {
  span.check()?;
  let (copies, length) = {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let (from, to) = times(&audiograph, &span);
    let copies: Vec<(ClipSnap, Duration, u32)> = pieces(&mut audiograph, &span, from, to)?.into_iter()
      .map(|clip| {
        let (after, track) = (start_of(&clip).saturating_sub(from), track_of(&clip));
        (clip, after, track)
      })
      .collect();
    (copies, to - from)
  };
  let target = Span { start: span.end, end: span.end + (span.end - span.start), ..span };
  lay(state, target, Some(length), copies, "repeated")
}
