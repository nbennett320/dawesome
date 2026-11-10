// which beat lines and ruler numbers the playlist draws at a zoom level, and
// which beats are on screen. kept free of p5 so it can be tested on its own

// how many beats the playlist shows across its width when it opens
export const BEATS_IN_VIEW = 16

// lines closer than this (pixels) would smear together
export const MIN_LINE_GAP = 8
// ruler numbers closer than this (pixels) would overlap
export const MIN_LABEL_GAP = 28

export interface GridSteps {
  // a line every this many beats
  lineEvery: number
  // a number every this many beats
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

// zoomed in, there's a line and a number for every beat. zoomed out, beat
// lines go first (bar lines always stay), and numbers thin out to every
// bar, then every 2, 4, 8... so they never overlap
export const gridSteps = (pixelsPerBeat: number, beatsPerBar: number): GridSteps => ({
  lineEvery: Math.min(stepFor(pixelsPerBeat, beatsPerBar, MIN_LINE_GAP), Math.max(1, beatsPerBar)),
  labelEvery: stepFor(pixelsPerBeat, beatsPerBar, MIN_LABEL_GAP),
})

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

// the zoom the playlist opens at: `BEATS_IN_VIEW` beats across its width.
// (a zoom of 1 shows the whole playlist)
export const openingScale = (totalBeats: number): number =>
  Math.max(1, totalBeats / BEATS_IN_VIEW)
