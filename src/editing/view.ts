// scrolling and zooming a view along time, the same in the playlist and the
// Piano Roll: what the wheel, a two-finger swipe and a pinch do. plain
// functions, with no canvas

// how far the mouse moves from where it was pressed before the press is a
// drag rather than a click, in screen pixels
export const DRAG_THRESHOLD_PX = 10

// a wheel that scrolls by lines: how many pixels a line is
export const WHEEL_LINE_PIXELS = 16
// how much zoom one pixel of ⌘+wheel gives, and the most one wheel event
// zooms by
export const WHEEL_ZOOM_PER_PIXEL = 0.01
export const ZOOM_SENSITIVITY = 0.035

// what a wheel event does: zoom along time by `factor`, or scroll by `dx`
// along time and `dy` through the lanes, in pixels
export type WheelIntent =
  | { kind: 'zoom', factor: number }
  | { kind: 'scroll', dx: number, dy: number }

type Wheel = Pick<WheelEvent, 'deltaX' | 'deltaY' | 'deltaMode' | 'metaKey' | 'ctrlKey' | 'shiftKey'>

// (the constants of WheelEvent, which isn't there to read them from in tests)
const DOM_DELTA_LINE = 1
const DOM_DELTA_PAGE = 2

// the wheel or a two-finger swipe scrolls: up and down through the lanes,
// left and right (or shift+wheel) along time. ⌘+wheel (⌃ too, which is how
// a pinch arrives outside WebKit) zooms
// - pageHeight: the view's height, for a wheel that scrolls by pages
export const wheelIntent = (ev: Wheel, pageHeight: number): WheelIntent => {
  let unit = 1
  if(ev.deltaMode === DOM_DELTA_LINE) unit = WHEEL_LINE_PIXELS
  else if(ev.deltaMode === DOM_DELTA_PAGE) unit = pageHeight

  let dx = ev.deltaX * unit
  let dy = ev.deltaY * unit

  if(ev.metaKey || ev.ctrlKey) {
    const factor = Math.exp(-dy * WHEEL_ZOOM_PER_PIXEL)
    return { kind: 'zoom', factor: Math.min(1 + ZOOM_SENSITIVITY, Math.max(1 - ZOOM_SENSITIVITY, factor)) }
  }

  // a mouse wheel with shift held (some systems already swap these)
  if(ev.shiftKey && dx === 0) {
    dx = dy
    dy = 0
  }
  return { kind: 'scroll', dx, dy }
}

// a view's zoom and pan along time
export interface Zoom {
  // how many times wider than at scale 1 everything is drawn
  scale: number
  // where the start of the view's content is drawn, in screen pixels from
  // the view's left edge (0, or less once it's scrolled along)
  pan: number
}

// zoom by `factor`, keeping the point at screen x `aroundX` in place. the
// scale stays between `minScale` and `maxScale`
export const zoomAround = (
  { scale, pan }: Zoom,
  factor: number,
  aroundX: number,
  minScale: number,
  maxScale = Infinity,
): Zoom => {
  if(!Number.isFinite(factor) || factor <= 0 || !(scale > 0)) return { scale, pan }

  const next = Math.min(Math.max(scale * factor, minScale), Math.max(minScale, maxScale))
  const applied = next / scale
  return { scale: next, pan: aroundX - (aroundX - pan) * applied }
}
