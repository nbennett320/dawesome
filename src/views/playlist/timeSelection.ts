// the time selection: a span of time over a run of adjacent tracks, made
// with ⇧ and the arrow keys as text is selected. one corner stays where it
// was started (the anchor), the other moves (the edge). the edge's track is
// the current track. kept free of p5 so it can be tested on its own. times
// are in beats, tracks are numbered from 0

export interface TimeSelection {
  anchorBeat: number
  edgeBeat: number
  anchorTrack: number
  edgeTrack: number
}

// what it covers: from `start` to `end` (start first), over the tracks from
// `top` to `bottom` (top first), both included
export interface Block {
  start: number
  end: number
  top: number
  bottom: number
}

export const blockOf = (sel: TimeSelection): Block => ({
  start: Math.min(sel.anchorBeat, sel.edgeBeat),
  end: Math.max(sel.anchorBeat, sel.edgeBeat),
  top: Math.min(sel.anchorTrack, sel.edgeTrack),
  bottom: Math.max(sel.anchorTrack, sel.edgeTrack),
})

// spans shorter than this are empty (float rounding)
const SAME = 1e-6

// whether it covers no time (just tracks, after ⇧↑/↓ alone)
export const isEmptySpan = (sel: TimeSelection): boolean => Math.abs(sel.edgeBeat - sel.anchorBeat) < SAME

// where a selection starts: nothing yet, at `beat` on `track`
export const startAt = (beat: number, track: number): TimeSelection => ({
  anchorBeat: beat, edgeBeat: beat, anchorTrack: track, edgeTrack: track,
})

// the edge moved along time to `beat`, the anchor staying put
export const withEdgeBeat = (sel: TimeSelection, beat: number): TimeSelection => ({ ...sel, edgeBeat: beat })

// ⇧↑/⇧↓: the edge moved to the track above (-1) or below (1), growing or
// shrinking the block, kept to the `trackCount` tracks there are
export const withEdgeTrack = (sel: TimeSelection, dir: -1 | 1, trackCount: number): TimeSelection => ({
  ...sel,
  edgeTrack: Math.max(0, Math.min(sel.edgeTrack + dir, trackCount - 1)),
})

// a plain ↑/↓: the whole block moved a track up (-1) or down (1), keeping
// its tracks and its time. null if it's already against the first or last
// track
export const shiftTracks = (sel: TimeSelection, dir: -1 | 1, trackCount: number): TimeSelection | null => {
  const { top, bottom } = blockOf(sel)
  if(top + dir < 0 || bottom + dir > trackCount - 1) return null
  return { ...sel, anchorTrack: sel.anchorTrack + dir, edgeTrack: sel.edgeTrack + dir }
}
