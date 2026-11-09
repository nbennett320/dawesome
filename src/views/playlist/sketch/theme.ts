// the playlist's colours, as [r, g, b]. they match the four-track controls
// (components/fourtrack/fourtrack.css): Tailwind slate, with sky accents

type RGB = [number, number, number]

export const theme = {
  font: 'system-ui',

  // the canvas, and every other track lane
  background: [255, 255, 255] as RGB,
  laneAlt: [248, 250, 252] as RGB, // slate-50
  // between tracks
  laneLine: [226, 232, 240] as RGB, // slate-200

  // the grid: a line per beat, darker on each bar
  beatLine: [226, 232, 240] as RGB, // slate-200
  barLine: [203, 213, 225] as RGB, // slate-300
  beatsPerBar: 4,

  // the ruler along the top
  ruler: [241, 245, 249] as RGB, // slate-100
  rulerLine: [203, 213, 225] as RGB, // slate-300
  rulerText: [148, 163, 184] as RGB, // slate-400
  rulerBarText: [71, 85, 105] as RGB, // slate-600

  // the playhead, while playing and while stopped
  playhead: [14, 165, 233] as RGB, // sky-500
  playheadStopped: [148, 163, 184] as RGB, // slate-400

  // clips
  clip: [255, 255, 255] as RGB,
  clipBorder: [203, 213, 225] as RGB, // slate-300
  clipLabel: [241, 245, 249] as RGB, // slate-100
  clipText: [51, 65, 85] as RGB, // slate-700
  clipWave: [100, 116, 139] as RGB, // slate-500
  // the dot on a clip whose settings were changed, like an amber LED
  modified: [245, 158, 11] as RGB, // amber-500

  // drops, the new-track area
  accent: [14, 165, 233] as RGB, // sky-500
  accentSoft: [125, 211, 252] as RGB, // sky-300
  idle: [148, 163, 184] as RGB, // slate-400

  // the scrollbar's thumb
  scrollbar: [100, 116, 139] as RGB, // slate-500
}
