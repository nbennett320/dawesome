import { clampLowWins } from '../util/helpers'

// trimming items by their edges, the same in the playlist (clips) and the
// Piano Roll (notes). plain functions, with no canvas. places and lengths
// are in the view's own unit along time: beats in the playlist, steps in
// the Piano Roll

export type Edge = 'left' | 'right'

// how wide the zone inside each edge of an item is, where pressing grabs
// the edge rather than the item. in screen pixels, whatever the zoom
export const EDGE_ZONE_PX = 6

// the edge of an item the mouse is over, if any
// - x: the mouse, in screen pixels from the item's left edge
// - width: the item's width, in screen pixels
// on a narrow item each zone is a third of it at most, so the middle third
// still moves it
export const edgeAt = (x: number, width: number): Edge | null => {
  if(!(width > 0) || x < 0 || x > width) return null

  const zone = Math.min(EDGE_ZONE_PX, width / 3)
  if(x < zone) return 'left'
  if(x > width - zone) return 'right'
  return null
}

// where an item starts and ends
export interface Span {
  start: number
  end: number
}

// an item, as far as trimming goes
export interface TrimItem extends Span {
  id: number
  // as far as its edges can go: where a clip's sample or pattern would
  // start and end, were all of it played; where a note's pattern does.
  // (the start can be before 0, where no edge can follow it)
  sourceStart: number
  sourceEnd: number
  // the shortest it can be
  minLength: number
}


// an item with one edge moved by `delta`, stopped where it can go no
// further: the source's ends, 0, and the other edge (less the shortest an
// item can be)
export const trimmed = (item: TrimItem, edge: Edge, delta: number): Span => {
  if(edge === 'left') {
    const earliest = Math.max(item.sourceStart, 0)
    return {
      start: clampLowWins(item.start + delta, earliest, item.end - item.minLength),
      end: item.end,
    }
  }

  // (a pattern clip whose pattern now ends before the clip starts has no
  // end to drag: it stays a step long, and keeps the window it had)
  if(item.sourceEnd <= item.start) return { start: item.start, end: item.end }

  const earliest = Math.min(item.start + item.minLength, item.sourceEnd)
  return {
    start: item.start,
    end: clampLowWins(item.end + delta, earliest, item.sourceEnd),
  }
}

// every item of a trim: the grabbed item's edge goes to `target` (where the
// mouse is, after snapping), and the same edge of each of the others moves
// as far. each stops at its own limits, wherever the others are
export const trimAll = (
  items: ReadonlyArray<TrimItem>,
  grabbedId: number,
  edge: Edge,
  target: number,
): Array<Span & { id: number }> => {
  const grabbed = items.find(item => item.id === grabbedId)
  if(!grabbed) return []

  const delta = target - (edge === 'left' ? grabbed.start : grabbed.end)
  return items.map(item => ({ id: item.id, ...trimmed(item, edge, delta) }))
}
