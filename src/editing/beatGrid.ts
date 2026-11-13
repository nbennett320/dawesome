// the grid along time that follows the zoom, the same in any view that
// shows beats: how far apart its lines are, how strong each line is, the
// bands shaded behind it, and what the step is called. plain functions,
// with no canvas. lengths are in beats, a beat being a quarter note

// how far apart grid lines are meant to be, in pixels. the grid takes the
// step nearest this, so lines end up between GRID_GAP / √2 and GRID_GAP × √2
// apart, and the step halves or doubles each time the zoom crosses √2 of it
export const GRID_GAP = 20
// lines closer than this (pixels) would smear together
export const MIN_LINE_GAP = 8

// the finest step: a 1/256 note
const FINEST = 1 / 64
// the coarsest step, in bars
const MOST_BARS = 512

// the steps a grid can have, finest first: a 1/256 note up to half a beat,
// a beat, the parts of a bar that divide it evenly (2 beats in 4/4), then
// 1, 2, 4 ... bars. in 3/4 a beat goes straight to a bar
export const gridStepOptions = (beatsPerBar: number): number[] => {
  const bar = Math.max(1, Math.round(beatsPerBar))
  const steps: number[] = []
  for(let s = FINEST; s < 1; s *= 2) steps.push(s)
  steps.push(1)
  for(let s = 2; s < bar; s *= 2) if(bar % s === 0) steps.push(s)
  for(let bars = 1; bars <= MOST_BARS; bars *= 2) steps.push(bars * bar)
  return steps
}

// the grid's step at this zoom: of the steps a grid can have, the one
// nearest GRID_GAP × 2^width pixels wide (nearest by ratio, not by
// difference), and never under MIN_LINE_GAP
// - width: makes the grid narrower (below 0) or wider (above 0), a doubling
//   each
export const adaptiveStep = (pixelsPerBeat: number, beatsPerBar: number, width = 0): number => {
  const steps = gridStepOptions(beatsPerBar)
  const coarsest = steps[steps.length - 1]
  if(!(pixelsPerBeat > 0) || !Number.isFinite(pixelsPerBeat)) return coarsest

  const target = GRID_GAP * 2 ** width / pixelsPerBeat
  const off = (step: number) => Math.abs(Math.log2(step / target))
  let best = coarsest
  for(const step of steps) {
    if(step * pixelsPerBeat < MIN_LINE_GAP) continue
    if(off(step) < off(best)) best = step
  }
  return best
}

// how strong a grid line is: 2 on a bar, 1 on a beat, 0 between beats
export type LineLevel = 0 | 1 | 2

export const lineLevel = (beat: number, beatsPerBar: number): LineLevel => {
  if(beat % Math.max(1, Math.round(beatsPerBar)) === 0) return 2
  return Number.isInteger(beat) ? 1 : 0
}

// how long the bands shaded behind the grid are, every other one darker:
// a few lines each, on something worth counting in. a beat while the step
// is under a beat (or 4 steps, when that's shorter), a bar while it's under
// a bar, then 4 steps
export const tileStep = (step: number, beatsPerBar: number): number => {
  const bar = Math.max(1, Math.round(beatsPerBar))
  if(step < 1) return Math.min(1, step * 4)
  if(step < bar) return bar
  return step * 4
}

// what the step is called: "1/16" (a sixteenth note), "1/4" (a beat),
// "1 Bar", "4 Bars"
export const stepLabel = (step: number, beatsPerBar: number): string => {
  const bar = Math.max(1, Math.round(beatsPerBar))
  if(step >= bar && step % bar === 0) {
    const bars = step / bar
    return bars === 1 ? '1 Bar' : `${bars} Bars`
  }
  // a whole note is 4 beats
  const per = 4 / step
  if(Number.isInteger(per)) return `1/${per}`
  // (a step that isn't a note's length, like 3 beats in 6/8)
  return `${step} beats`
}
