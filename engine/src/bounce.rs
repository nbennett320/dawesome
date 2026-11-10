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
use rodio::Source;

use crate::clip_commands::{changed_clips, ChangedClips};
use crate::daw;
use crate::history;
use crate::project_files;

// how long the end of a written-out pattern clip is faded over, so a hit
// still ringing there doesn't end in a click
const PATTERN_TAIL: Duration = Duration::from_millis(10);
// the rate written at when no output is open and there's no sample to go by
const FALLBACK_RATE: u32 = 44_100;
// what a sample is written as, so that it's read back as near as can be.
// the files are read back as 16-bit (see later.md), by multiplying by 32767
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
mod tests {
  use super::*;
  use std::sync::Arc;
  use std::time::Instant;
  use crate::event_dispatch::{add_clip, change_clip_settings, ClipWindow};
  use crate::project;

  const KICK: &str = "assets/assets_66-bd-01.wav";

  // a state whose project is saved in a folder of its own, which is returned
  fn saved_project(name: &str) -> (Arc<daw::InnerState>, PathBuf) {
    let dir = std::env::temp_dir().join(format!("dawesome-bounce-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(dir).unwrap();
    let state = Arc::new(daw::InnerState::default());
    state.project.lock().unwrap().reset(Some(dir.join("song.dawesome")));
    (state, dir)
  }

  // noise loud enough to clip with some gain, written to a file
  fn noise_wav(dir: &Path, name: &str, rate: u32, channels: u16, frames: u32) -> String {
    let path = dir.join(name);
    let spec = hound::WavSpec { channels, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    let mut seed = 7u32;
    for frame in 0..frames {
      for channel in 0..channels as u32 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let noise = ((seed >> 16) % 12_000) as f32 - 6_000.;
        let tone = (frame as f32 * (0.03 + channel as f32 * 0.01)).sin() * 14_000.;
        writer.write_sample((tone + noise) as i16).unwrap();
      }
    }
    writer.finalize().unwrap();
    path.to_string_lossy().into_owned()
  }

  // what's heard of the playlist from its start, on an engine at `rate`
  // (as the scheduler plays it: a beat at a time)
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

  // from the first frame that isn't silent to the last
  fn sounding(samples: &[f32]) -> (usize, usize) {
    let frames: Vec<bool> = samples.chunks(2).map(|frame| frame[0] != 0. || frame[1] != 0.).collect();
    (frames.iter().position(|&on| on).unwrap(), frames.iter().rposition(|&on| on).unwrap() + 1)
  }

  fn read(path: &Path) -> (hound::WavSpec, Vec<f32>) {
    let mut reader = hound::WavReader::open(path).unwrap();
    let spec = reader.spec();
    (spec, reader.samples::<f32>().map(|sample| sample.unwrap()).collect())
  }

  fn node(state: &daw::InnerState, id: u64) -> daw::AudioNode {
    state.playlist.audiograph.lock().unwrap().get_node_by_id(id).unwrap().clone()
  }

  const STEP: f32 = 1. / 32_768.;

  fn worst(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0., f32::max)
  }

  #[test]
  fn test_a_copy_plays_what_the_clip_did() {
    let rate = 48_000;
    let total = Duration::from_secs(5);
    for file_rate in [44_100, 48_000] {
      let (state, dir) = saved_project(&format!("plays-{}", file_rate));
      let path = noise_wav(&dir, "Noise.wav", file_rate, 2, file_rate * 2);
      let id = add_clip(&state, path.clone(), 2, Some(1.37)).unwrap().id;
      // trimmed, pitched down, faded in, panned and turned down
      crate::clip_commands::set_windows(&state, &[crate::clip_commands::WindowEdit {
        id, window: ClipWindow { start: 0.3, end: Some(1.6) },
      }]).unwrap();
      change_clip_settings(&state, id, daw::ClipSettings {
        pitch: -3., attack_ms: 50., pan: 0.5, gain_db: -6., decay_ms: 0.,
      }, None).unwrap();
      let before = node(&state, id);
      let original = play(&state, rate, total);
      state.history.lock().unwrap().clear();

      let changed = deep_copy(&state, &[id], Some(rate)).unwrap();
      let after = node(&state, id);

      // the same clip, where it was, playing its new file as it comes
      assert_eq!(changed.clips.len(), 1);
      assert_eq!(changed.clips[0].id, id);
      assert_eq!((after.start_offset, after.track_number), (before.start_offset, 2));
      assert_eq!(after.settings(), daw::ClipSettings::default());
      assert_eq!(after.window(), None);
      let file = dir.join("clips").join("Noise_copy_1.wav");
      assert_eq!(after.sample_path(), file.to_string_lossy());
      assert_eq!(changed.clips[0].path, file.to_string_lossy());

      // 32-bit float, stereo, at the engine's rate, as long as the clip
      // sounded for
      let (spec, samples) = read(&file);
      assert_eq!((spec.channels, spec.sample_rate, spec.bits_per_sample), (2, rate, 32));
      assert_eq!(spec.sample_format, hound::SampleFormat::Float);
      let (from, to) = sounding(&original);
      // (the clip fades in from nothing: its first frame is silent)
      assert!((samples.len() / 2) as i64 - (to - from) as i64 <= 1 && samples.len() / 2 >= to - from);

      // played back: the same frames, each within a 16-bit step
      let copy = play(&state, rate, total);
      assert_eq!(sounding(&copy).1, to, "{}Hz", file_rate);
      let off = worst(&copy, &original);
      assert!(off <= STEP, "off by {} steps at {}Hz", off / STEP, file_rate);
      // the file it played before is still there
      assert!(Path::new(&path).is_file());

      // undone, it's the clip it was, and the new file stays
      assert!(history::undo(&state).unwrap().is_some());
      let undone = node(&state, id);
      assert_eq!(undone.sample_path(), path);
      assert_eq!((undone.settings(), undone.window()), (before.settings(), before.window()));
      assert!(play(&state, rate, total) == original);
      assert!(file.is_file());
      // redone, it plays that file again, and no other is written
      assert!(history::redo(&state).unwrap().is_some());
      assert_eq!(node(&state, id).sample_path(), file.to_string_lossy());
      assert!(play(&state, rate, total) == copy);
      let names: Vec<_> = std::fs::read_dir(dir.join("clips")).unwrap().map(|e| e.unwrap().file_name()).collect();
      assert_eq!(names.len(), 1);

      let _ = std::fs::remove_dir_all(&dir);
    }
  }

  #[test]
  fn test_a_copy_of_a_clip_too_loud_is_cut_off_at_full_scale() {
    let rate = 44_100;
    let (state, dir) = saved_project("loud");
    let path = noise_wav(&dir, "loud.wav", rate, 1, rate);
    let id = add_clip(&state, path, 0, Some(0.)).unwrap().id;
    change_clip_settings(&state, id, daw::ClipSettings { gain_db: 12., ..Default::default() }, None).unwrap();
    let original = render_sample_unclipped(&node(&state, id), rate);
    assert!(original.iter().any(|sample| sample.abs() > 1.5), "the clip isn't loud enough to clip");

    deep_copy(&state, &[id], Some(rate)).unwrap();
    let (_, samples) = read(&dir.join("clips").join("loud_copy_1.wav"));
    assert_eq!(samples.len(), original.len());
    assert!(samples.iter().all(|sample| sample.abs() <= 1.));
    assert!(samples.iter().any(|sample| sample.abs() == 1.));
    // the original, cut off at full scale (written a hair off: see `written`)
    let clipped: Vec<f32> = original.iter().map(|sample| sample.clamp(-1., 1.)).collect();
    assert!(worst(&samples, &clipped) < 1e-4);
    // and played back, within a 16-bit step of that
    let copy = render_sample_unclipped(&node(&state, id), rate);
    assert!(worst(&copy, &clipped) <= STEP);
    let _ = std::fs::remove_dir_all(&dir);
  }

  fn render_sample_unclipped(node: &daw::AudioNode, rate: u32) -> Vec<f32> {
    node.voice(rate, node.start_offset.as_nanos() as i128, 0).collect()
  }

  #[test]
  fn test_copies_are_numbered_and_never_written_over_another_file() {
    let (state, dir) = saved_project("names");
    let path = noise_wav(&dir, "kick.wav", 44_100, 1, 4_410);
    let clips = dir.join("clips");
    std::fs::create_dir_all(clips.join("old")).unwrap();
    // names already there, whatever their case and whichever folder
    std::fs::write(clips.join("Kick_Copy_1.wav"), b"mine").unwrap();
    std::fs::write(clips.join("old").join("KICK_COPY_2.WAV"), b"mine too").unwrap();

    let (a, b) = (add_clip(&state, path.clone(), 0, Some(0.)).unwrap().id, add_clip(&state, path, 1, Some(4.)).unwrap().id);
    // two at once take the next two numbers
    let changed = deep_copy(&state, &[a, b], Some(44_100)).unwrap();
    let names: Vec<String> = changed.clips.iter()
      .map(|clip| Path::new(&clip.path).file_name().unwrap().to_string_lossy().into_owned())
      .collect();
    assert_eq!(names, vec!["kick_copy_3.wav", "kick_copy_4.wav"]);
    assert_eq!(std::fs::read(clips.join("Kick_Copy_1.wav")).unwrap(), b"mine");
    assert_eq!(std::fs::read(clips.join("old").join("KICK_COPY_2.WAV")).unwrap(), b"mine too");

    // a copy of a copy is numbered on, rather than "kick_copy_3_copy_1"
    let again = deep_copy(&state, &[a], Some(44_100)).unwrap();
    assert!(again.clips[0].path.ends_with("kick_copy_5.wav"), "{}", again.clips[0].path);
    // every file is inside the project's clips folder
    assert!(changed.clips.iter().chain(&again.clips).all(|clip| Path::new(&clip.path).parent() == Some(clips.as_path())));

    assert_eq!(project_files::without_number("kick_copy_12", "copy"), "kick");
    assert_eq!(project_files::without_number("Kick_Copy_3", "copy"), "Kick");
    assert_eq!(project_files::without_number("copy_cat", "copy"), "copy_cat");
    assert_eq!(project_files::without_number("kick_copy_", "copy"), "kick_copy_");
    assert_eq!(project_files::without_number("_copy_1", "copy"), "_copy_1");
    assert_eq!(project_files::file_stem_from("  a/b:c  "), "a_b_c");
    assert_eq!(project_files::file_stem_from(" .. "), "clip");
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_what_is_written_reads_back_as_the_nearest_16_bit_step() {
    // as rodio reads a float file, and the engine then plays it
    let read_back = |sample: f32| ((sample.clamp(-1., 1.) * 32_767.) as i16) as f32 / 32_768.;
    let mut worst = 0f32;
    for n in -200_000..=200_000 {
      let sample = n as f32 / 200_000.;
      worst = worst.max((read_back(written(sample)) - sample).abs());
    }
    // within a step everywhere (it's a whole step only at full scale)
    assert!(worst <= STEP, "{} steps", worst / STEP);
    // and every 16-bit sample reads back as itself, so a copy of a copy
    // doesn't creep
    for n in -32_767..=32_767i32 {
      let sample = n as f32 / 32_768.;
      assert_eq!(read_back(written(sample)), sample, "{}", n);
    }
    assert_eq!(written(0.), 0.);
  }

  #[test]
  fn test_a_clip_changed_while_its_file_is_written_is_left_alone() {
    let (state, dir) = saved_project("meanwhile");
    let path = noise_wav(&dir, "kick.wav", 44_100, 1, 4_410);
    let id = add_clip(&state, path, 0, Some(0.)).unwrap().id;
    let was = history::ClipSnap::Sample(node(&state, id));
    {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      assert!(unchanged(&audiograph, id, &was));
    }
    // moved, trimmed, re-pitched or gone, it's no longer what was written
    change_clip_settings(&state, id, daw::ClipSettings { pitch: 2., ..Default::default() }, None).unwrap();
    {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      assert!(!unchanged(&audiograph, id, &was));
      assert!(!unchanged(&audiograph, 9_999, &was));
    }
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_names_with_any_letters_are_numbered() {
    // (letters whose lower case is longer or shorter than they are)
    assert_eq!(project_files::without_number("\u{212A}\u{212A}_copy_1", "copy"), "\u{212A}\u{212A}");
    assert_eq!(project_files::without_number("İstanbul_copy_12", "copy"), "İstanbul");
    assert_eq!(project_files::without_number("ÉTÉ_Copy_3", "copy"), "ÉTÉ");
    // a very long name still leaves room for its number
    let long = "x".repeat(400);
    assert_eq!(project_files::file_stem_from(&long).chars().count(), 120);
  }

  #[test]
  fn test_an_unsaved_project_is_refused() {
    let state = daw::InnerState::default();
    let id = add_clip(&state, KICK.to_string(), 0, Some(0.)).unwrap().id;
    let before = project::capture(&state, None);
    let steps = state.history.lock().unwrap().len();
    let refused = deep_copy(&state, &[id], Some(44_100)).unwrap_err();
    assert!(refused.contains("saved first"), "{}", refused);
    assert_eq!(project::capture(&state, None), before);
    assert_eq!(state.history.lock().unwrap().len(), steps);
  }

  #[test]
  fn test_if_one_file_cant_be_written_nothing_changes() {
    let (state, dir) = saved_project("fails");
    let path = noise_wav(&dir, "kick.wav", 44_100, 1, 4_410);
    let ids: Vec<u64> = (0..3).map(|n| add_clip(&state, path.clone(), n, Some(n as f32 * 2.)).unwrap().id).collect();
    state.history.lock().unwrap().clear();
    let before = project::capture(&state, Some(&dir));
    // where the second file would go, there's a folder it can't be renamed onto
    let clips = dir.join("clips");
    std::fs::create_dir_all(clips.join(".kick_copy_2.wav.part").join("in the way")).unwrap();

    let failed = deep_copy(&state, &ids, Some(44_100)).unwrap_err();
    assert!(failed.contains("kick_copy_2.wav"), "{}", failed);
    assert_eq!(project::capture(&state, Some(&dir)), before);
    assert_eq!(state.history.lock().unwrap().len(), 0);
    // the first file, which was written, is gone again
    let left: Vec<String> = std::fs::read_dir(&clips).unwrap()
      .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
      .filter(|name| !name.starts_with('.'))
      .collect();
    assert!(left.is_empty(), "{:?}", left);

    // an id that names no clip: nothing is written at all
    assert!(deep_copy(&state, &[ids[0], 9_999], Some(44_100)).is_err());
    assert_eq!(project::capture(&state, Some(&dir)), before);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_selection_is_one_step() {
    let (state, dir) = saved_project("selection");
    let path = noise_wav(&dir, "kick.wav", 44_100, 1, 4_410);
    let ids: Vec<u64> = (0..3).map(|n| add_clip(&state, path.clone(), n, Some(n as f32 * 2.)).unwrap().id).collect();
    state.history.lock().unwrap().clear();
    let before = project::capture(&state, Some(&dir));

    let changed = deep_copy(&state, &ids, Some(44_100)).unwrap();
    assert_eq!(changed.clips.len(), 3);
    assert_eq!(state.history.lock().unwrap().len(), 1);
    assert_eq!(std::fs::read_dir(dir.join("clips")).unwrap().count(), 3);
    // one undo, and all three are as they were, with the files still there
    let undone = history::undo(&state).unwrap().unwrap();
    assert_eq!(undone.clips, Some(ids.clone()));
    assert_eq!(project::capture(&state, Some(&dir)), before);
    assert_eq!(std::fs::read_dir(dir.join("clips")).unwrap().count(), 3);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_copy_changes_no_other_clip_of_the_file() {
    let (state, dir) = saved_project("others");
    let path = noise_wav(&dir, "shared.wav", 44_100, 2, 22_050);
    let (a, b) = (add_clip(&state, path.clone(), 0, Some(0.)).unwrap().id, add_clip(&state, path.clone(), 1, Some(3.)).unwrap().id);
    change_clip_settings(&state, a, daw::ClipSettings { pitch: 4., gain_db: -8., ..Default::default() }, None).unwrap();
    let other = node(&state, b);
    let sound = render_sample(&other, 48_000);
    let bytes = std::fs::read(&path).unwrap();

    deep_copy(&state, &[a], Some(48_000)).unwrap();
    let after = node(&state, b);
    assert_eq!(after.sample_path(), path);
    assert!(after.same_clip(&other) && after.start_offset == other.start_offset);
    assert!(render_sample(&after, 48_000) == sound);
    // nor the file they both played
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let _ = std::fs::remove_dir_all(&dir);
  }

  // (run by hand, after docs/editing-project/perf/make_500_clips.py:
  // `DAWESOME_PERF_PROJECT=<the path it prints> cargo test five_hundred_clip_project -- --ignored --nocapture`)
  #[test]
  #[ignore = "needs the generated project"]
  fn test_the_five_hundred_clip_project_opens() {
    let path = match std::env::var("DAWESOME_PERF_PROJECT") {
      Ok(path) => PathBuf::from(path),
      Err(_) => return,
    };
    let state = daw::InnerState::default();
    let started = Instant::now();
    let loaded = project::load_from(&state, &path).unwrap();
    println!("opened in {:?}", started.elapsed());
    assert!(loaded.missing.is_empty(), "{:?}", loaded.missing);
    assert_eq!((loaded.clips.len(), loaded.pattern_clips.len()), (400, 100));
    assert_eq!(loaded.clips.iter().filter(|clip| clip.info.window.is_some()).count(), 100);
  }

  #[test]
  fn test_with_no_output_open_a_copy_is_at_its_samples_rate() {
    let (state, dir) = saved_project("no-device");
    let path = noise_wav(&dir, "high.wav", 96_000, 1, 9_600);
    let id = add_clip(&state, path, 0, Some(0.)).unwrap().id;
    deep_copy(&state, &[id], None).unwrap();
    let (spec, samples) = read(&dir.join("clips").join("high_copy_1.wav"));
    assert_eq!(spec.sample_rate, 96_000);
    assert_eq!(samples.len(), 9_600 * 2);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_pattern_clip_becomes_a_sample_clip_of_what_it_played() {
    let rate = 48_000;
    let total = Duration::from_secs(6);
    let (state, dir) = saved_project("pattern");
    let pattern = {
      let mut sequencer = state.playlist.sequencer.lock().unwrap();
      let kick = sequencer.add_row(KICK).unwrap();
      // (a long sample, still sounding when the clip ends)
      let snare = sequencer.add_row("assets/slide_guitar.wav").unwrap();
      let muted = sequencer.add_row("assets/assets_66-hh-01-or.wav").unwrap();
      let pattern = sequencer.add_pattern(1, 4).unwrap();
      sequencer.rename_pattern(pattern, "Verse: A").unwrap();
      for (row, step) in [(kick, 0), (kick, 6), (snare, 4), (snare, 15), (muted, 2)] {
        sequencer.set_step(pattern, row, step, true).unwrap();
      }
      // (quiet enough that nothing they add up to is over full scale,
      // which the file would cut off)
      sequencer.set_row_volume(snare, 0.2).unwrap();
      sequencer.set_row_volume(kick, 0.5).unwrap();
      sequencer.set_row_muted(muted, true).unwrap();
      pattern
    };
    let id = {
      let mut audiograph = state.playlist.audiograph.lock().unwrap();
      audiograph.set_tempo(97.);
      audiograph.add_pattern_node::<daw::timing::Note>(pattern, Duration::from_nanos(1_234_567_891), 3, None).unwrap()
    };
    // steps 2 up to 16: the first kick isn't played
    crate::clip_commands::set_windows(&state, &[crate::clip_commands::WindowEdit {
      id, window: ClipWindow { start: 2., end: None },
    }]).unwrap();
    let (start, length) = {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      let node = audiograph.get_pattern_node_by_id(id).unwrap().clone();
      (node.start_offset, node.duration(16, audiograph.beat_interval()))
    };
    let original = play(&state, rate, total);
    state.history.lock().unwrap().clear();

    let changed = deep_copy(&state, &[id], Some(rate)).unwrap();
    // a sample clip now, with the same id, where the pattern clip was
    assert!(changed.pattern_clips.is_empty());
    assert_eq!(changed.clips[0].id, id);
    assert!(state.playlist.audiograph.lock().unwrap().get_pattern_node_by_id(id).is_none());
    let after = node(&state, id);
    assert_eq!((after.start_offset, after.track_number), (start, 3));
    // named after its pattern, without what a file name can't have
    let file = dir.join("clips").join("Verse_ A_copy_1.wav");
    assert_eq!(after.sample_path(), file.to_string_lossy());

    // exactly as long as the clip was
    let (spec, samples) = read(&file);
    assert_eq!((spec.channels, spec.sample_rate), (2, rate));
    let frames = samples.len() / 2;
    let first = daw::run_frame(start.as_nanos() as i128, rate) as usize;
    let last = daw::run_frame((start + length).as_nanos() as i128, rate) as usize;
    assert_eq!(frames, last - first);

    // the same as the pattern clip played, up to the last 10ms, where the
    // snare on the last step, still ringing, is faded out
    let copy = play(&state, rate, total);
    let tail = rate as usize / 100;
    let body = first * 2..(last - tail) * 2;
    let off = worst(&copy[body.clone()], &original[body]);
    assert!(off <= STEP, "off by {} steps", off / STEP);
    // (the loudest sample of the last two milliseconds)
    let loudest = |samples: &[f32]| samples[(last - tail / 5) * 2..last * 2].iter().fold(0f32, |max, s| max.max(s.abs()));
    assert!(loudest(&original) > 0.001, "nothing is ringing at the clip's end: {}", loudest(&original));
    assert!(loudest(&copy) < loudest(&original) * 0.25);
    assert!(copy[last * 2..].iter().all(|&sample| sample == 0.));
    assert!(copy[..first * 2].iter().all(|&sample| sample == 0.));

    // undone, it's the pattern clip again, with its window
    let undone = history::undo(&state).unwrap().unwrap();
    assert_eq!(undone.clips, Some(vec![id]));
    let audiograph = state.playlist.audiograph.lock().unwrap();
    assert!(audiograph.get_node_by_id(id).is_none());
    let node = audiograph.get_pattern_node_by_id(id).unwrap();
    assert_eq!((node.pattern_id, node.start_offset, node.track_number), (pattern, start, 3));
    assert_eq!(node.window, Some(daw::PatternWindow { start: 2., end: None }));
    drop(audiograph);
    assert!(play(&state, rate, total) == original);
    assert!(file.is_file());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_saving_somewhere_new_takes_the_clips_folder_along() {
    let (state, dir) = saved_project("save-as");
    project::save_to(&state, &dir.join("song.dawesome")).unwrap();
    let path = noise_wav(&dir, "kick.wav", 44_100, 1, 4_410);
    let id = add_clip(&state, path, 0, Some(0.)).unwrap().id;
    deep_copy(&state, &[id], Some(44_100)).unwrap();
    let old = dir.join("clips").join("kick_copy_1.wav");
    assert_eq!(node(&state, id).sample_path(), old.to_string_lossy());

    // saved as another project, in a folder of its own
    let elsewhere = dir.join("elsewhere");
    let saved = project::save_project_to(&state, &elsewhere.join("second"), true).unwrap();
    let new_dir = Path::new(&saved.path).parent().unwrap().to_path_buf();
    let copied = new_dir.join("clips").join("kick_copy_1.wav");
    // the clip plays the copy in the new project's folder, and the project
    // it was saved from still has its own
    assert!(copied.is_file() && old.is_file());
    assert_eq!(std::fs::canonicalize(node(&state, id).sample_path()).unwrap(), std::fs::canonicalize(&copied).unwrap());
    assert_eq!(saved.moved_clips.len(), 1);
    assert!(saved.problems.is_empty(), "{:?}", saved.problems);

    // and the next copy goes into the new project's folder
    let again = deep_copy(&state, &[id], Some(44_100)).unwrap();
    assert!(Path::new(&again.clips[0].path).starts_with(std::fs::canonicalize(&new_dir).unwrap().join("clips"))
      || Path::new(&again.clips[0].path).starts_with(new_dir.join("clips")), "{}", again.clips[0].path);
    assert!(again.clips[0].path.ends_with("kick_copy_2.wav"));

    // reopened, the project finds its clip
    let reopened = daw::InnerState::default();
    let loaded = project::load_from(&reopened, Path::new(&saved.path)).unwrap();
    assert!(loaded.missing.is_empty(), "{:?}", loaded.missing);
    let _ = std::fs::remove_dir_all(&dir);
  }

  // (run by hand, in a release build: `cargo test --release minute -- --ignored`)
  #[test]
  #[ignore = "a timing test"]
  fn test_a_minute_is_written_in_under_two_seconds() {
    let (state, dir) = saved_project("minute");
    let path = noise_wav(&dir, "minute.wav", 44_100, 2, 44_100 * 60);
    let id = add_clip(&state, path, 0, Some(0.)).unwrap().id;
    change_clip_settings(&state, id, daw::ClipSettings { pitch: -2., attack_ms: 500., pan: -0.3, gain_db: -3., decay_ms: 800. }, None).unwrap();

    let started = Instant::now();
    deep_copy(&state, &[id], Some(48_000)).unwrap();
    let took = started.elapsed();
    println!("a minute's clip was written in {:?}", took);
    assert!(took < Duration::from_secs(2), "{:?}", took);
    let _ = std::fs::remove_dir_all(&dir);
  }
}
