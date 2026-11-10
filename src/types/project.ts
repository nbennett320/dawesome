import { ArmedTrack } from './audio'
import { ClipInfo, ClipWindow } from './clip'
import { MixerSettings } from './mixer'
import { SequencerState } from './sequencer'
import type { SnapNote } from '../state/slices/playlistSlice'

// project files (.dawesome) hold the playlist's clips and settings, and the
// mixer. the
// engine reads and writes them (see engine/src/project.rs, which these types
// mirror)

export const PROJECT_EXTENSION = 'dawesome'

export interface TimeSignature {
  numerator: number
  denominator: number
}

// the playlist's settings, as saved in a project
export interface ProjectSettings {
  tempo: number
  timeSignature: TimeSignature
  metronomeEnabled: boolean
  loopEnabled: boolean
  snapEnabled: boolean
  snapNote: SnapNote
  // whether recording counts in a bar first (left out by older engines)
  countIn?: boolean
}

// a clip after loading, with the new id the engine gave it
export interface LoadedClip extends ClipInfo {
  id: number
  // where the sample was found
  path: string
  trackNumber: number
  // beats from the start of the playlist
  startBeat: number
}

// a pattern placed in the playlist, after loading, with its new id
export interface LoadedPatternClip {
  id: number
  patternId: number
  trackNumber: number
  startBeat: number
  // the part of the pattern it plays, in steps. null for all of it
  window: ClipWindow | null
}

// clips as they are after a trim or a cut (see engine/src/clip_commands.rs)
export interface ChangedClips {
  clips: LoadedClip[]
  patternClips: LoadedPatternClip[]
}

// clips a paste or a repeat made, and how many of the clipboard's were
// skipped because their pattern no longer exists
export interface Placed extends ChangedClips {
  leftOut: number
}

// what a join made: the joined clip is the one in `clips` or `patternClips`,
// `removed` are the other clips, which are gone, and `sequencer` is set when
// joining pattern clips made a new pattern
export interface Joined extends ChangedClips {
  removed: number[]
  sequencer: SequencerState | null
}

// a clip that couldn't be loaded, so it was left out of the playlist
export interface MissingSample {
  path: string
  reason: string
}

// what the engine's load_project and new_project return
// a clip whose take was brought into the project's folder, and where it is now
export interface MovedClip {
  id: number
  path: string
}

// a project that's been saved (see save_project in engine/src/project.rs)
export interface SavedProject {
  // the file it was saved to: saved somewhere new, it's in a folder of its own
  path: string
  movedClips: MovedClip[]
  // the Sequencer, when a row's take was brought in too
  sequencer: SequencerState | null
  // takes that couldn't be brought in, which are still used from where they are
  problems: string[]
}

export interface LoadedProject extends ProjectSettings {
  clips: LoadedClip[]
  missing: MissingSample[]
  // every track's strip, up to the playlist's track count
  mixer: MixerSettings
  // the Sequencer tab's rows and patterns
  sequencer: SequencerState
  patternClips: LoadedPatternClip[]
  // the tracks armed to record (left out by older engines)
  armed?: ArmedTrack[]
}

// the clips recorded since the engine was last asked (see collect_recording
// in engine/src/audio_commands.rs)
export interface Recorded {
  recording: boolean
  clips: LoadedClip[]
  problems: string[]
}

// the project as it is now, after an undo or redo (see project_view in
// engine/src/history_commands.rs)
export interface ProjectView {
  tempo: number
  clips: LoadedClip[]
  patternClips: LoadedPatternClip[]
  mixer: MixerSettings
  sequencer: SequencerState
}

// what the engine's undo and redo return
export interface Travelled {
  // false when there was nothing to undo or redo, or it can't be done just
  // now (while recording): nothing changed
  changed: boolean
  // the clips it changed or put back, which the playlist selects. null when
  // it wasn't about clips (a fader, the tempo): the selection stays
  selected: number[] | null
  canUndo: boolean
  canRedo: boolean
  project: ProjectView
}
