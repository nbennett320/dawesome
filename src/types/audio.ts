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
  notice: string | null
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
