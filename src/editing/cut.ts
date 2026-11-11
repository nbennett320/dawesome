// the Cut tool, the same in the playlist (clips) and the Piano Roll
// (notes): where a cut lands, and which items it cuts. plain functions,
// with no canvas. places are in the view's own unit along time: beats in
// the playlist, steps in the Piano Roll
import { Span } from './edges'
import { Snapping, snapPoint } from './snap'

// an item, as far as cutting goes
export interface CutItem extends Span {
  id: number
  // the shortest an item can be: no cut is made nearer than this to an edge
  minLength: number
}

// whether an item can be cut at a place: inside it, and leaving both halves
// at least as long as an item can be
export const canCut = (clip: CutItem, beat: number): boolean =>
  beat >= clip.start + clip.minLength && beat <= clip.end - clip.minLength

// where a cut lands on the item under the mouse, with the mouse at `beat`:
// on the grid when snap is on, or on the playhead when the mouse is nearer
// to it and it isn't moving, unless Option is held (see snapPoint). null
// when that's nowhere the item can be cut, and no line is shown
export const cutPoint = (clip: CutItem, beat: number, snap: Snapping): number | null => {
  const at = snapPoint(beat, snap)
  return canCut(clip, at) ? at : null
}

// the items a cut at `beat` on the item under the mouse cuts: that item,
// selected or not, and, when it's one of the selection, every other
// selected item the cut is inside. nothing when the item itself can't be
// cut there
export const cutTargets = (
  clips: ReadonlyArray<CutItem>,
  hoveredId: number,
  selected: ReadonlySet<number>,
  beat: number,
): number[] => {
  const hovered = clips.find(clip => clip.id === hoveredId)
  if(!hovered || !canCut(hovered, beat)) return []
  if(!selected.has(hoveredId)) return [hoveredId]

  return [
    hoveredId,
    ...clips
      .filter(clip => clip.id !== hoveredId && selected.has(clip.id) && canCut(clip, beat))
      .map(clip => clip.id),
  ]
}
