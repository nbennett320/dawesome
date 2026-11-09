// saving and loading projects: the playlist's clips, their settings, the
// playlist's settings, and the mixer, in a JSON file (.dawesome)
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
use crate::event_dispatch::{clip_info, ClipInfo};

// the newest project format this version of the app reads and writes.
// bump it when a change means older apps would misread a project
pub const PROJECT_VERSION: u32 = 2;
// what a project without patterns is saved as, so older apps can still open
// it. version 2 added the sequencer and pattern clips
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
}

// a pattern placed in the playlist, as saved in a project
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPatternClip {
  pub pattern_id: u64,
  pub track_number: u32,
  // beats from the start of the playlist
  pub start_beat: f64,
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
    if !tempo.is_finite() || tempo <= 0. || tempo > 1000. {
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

// a pattern clip after loading, with its new id
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPatternClip {
  pub id: u64,
  pub pattern_id: u64,
  pub track_number: u32,
  pub start_beat: f32,
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
    let beat_of = |offset: Duration| if interval > 0. {
      tidy_beat(offset.as_secs_f64() / interval)
    } else {
      0.
    };

    let clips = audiograph.nodes.iter().map(|node| ProjectClip {
      path: stored_path(node.sample_path(), project_dir),
      track_number: node.track_number,
      start_beat: beat_of(node.start_offset),
      settings: node.settings(),
    }).collect();

    // in the order they start, so saving twice gives the same file
    let mut pattern_clips: Vec<ProjectPatternClip> = audiograph.pattern_nodes.iter().map(|node| ProjectPatternClip {
      pattern_id: node.pattern_id,
      track_number: node.track_number,
      start_beat: beat_of(node.start_offset),
    }).collect();
    pattern_clips.sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat).then(a.track_number.cmp(&b.track_number)));

    let mut sequencer = state.playlist.sequencer.lock().unwrap().saved();
    for row in &mut sequencer.rows {
      row.path = stored_path(&row.path, project_dir);
    }

    (audiograph.tempo(), clips, pattern_clips, sequencer)
  };

  let ui = *state.playlist.ui.lock().unwrap();

  let mut mixer = state.playlist.mixer.settings(0);
  mixer.tracks.retain(|track| track.settings != daw::ChannelSettings::default() || track.name.is_some());

  let uses_patterns = !sequencer.is_empty() || !pattern_clips.is_empty();
  Project {
    version: if uses_patterns { PROJECT_VERSION } else { PROJECT_VERSION_WITHOUT_PATTERNS },
    settings: ProjectSettings {
      tempo,
      time_signature: *state.playlist.time_signature.lock().unwrap(),
      metronome_enabled: state.metronome_enabled.load(Ordering::SeqCst),
      loop_enabled: state.playlist.loop_enabled.load(Ordering::SeqCst),
      snap_enabled: ui.snap_enabled,
      snap_note: ui.snap_subdivision,
    },
    clips,
    mixer,
    sequencer,
    pattern_clips,
  }
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

  let settings = project.settings;
  state.metronome_enabled.store(settings.metronome_enabled, Ordering::SeqCst);
  state.playlist.loop_enabled.store(settings.loop_enabled, Ordering::SeqCst);
  *state.playlist.time_signature.lock().unwrap() = settings.time_signature;
  {
    let mut ui = state.playlist.ui.lock().unwrap();
    ui.snap_enabled = settings.snap_enabled;
    ui.snap_subdivision = settings.snap_note;
  }

  state.playlist.mixer.apply(&project.mixer);

  let mut audiograph = state.playlist.audiograph.lock().unwrap();
  audiograph.clear();
  audiograph.set_tempo(settings.tempo);
  let interval = audiograph.beat_interval();

  let mut clips = Vec::new();
  let mut missing = Vec::new();

  for clip in &project.clips {
    let path = resolved_path(&clip.path, project_dir);
    let path_str = path.to_string_lossy().into_owned();

    if !path.is_file() {
      missing.push(MissingSample { path: path_str, reason: "file not found".into() });
      continue;
    }

    let start_beat = if clip.start_beat.is_finite() { clip.start_beat.max(0.) } else { 0. };
    let offset: Duration = interval.mul_f64(start_beat);

    match audiograph.construct_and_add_node(path_str.clone(), offset, clip.track_number) {
      Ok(id) => {
        let node = audiograph
          .get_mut_node_by_id(id)
          .expect("a node that was just added");
        node.set_settings(clip.settings);

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

  // the sequencer's rows, with their paths found like clips'
  let mut saved = project.sequencer.clone();
  for row in &mut saved.rows {
    row.path = resolved_path(&row.path, project_dir).to_string_lossy().into_owned();
  }
  let sequencer = {
    let mut sequencer = state.playlist.sequencer.lock().unwrap();
    for (path, reason) in sequencer.restore(saved) {
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
      Ok(id) => pattern_clips.push(LoadedPatternClip {
        id,
        pattern_id: clip.pattern_id,
        track_number: clip.track_number,
        start_beat: start_beat as f32,
      }),
      Err(e) => println!("left out a clip of pattern {}: {}", clip.pattern_id, e),
    }
  }

  let mixer = state.playlist.mixer.settings(audiograph.track_count());

  LoadedProject { settings, clips, missing, mixer, sequencer, pattern_clips }
}

// write the current playlist to a project file. the file is written next to
// it first and then moved into place, so a failed save never leaves a
// half-written project
pub fn save_to(state: &daw::InnerState, project_path: &Path) -> Result<(), String> {
  let dir = project_dir(project_path)?;
  let project = capture(state, Some(&dir));
  let json = serde_json::to_string_pretty(&project)
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

// replace the playlist with a project file. nothing changes if the file
// can't be read or isn't a project
pub fn load_from(state: &daw::InnerState, project_path: &Path) -> Result<LoadedProject, String> {
  let json = fs::read_to_string(project_path)
    .map_err(|e| format!("could not open '{}': {}", project_path.display(), e))?;
  let project = Project::from_json(&json)?;
  let dir = project_dir(project_path)?;

  Ok(apply(state, &project, Some(&dir)))
}

#[tauri::command]
pub fn save_project(state: daw::TState<'_>, path: String) -> Result<(), String> {
  save_to(&state, Path::new(&path))
}

#[tauri::command]
pub fn load_project(state: daw::TState<'_>, path: String) -> Result<LoadedProject, String> {
  load_from(&state, Path::new(&path))
}

// start an empty project with the default settings
#[tauri::command]
pub fn new_project(state: daw::TState<'_>) -> LoadedProject {
  apply(&state, &Project::default(), None)
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
    assert_eq!(saved.version, PROJECT_VERSION);
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
}
