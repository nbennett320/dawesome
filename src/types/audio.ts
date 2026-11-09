// the engine's audio devices and buffer size (see engine/src/audio_commands.rs)

export interface AudioSettings {
  // device names, or null for the system's default
  outputDevice: string | null
  inputDevice: string | null
  // frames per buffer, or null for the device's default
  bufferFrames: number | null
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

// a take that's been recorded
export interface TakeInfo {
  path: string
  seconds: number
  channels: number
  // samples lost because the disk couldn't keep up
  dropped: number
}

export interface RecordingStatus {
  recording: boolean
  // the take just finished, when recording stopped
  take: TakeInfo | null
}

// what to say about a take that's just been recorded
export const takeSummary = (take: TakeInfo): string => {
  const file = take.path.split(/[\\/]/).pop() ?? take.path
  const kind = take.channels === 1 ? 'mono' : 'stereo'
  const recorded = `Recorded ${file} (${take.seconds.toFixed(1)}s, ${kind})`
  return take.dropped > 0
    ? `${recorded}, but some audio was lost because the disk couldn't keep up`
    : recorded
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
