// which beat lines and ruler numbers the playlist draws at a zoom level, and
// which beats are on screen. kept free of p5 so it can be tested on its own
import { MAX_PIXELS_PER_BEAT, adaptiveStep } from '../../../editing/beatGrid'

// how many beats the playlist shows across its width when it opens
export const BEATS_IN_VIEW = 16

// ruler numbers closer than this (pixels) would overlap
export const MIN_LABEL_GAP = 28

export interface GridSteps {
  // a line every this many beats: the grid's step (see adaptiveStep), which
  // can be a part of a beat
  lineEvery: number
  // a number every this many beats: whole bars, or every beat once the grid
  // splits beats into 1/8 notes or finer (see rulerLabel)
  labelEvery: number
}

// the grid step (in beats) at or under which the ruler numbers beats too:
// a 1/8 note. beats are then at least GRID_GAP × √2 ≈ 28px apart, room for
// "12.3"
export const BEAT_LABELS_FROM = 1 / 2

// the smallest step (a bar, then 2, 4, 8... bars) that's at least `minGap`
// pixels wide
const barsFor = (pixelsPerBeat: number, beatsPerBar: number, minGap: number): number => {
  const bar = Math.max(1, Math.round(beatsPerBar))
  if(!(pixelsPerBeat > 0)) return bar

  let step = bar
  while(step * pixelsPerBeat < minGap) step *= 2
  return step
}

// lines follow the zoom: zoomed in they split beats into 1/8 and 1/16
// notes (zoom stops there, see maxScale), zoomed out they thin to every bar,
// 2 bars, 4... staying about GRID_GAP pixels apart. numbers count bars, one
// on every bar thinning to every 2, 4, 8... so they never overlap, and on
// every beat once the grid is down to 1/8 notes
// - width: the grid made narrower or wider (see adaptiveStep)
export const gridSteps = (pixelsPerBeat: number, beatsPerBar: number, width = 0): GridSteps => {
  const lineEvery = adaptiveStep(pixelsPerBeat, beatsPerBar, width)
  if(lineEvery <= BEAT_LABELS_FROM) return { lineEvery, labelEvery: 1 }
  // (a number goes on a line: never more often than the lines)
  const labelEvery = Math.max(barsFor(pixelsPerBeat, beatsPerBar, MIN_LABEL_GAP), lineEvery)
  return { lineEvery, labelEvery }
}

// what the ruler says at a whole beat (counted from 0): the bar's number on
// a bar ("3"), and bar.beat between them ("3.2" for bar 3's second beat),
// both counted from 1
export const rulerLabel = (beat: number, beatsPerBar: number): string => {
  const bar = Math.max(1, Math.round(beatsPerBar))
  const barNumber = Math.floor(beat / bar) + 1
  const inBar = beat - (barNumber - 1) * bar
  return inBar === 0 ? String(barNumber) : `${barNumber}.${inBar + 1}`
}

// the beats on screen, as [first, last): whole beats, with one to spare
// either side, kept within the playlist
// - transformX: how far the playlist is panned, in pixels (0 or less)
// - leftMargin: pixels off the left edge to include too, for what's drawn to
//   the right of its beat (a ruler number)
export const visibleBeats = (
  pixelsPerBeat: number,
  transformX: number,
  canvasWidth: number,
  totalBeats: number,
  leftMargin = 0,
): [number, number] => {
  if(!(pixelsPerBeat > 0)) return [0, 0]

  const first = Math.floor((-transformX - leftMargin) / pixelsPerBeat) - 1
  const last = Math.ceil((canvasWidth - transformX) / pixelsPerBeat) + 1
  return [Math.max(0, first), Math.min(totalBeats, last)]
}

// the pan that keeps the same beat at the left edge when the canvas goes
// from `oldWidth` to `newWidth` wide. zoom is relative to the width, so a
// pan left as it was would slide the timeline whenever a pane is resized
export const panForWidth = (transformX: number, oldWidth: number, newWidth: number): number =>
  (oldWidth > 0 && newWidth > 0 ? transformX * (newWidth / oldWidth) : transformX)

// the most the playlist can zoom in: until a beat is MAX_PIXELS_PER_BEAT
// wide, so the grid never gets finer than a 1/16 note. zoom is relative to
// the width, so a wider canvas reaches it sooner. never under 1, the whole
// playlist
export const maxScale = (canvasWidth: number, totalBeats: number): number =>
  (canvasWidth > 0 ? Math.max(1, MAX_PIXELS_PER_BEAT * totalBeats / canvasWidth) : Infinity)

// the zoom the playlist opens at: `BEATS_IN_VIEW` beats across its width.
// (a zoom of 1 shows the whole playlist)
export const openingScale = (totalBeats: number): number =>
  Math.max(1, totalBeats / BEATS_IN_VIEW)
