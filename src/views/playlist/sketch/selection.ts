// selecting clips with a box (shift+drag) and shift+click. kept free of p5
// so it can be tested on its own

// a point in playlist coordinates: x in playlist x (scale 1, no pan), y in
// playlist y (tracks laid out from the top, before scrolling)
export interface Point {
  x: number
  y: number
}

// a box being dragged out, from where the mouse was pressed to where it is
export interface Lasso {
  start: Point
  end: Point
  // what was selected when the box started. the box adds to it
  base: ReadonlySet<number>
}

export interface Rect {
  left: number
  top: number
  width: number
  height: number
}

// the box between two corners, whichever way it was dragged
export const lassoRect = ({ start, end }: Pick<Lasso, 'start' | 'end'>): Rect => ({
  left: Math.min(start.x, end.x),
  top: Math.min(start.y, end.y),
  width: Math.abs(end.x - start.x),
  height: Math.abs(end.y - start.y),
})

// whether two boxes overlap at all. touching edges count, so a zero-width
// box (a straight vertical drag) still picks up the clips it crosses
export const rectsTouch = (a: Rect, b: Rect): boolean =>
  a.left <= b.left + b.width
  && b.left <= a.left + a.width
  && a.top <= b.top + b.height
  && b.top <= a.top + a.height

// what's selected while a box is dragged: what was selected before, plus
// every clip the box touches
export const lassoSelection = (
  lasso: Lasso,
  clips: ReadonlyArray<{ id: number, box: Rect }>,
): Set<number> => {
  const rect = lassoRect(lasso)
  const selected = new Set(lasso.base)
  clips.forEach(({ id, box }) => {
    if(rectsTouch(rect, box)) selected.add(id)
  })
  return selected
}

// add a clip to the selection, or take it out if it's already in
export const toggleSelected = (selected: ReadonlySet<number>, id: number): Set<number> => {
  const next = new Set(selected)
  if(next.has(id)) next.delete(id)
  else next.add(id)
  return next
}

// forget clips that no longer exist (removed, or a project was loaded)
export const keepExisting = (selected: ReadonlySet<number>, ids: ReadonlyArray<number>): Set<number> => {
  const existing = new Set(ids)
  return new Set([...selected].filter(id => existing.has(id)))
}

// a clip's place in the playlist
export interface Placement {
  id: number
  beat: number
  track: number
}

// where a group of clips lands when one of them, the anchor, is dropped at
// `beat` on `track`. the same rule the engine uses (group_move in
// audiograph.rs): every clip moves by the anchor's change, cut short so no
// clip starts before 0 or after `maxBeat`, or goes above track 0 or below
// `maxTrack` (the new-track area). `beat` should already be snapped
export const groupLanding = (
  placed: ReadonlyArray<Placement>,
  anchorId: number,
  beat: number,
  track: number,
  maxBeat: number,
  maxTrack: number,
): Placement[] => {
  const anchor = placed.find(c => c.id === anchorId)
  if(!anchor) return []

  const clamp = (n: number, lo: number, hi: number) => Math.min(Math.max(n, lo), Math.max(lo, hi))
  const beats = placed.map(c => c.beat)
  const tracks = placed.map(c => c.track)
  const shift = clamp(beat - anchor.beat, -Math.min(...beats), maxBeat - Math.max(...beats))
  const down = clamp(track - anchor.track, -Math.min(...tracks), maxTrack - Math.max(...tracks))

  return placed.map(c => ({ id: c.id, beat: c.beat + shift, track: c.track + down }))
}
