// which beat lines and ruler numbers the playlist draws at a zoom level, and
// which beats are on screen. kept free of p5 so it can be tested on its own
import { adaptiveStep } from '../../../editing/beatGrid'

// how many beats the playlist shows across its width when it opens
export const BEATS_IN_VIEW = 16

// ruler numbers closer than this (pixels) would overlap
export const MIN_LABEL_GAP = 28

export interface GridSteps {
  // a line every this many beats: the grid's step (see adaptiveStep), which
  // can be a part of a beat
  lineEvery: number
  // a number every this many beats, always whole beats
  labelEvery: number
}

// the smallest step (a beat, a bar, then 2, 4, 8... bars) that's at least
// `minGap` pixels wide
const stepFor = (pixelsPerBeat: number, beatsPerBar: number, minGap: number): number => {
  if(!(pixelsPerBeat > 0)) return beatsPerBar
  if(pixelsPerBeat >= minGap) return 1

  let step = Math.max(1, beatsPerBar)
  while(step * pixelsPerBeat < minGap) step *= 2
  return step
}

// lines follow the zoom: zoomed in they split beats into 1/8, 1/16, 1/32
// notes and finer, zoomed out they thin to every bar, 2 bars, 4... staying
// about GRID_GAP pixels apart. numbers go on whole beats only, thinning to
// every bar, then every 2, 4, 8... so they never overlap
// - width: the grid made narrower or wider (see adaptiveStep)
export const gridSteps = (pixelsPerBeat: number, beatsPerBar: number, width = 0): GridSteps => {
  const lineEvery = adaptiveStep(pixelsPerBeat, beatsPerBar, width)
  // (a number goes on a line: never more often than the lines)
  const labelEvery = Math.max(stepFor(pixelsPerBeat, beatsPerBar, MIN_LABEL_GAP), lineEvery)
  return { lineEvery, labelEvery }
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

// the zoom the playlist opens at: `BEATS_IN_VIEW` beats across its width.
// (a zoom of 1 shows the whole playlist)
export const openingScale = (totalBeats: number): number =>
  Math.max(1, totalBeats / BEATS_IN_VIEW)
