import { painter } from '../../../theme/canvas'
import { RGBA, mixRgba } from '../../../theme/color'

// the playlist's colours come from the theme (engine/themes/base.json,
// 'playlist' section); draw with paint.fill(p, 'clip') and the like
export const paint = painter('playlist')
// the tracks' colors, by their key in the theme's `track` palette (see
// trackColorKey in types/mixer.ts)
export const trackPaint = painter('track')

export const theme = {
  font: 'system-ui',
}

// a pattern clip's colors in its track's color, worked out as the theme's
// amber pattern colors are from its LED colors (engine/themes/base.json),
// with the track's color as the LED: a pale body, and a stronger label,
// edge and hits, and a line where a clip longer than its pattern starts it
// over. `clip` is a sample
// clip's body and `ink` its text, which the track's color is mixed with
export const patternColors = (track: Readonly<RGBA>, clip: Readonly<RGBA>, ink: Readonly<RGBA>) => {
  // ledTint, ledLine and ledInk
  const tint = mixRgba(track, clip, 0.1)
  const line = mixRgba(track, tint, 0.55)
  const dark = mixRgba(track, ink, 0.6)
  return {
    card: tint,
    label: mixRgba(track, tint, 0.15),
    border: mixRgba(track, line, 0.5),
    hit: mixRgba(track, dark, 0.75),
    repeat: line,
  }
}
