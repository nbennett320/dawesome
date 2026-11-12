// "Deep copy": writing a clip out as a file of its own, and playing that
//
// the file holds the clip as it plays: its trim, pitch and speed, fades,
// pan and gain (not its track's strip, or the master), in stereo 32-bit
// float at the engine's rate. the clip is then switched to it, with every
// setting back at its default, so nothing is applied twice. the file it
// played before is left alone
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use rodio::source::UniformSourceIterator;

use crate::commands::clip::{changed_clips, ChangedClips};
use crate::daw;
use crate::history;
use crate::project_files;

// how long the end of a written-out pattern clip is faded over, so a hit
// still ringing there doesn't end in a click
const PATTERN_TAIL: Duration = Duration::from_millis(10);
// the rate written at when no output is open and there's no sample to go by
const FALLBACK_RATE: u32 = 44_100;
// what a sample is written as, so that it's read back as near as can be.
// the files are read back as 16-bit, by multiplying by 32767
// and dropping the fraction, which on its own can land two 16-bit steps
// from the sample that was written, and a step lower each time a copy is
// copied. written a hair louder (0.0003 dB) and half a step further from
// zero, what's read back is the nearest 16-bit step to the sample: within
// half a step of it (a whole one only at full scale), and a copy of a copy
// reads back the same
fn written(sample: f32) -> f32 {
  let half = if sample > 0. { 0.5 } else if sample < 0. { -0.5 } else { 0. };
  ((sample * 32_768. + half) / 32_767.).clamp(-1., 1.)
}

lazy_static! {
  // one clip writer at a time, from choosing a file's name to the clip
  // playing it: two at once could choose the same name
  static ref WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

// (held by Deep copy and Join while they work)
pub(crate) fn writing() -> std::sync::MutexGuard<'static, ()> {
  // (a writer that panicked hasn't left anything this guards half done)
  WRITING.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// a sample clip as it's heard in a run from the playlist's start: left and
// right, at `rate`, nothing above full scale
pub fn render_sample(node: &daw::AudioNode, rate: u32) -> Vec<f32> {
  node.voice(rate, node.start_offset.as_nanos() as i128, 0)
    .map(|sample| sample.clamp(-1., 1.))
    .collect()
}

// a pattern clip's hits as they are now, ready to be mixed (see
// mix_pattern): how many frames the clip lasts, and each hit's sound with
// the frame of the clip it starts on. the steps lit, each row at its
// volume, muted rows silent, at the tempo as it is
pub struct PatternHits {
  frames: usize,
  hits: Vec<(usize, daw::VoiceSource)>,
}

pub fn pattern_hits(audiograph: &daw::AudioGraph<'static>, node: &daw::PatternNode, rate: u32) -> Result<PatternHits, String> {
  let sequencer = audiograph.sequencer.lock().unwrap();
  let pattern = sequencer.pattern(node.pattern_id)
    .ok_or_else(|| format!("no pattern with id {}", node.pattern_id))?;
  let beat = audiograph.beat_interval();
  let frame = |at: Duration| daw::run_frame(at.as_nanos() as i128, rate);

  let starts = frame(node.start_offset);
  let frames = (frame(node.start_offset + node.duration(pattern.length, beat)) - starts).max(0) as usize;
  let hits = sequencer.hit_steps(node.pattern_id).into_iter()
    .filter(|(_, step)| node.plays(*step, pattern.length))
    // on the frame the engine would start it on
    .map(|(row, step)| {
      let at = (frame(node.hit_at(step, beat)) - starts).max(0) as usize;
      (at, Box::new(row.source()) as daw::VoiceSource)
    })
    .collect();
  Ok(PatternHits { frames, hits })
}

// a pattern clip as it's heard, for as long as the clip is. a hit still
// sounding at the clip's end is faded out
pub fn mix_pattern(PatternHits { frames, hits }: PatternHits, rate: u32) -> Vec<f32> {
  let mut out = vec![0f32; frames * 2];
  for (at, source) in hits {
    let hit = UniformSourceIterator::<_, f32>::new(source, 2, rate);
    for (sample, mixed) in hit.zip(out.iter_mut().skip(at * 2)) {
      *mixed += sample;
    }
  }

  let tail = ((PATTERN_TAIL.as_secs_f64() * rate as f64).round() as usize).min(frames);
  for i in 0..tail {
    let level = i as f32 / tail as f32;
    let at = (frames - 1 - i) * 2;
    out[at] *= level;
    out[at + 1] *= level;
  }
  for sample in out.iter_mut() { *sample = sample.clamp(-1., 1.); }
  out
}

// write stereo samples to a 32-bit float WAV. written beside its place
// under another name first, so a file that's half written never has the
// name. nothing is ever written over: the finished file is given its name
// by a link, which fails if the name's taken, rather than by a rename,
// which would replace what's there
pub fn write_wav(path: &Path, rate: u32, samples: &[f32]) -> Result<(), String> {
  if path.exists() {
    return Err(format!("'{}' is already there", path.display()));
  }
  let dir = path.parent().unwrap_or_else(|| Path::new(""));
  std::fs::create_dir_all(dir)
    .map_err(|e| format!("could not make the folder '{}': {}", dir.display(), e))?;
  let name = path.file_name().unwrap_or_default().to_string_lossy();
  let part = dir.join(format!(".{}.part", name));

  let written = (|| -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
      channels: 2,
      sample_rate: rate,
      bits_per_sample: 32,
      sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&part, spec)?;
    for &sample in samples { writer.write_sample(written(sample))?; }
    writer.finalize()
  })();

  let placed = written
    .map_err(|e| format!("could not write '{}': {}", path.display(), e))
    .and_then(|_| std::fs::hard_link(&part, path)
      .map_err(|e| format!("could not write '{}': {}", path.display(), e)));
  let _ = std::fs::remove_file(&part);
  placed
}

// what a clip's new file is named after: its sample's file (less any
// number an earlier copy gave it), or its pattern
pub(crate) fn source_name(audiograph: &daw::AudioGraph<'static>, id: u64, tag: &str) -> Option<String> {
  if let Some(node) = audiograph.get_node_by_id(id) {
    let stem = daw::sample_name(node.sample_path());
    return Some(project_files::file_stem_from(&project_files::without_number(&stem, tag)));
  }
  let node = audiograph.get_pattern_node_by_id(id)?;
  let name = audiograph.sequencer.lock().unwrap().pattern(node.pattern_id)
    .map(|pattern| pattern.name.clone().unwrap_or_else(|| format!("pattern {}", pattern.id + 1)));
  Some(project_files::file_stem_from(&name.unwrap_or_else(|| "pattern".into())))
}

// what Deep copy and Join say for a project that's never been saved: the
// frontend looks for this, saves the project, and asks again (see
// NEEDS_SAVING in src/state/slices/playlistSlice.ts)
pub const NEEDS_SAVING: &str = "The project has to be saved first, so its clips have somewhere to go.";

// the folder written-out clips go in, for a project that's been saved
pub fn clips_dir(state: &daw::InnerState) -> Result<PathBuf, String> {
  state.project.lock().unwrap().path.as_deref()
    .map(project_files::clips_dir_for)
    .ok_or_else(|| NEEDS_SAVING.to_string())
}

// write each clip out to a file of its own in the project's clips folder,
// and switch the clip to it: the same clip, where it was, playing the new
// file with its settings at their defaults. a pattern clip becomes a
// sample clip. one step to undo (which leaves the files where they are)
//
// every file is written before any clip is changed: if one can't be, the
// files written so far are removed and no clip changes
#[tauri::command(async)]
pub fn deep_copy_clips(state: daw::TState<'_>, ids: Vec<u64>) -> Result<ChangedClips, String> {
  deep_copy(&state, &ids, daw::open_engine_rate())
}

// - rate: the engine's rate. None when no output is open: each clip is
//   then written at its sample's own rate
pub(crate) fn deep_copy(state: &daw::InnerState, ids: &[u64], rate: Option<u32>) -> Result<ChangedClips, String> {
  let dir = clips_dir(state)?;
  if ids.is_empty() { return Ok(ChangedClips::default()); }
  let _writing = writing();

  // each clip as it is now, and what its file will be called. only that
  // much is done with the graph's lock, which playback needs
  enum Source {
    Sample(daw::AudioNode),
    Pattern(daw::PatternNode, PatternHits),
  }
  let sources: Vec<(u64, PathBuf, u32, Source)> = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let mut taken = HashSet::new();
    let mut sources = Vec::new();
    for &id in ids {
      let name = source_name(&audiograph, id, "copy").ok_or_else(|| format!("no node with id {}", id))?;
      let path = project_files::numbered_path(&dir, &name, "copy", &taken);
      taken.insert(path.file_name().unwrap_or_default().to_string_lossy().to_lowercase());

      match audiograph.get_node_by_id(id) {
        Some(node) => sources.push((id, path, rate.unwrap_or_else(|| node.sample_rate()), Source::Sample(node.clone()))),
        None => {
          let node = audiograph.get_pattern_node_by_id(id).expect("a clip that was just found");
          let rate = rate.unwrap_or(FALLBACK_RATE);
          sources.push((id, path, rate, Source::Pattern(node.clone(), pattern_hits(&audiograph, node, rate)?)));
        }
      }
    }
    sources
  };

  // the sound, the files, then the clips that will play them
  let mut written: Vec<PathBuf> = Vec::new();
  let mut was: Vec<(u64, history::ClipSnap)> = Vec::new();
  let made = (|| -> Result<Vec<(u64, daw::AudioNode)>, String> {
    let mut clips = Vec::new();
    for (id, path, rate, source) in sources {
      let samples = match source {
        Source::Sample(node) => {
          let samples = render_sample(&node, rate);
          was.push((id, history::ClipSnap::Sample(node)));
          samples
        }
        Source::Pattern(node, hits) => {
          was.push((id, history::ClipSnap::Pattern(node)));
          mix_pattern(hits, rate)
        }
      };
      if samples.is_empty() {
        return Err(format!("'{}' would be empty: the clip plays nothing.", path.file_name().unwrap_or_default().to_string_lossy()));
      }
      write_wav(&path, rate, &samples)?;
      written.push(path.clone());
      let node = daw::AudioNode::new(id, path.to_string_lossy().into_owned(), Duration::ZERO, 0)?;
      clips.push((id, node));
    }
    Ok(clips)
  })();
  let made = match made {
    Ok(made) => made,
    Err(e) => {
      for path in &written { let _ = std::fs::remove_file(path); }
      return Err(e);
    }
  };

  let swapped = history::clip_edit(state, ids, None, || {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    // a clip removed or changed while its file was being written: the file
    // isn't what the clip plays any more, and nothing changes
    if !was.iter().all(|(id, clip)| unchanged(&audiograph, *id, clip)) {
      return Err(CHANGED_MEANWHILE.into());
    }
    for (id, mut node) in made {
      let (start, track) = audiograph.placement(id).expect("a clip that was just found");
      node.start_offset = start;
      node.track_number = track;
      audiograph.remove_node(id)?;
      audiograph.add_node(node);
    }
    Ok(())
  }, |_| vec![]);
  if let Err(e) = swapped {
    for path in &written { let _ = std::fs::remove_file(path); }
    return Err(e);
  }

  Ok(changed_clips(state, ids))
}

pub const CHANGED_MEANWHILE: &str =
  "A clip was changed while it was being written out, so nothing was done. Try again.";

// whether a clip is still as it was when it was written out: there, in
// the same place, playing the same thing the same way
pub(crate) fn unchanged(audiograph: &daw::AudioGraph<'static>, id: u64, was: &history::ClipSnap) -> bool {
  match was {
    history::ClipSnap::Sample(was) => audiograph.get_node_by_id(id)
      .map_or(false, |node| node.same_clip(was) && node.start_offset == was.start_offset),
    history::ClipSnap::Pattern(was) => audiograph.get_pattern_node_by_id(id) == Some(was),
  }
}

#[cfg(test)]
mod tests;
