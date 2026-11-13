// a trim held back for the play cursor. a clear that empties a pattern's
// last bars trims them away (see trimmedLength), but not from under the
// play cursor: while the pattern plays on its own with the cursor in those
// bars, the clear is sent untrimmed and its trim is held here, to be sent
// once the cursor has left them (it went round to the start, or the pattern
// stopped). notes put back in them meanwhile call it off. the playlist
// playing holds nothing back
//
// a held trim is followed a frame at a time, outside React: following it
// renders nothing
import type { RootState } from './store'
import type { PatternRun } from './slices/sequencerSlice'
import { Pattern, stepAt, trimmedLength } from '../types/sequencer'
import { currentGesture, newGesture } from './gesture'

// the state a held trim follows
export type SequencerRoot = Pick<RootState, 'sequencer'>

// how an edit is sent: whether the engine trims after it, and the gesture
// it's part of
export interface EditSend {
  trim: boolean
  gesture: number | null
}

interface Held {
  patternId: number
  stepsPerBar: number
  getState: () => SequencerRoot
  // sends the trim
  send: () => void
  // the pattern as last seen, and the length it trims to (worked out again
  // only when the pattern changes, as that looks at every note)
  seen: Pattern
  to: number
}

let held: Held | null = null
let frame = 0

// whether a pattern playing on its own has its cursor at or past step `to`:
// in the bars a trim to `to` would take away
const cursorPast = (pattern: Pattern, to: number, run: PatternRun | null): boolean => {
  if(run?.patternId !== pattern.id) return false
  const step = stepAt(Date.now() - run.startTimeMs, pattern, run.beatIntervalMs)
  return step !== null && step >= to
}

// once a frame while a trim is held: sent once the cursor has left the bars
// it takes away, or dropped when there's nothing left to trim (notes went
// back in them, or the pattern went)
const follow = () => {
  frame = 0
  const waiting = held
  if(!waiting) return
  const { patterns, run } = waiting.getState().sequencer
  const pattern = patterns.find(p => p.id === waiting.patternId)
  if(pattern && pattern !== waiting.seen) {
    waiting.seen = pattern
    waiting.to = trimmedLength(pattern, waiting.stepsPerBar)
  }

  if(!pattern || waiting.to === pattern.length) {
    held = null
  } else if(cursorPast(pattern, waiting.to, run)) {
    frame = requestAnimationFrame(follow)
  } else {
    held = null
    waiting.send()
  }
}

// how to send a clear that leaves its pattern as `after`: with its trim,
// unless the pattern plays on its own with the cursor in the bars the trim
// would take away. then the clear goes untrimmed, and the trim is held
// until the cursor has left them, when `trim` sends it with the clear's
// gesture, so the two are undone as one (a clear made without the mouse is
// given a gesture for it). one trim is held at a time: one held for another
// pattern is sent first, and one for the same pattern is replaced
export const sendClear = (
  after: Pattern, stepsPerBar: number, getState: () => SequencerRoot, trim: (gesture: number) => void,
): EditSend => {
  const { run } = getState().sequencer
  // (only the pattern playing on its own holds a trim back)
  const to = run?.patternId === after.id ? trimmedLength(after, stepsPerBar) : null
  if(to === null || !cursorPast(after, to, run)) return { trim: true, gesture: currentGesture() }

  const gesture = currentGesture() ?? newGesture()
  if(held && held.patternId !== after.id) held.send()
  held = { patternId: after.id, stepsPerBar, getState, send: () => trim(gesture), seen: after, to }
  if(!frame) frame = requestAnimationFrame(follow)
  return { trim: false, gesture }
}

// forget the trim held, if any: the sequencer it was for is being replaced
// (by an undo or a redo, or another project)
export const dropHeldTrim = (): void => {
  held = null
  cancelAnimationFrame(frame)
  frame = 0
}
