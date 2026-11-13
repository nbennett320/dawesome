import { STEPS_PER_BEAT } from '../../types/sequencer'

// where things are in the step grid, in px, worked out rather than measured:
// the rows render only the bars in view (see useVisibleBars), and the
// playing step's column is placed over them (see PlayingStep). steps.scss
// draws keys this size and this far apart

// a key, and the gap to the next in its beat
export const KEY_W = 22
export const KEY_GAP = 3
// the gap between beats, and between bars
export const BEAT_GAP = 6
export const BAR_GAP = 12
// where the grid starts in a row: after its controls (RowControls, w-80)
// and the row's gap
export const GRID_LEFT = 320 + 8

const KEY_PITCH = KEY_W + KEY_GAP
const BEAT_W = STEPS_PER_BEAT * KEY_W + (STEPS_PER_BEAT - 1) * KEY_GAP

// where a beat starts
export const beatX = (beat: number, beatsPerBar: number): number =>
  beat * (BEAT_W + BEAT_GAP) + Math.floor(beat / beatsPerBar) * (BAR_GAP - BEAT_GAP)

// where a bar starts
export const barX = (bar: number, beatsPerBar: number): number => beatX(bar * beatsPerBar, beatsPerBar)

// where a step's key starts
export const stepX = (step: number, beatsPerBar: number): number =>
  beatX(Math.floor(step / STEPS_PER_BEAT), beatsPerBar) + (step % STEPS_PER_BEAT) * KEY_PITCH

// how wide the keys of `steps` steps are, from the first's left to the
// last's right
export const gridWidth = (steps: number, beatsPerBar: number): number =>
  (steps > 0 ? stepX(steps - 1, beatsPerBar) + KEY_W : 0)

// where a time in steps (a note's start or end) is, across the keys: a step
// is its key's width, so a note fills the keys it plays over
export const timeX = (time: number, beatsPerBar: number, end = false): number => {
  const step = end ? Math.ceil(time) - 1 : Math.floor(time)
  return stepX(step, beatsPerBar) + (time - step) * KEY_W
}

// the bars to render, `from` up to (not including) `to`: those with any
// part in the view (`left` and `width` across the grid, in px), and one
// more either side, of `bars`
export const visibleBars = (
  left: number, width: number, beatsPerBar: number, bars: number,
): { from: number, to: number } => {
  const pitch = barX(1, beatsPerBar)
  return {
    from: Math.max(0, Math.floor(left / pitch) - 1),
    to: Math.min(bars, Math.ceil((left + width) / pitch) + 1),
  }
}
