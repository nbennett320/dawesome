// how the selection box, the Cut tool's line and a grabbed edge look, the
// same in the playlist and the Piano Roll. drawn on a 2D canvas (the
// playlist's p5 canvas is one too), in the colours of the theme's playlist
import { painter } from '../theme/canvas'
import { Edge } from './edges'

const paint = painter('playlist')

const DASH = [4, 3]

// the selection box: a dashed outline over a faint fill. in screen pixels
export const drawLasso = (ctx: CanvasRenderingContext2D, x: number, y: number, width: number, height: number) => {
  ctx.save()
  ctx.fillStyle = paint.css('lassoFill', 0.08)
  ctx.strokeStyle = paint.css('selectedBorder')
  ctx.lineWidth = 1
  ctx.setLineDash(DASH)
  ctx.fillRect(x, y, width, height)
  ctx.strokeRect(x, y, width, height)
  ctx.restore()
}

// the Cut tool's line: dashed, down an item where a click would cut it.
// `lineWidth` is 1.5 screen pixels unless the canvas is stretched along x
export const drawCutLine = (
  ctx: CanvasRenderingContext2D,
  x: number,
  top: number,
  bottom: number,
  lineWidth = 1.5,
) => {
  ctx.save()
  ctx.strokeStyle = paint.css('accent')
  ctx.lineWidth = lineWidth
  ctx.setLineDash(DASH)
  ctx.beginPath()
  ctx.moveTo(x, top)
  ctx.lineTo(x, bottom)
  ctx.stroke()
  ctx.restore()
}

// how wide the bar on an edge that's hovered or being dragged is
export const EDGE_BAR_WIDTH = 2

// a bar on the edge of an item the mouse can grab, or is dragging, to trim
// it. `left` and `right` are the item's edges, in screen pixels
export const drawEdgeBar = (
  ctx: CanvasRenderingContext2D,
  edge: Edge,
  left: number,
  right: number,
  top: number,
  height: number,
) => {
  ctx.save()
  ctx.fillStyle = paint.css('accent')
  ctx.fillRect(edge === 'left' ? left : right - EDGE_BAR_WIDTH, top, EDGE_BAR_WIDTH, height)
  ctx.restore()
}
