import { ClipInfo } from './clip'
import { MixerSettings } from './mixer'
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

// a clip that couldn't be loaded, so it was left out of the playlist
export interface MissingSample {
  path: string
  reason: string
}

// what the engine's load_project and new_project return
export interface LoadedProject extends ProjectSettings {
  clips: LoadedClip[]
  missing: MissingSample[]
  // every track's strip, up to the playlist's track count
  mixer: MixerSettings
}
