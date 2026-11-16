// a dragged clip lifts off the playlist like a sticker: it grows a touch and
// a soft shadow drops under it, eased in over LIFT.ms after the drag
// starts. drawn in canvas pixels, so zoom doesn't stretch any of it

import { clamp } from '../../../util/helpers'
import { paint } from './theme'

export const LIFT = {
  ms: 120,
  // how much bigger a lifted clip is drawn, around the mouse
  grow: 0.02,
  // the shadow (the theme's liftShadow), in css pixels: its blur, and how
  // far below the card
  shadowBlur: 10,
  shadowY: 4,
  radius: 3,
} as const

export interface Rect {
  x: number
  y: number
  w: number
  h: number
}

// how far lifted, 0 to 1, `elapsed` ms after the drag started. eased out,
// so it leaves the playlist quickly and settles. `still` (reduced motion)
// is lifted at once
export const liftAmount = (elapsed: number, still: boolean): number => {
  if(still) return 1
  const t = clamp(elapsed / LIFT.ms, 0, 1)
  return 1 - (1 - t) ** 3
}

// the cards' shadow, only outside them: a card is see-through, and its
// own shadow would darken it. the cards' fill is clipped away with them,
// leaving just the shadow
export const drawLiftShadow = (ctx: CanvasRenderingContext2D, cards: Rect[], lift: number, density: number) => {
  if(lift <= 0 || !cards.length) return
  const { width, height } = ctx.canvas
  ctx.save()
  ctx.setTransform(1, 0, 0, 1, 0, 0)
  ctx.beginPath()
  ctx.rect(0, 0, width, height)
  cards.forEach(c => ctx.roundRect(c.x, c.y, c.w, c.h, LIFT.radius * density))
  ctx.clip('evenodd')
  ctx.shadowColor = paint.css('liftShadow', lift)
  ctx.shadowBlur = LIFT.shadowBlur * density * lift
  ctx.shadowOffsetY = LIFT.shadowY * density * lift
  ctx.fillStyle = paint.css('liftShadow')
  ctx.beginPath()
  cards.forEach(c => ctx.roundRect(c.x, c.y, c.w, c.h, LIFT.radius * density))
  ctx.fill()
  ctx.restore()
}
