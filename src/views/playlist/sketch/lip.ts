// a lane's label: the track's name printed at the top right of its lane,
// like the lettering on a four-track's panel, with no box around it, and its
// color LED set in a recess beside it. the LED is
// unlit in the track's color while it's quiet, and lit while the playhead
// plays one of its clips. drawn in canvas pixels, so it stays the same size
// and place however the playlist is zoomed or panned (it scrolls with its
// lane)

export const LIP = {
  height: 18,
  // below the lane's top line, so it stays inside its own lane
  inset: 4,
  // from the right edge (or the scrollbar)
  margin: 6,
  // inside the tab: before the name, between it and the LED, after the LED
  padLeft: 7,
  gap: 6,
  padRight: 5,
  // the name's lettering (as .ft-label), and the most of it shown (longer
  // names are cut short)
  font: '600 10px system-ui',
  letterSpacing: '1.2px',
  // how far left of the label clips start fading out behind it
  fade: 16,
  maxText: 110,
  // the recess the LED sits in, and the LED
  wellRadius: 4.5,
  ledRadius: 3,
} as const

// the tab's width for a name `textWidth` pixels wide (as LIP.font)
export const lipWidth = (textWidth: number): number =>
  LIP.padLeft + Math.min(textWidth, LIP.maxText) + LIP.gap + LIP.wellRadius * 2 + LIP.padRight

// as much of a name as fits in LIP.maxText, with an ellipsis if cut short.
// ctx.font and ctx.letterSpacing must already be LIP's
export const fitName = (ctx: Pick<CanvasRenderingContext2D, 'measureText'>, name: string): string => {
  if(ctx.measureText(name).width <= LIP.maxText) return name
  let n = name.length
  while(n > 0 && ctx.measureText(`${name.slice(0, n)}…`).width > LIP.maxText) n--
  return `${name.slice(0, n).trimEnd()}…`
}

export interface Box {
  x: number
  y: number
  w: number
  h: number
}

// where a lane's label is: `laneTop` and `laneHeight` place the lane in
// canvas pixels, `right` is the right edge it's pinned to, `width` the
// label's (lipWidth), and `top` where the ruler ends. it's sticky: once its
// lane scrolls up under the ruler, it stays just below the ruler, until the
// lane's bottom pushes it up out of view
export const lipBox = (laneTop: number, laneHeight: number, right: number, width: number, top: number): Box => {
  const natural = laneTop + LIP.inset
  const lowest = laneTop + laneHeight - LIP.inset - LIP.height
  return {
    x: right - LIP.margin - width,
    y: Math.min(Math.max(natural, top + LIP.inset), lowest),
    w: width,
    h: LIP.height,
  }
}

export const inBox = (box: Box, x: number, y: number): boolean =>
  x >= box.x && x <= box.x + box.w && y >= box.y && y <= box.y + box.h

// the middle of a label's LED
export const ledCentre = ({ x, y, w, h }: Box) => ({
  x: x + w - LIP.padRight - LIP.wellRadius,
  y: y + h / 2,
})

// how far around the LED a click still lands on it
const LED_SLOP = 3

// whether a point is on a label's LED (which mutes and solos its track)
export const onLed = (box: Box, x: number, y: number): boolean => {
  const c = ledCentre(box)
  return Math.hypot(x - c.x, y - c.y) <= LIP.wellRadius + LED_SLOP
}

type RGBA = readonly [number, number, number, number]

const css = ([r, g, b, a]: RGBA, alpha = 1) => `rgba(${r}, ${g}, ${b}, ${+((a / 255) * alpha).toFixed(3)})`

// `amount` of a over b
const mix = (a: RGBA, b: RGBA, amount: number): RGBA =>
  [0, 1, 2, 3].map(i => Math.round(a[i] * amount + b[i] * (1 - amount))) as unknown as RGBA

export interface LipColors {
  // the lane under it, which the clips fade into behind the label
  lane: RGBA
  // the lettering, and its lighter grey while the track can't be heard
  ink: RGBA
  inkQuiet: RGBA
  // the recess the LED sits in
  well: RGBA
  // the panel the unlit LED is tinted towards
  panel: RGBA
  // the ring round the LED when the track is muted or soloed, in the
  // mixer's mute and solo LED colors
  mute: RGBA
  solo: RGBA
  // the track's color
  led: RGBA
}

// how a label is drawn: its LED lit (playing a clip), and the track's state
// in the mixer
export interface LipLook {
  lit: boolean
  // muted, silenced by another track's solo (a dim mute ring), or soloed
  ring: 'mute' | 'silenced' | 'solo' | null
}

// an unlit LED's tint: stronger than the fixed LEDs', as on the mixer's
// jewels, so a quiet track's color still shows (see ledTint)
const UNLIT = 0.45

// `name` is already fitted (fitName), in capitals
export const drawLip = (ctx: CanvasRenderingContext2D, box: Box, name: string, colors: LipColors, look: LipLook) => {
  const { x, y, w, h } = box
  const { x: cx, y: cy } = ledCentre(box)
  const { lit, ring } = look

  ctx.save()

  // clips under it fade into the lane, so the lettering reads over them
  const fade = ctx.createLinearGradient(x - LIP.fade, 0, x + LIP.padLeft, 0)
  fade.addColorStop(0, css(colors.lane, 0))
  fade.addColorStop(1, css(colors.lane, 0.85))
  ctx.fillStyle = fade
  ctx.fillRect(x - LIP.fade, y, w + LIP.fade, h)

  // the name, printed on the panel like its other labels (.ft-label)
  ctx.font = LIP.font
  ctx.letterSpacing = LIP.letterSpacing
  ctx.textAlign = 'left'
  ctx.textBaseline = 'middle'
  // lighter while the track can't be heard, as the mixer's name is
  ctx.fillStyle = css(ring === 'mute' || ring === 'silenced' ? colors.inkQuiet : colors.ink)
  ctx.fillText(name, x + LIP.padLeft, cy + 0.5)

  // the recess
  ctx.beginPath()
  ctx.arc(cx, cy, LIP.wellRadius, 0, Math.PI * 2)
  ctx.fillStyle = css(colors.well)
  ctx.fill()
  if(ring) {
    ctx.lineWidth = 1.5
    ctx.strokeStyle = ring === 'solo' ? css(colors.solo) : css(colors.mute, ring === 'silenced' ? 0.45 : 1)
    ctx.stroke()
  }

  // the LED, glowing when lit
  ctx.beginPath()
  ctx.arc(cx, cy, LIP.ledRadius, 0, Math.PI * 2)
  if(lit) {
    ctx.shadowColor = css(colors.led, 0.7)
    ctx.shadowBlur = 6
    ctx.fillStyle = css(colors.led)
  } else {
    ctx.fillStyle = css(mix(colors.led, colors.panel, UNLIT))
  }
  ctx.fill()

  ctx.restore()
}
