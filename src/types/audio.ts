// the engine's audio devices and buffer size (see engine/src/audio_commands.rs)

export interface AudioSettings {
  // device names, or null for the system's default
  outputDevice: string | null
  inputDevice: string | null
  // frames per buffer, or null for the device's default
  bufferFrames: number | null
  // how much earlier recordings are placed than the devices' reported latency
  // says, for devices that don't report all of it (negative is later)
  recordOffsetMs: number
}

// the output that's playing
export interface OutputStatus {
  device: string
  sampleRate: number
  // what the device actually asks for, which can be more than was set
  bufferFrames: number
  // from rendering a frame to hearing it, as the device reports it
  latencyMs: number
}

export interface AudioStatus {
  settings: AudioSettings
  outputs: string[]
  inputs: string[]
  // the buffer sizes the chosen output takes
  bufferChoices: number[]
  // null when the output can't be opened
  output: OutputStatus | null
  // null unless something's listening to the input
  input: InputStatus | null
  notice: string | null
}

// the input, while it's open (for the meter, or recording)
export interface InputStatus {
  device: string
  channels: number
  // from a frame coming into the device to the engine getting it
  latencyMs: number
}

// the input channels a track records: one, or a pair from `first`
// (counted from 0)
export interface InputChoice {
  first: number
  stereo: boolean
}

export interface ArmedTrack {
  trackNumber: number
  input: InputChoice
}

// see get_record_setup in engine/src/audio_commands.rs
export interface RecordSetup {
  armed: ArmedTrack[]
  // null when there's no input to record from
  inputChannels: number | null
  inputError: string | null
}

// the first two channels, or the only one
export const defaultInput = (channels: number | null): InputChoice =>
  ({ first: 0, stereo: (channels ?? 2) >= 2 })

export const inputLabel = ({ first, stereo }: InputChoice): string =>
  (stereo ? `In ${first + 1}+${first + 2}` : `In ${first + 1}`)

// every channel on its own, then each pair (1+2, 3+4, …)
export const inputChoices = (channels: number): InputChoice[] => [
  ...Array.from({ length: channels }, (_, first) => ({ first, stereo: false })),
  ...Array.from({ length: Math.floor(channels / 2) }, (_, pair) => ({ first: pair * 2, stereo: true })),
]

// a choice as a select's value, and back
export const inputKey = ({ first, stereo }: InputChoice): string => `${first}${stereo ? 's' : 'm'}`
export const parseInputKey = (key: string): InputChoice | null => {
  const match = /^(\d+)([sm])$/.exec(key)
  return match ? { first: Number(match[1]), stereo: match[2] === 's' } : null
}

// where a level (0 to 1, where 1 is the loudest a sample can be) sits on a
// meter showing the top 60 dB
export const meterPosition = (level: number): number => {
  if(!(level > 0)) return 0
  const db = 20 * Math.log10(level)
  return Math.min(1, Math.max(0, (db + 60) / 60))
}

// how long a buffer of `frames` lasts
export const bufferMs = (frames: number, sampleRate: number): number =>
  (sampleRate > 0 ? frames / sampleRate * 1000 : 0)

export const msLabel = (ms: number): string =>
  `${ms < 10 ? ms.toFixed(1) : Math.round(ms)} ms`

export const rateLabel = (sampleRate: number): string =>
  `${Number((sampleRate / 1000).toFixed(1))} kHz`

// why the device's buffer isn't the one that was set, if it isn't
export const bufferNote = (settings: AudioSettings, output: OutputStatus | null): string | null => {
  const asked = settings.bufferFrames
  if(asked === null || !output || output.bufferFrames <= 0 || output.bufferFrames === asked) return null

  return `Set to ${asked}, but ${output.device} won't go lower than ${output.bufferFrames}`
}
