import { clamp } from '../util/helpers'

// how a clip in the playlist is played back, adjusted in the Sample tab
//
// pitch and speed are linked, like a tape or record: raising the pitch
// plays the clip faster and shorter. (independent pitch and speed would
// need time-stretching, which can come later)
export interface ClipSettings {
  // volume change in decibels, 0 is unchanged
  gainDb: number
  // stereo position, -1 is hard left, 0 is center, 1 is hard right
  pan: number
  // semitones up or down, which also changes the speed
  pitch: number
  // fade in from silence over this many milliseconds at the clip's start
  attackMs: number
  // fade out to silence over this many milliseconds at the clip's end
  decayMs: number
}

export type ClipSetting = keyof ClipSettings

// the part of its source a clip plays: seconds of the sample for a sample
// clip, steps of the pattern for a pattern clip (mirrors ClipWindow in
// engine/src/event_dispatch.rs)
export interface ClipWindow {
  start: number
  // null: to the source's end
  end: number | null
}

// a clip's settings and lengths, from the engine's get_clip and
// set_clip_settings (mirrors ClipInfo in engine/src/event_dispatch.rs)
export interface ClipInfo {
  settings: ClipSettings
  // seconds, before the settings change the clip's speed
  sampleDuration: number
  // seconds the clip plays for
  playedDuration: number
  // the part of the sample it plays, null for all of it
  window: ClipWindow | null
}

// what the playlist shows differently for a clip whose settings changed
export interface ClipOverride {
  // seconds the clip plays for, once its pitch changes its speed
  // (unknown until the clip's sample length has loaded)
  playedDuration?: number
  // whether any setting differs from the default
  modified: boolean
  // the sample's length in seconds, before any change of speed (unknown
  // until it has loaded), and how much faster than that the clip plays:
  // what the playlist needs to trim the clip, and to draw it trimmed
  sampleDuration?: number
  rate?: number
  // how much louder it plays (its gain, as a multiplier), which its waveform
  // in the playlist is drawn at
  gain?: number
}

export const defaultClipSettings: ClipSettings = {
  gainDb: 0,
  pan: 0,
  pitch: 0,
  attackMs: 0,
  decayMs: 0,
}

// the range of each setting. attack and decay are also limited to the
// clip's length, which depends on the clip (see clampClipSettings)
export const clipSettingRanges: Record<ClipSetting, { min: number, max: number }> = {
  gainDb: { min: -60, max: 24 },
  pan: { min: -1, max: 1 },
  pitch: { min: -24, max: 24 },
  attackMs: { min: 0, max: 10_000 },
  decayMs: { min: 0, max: 10_000 },
}


// how much faster than the original the clip plays: 12 semitones up is
// twice as fast, 12 down is half as fast
export const playbackRate = (settings: ClipSettings): number => 2 ** (settings.pitch / 12)

// the clip's length in seconds once it's sped up or slowed down
export const playedDuration = (settings: ClipSettings, sampleDuration: number): number =>
  sampleDuration / playbackRate(settings)

// the seconds a clip plays for once it's trimmed (window, in seconds of the
// sample) and sped up or slowed down
export const windowedDuration = (
  settings: ClipSettings,
  sampleDuration: number,
  window: ClipWindow | null | undefined,
): number =>
  Math.max(0, ((window?.end ?? sampleDuration) - (window?.start ?? 0)) / playbackRate(settings))

// the volume multiplier for a gain in decibels
export const gainFactor = (gainDb: number): number => 10 ** (gainDb / 20)

// keep settings in range. with the clip's played length (seconds), attack
// and decay are kept within it too
export const clampClipSettings = (
  settings: ClipSettings,
  playedSeconds?: number,
): ClipSettings => {
  const clamped = { ...settings }

  ;(Object.keys(clipSettingRanges) as ClipSetting[]).forEach(key => {
    const { min, max } = clipSettingRanges[key]
    const value = Number.isFinite(clamped[key]) ? clamped[key] : defaultClipSettings[key]
    clamped[key] = clamp(value, min, max)
  })

  if(playedSeconds !== undefined) {
    const lengthMs = playedSeconds * 1000
    clamped.attackMs = Math.min(clamped.attackMs, lengthMs)
    clamped.decayMs = Math.min(clamped.decayMs, lengthMs)
  }

  return clamped
}

// the attack/decay envelope's level (0 to 1) at `t` seconds into a clip
// that plays for `playedSeconds`. where the fades overlap, the quieter wins
export const envelopeAt = (
  settings: ClipSettings,
  t: number,
  playedSeconds: number,
): number => {
  const attack = settings.attackMs / 1000
  const decay = settings.decayMs / 1000
  const fadeIn = attack > 0 ? t / attack : 1
  const fadeOut = decay > 0 ? (playedSeconds - t) / decay : 1

  return clamp(Math.min(fadeIn, fadeOut), 0, 1)
}

export const isDefaultClipSettings = (settings: ClipSettings): boolean =>
  (Object.keys(defaultClipSettings) as ClipSetting[])
    .every(key => settings[key] === defaultClipSettings[key])
