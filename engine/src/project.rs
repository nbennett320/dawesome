// saving and loading projects: the playlist's clips, their settings, the
// playlist's settings, the mixer and its effects, in a JSON file (.dawesome)
//
// an effect is saved with its plugin's id, bundle and settings (base64). one
// whose plugin can't be loaded comes back missing, keeping its settings, and
// is saved again just as it was read
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
use crate::event_dispatch::{clip_info, ClipInfo, ClipWindow};
use crate::project_files::{self, recordings_dir_for};

// the newest project format this version of the app reads and writes.
// bump it when a change means older apps would misread a project
pub const PROJECT_VERSION: u32 = 3;
// what a project with patterns but no trimmed clips is saved as, so older
// apps can still open it. version 3 added clips that play part of their
// sample or pattern
pub const PROJECT_VERSION_WITHOUT_WINDOWS: u32 = 2;
// what a project without patterns or trimmed clips is saved as, so older
// apps can still open it. version 2 added the sequencer and pattern clips
pub const PROJECT_VERSION_WITHOUT_PATTERNS: u32 = 1;

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
  // the part of the pattern it plays, in steps. left out for all of it
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub window: Option<ClipWindow>,
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
  // only tracks changed from their defaults, or named, are saved
  pub mixer: daw::MixerSettings,
  // the Sequencer tab's rows (with their sample paths saved like clips') and
  // patterns, left out when there are none
  #[serde(skip_serializing_if = "daw::SavedSequencer::is_empty")]
  pub sequencer: daw::SavedSequencer,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub pattern_clips: Vec<ProjectPatternClip>,
  // the tracks armed to record, and the input channels each records
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub armed: Vec<crate::recording::ArmedTrack>,
  // each channel's effects, in order, left out when there are none
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub effects: Vec<crate::effects::SavedRack>,
}

impl Default for Project {
  fn default() -> Self {
    Project {
      version: PROJECT_VERSION_WITHOUT_PATTERNS,
      settings: ProjectSettings::default(),
      clips: Vec::new(),
      mixer: daw::MixerSettings::default(),
      sequencer: daw::SavedSequencer::default(),
      pattern_clips: Vec::new(),
      armed: Vec::new(),
      effects: Vec::new(),
    }
  }
}

impl Project {
  // read a project from its JSON, refusing ones this app can't play
  pub fn from_json(json: &str) -> Result<Self, String> {
    let project: Project = serde_json::from_str(json)
      .map_err(|e| format!("not a valid project file: {}", e))?;
    project.validate()?;

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
  // the part of the pattern it plays, in steps. None: all of it
  pub window: Option<ClipWindow>,
}

impl LoadedPatternClip {
  pub fn of(node: &daw::PatternNode, interval: Duration) -> Self {
    LoadedPatternClip {
      id: node.id,
      pattern_id: node.pattern_id,
      track_number: node.track_number,
      start_beat: beats(node.start_offset, interval),
      window: node.window.map(ClipWindow::of_pattern),
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
  // marked missing)
  pub missing_effects: Vec<crate::effects::MissingEffect>,
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
  let (tempo, clips, pattern_clips, sequencer) = {
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

    let clips: Vec<ProjectClip> = audiograph.nodes.iter().map(|node| ProjectClip {
      path: stored_path(node.sample_path(), project_dir),
      track_number: node.track_number,
      start_beat: beat_of(node.start_offset, node.window().is_some()),
      settings: node.settings(),
      window: node.window().map(ClipWindow::of_sample),
    }).collect();

    // in the order they start, so saving twice gives the same file
    let mut pattern_clips: Vec<ProjectPatternClip> = audiograph.pattern_nodes.iter().map(|node| ProjectPatternClip {
      pattern_id: node.pattern_id,
      track_number: node.track_number,
      start_beat: beat_of(node.start_offset, node.window.is_some()),
      window: node.window.map(ClipWindow::of_pattern),
    }).collect();
    pattern_clips.sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat).then(a.track_number.cmp(&b.track_number)));

    let mut sequencer = state.playlist.sequencer.lock().unwrap().saved();
    for row in &mut sequencer.rows {
      row.path = stored_path(&row.path, project_dir);
    }

    (audiograph.tempo(), clips, pattern_clips, sequencer)
  };

  let ui = *state.playlist.ui.lock().unwrap();
  // (with no lock held: the plugins give their settings on the main thread)
  let effects = state.playlist.effects.saved();

  let mut mixer = state.playlist.mixer.settings(0);
  mixer.tracks.retain(|track| track.settings != daw::ChannelSettings::default() || track.name.is_some());

  let uses_patterns = !sequencer.is_empty() || !pattern_clips.is_empty();
  let uses_windows = clips.iter().any(|clip| clip.window.is_some())
    || pattern_clips.iter().any(|clip| clip.window.is_some());
  Project {
    version: if uses_windows {
      PROJECT_VERSION
    } else if uses_patterns {
      PROJECT_VERSION_WITHOUT_WINDOWS
    } else {
      PROJECT_VERSION_WITHOUT_PATTERNS
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
    mixer,
    sequencer,
    pattern_clips,
    armed: state.recorder.lock().unwrap().armed(),
    effects,
  }
}

// a saved pattern clip window made sensible: no negative start, and no end
// that isn't after it. None if that leaves all of the pattern
fn sanitised_pattern_window(window: ClipWindow) -> Option<daw::PatternWindow> {
  let start = if window.start.is_finite() && window.start > 0. { window.start } else { 0. };
  let end = window.end.filter(|end| end.is_finite() && *end > start);
  if start == 0. && end.is_none() { None } else { Some(daw::PatternWindow { start, end }) }
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

  // the sequencer's rows, with their paths found like clips'
  let mut saved = project.sequencer.clone();
  for row in &mut saved.rows {
    row.path = resolved_path(&row.path, project_dir).to_string_lossy().into_owned();
  }
  let row_samples = daw::RowSample::load_rows(&saved);
  // the effects, with no lock held: their plugins load on the main thread
  let missing_effects = state.playlist.effects.load_saved(&project.effects);
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
    for (path, reason) in sequencer.restore_loaded(saved, row_samples) {
      missing.push(MissingSample { path, reason });
    }
    sequencer.state()
  };

  // pattern clips, each with a new id. a clip of a pattern the project
  // doesn't have is left out
  let mut pattern_clips = Vec::new();
  for clip in &project.pattern_clips {
    if !sequencer.patterns.iter().any(|p| p.id == clip.pattern_id) {
      println!("left out a clip of pattern {}, which isn't in the project", clip.pattern_id);
      continue;
    }
    let start_beat = if clip.start_beat.is_finite() { clip.start_beat.max(0.) } else { 0. };
    match audiograph.add_pattern_node::<daw::timing::Note>(
      clip.pattern_id, interval.mul_f64(start_beat), clip.track_number, None,
    ) {
      Ok(id) => {
        // kept as saved, even past the pattern's current end: lengthening
        // the pattern later gives the clip its steps back
        let window = clip.window.and_then(sanitised_pattern_window);
        let node = audiograph.pattern_nodes.iter_mut().find(|n| n.id == id).expect("a node that was just added");
        node.window = window;
        pattern_clips.push(LoadedPatternClip {
          id,
          pattern_id: clip.pattern_id,
          track_number: clip.track_number,
          start_beat: start_beat as f32,
          window: window.map(ClipWindow::of_pattern),
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
  }
}

// write the current playlist to a project file. the file is written next to
// it first and then moved into place, so a failed save never leaves a
// half-written project
pub fn save_to(state: &daw::InnerState, project_path: &Path) -> Result<(), String> {
  let dir = project_dir(project_path)?;
  write_project(&capture(state, Some(&dir)), project_path)
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
  let from = resolved_parent(from);
  let to = resolved_parent(to);

  let mut changed = false;
  let mut follow = |stored: &mut String| {
    let resolved = resolved_parent(&resolved_path(stored, Some(&dir)));
    if let Ok(rest) = resolved.strip_prefix(&from) {
      let moved = if rest.as_os_str().is_empty() { to.clone() } else { to.join(rest) };
      *stored = stored_path(&moved.to_string_lossy(), Some(&dir));
      changed = true;
    }
  };
  project.clips.iter_mut().for_each(|clip| follow(&mut clip.path));
  project.sequencer.rows.iter_mut().for_each(|row| follow(&mut row.path));

  if changed { write_project(&project, project_path) } else { Ok(()) }
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
      sequencer = Some(seq.state());
    }
    drop(seq);
    drop(audiograph);
    // clips and rows that undoing would put back follow their files too
    state.history.lock().unwrap().move_paths(&moved);
  }

  save_to(state, &target)?;
  state.project.lock().unwrap().reset(Some(target.clone()));

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
  loaded
}

#[cfg(test)]
mod tests {
  use super::*;

  // an empty folder for one test's files
  fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
      .join(format!("dawesome-project-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::canonicalize(dir).unwrap()
  }

  fn add_clip(state: &daw::InnerState, path: &str, track: u32, beat: f64) -> u64 {
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let offset = audiograph.beat_interval().mul_f64(beat);
    audiograph.construct_and_add_node(path.to_string(), offset, track).unwrap()
  }

  // a take (any short sample will do), as it would be recorded
  fn record_take(dir: &Path, name: &str) -> String {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    fs::copy(daw::METRONOME_TICK_PATH, &path).unwrap();
    path.to_string_lossy().into_owned()
  }

  fn saved_json(path: &str) -> Project {
    Project::from_json(&fs::read_to_string(path).unwrap()).unwrap()
  }

  #[test]
  fn test_count_in_is_saved() {
    let dir = test_dir("count-in");
    let state = daw::InnerState::default();
    state.playlist.count_in.store(true, Ordering::SeqCst);
    let file = dir.join("song.dawesome");
    save_to(&state, &file).unwrap();

    let loaded = daw::InnerState::default();
    assert!(load_from(&loaded, &file).unwrap().settings.count_in);
    assert!(loaded.playlist.count_in.load(Ordering::SeqCst));
    // projects from before it was saved don't count in
    assert!(!Project::from_json("{}").unwrap().settings.count_in);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_armed_tracks_are_saved() {
    use crate::recording::{ArmedTrack, InputChoice};
    let dir = test_dir("armed");
    let state = daw::InnerState::default();
    let armed = vec![
      ArmedTrack { track_number: 0, input: InputChoice { first: 0, stereo: true } },
      ArmedTrack { track_number: 2, input: InputChoice { first: 3, stereo: false } },
    ];
    state.recorder.lock().unwrap().set_armed(&armed);
    let file = dir.join("song.dawesome");
    save_to(&state, &file).unwrap();

    let loaded = daw::InnerState::default();
    assert_eq!(load_from(&loaded, &file).unwrap().armed, armed);
    assert_eq!(loaded.recorder.lock().unwrap().armed(), armed);

    // a new project has nothing armed
    apply(&loaded, &Project::default(), None);
    assert!(loaded.recorder.lock().unwrap().armed().is_empty());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_first_save_makes_a_folder_and_brings_in_takes() {
    let dir = test_dir("first-save");
    let state = daw::InnerState::default();
    // recorded before the project had a file
    let session = dir.join("unsaved-session");
    state.project.lock().unwrap().session = Some(session.clone());
    let take = record_take(&session, "untitled_11092026_0.wav");
    let clip = add_clip(&state, &take, 0, 0.);
    let sample = add_clip(&state, daw::METRONOME_TICK_PATH, 1, 4.);
    let row = state.playlist.sequencer.lock().unwrap().add_row(&take).unwrap();

    let saved = save_project_to(&state, &dir.join("song.dawesome"), true).unwrap();

    let project_file = dir.join("song").join("song.dawesome");
    let recorded = dir.join("song").join("recordings").join("song_11092026_0.wav");
    assert_eq!(saved.path, project_file.to_string_lossy());
    assert!(project_file.is_file());
    assert!(recorded.is_file());
    assert!(!Path::new(&take).exists());
    // the unsaved folder is tidied away once it's empty
    assert!(!session.exists());
    assert!(saved.problems.is_empty());

    // the clip and the row follow their take; the other sample stays put
    assert_eq!(saved.moved_clips, vec![MovedClip { id: clip, path: recorded.to_string_lossy().into_owned() }]);
    let audiograph = state.playlist.audiograph.lock().unwrap();
    assert_eq!(audiograph.get_node_by_id(clip).unwrap().sample_path(), recorded.to_string_lossy());
    assert_eq!(audiograph.get_node_by_id(sample).unwrap().sample_path(), daw::METRONOME_TICK_PATH);
    drop(audiograph);
    let rows = saved.sequencer.expect("the sequencer, with its row moved").rows;
    assert_eq!(rows.iter().find(|r| r.id == row).unwrap().path, recorded.to_string_lossy());

    // saved relative to the project, so the folder can be moved
    let project = saved_json(&saved.path);
    assert_eq!(project.clips[0].path, "recordings/song_11092026_0.wav");
    assert_eq!(project.sequencer.rows[0].path, "recordings/song_11092026_0.wav");

    // and takes recorded from now on go in its recordings folder
    let mut location = state.project.lock().unwrap();
    assert_eq!(location.recordings_dir().unwrap(), dir.join("song").join("recordings"));
    assert_eq!(location.name(), "song");
    drop(location);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_first_save_keeps_folders_and_unused_takes() {
    let dir = test_dir("first-save-folders");
    let state = daw::InnerState::default();
    let session = dir.join("unsaved-session");
    state.project.lock().unwrap().session = Some(session.clone());
    let filed = record_take(&session.join("Vocals"), "untitled_11092026_0.wav");
    let unused = record_take(&session, "untitled_11092026_1.wav");
    let clip = add_clip(&state, &filed, 0, 0.);

    let saved = save_project_to(&state, &dir.join("song.dawesome"), true).unwrap();

    let recordings = dir.join("song").join("recordings");
    let moved = recordings.join("Vocals").join("song_11092026_0.wav");
    assert!(saved.problems.is_empty(), "{:?}", saved.problems);
    assert!(moved.is_file());
    assert!(recordings.join("song_11092026_1.wav").is_file());
    assert!(!Path::new(&filed).exists() && !Path::new(&unused).exists());
    assert!(!session.exists());

    assert_eq!(saved.moved_clips, vec![MovedClip { id: clip, path: moved.to_string_lossy().into_owned() }]);
    assert_eq!(saved_json(&saved.path).clips[0].path, "recordings/Vocals/song_11092026_0.wav");

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_save_as_copies_takes_into_the_new_project() {
    let dir = test_dir("save-as");
    let state = daw::InnerState::default();
    let old_file = dir.join("song").join("song.dawesome");
    let take = record_take(&dir.join("song").join("recordings"), "song_11092026_0.wav");
    add_clip(&state, &take, 0, 0.);
    save_project_to(&state, &old_file, false).unwrap();

    let saved = save_project_to(&state, &dir.join("remix.dawesome"), true).unwrap();

    let copy = dir.join("remix").join("recordings").join("remix_11092026_0.wav");
    assert_eq!(saved.path, dir.join("remix").join("remix.dawesome").to_string_lossy());
    assert!(copy.is_file());
    // the old project still has its take, and still plays it
    assert!(Path::new(&take).is_file());
    assert_eq!(saved_json(old_file.to_str().unwrap()).clips[0].path, "recordings/song_11092026_0.wav");
    assert_eq!(saved_json(&saved.path).clips[0].path, "recordings/remix_11092026_0.wav");

    // saving again leaves the takes where they are
    let again = save_project_to(&state, Path::new(&saved.path), false).unwrap();
    assert!(again.moved_clips.is_empty());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_a_project_saved_before_folders_stays_where_it_is() {
    let dir = test_dir("bare");
    let state = daw::InnerState::default();
    add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
    let file = dir.join("old.dawesome");

    let saved = save_project_to(&state, &file, false).unwrap();

    assert_eq!(saved.path, file.to_string_lossy());
    assert!(!dir.join("old").exists());
    // its takes go next to it
    assert_eq!(state.project.lock().unwrap().recordings_dir().unwrap(), dir.join("recordings"));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_opening_and_new_projects_move_where_takes_go() {
    let dir = test_dir("open");
    let state = daw::InnerState::default();
    let file = dir.join("song").join("song.dawesome");
    save_project_to(&state, &file, false).unwrap();

    let other = daw::InnerState::default();
    load_from(&other, &file).unwrap();
    assert_eq!(other.project.lock().unwrap().path.as_deref(), Some(file.as_path()));

    apply(&other, &Project::default(), None);
    other.project.lock().unwrap().reset(None);
    assert_eq!(other.project.lock().unwrap().name(), project_files::UNTITLED);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_clips_are_saved_on_their_beats_after_a_tempo_change() {
    let sample = fs::canonicalize(daw::METRONOME_TICK_PATH).unwrap();
    let state = daw::InnerState::default();
    add_clip(&state, sample.to_str().unwrap(), 1, 4.5);
    add_clip(&state, sample.to_str().unwrap(), 0, 200.);

    // placed at 120bpm, saved at 93 (then 77): the same beats, to the digit
    state.set_tempo(93.);
    state.set_tempo(77.);
    let beats: Vec<f64> = capture(&state, None).clips.iter().map(|clip| clip.start_beat).collect();
    assert_eq!(beats, vec![4.5, 200.]);
  }

  #[test]
  fn test_save_and_load_round_trip() {
    let dir = test_dir("round-trip");
    let project_path = dir.join("song.dawesome");
    let sample = fs::canonicalize(daw::METRONOME_TICK_PATH).unwrap();
    let sample = sample.to_str().unwrap();

    let state = daw::InnerState::default();
    state.set_tempo(90.);
    *state.playlist.time_signature.lock().unwrap() =
      daw::timing::TimeSignature { numerator: 3, denominator: 4 };
    state.playlist.ui.lock().unwrap().snap_subdivision = daw::timing::Note::EighthNote;
    state.metronome_enabled.store(false, Ordering::SeqCst);

    add_clip(&state, sample, 0, 0.);
    let id = add_clip(&state, sample, 2, 4.5);
    state.playlist.audiograph.lock().unwrap()
      .get_mut_node_by_id(id).unwrap()
      .set_settings(daw::ClipSettings { gain_db: -6., pitch: 12., attack_ms: 5., ..Default::default() });

    save_to(&state, &project_path).unwrap();
    assert!(!dir.join("song.dawesome.tmp").exists());

    // a fresh app with a clip of its own, which loading replaces
    let loaded_state = daw::InnerState::default();
    let old_id = add_clip(&loaded_state, sample, 1, 1.);
    let loaded = load_from(&loaded_state, &project_path).unwrap();

    assert!(loaded.missing.is_empty());
    assert_eq!(loaded.settings.tempo, 90.);
    assert_eq!(loaded.settings.time_signature.numerator, 3);
    assert_eq!(loaded.settings.snap_note, daw::timing::Note::EighthNote);
    assert!(!loaded.settings.metronome_enabled);
    assert_eq!(loaded.clips.len(), 2);
    assert!(loaded.clips.iter().all(|c| c.id != old_id));

    let clip = loaded.clips.iter().find(|c| c.track_number == 2).unwrap();
    assert!((clip.start_beat - 4.5).abs() < 1e-4);
    assert_eq!(clip.info.settings.pitch, 12.);
    assert_eq!(clip.info.settings.gain_db, -6.);

    // saving the loaded project again gives the same project
    assert_eq!(capture(&loaded_state, Some(&dir)), capture(&state, Some(&dir)));
    assert_eq!(loaded_state.playlist.audiograph.lock().unwrap().nodes.len(), 2);

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_sample_paths_relative_to_the_project() {
    let dir = test_dir("relative");
    fs::create_dir_all(dir.join("samples")).unwrap();
    let sample = dir.join("samples").join("tick.wav");
    fs::copy(daw::METRONOME_TICK_PATH, &sample).unwrap();

    let state = daw::InnerState::default();
    add_clip(&state, sample.to_str().unwrap(), 0, 0.);
    // added by a relative path, which is saved as absolute
    add_clip(&state, daw::METRONOME_TICK_PATH, 1, 0.);
    save_to(&state, &dir.join("song.dawesome")).unwrap();

    let project = Project::from_json(
      &fs::read_to_string(dir.join("song.dawesome")).unwrap()).unwrap();
    let paths: Vec<&str> = project.clips.iter().map(|c| c.path.as_str()).collect();
    assert!(paths.contains(&"samples/tick.wav"));
    assert!(paths.iter().any(|p| Path::new(p).is_absolute()));

    // the project folder still loads after it's moved
    let moved = test_dir("relative-moved");
    fs::remove_dir_all(&moved).unwrap();
    fs::rename(&dir, &moved).unwrap();

    let loaded = load_from(&daw::InnerState::default(), &moved.join("song.dawesome")).unwrap();
    assert!(loaded.missing.is_empty());
    assert!(loaded.clips.iter().any(|c| Path::new(&c.path) == moved.join("samples/tick.wav")));

    fs::remove_dir_all(moved).unwrap();
  }

  #[test]
  fn test_missing_samples_are_left_out() {
    let dir = test_dir("missing");
    let project_path = dir.join("song.dawesome");
    let sample = fs::canonicalize(daw::METRONOME_TICK_PATH).unwrap();
    fs::write(&project_path, format!(r#"{{
      "version": 1,
      "tempo": 100,
      "clips": [
        {{ "path": "gone.wav", "trackNumber": 0, "startBeat": 0 }},
        {{ "path": {:?}, "trackNumber": 1, "startBeat": 2 }}
      ]
    }}"#, sample.to_str().unwrap())).unwrap();

    let loaded = load_from(&daw::InnerState::default(), &project_path).unwrap();

    // the rest of the project still loads, with defaults for what's left out
    assert_eq!(loaded.settings.tempo, 100.);
    assert!(loaded.settings.loop_enabled);
    assert_eq!(loaded.clips.len(), 1);
    assert_eq!(loaded.clips[0].info.settings, daw::ClipSettings::default());
    assert_eq!(loaded.missing.len(), 1);
    assert_eq!(Path::new(&loaded.missing[0].path), dir.join("gone.wav"));

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_bad_projects_change_nothing() {
    let dir = test_dir("bad");
    let state = daw::InnerState::default();
    add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);

    let bad = [
      "not json",
      r#"{"version": 99}"#,
      r#"{"tempo": 0}"#,
      r#"{"timeSignature": {"numerator": 4, "denominator": 0}}"#,
      r#"{"clips": [{"trackNumber": 0}]}"#,
    ];
    for (i, json) in bad.iter().enumerate() {
      let path = dir.join(format!("bad-{}.dawesome", i));
      fs::write(&path, json).unwrap();
      assert!(load_from(&state, &path).is_err(), "{} should not load", json);
    }
    assert!(load_from(&state, &dir.join("nowhere.dawesome")).is_err());

    assert_eq!(state.playlist.audiograph.lock().unwrap().nodes.len(), 1);
    assert_eq!(state.tempo(), 120.);

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_new_project_clears_the_playlist() {
    let state = daw::InnerState::default();
    let old_id = add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
    state.set_tempo(80.);

    let loaded = apply(&state, &Project::default(), None);

    assert!(loaded.clips.is_empty());
    assert_eq!(loaded.settings, ProjectSettings::default());
    assert_eq!(state.tempo(), 120.);
    assert!(state.playlist.audiograph.lock().unwrap().nodes.is_empty());

    // ids aren't reused, so a stale id from the old project names nothing
    let new_id = add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
    assert!(new_id > old_id);
  }

  #[test]
  fn test_mixer_saved_and_loaded() {
    let dir = test_dir("mixer");
    let project_path = dir.join("mix.dawesome");

    let state = daw::InnerState::default();
    state.playlist.mixer.set_volume_db(2, -12.);
    state.playlist.mixer.set_solo(2, true);
    state.playlist.mixer.set_master_volume_db(-3.);
    state.playlist.mixer.set_name(3, "Bass");
    save_to(&state, &project_path).unwrap();

    // only the changed and named tracks are saved
    let saved = Project::from_json(&fs::read_to_string(&project_path).unwrap()).unwrap();
    let saved_tracks: Vec<u32> = saved.mixer.tracks.iter().map(|t| t.track_number).collect();
    assert_eq!(saved_tracks, vec![2, 3]);

    let other = daw::InnerState::default();
    other.playlist.mixer.set_mute(0, true);
    let loaded = load_from(&other, &project_path).unwrap();

    let track = |n: u32| loaded.mixer.tracks.iter().find(|t| t.track_number == n).unwrap().settings;
    let name = |n: u32| loaded.mixer.tracks.iter().find(|t| t.track_number == n).unwrap().name.clone();
    assert_eq!(name(3).as_deref(), Some("Bass"));
    assert_eq!(name(2), None);
    assert_eq!(track(2), daw::ChannelSettings { volume_db: -12., pan: 0., mute: false, solo: true });
    // the loading app's own changes don't carry over
    assert_eq!(track(0), daw::ChannelSettings::default());
    assert_eq!(loaded.mixer.master.volume_db, -3.);
    // every track the playlist shows is listed
    assert_eq!(loaded.mixer.tracks.len(), daw::defaults::NUM_OF_TRACKS as usize);

    // an older project without a mixer loads with a flat one
    let old = Project::from_json(r#"{"version": 1, "tempo": 100}"#).unwrap();
    let loaded = apply(&other, &old, None);
    assert!(loaded.mixer.tracks.iter().all(|t| t.settings == daw::ChannelSettings::default()));
    assert_eq!(loaded.mixer.master.volume_db, 0.);

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_patterns_saved_and_loaded() {
    let dir = test_dir("patterns");
    let project_path = dir.join("beat.dawesome");
    fs::create_dir_all(dir.join("drums")).unwrap();
    let kick = dir.join("drums").join("kick.wav");
    fs::copy("assets/assets_66-bd-01.wav", &kick).unwrap();

    let state = daw::InnerState::default();
    let pattern = {
      let mut audiograph = state.playlist.audiograph.lock().unwrap();
      let (row, pattern) = {
        let mut seq = audiograph.sequencer.lock().unwrap();
        let row = seq.add_row(kick.to_str().unwrap()).unwrap();
        let pattern = seq.add_pattern(2, 4).unwrap();
        seq.set_step(pattern, row, 0, true).unwrap();
        seq.set_step(pattern, row, 12, true).unwrap();
        seq.rename_pattern(pattern, "intro").unwrap();
        seq.set_row_volume(row, 0.5).unwrap();
        (row, pattern)
      };
      let _ = row;
      let beat = audiograph.beat_interval();
      audiograph.add_pattern_node::<daw::timing::Note>(pattern, beat * 4, 1, None).unwrap();
      audiograph.add_pattern_node::<daw::timing::Note>(pattern, beat * 0, 0, None).unwrap();
      pattern
    };
    save_to(&state, &project_path).unwrap();

    let saved = Project::from_json(&fs::read_to_string(&project_path).unwrap()).unwrap();
    assert_eq!(saved.version, PROJECT_VERSION_WITHOUT_WINDOWS);
    // inside the project's folder, so saved relative to it
    assert_eq!(saved.sequencer.rows[0].path, "drums/kick.wav");
    // in the order they start
    assert_eq!(saved.pattern_clips.iter().map(|c| c.start_beat).collect::<Vec<_>>(), vec![0., 4.]);

    let other = daw::InnerState::default();
    let loaded = load_from(&other, &project_path).unwrap();
    assert!(loaded.missing.is_empty());
    assert_eq!(loaded.sequencer.rows.len(), 1);
    assert_eq!(loaded.sequencer.rows[0].volume, 0.5);
    assert_eq!(loaded.sequencer.patterns[0].name.as_deref(), Some("intro"));
    assert_eq!(loaded.sequencer.patterns[0].length, 32);
    assert_eq!(loaded.pattern_clips.len(), 2);
    assert!(loaded.pattern_clips.iter().all(|c| c.pattern_id == pattern));
    assert_eq!(other.playlist.audiograph.lock().unwrap().pattern_nodes.len(), 2);

    // and saved again, it's the same project
    assert_eq!(capture(&other, Some(&dir)), capture(&state, Some(&dir)));

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_projects_without_patterns_stay_version_1() {
    let state = daw::InnerState::default();
    add_clip(&state, daw::METRONOME_TICK_PATH, 0, 0.);
    let project = capture(&state, None);
    assert_eq!(project.version, PROJECT_VERSION_WITHOUT_PATTERNS);

    // and leave the new fields out of the file
    let json = serde_json::to_value(&project).unwrap();
    assert!(json.get("sequencer").is_none());
    assert!(json.get("patternClips").is_none());
  }

  fn edit(id: u64, start: f64, end: Option<f64>) -> crate::clip_commands::WindowEdit {
    crate::clip_commands::WindowEdit { id, window: ClipWindow { start, end } }
  }

  // a one bar pattern with a kick step, and a clip of it on a beat
  fn add_pattern_clip(state: &daw::InnerState, bars: u32, beat: f64) -> (u64, u64) {
    let pattern = {
      let mut seq = state.playlist.sequencer.lock().unwrap();
      let row = seq.add_row("assets/assets_66-bd-01.wav").unwrap();
      let pattern = seq.add_pattern(bars, 4).unwrap();
      seq.set_step(pattern, row, 0, true).unwrap();
      pattern
    };
    let mut audiograph = state.playlist.audiograph.lock().unwrap();
    let offset = audiograph.beat_interval().mul_f64(beat);
    let clip = audiograph.add_pattern_node::<daw::timing::Note>(pattern, offset, 1, None).unwrap();
    (pattern, clip)
  }

  fn sorted_nodes(state: &daw::InnerState) -> Vec<daw::AudioNode> {
    let mut nodes = state.playlist.audiograph.lock().unwrap().nodes.clone();
    nodes.sort_by_key(|node| node.start_offset);
    nodes
  }

  #[test]
  fn test_a_trimmed_clip_is_saved_and_loaded() {
    let dir = test_dir("trimmed");
    let file = dir.join("song.dawesome");
    let state = daw::InnerState::default();
    let id = add_clip(&state, "assets/assets_66-bd-01.wav", 0, 2.);
    crate::clip_commands::set_windows(&state, &[edit(id, 0.02, Some(0.08))]).unwrap();
    let before = sorted_nodes(&state).remove(0);
    assert!(before.window().is_some());
    save_to(&state, &file).unwrap();

    let other = daw::InnerState::default();
    let loaded = load_from(&other, &file).unwrap();
    let after = sorted_nodes(&other).remove(0);
    assert_eq!(after.window(), before.window());
    assert_eq!(after.start_offset, before.start_offset);
    assert_eq!(after.settings(), before.settings());
    assert_eq!(loaded.clips.len(), 1);
    assert_eq!(loaded.clips[0].info.window, before.window().map(ClipWindow::of_sample));

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_the_two_halves_of_a_cut_clip_come_back_to_the_nanosecond() {
    let dir = test_dir("cut");
    let file = dir.join("song.dawesome");
    let state = daw::InnerState::default();
    let id = add_clip(&state, "assets/assets_66-bd-01.wav", 0, 2.337);
    crate::event_dispatch::change_clip_settings(
      &state, id, daw::ClipSettings { pitch: -7., ..Default::default() }, None,
    ).unwrap();
    // a beat inside the clip
    let beat = {
      let audiograph = state.playlist.audiograph.lock().unwrap();
      let node = audiograph.get_node_by_id(id).unwrap();
      (node.start_offset + node.duration() / 2).as_secs_f64() / audiograph.beat_interval().as_secs_f64()
    };
    crate::clip_commands::split(&state, &[id], beat).unwrap();
    let before = sorted_nodes(&state);
    assert_eq!(before.len(), 2);
    save_to(&state, &file).unwrap();

    let other = daw::InnerState::default();
    load_from(&other, &file).unwrap();
    let after = sorted_nodes(&other);
    assert_eq!(after.len(), 2);
    let (left, right) = (&after[0], &after[1]);
    assert_eq!(left.origin(), right.origin());
    assert_eq!(left.start_offset + left.duration(), right.start_offset);
    for (loaded, saved) in after.iter().zip(&before) {
      assert_eq!(loaded.start_offset, saved.start_offset);
      assert_eq!(loaded.window(), saved.window());
    }

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_the_version_follows_what_a_project_uses() {
    // nothing trimmed or patterned: version 1, with no window in the file
    let state = daw::InnerState::default();
    let id = add_clip(&state, "assets/assets_66-bd-01.wav", 0, 0.);
    let plain = capture(&state, None);
    assert_eq!(plain.version, 1);
    assert!(!serde_json::to_string(&plain).unwrap().contains("\"window\""));

    // a trimmed sample clip, without patterns
    crate::clip_commands::set_windows(&state, &[edit(id, 0.02, Some(0.08))]).unwrap();
    let trimmed = capture(&state, None);
    assert_eq!(trimmed.version, 3);
    assert!(serde_json::to_string(&trimmed).unwrap().contains("\"window\""));

    // patterns, not trimmed
    let state = daw::InnerState::default();
    let (_, clip) = add_pattern_clip(&state, 1, 0.);
    assert_eq!(capture(&state, None).version, 2);

    // a trimmed pattern clip
    crate::clip_commands::set_windows(&state, &[edit(clip, 4., Some(12.))]).unwrap();
    let trimmed = capture(&state, None);
    assert_eq!(trimmed.version, 3);
    assert_eq!(trimmed.pattern_clips[0].window, Some(ClipWindow { start: 4., end: Some(12.) }));
  }

  #[test]
  fn test_a_pattern_clips_window_is_kept_past_the_patterns_end() {
    let dir = test_dir("pattern-window");
    let file = dir.join("song.dawesome");
    let state = daw::InnerState::default();
    let (pattern, clip) = add_pattern_clip(&state, 2, 0.);
    crate::clip_commands::set_windows(&state, &[edit(clip, 4., Some(24.))]).unwrap();
    state.playlist.sequencer.lock().unwrap().set_pattern_bars(pattern, 1, 4).unwrap();
    save_to(&state, &file).unwrap();

    let other = daw::InnerState::default();
    let loaded = load_from(&other, &file).unwrap();
    let window = Some(daw::PatternWindow { start: 4., end: Some(24.) });
    assert_eq!(other.playlist.audiograph.lock().unwrap().pattern_nodes[0].window, window);
    assert_eq!(loaded.pattern_clips[0].window, Some(ClipWindow { start: 4., end: Some(24.) }));

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_an_older_file_loads_whole_and_is_left_alone() {
    let dir = test_dir("older");
    let file = dir.join("old.dawesome");
    let sample = fs::canonicalize("assets/assets_66-bd-01.wav").unwrap();
    let sample = sample.to_str().unwrap();
    let json = format!(r#"{{
  "version": 2,
  "clips": [
    {{ "path": "{sample}", "trackNumber": 0, "startBeat": 1.5 }}
  ],
  "sequencer": {{
    "rows": [{{ "id": 0, "path": "{sample}" }}],
    "patterns": [{{ "id": 1, "name": null, "length": 16, "steps": {{ "0": [0, 4] }} }}]
  }},
  "patternClips": [
    {{ "patternId": 1, "trackNumber": 1, "startBeat": 4 }}
  ]
}}
"#);
    fs::write(&file, &json).unwrap();

    let state = daw::InnerState::default();
    let loaded = load_from(&state, &file).unwrap();
    assert!(loaded.missing.is_empty());
    assert_eq!(loaded.clips.len(), 1);
    assert_eq!(loaded.pattern_clips.len(), 1);
    assert_eq!(loaded.clips[0].info.window, None);
    assert_eq!(loaded.pattern_clips[0].window, None);
    let audiograph = state.playlist.audiograph.lock().unwrap();
    assert!(audiograph.nodes.iter().all(|node| node.window().is_none()));
    assert!(audiograph.pattern_nodes.iter().all(|node| node.window.is_none()));
    drop(audiograph);
    assert_eq!(fs::read(&file).unwrap(), json.as_bytes());

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_a_newer_file_is_refused() {
    let error = Project::from_json(r#"{"version": 4}"#).unwrap_err();
    assert!(error.contains("saved by a newer version"), "{}", error);
    assert!(Project::from_json(r#"{"version": 3}"#).is_ok());
  }

  #[test]
  fn test_missing_row_samples_and_unknown_patterns() {
    let state = daw::InnerState::default();
    let project = Project::from_json(r#"{
      "version": 2,
      "sequencer": {
        "rows": [{ "id": 0, "path": "/nowhere/kick.wav" }],
        "patterns": [{ "id": 3, "name": null, "length": 16, "steps": { "0": [0, 4] } }]
      },
      "patternClips": [
        { "patternId": 3, "trackNumber": 0, "startBeat": 2 },
        { "patternId": 8, "trackNumber": 0, "startBeat": 0 }
      ]
    }"#).unwrap();

    let loaded = apply(&state, &project, None);
    assert_eq!(loaded.missing.len(), 1);
    assert_eq!(loaded.missing[0].path, "/nowhere/kick.wav");
    // the pattern is kept, without the missing row's steps
    assert_eq!(loaded.sequencer.patterns.len(), 1);
    assert!(loaded.sequencer.patterns[0].steps.is_empty());
    // only the clip of a pattern that exists
    assert_eq!(loaded.pattern_clips.len(), 1);
    assert_eq!(loaded.pattern_clips[0].start_beat, 2.);

    // a new project clears the sequencer
    let fresh = apply(&state, &Project::default(), None);
    assert!(fresh.sequencer.patterns.is_empty());
    assert!(fresh.pattern_clips.is_empty());
  }

  // each channel's effects, without their ids: (track, [(name, bypassed, missing)])
  use crate::effects::SavedEffect;

  fn rack_shape(racks: &crate::effects::Racks) -> Vec<(Option<u32>, Vec<(String, bool, bool)>)> {
    racks.iter()
      .map(|r| (r.track, r.slots.iter().map(|s| (s.name.clone(), s.bypassed, s.missing)).collect()))
      .collect()
  }

  #[test]
  fn test_effects_saved_and_loaded() {
    use crate::daw::Channel;
    use crate::effects::testing::*;
    let dir = test_dir("effects");
    let file = dir.join("fx.dawesome");

    let mut rig = rig();
    let state = rig.state.clone();
    let effects = &state.playlist.effects;
    let a = effects.add(Channel::Track(1), &gain().id, None).unwrap();
    let b = effects.add(Channel::Track(1), &gain().id, None).unwrap();
    let m = effects.add(Channel::Master, &gain().id, None).unwrap();
    // the second ahead of the first, so order shows
    effects.move_to(b, Channel::Track(1), 0).unwrap();
    rig.turn(a, 0.25);
    rig.turn(m, 0.75);
    effects.set_bypassed(b, true).unwrap();
    let heard = rig.level(Some(1));
    save_to(&state, &file).unwrap();

    let saved = saved_json(file.to_str().unwrap());
    assert_eq!(saved.effects.len(), 2);
    assert_eq!(saved.effects[0].track, Some(1));
    assert_eq!(saved.effects[1].track, None);
    let first = &saved.effects[0].slots[1];
    assert_eq!((first.plugin_id.as_str(), first.path.as_str(), first.bypassed), (gain().id.as_str(), "/fake/Gain.vst3", false));
    assert_eq!(base64::decode(&first.state).unwrap(), 0.25f32.to_le_bytes());
    assert!(saved.effects[0].slots[0].bypassed);

    let mut other = rig_with_an_effect();
    let loaded = load_from(&other.state, &file).unwrap();
    assert!(loaded.missing_effects.is_empty());
    assert_eq!(rack_shape(&loaded.effects), vec![
      (Some(1), vec![("Gain".into(), true, false), ("Gain".into(), false, false)]),
      (None, vec![("Gain".into(), false, false)]),
    ]);
    assert_eq!(loaded.effects, other.state.playlist.effects.racks());
    assert!((other.level(Some(1)) - heard).abs() < 1e-6);
    assert!((heard - 0.2 * 0.25 * 0.75).abs() < 1e-6);
    // the old project's effects went
    assert_eq!(other.loader.alive.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(capture(&other.state, Some(&dir)).effects, capture(&state, Some(&dir)).effects);

    fs::remove_dir_all(dir).unwrap();
  }

  // a rig that already has an effect, which loading a project replaces
  fn rig_with_an_effect() -> crate::effects::testing::Rig {
    let rig = crate::effects::testing::rig();
    rig.effects().add(crate::daw::Channel::Track(7), &crate::effects::testing::gain().id, None).unwrap();
    rig
  }

  #[test]
  fn test_missing_effects_are_kept_and_saved_as_they_were() {
    use crate::effects::testing::*;
    let dir = test_dir("missing-effects");
    let file = dir.join("old.dawesome");
    let absent = absent();
    // an effect that isn't installed, one found by its saved path though
    // the catalog doesn't know it, and one with damaged settings
    let json = serde_json::json!({
      "effects": [
        { "track": 0, "slots": [
          { "pluginId": absent.id, "name": "Absent", "vendor": "Fake Audio", "path": "/gone/Absent.vst3",
            "bypassed": true, "state": base64::encode([1u8, 2, 3]) },
          { "pluginId": "vst3:00000000000000000000000000000099", "name": "Elsewhere", "vendor": "Them",
            "path": "/fake/Elsewhere.vst3", "state": base64::encode(0.5f32.to_le_bytes()) },
        ] },
        { "track": null, "slots": [
          { "pluginId": gain().id, "name": "Gain", "vendor": "Fake Audio", "path": "/fake/Gain.vst3", "state": "!!" },
        ] },
      ],
    });
    fs::write(&file, json.to_string()).unwrap();

    let mut rig = rig();
    let loaded = load_from(&rig.state, &file).unwrap();
    assert_eq!(rack_shape(&loaded.effects), vec![
      (Some(0), vec![("Absent".into(), true, true), ("Elsewhere".into(), false, false)]),
      (None, vec![("Gain".into(), false, true)]),
    ]);
    let names: Vec<(Option<u32>, String)> = loaded.missing_effects.iter().map(|m| (m.track, m.name.clone())).collect();
    assert_eq!(names, vec![(Some(0), "Absent".to_string()), (None, "Gain".to_string())]);
    assert_eq!(loaded.missing_effects[0].reason, "not installed");
    assert_eq!(loaded.missing_effects[0].path, "/gone/Absent.vst3");
    // only the one that loaded is heard
    assert!((rig.level(Some(0)) - 0.1).abs() < 1e-6);
    let wire = serde_json::to_value(&loaded).unwrap();
    assert_eq!(wire["missingEffects"][0]["name"], "Absent");
    assert_eq!(wire["effects"][0]["slots"][0]["missing"], true);

    let again = dir.join("again.dawesome");
    save_to(&rig.state, &again).unwrap();
    let before = saved_json(file.to_str().unwrap()).effects;
    let after = saved_json(again.to_str().unwrap()).effects;
    assert_eq!(after[0].slots[0], before[0].slots[0]);
    // (settings that can't be read at all aren't kept)
    assert_eq!(after[1].slots[0], SavedEffect { state: String::new(), ..before[1].slots[0].clone() });
    assert_eq!(loaded.missing_effects[1].reason, "its saved settings are damaged");
    assert_eq!(after[0].slots[1].state, before[0].slots[1].state);

    fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn test_projects_without_effects_load_with_none() {
    let rig = rig_with_an_effect();
    assert!(Project::from_json("{}").unwrap().effects.is_empty());
    let loaded = apply(&rig.state, &Project::from_json(r#"{"tempo": 100}"#).unwrap(), None);
    assert!(loaded.effects.is_empty());
    assert!(rig.effects().racks().is_empty());
    assert_eq!(rig.loader.alive.load(std::sync::atomic::Ordering::SeqCst), 0);
    // and a project without effects is saved without them
    let json = serde_json::to_value(capture(&rig.state, None)).unwrap();
    assert!(json.get("effects").is_none());
  }

  #[test]
  fn test_new_project_clears_the_effects() {
    let rig = rig_with_an_effect();
    let loaded = apply(&rig.state, &Project::default(), None);
    assert!(loaded.effects.is_empty());
    assert!(rig.effects().racks().is_empty());
  }
}
