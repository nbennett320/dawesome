// the loop in the ruler: a region drawn with shift+drag, whose edges can be
// dragged, and the end flag, which can be dragged when there's no region.
// kept free of p5 so it can be tested on its own. everything is in beats

import { LoopRegion } from '../../../types/transport'

// how near (in screen pixels) a press has to be to an edge or the flag to
// grab it
export const LOOP_GRAB_PX = 5

// the loop as the ruler shows it: a region (start before end), or with
// none, the end flag (null when it isn't shown: looping is off)
export interface LoopMarks {
  region: LoopRegion | null
  flag: number | null
}

// what's being dragged: a region's start or end, the end flag, or a new
// region being drawn
export type LoopHandle = 'start' | 'end' | 'flag' | 'draw'

// a drag in the ruler: the edge that stays put (`anchor`, for a region) and
// the one following the mouse (`beat`), and where that was when the drag
// started, so a click that doesn't move it changes nothing
export interface LoopDrag {
  handle: LoopHandle
  anchor: number
  beat: number
  from: number
}

// the loop's end flag is never before the first beat (see set_loop_end)
export const MIN_LOOP_END = 1

// the loop the transport describes (see PlaylistTransport): a region, or the
// end flag while looping. none before the engine has said
export const loopMarks = (t: {
  loopStartBeats: number, loopEndBeats: number, loopRegion: boolean, loopEnabled: boolean,
} | null): LoopMarks => {
  if(!t) return { region: null, flag: null }
  if(t.loopRegion && t.loopEndBeats > t.loopStartBeats) {
    return { region: { start: t.loopStartBeats, end: t.loopEndBeats }, flag: null }
  }
  return { region: null, flag: t.loopEnabled && t.loopEndBeats > 0 ? t.loopEndBeats : null }
}

// the edge (or the flag) a press at `beat` grabs, if it's within `reach`
// beats of one. of a region's two edges, the nearer. a press between two
// edges that are equally near (a very short region) takes the one on the
// press's side of the middle
export const handleAt = (marks: LoopMarks, beat: number, reach: number): 'start' | 'end' | 'flag' | null => {
  const { region, flag } = marks
  if(region) {
    const toStart = Math.abs(beat - region.start)
    const toEnd = Math.abs(beat - region.end)
    if(Math.min(toStart, toEnd) > reach) return null
    if(toStart === toEnd) return beat < (region.start + region.end) / 2 ? 'start' : 'end'
    return toStart < toEnd ? 'start' : 'end'
  }
  return flag !== null && Math.abs(beat - flag) <= reach ? 'flag' : null
}

// a drag of `handle`, from a press at the (snapped) `beat`
export const startLoopDrag = (handle: LoopHandle, marks: LoopMarks, beat: number): LoopDrag => {
  const { region, flag } = marks
  if(handle === 'start' && region) return { handle, anchor: region.end, beat: region.start, from: region.start }
  if(handle === 'end' && region) return { handle, anchor: region.start, beat: region.end, from: region.end }
  if(handle === 'flag' && flag !== null) return { handle, anchor: 0, beat: flag, from: flag }
  // a new region starts with both edges where it was pressed
  return { handle: 'draw', anchor: beat, beat, from: beat }
}

// the drag, its moving edge at the (snapped) `beat` now. the end flag
// stays at least a beat in
export const moveLoopDrag = (drag: LoopDrag, beat: number): LoopDrag =>
  ({ ...drag, beat: drag.handle === 'flag' ? Math.max(MIN_LOOP_END, beat) : beat })

// the region a drag gives: its edges in order (an edge dragged past the
// other swaps with it), or none, with both edges on the same beat
export const draggedRegion = (drag: LoopDrag): LoopRegion | null => {
  const start = Math.min(drag.anchor, drag.beat)
  const end = Math.max(drag.anchor, drag.beat)
  return end - start > 1e-6 ? { start, end } : null
}

// the loop as the ruler shows it while a drag is under way (or waiting for
// the engine to say it's done)
export const draggedMarks = (marks: LoopMarks, drag: LoopDrag | null): LoopMarks => {
  if(!drag) return marks
  if(drag.handle === 'flag') return { region: null, flag: drag.beat }
  return { region: draggedRegion(drag), flag: null }
}

// what letting go of a drag asks the engine for: a region (between two
// edges, in either order: the same beat for none), the end flag pinned, or
// nothing, for an edge or flag that didn't move. a shift+click that didn't
// move asks for a region with no length, which clears the region
export type LoopChange =
  | { kind: 'region', start: number, end: number }
  | { kind: 'end', beat: number }
  | null

export const loopChange = (drag: LoopDrag): LoopChange => {
  if(drag.handle !== 'draw' && Math.abs(drag.beat - drag.from) < 1e-9) return null
  if(drag.handle === 'flag') return { kind: 'end', beat: drag.beat }
  return { kind: 'region', start: drag.anchor, end: drag.beat }
}

// what a double-click in the ruler at `beat` clears: the region, when it's
// on it (or near enough an edge to have grabbed it), or a dragged end flag
// it's on, which goes back to following the last clip
export const doubleClickClears = (
  marks: LoopMarks, pinned: boolean, beat: number, reach: number,
): 'region' | 'end' | null => {
  const { region, flag } = marks
  if(region) return beat >= region.start - reach && beat <= region.end + reach ? 'region' : null
  return pinned && flag !== null && Math.abs(beat - flag) <= reach ? 'end' : null
}

// the parts of the playlist outside the region, dimmed down the lanes while
// it loops: before its start and after its end, up to the playlist's end
export const outsideRegion = (region: LoopRegion | null, beats: number): Array<{ start: number, end: number }> => {
  if(!region) return []
  return [{ start: 0, end: region.start }, { start: region.end, end: beats }]
    .filter(part => part.end - part.start > 0)
}
