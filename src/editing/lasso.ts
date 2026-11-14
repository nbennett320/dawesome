// selecting with a box (shift+drag) and shift+click, the same in the
// playlist (clips) and the Piano Roll (notes). plain functions, with no
// canvas, so they can be tested on their own. an "item" is whatever a view
// lays out on its lanes; a view gives each an id and a box

// a point in a view's own coordinates, which don't change as it's scrolled
// or zoomed: in the playlist, x at scale 1 with no pan, and y with the
// tracks laid out from the top; in the Piano Roll, steps and rows
export interface Point {
  x: number
  y: number
}

// a box being dragged out, from where the mouse was pressed to where it is
export interface Lasso {
  start: Point
  end: Point
  // what was selected when the box started. it stays selected until the
  // box touches an item, then the box's items replace it
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
// box (a straight vertical drag) still picks up the items it crosses
export const rectsTouch = (a: Rect, b: Rect): boolean =>
  a.left <= b.left + b.width
  && b.left <= a.left + a.width
  && a.top <= b.top + b.height
  && b.top <= a.top + a.height

// what's selected while a box is dragged: every item the box touches, in
// place of what was selected before. a box that touches nothing leaves the
// selection as it was
export const lassoSelection = (
  lasso: Lasso,
  items: ReadonlyArray<{ id: number, box: Rect }>,
): Set<number> => {
  const rect = lassoRect(lasso)
  const touched = new Set(items.filter(({ box }) => rectsTouch(rect, box)).map(({ id }) => id))
  return touched.size ? touched : new Set(lasso.base)
}

// a press with shift held, until it's clear what it is: a click (toggle the
// item under it, if any) or the start of a box
export interface ShiftPress {
  // where it went down on the canvas, in pixels
  screen: Point
  // the same spot in the view's coordinates, where a box would start
  point: Point
  // the item under it, if any
  itemId: number | null
}

// the item at a point in the view's coordinates: the one drawn last (on
// top) of those whose box holds it, or none
export const itemAt = (point: Point, items: ReadonlyArray<{ id: number, box: Rect }>): number | null => {
  const under = items.filter(({ box }) => rectsTouch({ left: point.x, top: point.y, width: 0, height: 0 }, box))
  return under.length ? under[under.length - 1].id : null
}

// whether the mouse has moved far enough from a shift press to be dragging
// out a box, rather than clicking (the same distance that makes a press on
// an item a drag)
export const startsLasso = (press: ShiftPress, mouse: Point, threshold: number): boolean =>
  Math.hypot(mouse.x - press.screen.x, mouse.y - press.screen.y) > threshold

// the box a shift press turns into once it's dragged. it replaces what's
// selected once it touches an item, and starts where the press was, so a
// press on an item takes that item in
export const lassoFrom = (press: ShiftPress, selected: ReadonlySet<number>): Lasso => ({
  start: press.point,
  end: press.point,
  base: new Set(selected),
})

// add an item to the selection, or take it out if it's already in
export const toggleSelected = (selected: ReadonlySet<number>, id: number): Set<number> => {
  const next = new Set(selected)
  if(next.has(id)) next.delete(id)
  else next.add(id)
  return next
}

// what's selected after a shift press is let go without becoming a box: the
// item under it is added, or taken out if it was in. on empty space, nothing
// changes
export const shiftClick = (press: ShiftPress, selected: ReadonlySet<number>): Set<number> =>
  (press.itemId === null ? new Set(selected) : toggleSelected(selected, press.itemId))

// what's selected once a shift press is let go, with the mouse at `mouse`
// (`screen` in canvas pixels, `point` in the view's coordinates). let go
// within the drag distance it was a click; further away it was a box, even
// if no frame was drawn between the move and the release. a click on an
// item that has gone since the press (deleted, exploded) changes nothing
export const shiftRelease = (
  press: ShiftPress,
  mouse: { screen: Point, point: Point },
  threshold: number,
  selected: ReadonlySet<number>,
  items: ReadonlyArray<{ id: number, box: Rect }>,
): Set<number> => {
  if(startsLasso(press, mouse.screen, threshold)) {
    return lassoSelection({ ...lassoFrom(press, selected), end: mouse.point }, items)
  }
  if(!items.some(item => item.id === press.itemId)) return new Set(selected)
  return shiftClick(press, selected)
}

// forget items that no longer exist (removed, or a project was loaded)
export const keepExisting = (selected: ReadonlySet<number>, ids: ReadonlyArray<number>): Set<number> => {
  const existing = new Set(ids)
  return new Set([...selected].filter(id => existing.has(id)))
}
