// saving and loading projects: the playlist's clips, their settings, the
// playlist's settings, the mixer and its effects, in a JSON file (.dawesome)
//
// an effect is saved with its plugin's id, bundle and settings (base64). one
// whose plugin can't be loaded comes back missing, keeping its settings, and
// is saved again just as it was read. the sequencer's synth rows' plugins
// are saved the same way, in their rows
//
// a project saved by the stage-1 build of plugins (an instrument in a
// track's rack, notes as a plain list in a pattern) loads without those, and
// says so
// (see drop_stage_one). one saved before version 4 has its sequencer
// rewritten for this one as it's read (see merge_pattern_kinds)
//
// sample paths inside the project's folder are saved relative to it, so a
// project folder can be moved or shared. other samples are saved as
// absolute paths
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::daw;
use crate::commands::clip::{clip_info, ClipInfo, ClipWindow};
use crate::project_files::{self, recordings_dir_for};

// the newest project format this version of the app reads and writes.
// bump it when a change means older apps would misread a project. version 4
// has a sequencer of rows and patterns as daw_core/sequencer.rs has them,
// and pattern clips saved with their own start and length
pub const PROJECT_VERSION: u32 = 4;
// what a project without a sequencer or pattern clips is saved as, so older
// apps can still open it. version 3 added clips that play part of their
// sample
pub const PROJECT_VERSION_WITHOUT_PATTERNS: u32 = 3;
// what a project with neither those nor trimmed clips is saved as
pub const PROJECT_VERSION_WITHOUT_WINDOWS: u32 = 1;

// the playlist's settings, as saved in a project
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectSettings {
  pub tempo: f32,
  pub time_signature: daw::timing::TimeSignature,
  pub metronome_enabled: bool,
  pub loop_enabled: bool,
  pub snap_enabled: bool,
  pub snap_note: daw::timing::Note,
  // whether recording counts in a bar first
  pub count_in: bool,
}

impl Default for ProjectSettings {
  fn default() -> Self {
    ProjectSettings {
      tempo: daw::defaults::TEMPO,
      time_signature: daw::timing::TimeSignature { numerator: 4, denominator: 4 },
      metronome_enabled: true,
      loop_enabled: true,
      snap_enabled: daw::defaults::SNAP_ENABLED,
      snap_note: daw::defaults::SNAP_SUBDIVISION,
      count_in: false,
    }
  }
}

// a clip, as saved in a project
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectClip {
  // relative to the project's folder, or absolute
  pub path: String,
  pub track_number: u32,
  // beats from the start of the playlist
  pub start_beat: f64,
  #[serde(default)]
  pub settings: daw::ClipSettings,
  // the part of the sample it plays, in seconds. left out for all of it
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub window: Option<ClipWindow>,
}

// a pattern placed in the playlist, as saved in a project
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPatternClip {
  pub pattern_id: u64,
  pub track_number: u32,
  // beats from the start of the playlist
  pub start_beat: f64,
  // where in the pattern the clip starts, in steps (see PatternNode::start).
  // left out at its start
  #[serde(default, skip_serializing_if = "is_zero")]
  pub start: f64,
  // the clip's own length, in steps (see saved_span for one that's left
  // out or makes no sense)
  #[serde(default)]
  pub length: f64,
}

fn is_zero(steps: &f64) -> bool {
  *steps == 0.
}

// a project file. missing fields take their defaults, so hand edited or
// older files still load
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
  pub version: u32,
  #[serde(flatten)]
  pub settings: ProjectSettings,
  pub clips: Vec<ProjectClip>,
  // the fewest tracks the playlist shows (tracks added below the last clip),
  // left out when it's the default number
  #[serde(skip_serializing_if = "is_default_track_count")]
  pub tracks: u32,
  // only tracks changed from their defaults, or named, are saved
  pub mixer: daw::MixerSettings,
  // the Sequencer tab's rows (with their sample paths saved like clips'),
  // patterns and folders, left out when there are none. under a key of its
  // own, which older apps don't read: they get as far as the version, and
  // say the project is newer
  #[serde(rename = "patternSequencer", skip_serializing_if = "daw::SavedSequencer::is_empty")]
  pub sequencer: daw::SavedSequencer,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub pattern_clips: Vec<ProjectPatternClip>,
  // the tracks armed to record, and the input channels each records
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub armed: Vec<crate::recording::ArmedTrack>,
  // each channel's effects, in order, left out when there are none
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub effects: Vec<crate::effects::SavedRack>,
  // what was left out of the file as it was read, a sentence for each kind
  // of thing (never saved)
  #[serde(skip)]
  pub dropped: Vec<String>,
}

fn is_default_track_count(tracks: &u32) -> bool {
  *tracks == daw::defaults::NUM_OF_TRACKS
}

// the most tracks a project file can ask for, so a bad one can't make the
// playlist (and mixer) endless
const MAX_SAVED_TRACKS: u32 = 1024;

impl Default for Project {
  fn default() -> Self {
    Project {
      version: PROJECT_VERSION_WITHOUT_WINDOWS,
      settings: ProjectSettings::default(),
      clips: Vec::new(),
      tracks: daw::defaults::NUM_OF_TRACKS,
      mixer: daw::MixerSettings::default(),
      sequencer: daw::SavedSequencer::default(),
      pattern_clips: Vec::new(),
      armed: Vec::new(),
      effects: Vec::new(),
      dropped: Vec::new(),
    }
  }
}

impl Project {
  // read a project from its JSON, refusing ones this app can't play
  pub fn from_json(json: &str) -> Result<Self, String> {
    let mut value: serde_json::Value = serde_json::from_str(json)
      .map_err(|e| format!("not a valid project file: {}", e))?;
    let dropped = drop_stage_one(&mut value);
    merge_pattern_kinds(&mut value)?;
    let mut project: Project = serde_json::from_value(value)
      .map_err(|e| format!("not a valid project file: {}", e))?;
    project.validate()?;
    project.dropped = dropped;

    Ok(project)
  }

  fn validate(&self) -> Result<(), String> {
    if self.version > PROJECT_VERSION {
      return Err(format!(
        "this project was saved by a newer version of dawesome (format {}, this version reads up to {})",
        self.version, PROJECT_VERSION));
    }

    let tempo = self.settings.tempo;
    if !daw::timing::valid_tempo(tempo) {
      return Err(format!("the project's tempo ({}) is out of range", tempo));
    }

    let sig = self.settings.time_signature;
    if sig.numerator == 0 || sig.denominator == 0 {
      return Err(format!(
        "the project's time signature ({}/{}) is invalid", sig.numerator, sig.denominator));
    }

    Ok(())
  }
}

// a clip after loading, with its new id and the frontend's names
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadedClip {
  pub id: u64,
  // where the sample was found
  pub path: String,
  pub track_number: u32,
  pub start_beat: f32,
  // the clip's settings and lengths, as get_clip returns them
  #[serde(flatten)]
  pub info: ClipInfo,
}

impl LoadedClip {
  // a clip in the playlist, as the frontend shows it
  // - interval: a beat's length
  pub fn of(node: &daw::AudioNode, interval: Duration) -> Self {
    LoadedClip {
      id: node.id,
      path: node.sample_path().to_string(),
      track_number: node.track_number,
      start_beat: beats(node.start_offset, interval),
      info: clip_info(node),
    }
  }
}

// a pattern clip after loading, with its new id
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPatternClip {
  pub id: u64,
  pub pattern_id: u64,
  pub track_number: u32,
  pub start_beat: f32,
  // where in its pattern it starts and ends, in steps (see
  // ClipWindow::of_pattern)
  pub window: ClipWindow,
}

impl LoadedPatternClip {
  pub fn of(node: &daw::PatternNode, interval: Duration) -> Self {
    LoadedPatternClip {
      id: node.id,
      pattern_id: node.pattern_id,
      track_number: node.track_number,
      start_beat: beats(node.start_offset, interval),
      window: ClipWindow::of_pattern(node),
    }
  }
}

// an offset into the playlist, in beats
fn beats(offset: Duration, interval: Duration) -> f32 {
  if interval.is_zero() { 0. } else { (offset.as_secs_f64() / interval.as_secs_f64()) as f32 }
}

// a clip that couldn't be loaded, so it was left out
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MissingSample {
  pub path: String,
  pub reason: String,
}

// take what the stage-1 build of plugins saved, and this one doesn't read,
// out of a project's JSON: instruments in racks (an effect saved with
// "kind": "instrument"), and notes in patterns as a plain list. gives a
// sentence for each kind of thing taken (P28: they're not converted)
fn drop_stage_one(project: &mut serde_json::Value) -> Vec<String> {
  let mut instruments = 0;
  if let Some(racks) = project.get_mut("effects").and_then(|e| e.as_array_mut()) {
    for rack in racks.iter_mut() {
      if let Some(slots) = rack.get_mut("slots").and_then(|s| s.as_array_mut()) {
        let before = slots.len();
        slots.retain(|slot| slot.get("kind").and_then(|k| k.as_str()) != Some("instrument"));
        instruments += before - slots.len();
      }
    }
  }
  let mut patterns = 0;
  if let Some(list) = project.pointer_mut("/sequencer/patterns").and_then(|p| p.as_array_mut()) {
    for pattern in list.iter_mut().filter_map(|p| p.as_object_mut()) {
      if let Some(notes) = pattern.get("notes").and_then(|n| n.as_array()) {
        if !notes.is_empty() { patterns += 1; }
        pattern.remove("notes");
      }
    }
  }

  let mut dropped = Vec::new();
  let plural = |n: usize, one: &str, many: &str| if n == 1 { format!("1 {}", one) } else { format!("{} {}", n, many) };
  if instruments > 0 {
    dropped.push(format!(
      "{} in a mixer channel's effects {} left out: instruments are rows of the Pattern Sequencer now",
      plural(instruments, "instrument", "instruments"), if instruments == 1 { "was" } else { "were" }));
  }
  if patterns > 0 {
    dropped.push(format!(
      "The notes in {} were left out: notes belong to a row of the Pattern Sequencer now",
      plural(patterns, "pattern", "patterns")));
  }
  dropped
}

// what the frontend needs to show a project once it's loaded
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadedProject {
  #[serde(flatten)]
  pub settings: ProjectSettings,
  pub clips: Vec<LoadedClip>,
  pub missing: Vec<MissingSample>,
  // every track's strip, up to the playlist's track count
  pub mixer: daw::MixerSettings,
  pub sequencer: daw::SequencerState,
  pub pattern_clips: Vec<LoadedPatternClip>,
  pub armed: Vec<crate::recording::ArmedTrack>,
  // each channel's effects
  pub effects: crate::effects::Racks,
  // effects whose plugins couldn't be loaded (they're in their racks,
  // marked missing), and synth rows' (in their rows, marked missing)
  pub missing_effects: Vec<crate::effects::MissingEffect>,
  // what the file had that was left out, a sentence for each kind of thing
  // (for the project notice; empty when nothing was)
  pub dropped: Vec<String>,
}

// the folder a project file is in, which its relative sample paths start from
fn project_dir(project_path: &Path) -> Result<PathBuf, String> {
  let dir = match project_path.parent() {
    Some(dir) if !dir.as_os_str().is_empty() => dir,
    _ => Path::new("."),
  };

  fs::canonicalize(dir)
    .map_err(|e| format!("could not find the folder '{}': {}", dir.display(), e))
}

// a sample's path as saved: relative (with forward slashes) when it's inside
// the project's folder, otherwise absolute
fn stored_path(sample_path: &str, project_dir: Option<&Path>) -> String {
  // samples can be added by relative paths, which wouldn't mean the same
  // thing once the project is loaded from elsewhere
  let absolute = fs::canonicalize(sample_path)
    .unwrap_or_else(|_| PathBuf::from(sample_path));

  match project_dir.and_then(|dir| absolute.strip_prefix(dir).ok()) {
    Some(relative) => relative
      .components()
      .map(|c| c.as_os_str().to_string_lossy())
      .collect::<Vec<_>>()
      .join("/"),
    None => absolute.to_string_lossy().into_owned(),
  }
}

// where a saved sample path points, for a project in `project_dir`
fn resolved_path(stored: &str, project_dir: Option<&Path>) -> PathBuf {
  let path = Path::new(stored);

  match project_dir {
    Some(dir) if path.is_relative() => dir.join(path),
    _ => path.to_path_buf(),
  }
}

// round away floating point noise, so saved beats read like 4 rather than
// 3.9999999999
fn tidy_beat(beat: f64) -> f64 {
  (beat * 1e6).round() / 1e6
}

// the current playlist as a project, with sample paths relative to
// `project_dir` where they can be
pub fn capture(state: &daw::InnerState, project_dir: Option<&Path>) -> Project {
  let (tempo, clips, pattern_clips, sequencer, tracks) = {
    let audiograph = state.playlist.audiograph.lock().unwrap();
    let interval = audiograph.beat_interval().as_secs_f64();
    // a trimmed clip's beat isn't rounded: the two halves of a cut clip only
    // play exactly what the uncut one did if they come back from a project to
    // the nanosecond, and rounding to six places moves a clip by up to half a
    // microsecond
    let beat_of = |offset: Duration, trimmed: bool| if interval > 0. {
      let beat = offset.as_secs_f64() / interval;
      if trimmed { beat } else { tidy_beat(beat) }
    } else {
      0.
    };

    let mut clips: Vec<ProjectClip> = audiograph.nodes.iter().map(|node| ProjectClip {
      path: stored_path(node.sample_path(), project_dir),
      track_number: node.track_number,
      start_beat: beat_of(node.start_offset, node.window().is_some()),
      settings: node.settings(),
      window: node.window().map(ClipWindow::of_sample),
    }).collect();
    // in the order they start, as the pattern clips are: undoing puts a clip
    // back at the end of the graph's list, and that isn't a change
    clips.sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat)
      .then(a.track_number.cmp(&b.track_number))
      .then_with(|| a.path.cmp(&b.path)));

    // in the order they start, so saving twice gives the same file. a clip
    // that isn't all of its pattern, once, is trimmed (a half of a cut clip,
    // or a clip that loops it)
    let sequencer = state.playlist.sequencer.lock().unwrap();
    let mut pattern_clips: Vec<ProjectPatternClip> = audiograph.pattern_nodes.iter().map(|node| {
      let whole = sequencer.pattern(node.pattern_id).map(|pattern| pattern.length as f64);
      ProjectPatternClip {
        pattern_id: node.pattern_id,
        track_number: node.track_number,
        start_beat: beat_of(node.start_offset, node.start != 0. || Some(node.length) != whole),
        start: node.start,
        length: node.length,
      }
    }).collect();
    pattern_clips.sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat).then(a.track_number.cmp(&b.track_number)));

    let mut sequencer = sequencer.saved();
    for path in sequencer.sample_paths_mut() {
      *path = stored_path(path, project_dir);
    }

    (audiograph.tempo(), clips, pattern_clips, sequencer, audiograph.min_tracks)
  };

  let ui = *state.playlist.ui.lock().unwrap();
  // (with no lock held: the plugins give their settings on the main thread,
  // and their open windows where they are)
  crate::effect_windows::keep_positions(&state.playlist.effects);
  let effects = state.playlist.effects.saved();
  // each synth row's plugin's settings are its master copy's (passed on to
  // the others first, so what's saved is what's heard)
  state.playlist.effects.push_synth_states(true);
  let mut sequencer = sequencer;
  for synth in sequencer.synths_mut() {
    synth.state = base64::encode(state.playlist.effects.state_of(synth.slot).unwrap_or_default());
    synth.window = state.playlist.effects.window_position(synth.slot);
  }

  let mut mixer = state.playlist.mixer.settings(0);
  mixer.tracks.retain(|track| track.settings != daw::ChannelSettings::default() || track.labelled());

  let uses_patterns = !sequencer.is_empty() || !pattern_clips.is_empty();
  let uses_windows = clips.iter().any(|clip| clip.window.is_some());
  Project {
    version: if uses_patterns {
      PROJECT_VERSION
    } else if uses_windows {
      PROJECT_VERSION_WITHOUT_PATTERNS
    } else {
      PROJECT_VERSION_WITHOUT_WINDOWS
    },
    settings: ProjectSettings {
      tempo,
      time_signature: *state.playlist.time_signature.lock().unwrap(),
      metronome_enabled: state.metronome_enabled.load(Ordering::SeqCst),
      loop_enabled: state.playlist.loop_enabled.load(Ordering::SeqCst),
      snap_enabled: ui.snap_enabled,
      snap_note: ui.snap_subdivision,
      count_in: state.playlist.count_in.load(Ordering::SeqCst),
    },
    clips,
    tracks,
    mixer,
    sequencer,
    pattern_clips,
    armed: state.recorder.lock().unwrap().armed(),
    effects,
    dropped: Vec::new(),
  }
}

// where in its pattern, `length` steps long, a saved pattern clip starts,
// and how long it is, in steps: a start that makes no sense is the
// pattern's start, and a length that's left out or makes no sense is to
// the pattern's end (see PatternNode::set_span for the rest)
fn saved_span(clip: &ProjectPatternClip, length: u32) -> (f64, f64) {
  let start = if clip.start.is_finite() { clip.start.max(0.) } else { 0. };
  let own = if clip.length.is_finite() && clip.length >= daw::MIN_PATTERN_CLIP {
    clip.length
  } else {
    (length as f64 - start).max(daw::MIN_PATTERN_CLIP)
  };
  (start, own)
}

// replace the playlist with a project. playback stops, and every clip gets a
// new id. clips whose samples can't be found or read are left out and listed
// in `missing`, rather than failing the whole load
pub fn apply(
  state: &daw::InnerState,
  project: &Project,
  project_dir: Option<&Path>,
) -> LoadedProject {
  daw::stop_playlist(state);
  // another project's changes can't be undone in this one, nor its clips
  // pasted into it
  state.history.lock().unwrap().clear();
  state.clipboard.lock().unwrap().clear();

  // the samples are decoded before taking any locks: it takes a while, and
  // playback (and the playhead, polled from the UI) needs the graph's
  let decoded: Vec<(String, Result<daw::AudioNode, String>)> = project.clips.iter()
    .map(|clip| {
      let path = resolved_path(&clip.path, project_dir);
      let path_str = path.to_string_lossy().into_owned();
      let node = if path.is_file() {
        daw::AudioNode::new(0, path_str.clone(), Duration::ZERO, clip.track_number)
      } else {
        Err("file not found".into())
      };
      (path_str, node)
    })
    .collect();

  // the sequencer's sample rows, with their paths found like clips', and its
  // synth rows' plugins under new slot ids (slot ids aren't reused in a
  // session)
  let mut saved = project.sequencer.clone();
  for path in saved.sample_paths_mut() {
    *path = resolved_path(path, project_dir).to_string_lossy().into_owned();
  }
  saved.renumber_slots(|| state.playlist.effects.new_id());
  let row_sounds = daw::RowSound::load_rows(&saved);
  // the effects and synths, with no lock held: their plugins load on the
  // main thread. the synths' plugins don't follow the rows until they're
  // the new project's
  let pause = state.playlist.effects.pause_synths();
  let mut missing_effects = state.playlist.effects.load_saved(&project.effects);
  missing_effects.extend(state.playlist.effects.load_synths(&saved.rows));
  // the UI isn't frozen while that loads: anything played meanwhile stops
  // before the playlist changes under it
  daw::stop_playlist(state);

  let settings = project.settings;
  state.metronome_enabled.store(settings.metronome_enabled, Ordering::SeqCst);
  state.playlist.loop_enabled.store(settings.loop_enabled, Ordering::SeqCst);
  state.playlist.count_in.store(settings.count_in, Ordering::SeqCst);
  *state.playlist.time_signature.lock().unwrap() = settings.time_signature;
  {
    let mut ui = state.playlist.ui.lock().unwrap();
    ui.snap_enabled = settings.snap_enabled;
    ui.snap_subdivision = settings.snap_note;
  }

  state.playlist.mixer.apply(&project.mixer);
  state.recorder.lock().unwrap().set_armed(&project.armed);

  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  audiograph.clear();
  audiograph.min_tracks = project.tracks.clamp(daw::defaults::NUM_OF_TRACKS, MAX_SAVED_TRACKS);
  audiograph.set_tempo(settings.tempo);
  let interval = audiograph.beat_interval();

  let mut clips = Vec::new();
  let mut missing = Vec::new();

  for (clip, (path_str, node)) in project.clips.iter().zip(decoded) {
    let start_beat = if clip.start_beat.is_finite() { clip.start_beat.max(0.) } else { 0. };

    match node {
      Ok(mut node) => {
        node.start_offset = interval.mul_f64(start_beat);
        let id = audiograph.insert_node(node);
        let node = audiograph
          .get_mut_node_by_id(id)
          .expect("a node that was just added");
        node.set_settings(clip.settings);
        // the clip stays where it was heard: only the window changes, not
        // where it starts
        if let Some(window) = clip.window {
          let (start, end) = window.sample_times();
          node.restore_window(start, end);
        }

        clips.push(LoadedClip {
          id,
          path: path_str,
          track_number: clip.track_number,
          start_beat: start_beat as f32,
          info: clip_info(node),
        });
      }
      Err(reason) => missing.push(MissingSample { path: path_str, reason }),
    }
  }

  let sequencer = {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    for (path, reason) in sequencer.restore_loaded(saved, row_sounds) {
      missing.push(MissingSample { path, reason });
    }
    sequencer.state(&*state.playlist.effects)
  };

  // pattern clips, each with a new id. a clip of a pattern the project
  // doesn't have is left out
  let mut pattern_clips = Vec::new();
  for clip in &project.pattern_clips {
    let Some(pattern) = sequencer.patterns.iter().find(|p| p.id == clip.pattern_id) else {
      println!("left out a clip of pattern {}, which isn't in the project", clip.pattern_id);
      continue;
    };
    let start_beat = if clip.start_beat.is_finite() { clip.start_beat.max(0.) } else { 0. };
    match audiograph.add_pattern_node::<daw::timing::Note>(
      clip.pattern_id, interval.mul_f64(start_beat), clip.track_number, None,
    ) {
      Ok(id) => {
        let (start, length) = saved_span(clip, pattern.length);
        let node = audiograph.pattern_nodes.iter_mut().find(|n| n.id == id).expect("a node that was just added");
        node.set_span(start, length);
        pattern_clips.push(LoadedPatternClip {
          id,
          pattern_id: clip.pattern_id,
          track_number: clip.track_number,
          start_beat: start_beat as f32,
          window: ClipWindow::of_pattern(node),
        });
      }
      Err(e) => println!("left out a clip of pattern {}: {}", clip.pattern_id, e),
    }
  }

  let mixer = state.playlist.mixer.settings(audiograph.track_count());
  // anything changed while the samples were loading (a fader moved, a step
  // lit) was a change to the project that's gone: its history goes too
  state.history.lock().unwrap().clear();
  // and anything copied meanwhile was the old project's
  state.clipboard.lock().unwrap().clear();
  drop(audiograph);
  // each track gets a copy of the synths it plays
  drop(pause);
  crate::commands::synth::follow(state);
  // in bars of the project's time signature, and as long as its clips
  daw::fit_playlist(state);

  LoadedProject {
    settings,
    clips,
    missing,
    mixer,
    sequencer,
    pattern_clips,
    armed: project.armed.clone(),
    effects: state.playlist.effects.racks(),
    missing_effects,
    dropped: project.dropped.clone(),
  }
}

// write the current playlist to a project file. the file is written next to
// it first and then moved into place, so a failed save never leaves a
// half-written project. returns what was written
pub fn save_to(state: &daw::InnerState, project_path: &Path) -> Result<Project, String> {
  let dir = project_dir(project_path)?;
  let project = capture(state, Some(&dir));
  write_project(&project, project_path)?;
  Ok(project)
}

// the folder the current project's samples are saved relative to: its
// file's, or none for a project that's never been saved
fn current_dir(state: &daw::InnerState) -> Option<PathBuf> {
  let path = state.project.lock().unwrap().path.clone();
  path.and_then(|path| project_dir(&path).ok())
}

// remember the project as it is now as unchanged: it's just been started,
// opened or saved. (captured before the project's lock is taken, as
// capturing takes the other locks)
pub fn mark_unchanged(state: &daw::InnerState) {
  let project = capture(state, current_dir(state).as_deref());
  state.project.lock().unwrap().baseline = Some(project);
}

// whether the project differs from how it was last started, opened or
// saved: anything a save would write differently. undoing back to how it
// was is no change
pub fn has_unsaved_changes(state: &daw::InnerState) -> bool {
  let now = capture(state, current_dir(state).as_deref());
  // (no baseline only before the app has started: say changed, to be safe)
  state.project.lock().unwrap().baseline.as_ref().map_or(true, |was| *was != now)
}

// write a project file, all at once: a file that's half written is never
// left in its place
fn write_project(project: &Project, project_path: &Path) -> Result<(), String> {
  let json = serde_json::to_string_pretty(project)
    .map_err(|e| format!("could not write the project: {}", e))?;

  let mut tmp = project_path.as_os_str().to_owned();
  tmp.push(".tmp");
  let tmp = PathBuf::from(tmp);

  fs::write(&tmp, json)
    .and_then(|_| fs::rename(&tmp, project_path))
    .map_err(|e| {
      let _ = fs::remove_file(&tmp);
      format!("could not save '{}': {}", project_path.display(), e)
    })
}

// a path that may not be there any more, with its folder resolved (links
// and all), so it compares with the resolved paths a project file's point to
fn resolved_parent(path: &Path) -> PathBuf {
  match (path.parent().and_then(|parent| fs::canonicalize(parent).ok()), path.file_name()) {
    (Some(parent), Some(name)) => parent.join(name),
    _ => path.to_path_buf(),
  }
}

// a take, or a folder of them, moved in the recordings folder. that happens
// on disk straight away, so the saved project file follows it: opening the
// project before it's next saved still finds its takes. only the moved
// paths change in the file, so other changes stay unsaved
pub fn follow_moved_recording(project_path: &Path, from: &Path, to: &Path) -> Result<(), String> {
  if !project_path.is_file() { return Ok(()); }
  let json = fs::read_to_string(project_path)
    .map_err(|e| format!("could not open '{}': {}", project_path.display(), e))?;
  let mut project = Project::from_json(&json)?;
  let dir = project_dir(project_path)?;

  if follow_moved(&mut project, Some(&dir), from, to) {
    write_project(&project, project_path)
  } else {
    Ok(())
  }
}

// the same for the project as it was last saved (see mark_unchanged), whose
// clips now play from where the take is: moving it isn't a change
pub fn follow_moved_in_baseline(state: &daw::InnerState, from: &Path, to: &Path) {
  let dir = current_dir(state);
  if let Some(baseline) = state.project.lock().unwrap().baseline.as_mut() {
    follow_moved(baseline, dir.as_deref(), from, to);
  }
}

// a project's clips and sample rows playing `from` (a take, or anything in
// a folder) play from `to` instead. whether any did
fn follow_moved(project: &mut Project, dir: Option<&Path>, from: &Path, to: &Path) -> bool {
  let from = resolved_parent(from);
  let to = resolved_parent(to);

  let mut changed = false;
  let mut follow = |stored: &mut String| {
    let resolved = resolved_parent(&resolved_path(stored, dir));
    if let Ok(rest) = resolved.strip_prefix(&from) {
      let moved = if rest.as_os_str().is_empty() { to.clone() } else { to.join(rest) };
      *stored = stored_path(&moved.to_string_lossy(), dir);
      changed = true;
    }
  };
  project.clips.iter_mut().for_each(|clip| follow(&mut clip.path));
  project.sequencer.sample_paths_mut().for_each(|path| follow(path));
  changed
}

// replace the playlist with a project file. nothing changes if the file
// can't be read or isn't a project
pub fn load_from(state: &daw::InnerState, project_path: &Path) -> Result<LoadedProject, String> {
  let json = fs::read_to_string(project_path)
    .map_err(|e| format!("could not open '{}': {}", project_path.display(), e))?;
  let project = Project::from_json(&json)?;
  let dir = project_dir(project_path)?;

  let loaded = apply(state, &project, Some(&dir));
  state.project.lock().unwrap().reset(Some(project_path.to_path_buf()));
  // as it loaded: samples left out for being missing aren't a change
  mark_unchanged(state);
  Ok(loaded)
}

// a clip whose take was brought into the project, and where it is now
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovedClip {
  pub id: u64,
  pub path: String,
}

// a project that's been saved
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedProject {
  // the file it was saved to, which (saved somewhere new) is in a folder of
  // its own
  pub path: String,
  pub moved_clips: Vec<MovedClip>,
  // the sequencer, when a row's take was brought in too
  pub sequencer: Option<daw::SequencerState>,
  // takes that couldn't be brought into the project. they're still used
  // from where they are
  pub problems: Vec<String>,
}

// save the project, carrying on in the saved file
// - somewhere_new: a first save, or Save As. the project gets a folder of
//   its own (unless it's already in one). otherwise it's saved just where
//   `path` says, so a project saved before folders stays as it was
//
// takes recorded before the project was first saved are moved into its
// recordings folder. saving as somewhere new copies the old project's takes
// in, so the old project keeps its own. either way the whole folder comes
// over (its folders, and takes nothing uses), and takes are named after the
// project, keeping the day they were recorded
pub fn save_project_to(
  state: &daw::InnerState,
  path: &Path,
  somewhere_new: bool,
) -> Result<SavedProject, String> {
  let target = if somewhere_new { project_files::project_file_for(path) } else { path.to_path_buf() };
  if let Some(dir) = target.parent().filter(|dir| !dir.as_os_str().is_empty()) {
    fs::create_dir_all(dir)
      .map_err(|e| format!("could not make the folder '{}': {}", dir.display(), e))?;
  }
  let name = project_files::project_name(&target).unwrap_or_else(|| project_files::UNTITLED.into());

  // where the project's takes are now, and whether they stay there too.
  // and its written-out clips (Deep copy, Join), which only a saved project
  // has, and which the project it's saved from always keeps
  let (takes, clips) = {
    let location = state.project.lock().unwrap();
    let takes = match (&location.path, &location.session) {
      (Some(path), _) => Some((recordings_dir_for(path), true)),
      (None, Some(session)) => Some((session.clone(), false)),
      (None, None) => None,
    };
    (takes, location.path.as_deref().map(|path| (project_files::clips_dir_for(path), true)))
  };
  let folders = [
    (takes, recordings_dir_for(&target)),
    (clips, project_files::clips_dir_for(&target)),
  ];

  let mut moved_clips = Vec::new();
  let mut sequencer = None;
  let mut problems = Vec::new();

  for (from, to) in folders {
    let (from, copy) = match from.filter(|(from, _)| from.is_dir() && !project_files::same_dir(from, &to)) {
      Some(from) => from,
      None => continue,
    };
    // the whole folder comes over, folders and unused files too, so the
    // sidebar shows the same after saving. (before taking the locks:
    // copying takes a while, and the clips' sounds are already loaded)
    let (relocated, errors) = project_files::relocate_takes(&from, &to, &name, copy);
    problems.extend(errors);

    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let mut seq = state.playlist.sequencer.lock().unwrap();

    for node in audiograph.nodes.iter_mut() {
      if let Some(take) = relocated.iter().find(|take| take.from == node.sample_path()) {
        node.set_sample_path(take.to.clone());
        moved_clips.push(MovedClip { id: node.id, path: take.to.clone() });
      }
    }

    let moved: Vec<(String, String)> = relocated.into_iter().map(|take| (take.from, take.to)).collect();
    if seq.move_row_samples(&moved) {
      sequencer = Some(seq.state(&*state.playlist.effects));
    }
    drop(seq);
    drop(audiograph);
    // clips and rows that undoing would put back follow their files too
    state.history.lock().unwrap().move_paths(&moved);
  }

  let saved = save_to(state, &target)?;
  {
    let mut location = state.project.lock().unwrap();
    location.reset(Some(target.clone()));
    location.baseline = Some(saved);
  }

  for problem in &problems { println!("saving the project: {}", problem); }
  Ok(SavedProject {
    path: target.to_string_lossy().into_owned(),
    moved_clips,
    sequencer,
    problems,
  })
}

// (async, like load_project: off the main thread, as it can take a while)
#[tauri::command(async)]
pub fn save_project(state: daw::TState<'_>, path: String, somewhere_new: bool) -> Result<SavedProject, String> {
  save_project_to(&state, Path::new(&path), somewhere_new)
}

// the file a project chosen to be saved at `path` would be saved to
// somewhere new (in a folder of its own)
#[tauri::command]
pub fn project_file_for(path: String) -> String {
  project_files::project_file_for(Path::new(&path)).to_string_lossy().into_owned()
}

// (async: off the main thread, as decoding every sample takes a while)
#[tauri::command(async)]
pub fn load_project(state: daw::TState<'_>, path: String) -> Result<LoadedProject, String> {
  load_from(&state, Path::new(&path))
}

// start an empty project with the default settings
#[tauri::command]
pub fn new_project(state: daw::TState<'_>) -> LoadedProject {
  let loaded = apply(&state, &Project::default(), None);
  state.project.lock().unwrap().reset(None);
  mark_unchanged(&state);
  loaded
}

// whether quitting, or replacing the project, would lose changes
#[tauri::command]
pub fn project_has_unsaved_changes(state: daw::TState<'_>) -> bool {
  has_unsaved_changes(&state)
}

mod merge_pattern_kinds;
use merge_pattern_kinds::merge_pattern_kinds;

#[cfg(test)]
mod tests;
