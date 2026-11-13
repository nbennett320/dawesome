// the mixer: a channel strip per playlist track, summed into the master.
// mirrors engine/src/daw/daw_core/mixer.rs

// the quietest a fader goes. at or below this it's silent (shown as -∞)
export const MIN_DB = -60
// the loudest a fader goes
export const MAX_DB = 6

export interface ChannelSettings {
  // fader position in decibels, MIN_DB (silent) to MAX_DB
  volumeDb: number
  // -1 is hard left, 0 is center, 1 is hard right
  pan: number
  mute: boolean
  solo: boolean
}

export const defaultChannelSettings: ChannelSettings = {
  volumeDb: 0,
  pan: 0,
  mute: false,
  solo: false,
}

export interface TrackMix extends ChannelSettings {
  trackNumber: number
  // the name it was given, if any
  name?: string
  // the color it was given, if any (see TRACK_COLORS)
  color?: number
}

// what the engine's get_mixer returns (and load_project, as `mixer`)
export interface MixerSettings {
  tracks: TrackMix[]
  master: ChannelSettings
}

// each strip's loudest sample since the last reading, 0 to 1 (above 1 clips)
export interface MixerLevels {
  // [track number, peak]
  tracks: Array<[number, number]>
  master: number
}

// faders and meters share a scale that gives more room near 0 dB, where
// mixing happens, than to the quiet end. [dB, position from the bottom]
const SCALE: Array<[number, number]> = [
  [MIN_DB, 0],
  [-40, 0.15],
  [-20, 0.4],
  [-10, 0.6],
  [0, 0.8],
  [MAX_DB, 1],
]

// the dB values marked beside a fader
export const SCALE_MARKS = [6, 0, -10, -20, -40]

// where a level in dB sits on a fader, 0 (bottom) to 1 (top)
export const dbToPosition = (db: number): number => {
  if(!Number.isFinite(db) || db <= MIN_DB) return 0
  if(db >= MAX_DB) return 1

  const i = SCALE.findIndex(([d]) => d >= db)
  const [d0, p0] = SCALE[i - 1]
  const [d1, p1] = SCALE[i]
  return p0 + (db - d0) / (d1 - d0) * (p1 - p0)
}

// the level in dB at a position on a fader, 0 (bottom) to 1 (top)
export const positionToDb = (position: number): number => {
  if(!(position > 0)) return MIN_DB
  if(position >= 1) return MAX_DB

  const i = SCALE.findIndex(([, p]) => p >= position)
  const [d0, p0] = SCALE[i - 1]
  const [d1, p1] = SCALE[i]
  return d0 + (position - p0) / (p1 - p0) * (d1 - d0)
}

// a meter reading (0 to 1, as a multiplier) in dB
export const gainToDb = (gain: number): number =>
  (gain > 0 ? 20 * Math.log10(gain) : -Infinity)

// e.g. "-6.0", "+3.5", "-∞"
export const formatDb = (db: number): string => {
  if(!Number.isFinite(db) || db <= MIN_DB) return '-∞'
  const rounded = Math.round(db * 10) / 10
  return `${rounded > 0 ? '+' : ''}${rounded.toFixed(1)}`
}

// e.g. "C", "L 50", "R 100"
export const formatPan = (pan: number): string => {
  const percent = Math.round(Math.abs(pan) * 100)
  if(percent === 0) return 'C'
  return `${pan < 0 ? 'L' : 'R'} ${percent}`
}

// the longest a track's name can be
export const MAX_NAME_CHARS = 40

// what an unnamed track is called. tracks are numbered from 0 in the playlist
export const trackLabel = (trackNumber: number) => `Track ${trackNumber + 1}`

// the colors a track can be, as the engine stores them: a track's color is
// its place in this list, and each is a key of the theme's `track` palette
// (engine/themes/base.json). mirrors TRACK_COLORS in mixer.rs
export const TRACK_COLORS = ['red', 'orange', 'yellow', 'green', 'teal', 'blue', 'violet', 'pink'] as const
export type TrackColor = typeof TRACK_COLORS[number]

// the color a track is until it's given one: the palette in turn, so
// neighbouring tracks differ (default_track_color in mixer.rs)
export const defaultTrackColor = (trackNumber: number): number => trackNumber % TRACK_COLORS.length

// a track's color as a theme key. anything out of range wraps into the palette
export const trackColorKey = (color: number): TrackColor =>
  TRACK_COLORS[((Math.trunc(color) % TRACK_COLORS.length) + TRACK_COLORS.length) % TRACK_COLORS.length]
