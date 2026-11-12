import { painter } from '../../../theme/canvas'

// the playlist's colours come from the theme (engine/themes/base.json,
// 'playlist' section); draw with paint.fill(p, 'clip') and the like
export const paint = painter('playlist')
// the tracks' colors, by their key in the theme's `track` palette (see
// trackColorKey in types/mixer.ts)
export const trackPaint = painter('track')

export const theme = {
  font: 'system-ui',

  // the grid: a line per beat, darker on each bar
  beatsPerBar: 4,
}
