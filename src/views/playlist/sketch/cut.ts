// the Cut tool: where a cut lands, and which clips it cuts. kept free of p5
// so it can be tested on its own
import { Snapping, Span, snapPoint } from './edges'

// a clip, as far as cutting goes. in beats of the playlist
export interface CutClip extends Span {
  id: number
  // the shortest a clip can be: no cut is made nearer than this to an edge
  minBeats: number
}

// whether a clip can be cut at a beat: inside it, and leaving both halves
// at least as long as a clip can be
export const canCut = (clip: CutClip, beat: number): boolean =>
  beat >= clip.startBeat + clip.minBeats && beat <= clip.endBeat - clip.minBeats

// where a cut lands on the clip under the mouse, with the mouse at `beat`:
// on the grid when snap is on, or on the playhead when the mouse is nearer
// to it and it isn't moving, unless Option is held (see snapPoint). null
// when that's nowhere the clip can be cut, and no line is shown
export const cutPoint = (clip: CutClip, beat: number, snap: Snapping): number | null => {
  const at = snapPoint(beat, snap)
  return canCut(clip, at) ? at : null
}

// the clips a cut at `beat` on the clip under the mouse cuts: that clip,
// selected or not, and, when it's one of the selection, every other
// selected clip the cut is inside. nothing when the clip itself can't be
// cut there
export const cutTargets = (
  clips: ReadonlyArray<CutClip>,
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
