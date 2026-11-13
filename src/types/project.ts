import { ArmedTrack } from './audio'
import { ClipInfo, ClipWindow } from './clip'
import { Racks } from './effects'
import { MixerSettings } from './mixer'
import { Row, SequencerState, SynthInstrument, isSynthRow } from './sequencer'
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
  // where it starts and ends in its pattern, in steps: its own length, which
  // can be past the pattern's end (the clip plays the pattern again from
  // there). the end is never null
  window: ClipWindow
}

// clips as they are after a trim or a cut (see engine/src/commands/clip.rs)
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

// an effect or synth whose plugin couldn't be loaded. it stays as a
// placeholder with its settings kept. track is null for the master (and,
// presumably, for a synth, which isn't on a track)
export interface MissingEffect {
  track: number | null
  name: string
  vendor: string
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
  // each channel's effects (left out by older engines: none)
  effects?: Racks
  // plugins that couldn't be loaded, effects and synths (left out by older
  // engines: none)
  missingEffects?: MissingEffect[]
  // what was left out because it was saved in a way that's no longer read
  // (an instrument in a rack, notes in a drum pattern), as sentences to show
  // (left out by older engines: nothing)
  dropped?: string[]
}

// the engine lists missing synths with the missing effects. the synth rows
// say which they are (`missing`), so each takes the first entry with its
// name (or its plugin's name, which a numbered synth's starts with) and
// vendor, and the rest are effects
export const splitMissing = (
  missingEffects: MissingEffect[],
  rows: Row[],
): { effects: MissingEffect[], synths: SynthInstrument[] } => {
  const effects = [...missingEffects]
  const missing = rows.filter(isSynthRow).map(row => row.instrument).filter(synth => synth.missing)
  missing.forEach(synth => {
    const i = effects.findIndex(effect => effect.vendor === synth.vendor
      && (effect.name === synth.name || synth.name.startsWith(`${effect.name} `)))
    if(i >= 0) effects.splice(i, 1)
  })
  return { effects, synths: missing }
}

// the clips recorded since the engine was last asked (see collect_recording
// in engine/src/commands/audio.rs)
export interface Recorded {
  recording: boolean
  clips: LoadedClip[]
  problems: string[]
}

// the project as it is now, after an undo or redo (see project_view in
// engine/src/commands/history.rs)
export interface ProjectView {
  tempo: number
  clips: LoadedClip[]
  patternClips: LoadedPatternClip[]
  mixer: MixerSettings
  sequencer: SequencerState
  // each channel's effects (left out by older engines: none)
  effects?: Racks
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
