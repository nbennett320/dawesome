// the project's tape (T14, T20, T22 in context/tape-project): the switch,
// the machine's controls (the Transport tab), and which tracks are on tape,
// each with its drive (the Mixer tab). mirrors engine/src/daw/tape_settings.rs
//
// only the settings so far: nothing changes the sound yet. how tracks on
// tape are worked out (print or live, T23) is a preference (tapeMode)

// the machine's controls
export interface TapeMachine {
  // the project switch: off, the engine plays as it would without tape
  enabled: boolean
  // its place in MACHINES (what's saved)
  machine: number
  // how worn the tape is, 0 (fresh) to 100 %
  age: number
  // wow and flutter, 100 % is the real deck's
  wobble: number
  // hiss, 100 % is the real deck's
  hiss: number
  // the deck's noise reduction
  dbx: boolean
  outputDb: number
  // the PITCH knob: how much faster (below 0, slower) the whole song plays,
  // in %, pitch and tempo together, as a tape deck's varispeed. only with
  // the tape on
  pitch: number
}

// a track's place on the tape
export interface TrackTape {
  trackNumber: number
  // whether its audio goes through the tape before its mixer channel
  onTape: boolean
  // its record level onto the tape, like a Portastudio's input trim
  driveDb: number
}

// what the engine's get_tape returns (and load_project, as `tape`): only
// tracks changed from their defaults are listed
export interface TapeSettings extends TapeMachine {
  tracks?: TrackTape[]
}

// the machines, by their place here. the second deck's real name ships
// after the machine check (step 0, see context/tape-project/04-recording.md)
export const MACHINES = ['Porta 414', 'Deck 2'] as const

export const defaultTapeMachine: TapeMachine = {
  enabled: false,
  machine: 0,
  age: 0,
  wobble: 100,
  hiss: 100,
  dbx: true,
  outputDb: 0,
  pitch: 0,
}

export const DEFAULT_DRIVE_DB = 0

// each knob's range and default
export const AGE = { min: 0, max: 100, default: 0 }
export const WOBBLE = { min: 0, max: 200, default: 100 }
export const HISS = { min: 0, max: 200, default: 100 }
export const OUTPUT_DB = { min: -24, max: 12, default: 0 }
export const DRIVE_DB = { min: -12, max: 18, default: DEFAULT_DRIVE_DB }
// ±12 % with a click at 0, like a Portastudio's PITCH knob (T27)
export const PITCH = { min: -12, max: 12, default: 0 }
// how near 0 a drag has to come for the knob to click into place there
export const PITCH_DETENT = 0.5

// e.g. "40%"
export const formatPercent = (percent: number): string => `${Math.round(percent)}%`

// e.g. "+3.5%", "0%" (at its click), "-12.0%"
export const formatPitch = (percent: number): string => {
  const rounded = Math.round(percent * 10) / 10
  if(rounded === 0) return '0%'
  return `${rounded > 0 ? '+' : ''}${rounded.toFixed(1)}%`
}

// e.g. "+3.0 dB", "0.0 dB", "-6.5 dB"
export const formatGainDb = (db: number): string => {
  const rounded = Math.round(db * 10) / 10
  return `${rounded > 0 ? '+' : ''}${rounded.toFixed(1)} dB`
}
