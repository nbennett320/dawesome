import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItemPixelOffset } from '../../../types/playlist'
import { paint } from './theme'
import { Peaks, buildPeaks, peakRange } from './peaks'

// each waveform's levels, summarised for drawing a pixel column at a time.
// built the first time a waveform is drawn, and kept with its levels, which
// a clip keeps when it's rebuilt (see Renderer.updateWithProps)
const peaksCache = new WeakMap<Float32Array, Peaks>()
const peaksOf = (levels: Float32Array): Peaks => {
  let peaks = peaksCache.get(levels)
  if(!peaks) {
    peaks = buildPeaks(levels)
    peaksCache.set(levels, peaks)
  }
  return peaks
}

interface Props extends PlaylistComponentProps {
  // id: number
  soundData: Float32Array
  duration: number
  startBeat: number
  trackNumber: number
  minHeight: number
  maxHeight: number
  trackHeight: number
  pixelOffset: PlaylistItemPixelOffset
}

// a level (0..1, silence at 0.5) as a clip's gain changes it: further from
// the middle by the gain, and flat at the clip's edges past full scale, as
// the sound clips
export const gainedLevel = (level: number, gain: number): number =>
  Math.min(1, Math.max(0, 0.5 + (level - 0.5) * gain))

class Waveform extends PlaylistComponent {
  // the level (0..1) at evenly spaced points along the whole sample
  soundData: Float32Array
  // the clip's gain, as a multiplier, which the waveform's drawn at
  gain = 1
  // the part of the sample the clip shows, as fractions of it: from, to.
  // all of it, unless the clip is trimmed
  range: [number, number] = [0, 1]
  duration: number
  // where the clip starts, in beats from the playlist start
  startBeat: number
  trackNumber: number
  pixelOffset: PlaylistItemPixelOffset
  height: number
  maxHeight: number
  minHeight: number
  trackHeight: number
  labelHeight: number = staticDefaults.PlaylistObject.labelHeight
  tabHeight: number = staticDefaults.PlaylistObject.tabHeight

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
    this.soundData = props.soundData
    this.duration = props.duration
    this.startBeat = props.startBeat
    this.trackNumber = props.trackNumber
    this.pixelOffset = props.pixelOffset
    this.minHeight = props.minHeight
    this.maxHeight = props.maxHeight
    this.height = props.maxHeight - props.minHeight
    this.trackHeight = props.trackHeight
  }

  // left edge, in playlist x (see Renderer.screenToPlaylistX)
  // computed from beats on every call, so it follows resizes
  #getX = () => (this.renderer as Renderer).beatToX(this.startBeat)

  // the clip is as wide as its duration on the timeline
  #getWidth = () => (this.renderer as Renderer).secondsToWidth(this.duration)


  boundingBox = () => ({
    minHeight: this.minHeight,
    maxHeight: this.maxHeight,
    bottom: this.maxHeight,
    top: this.minHeight,
    left: this.#getX(),
    right: this.#getX() + this.#getWidth(),
    height: this.height,
    width: this.#getWidth(),
  })
  
  // the waveform is the line through the engine's levels (0..1), spread
  // evenly across the clip's width and filling the clip below its label, at
  // the clip's gain
  //
  // a clip usually has many points to each pixel (about 2000 a second of
  // sound), so rather than a line per point, each column of pixels on screen
  // is filled from the lowest to the highest point the line passes through
  // there, all in one path. zoomed in past a point per pixel, the line is
  // traced through the points on screen. either way only what's on screen
  // is drawn, and it looks the same as a line per point
  render = () => {
    const { minHeight, height, labelHeight, p } = this
    const count = this.soundData.length
    if(count < 2) return

    // (this used to subtract the playlist's distance from the top of the
    // window, taken from where the clip was dropped, so clips loaded from a
    // project, which were never dropped, were drawn a track too low)
    const bodyTop = minHeight + labelHeight
    const bodyHeight = height - labelHeight
    const left = this.#getX()
    const width = this.#getWidth()
    if(width <= 0 || bodyHeight <= 0) return

    // where this draws on the canvas, in its device pixels: whatever zoom,
    // pan, scroll, or drag offset the canvas is transformed by, and the
    // screen's pixel density. only x is ever scaled by zoom
    const ctx = p.drawingContext as CanvasRenderingContext2D
    const { a: scaleX, e: offsetX, d: scaleY, f: offsetY } = ctx.getTransform()
    const { width: canvasWidth, height: canvasHeight } = ctx.canvas
    if(scaleX <= 0 || scaleY <= 0) return

    // scrolled out of view, above or below
    const deviceTop = bodyTop * scaleY + offsetY
    if(deviceTop + bodyHeight * scaleY < 0 || deviceTop > canvasHeight) return

    // the points the clip shows: all of them, or a trimmed clip's part
    const [from, to] = this.range
    const trimmed = from > 0 || to < 1
    const firstPoint = from * count
    const shown = (to - from) * count
    if(shown <= 0) return

    // the device x of point i, and the (fractional) point at device x
    const pointsPerUnit = shown / width
    const deviceX = (i: number) => (left + (i - firstPoint) / pointsPerUnit) * scaleX + offsetX
    const pointAt = (x: number) => ((x - offsetX) / scaleX - left) * pointsPerUnit + firstPoint
    const { gain } = this
    const y = (level: number) => bodyTop + gainedLevel(level, gain) * bodyHeight

    // the columns on screen, left of the canvas's right edge
    const first = Math.max(0, Math.floor(deviceX(firstPoint)))
    const last = Math.min(canvasWidth, Math.ceil(deviceX(Math.min(count - 1, to * count))))
    if(first >= last) return

    // the line is .75px wide, as it was when it was drawn a line per point
    const lineWidth = .75

    p.push()
    // a trimmed clip's line stops at its edges, though the points just past
    // them shape its last stretch
    if(trimmed) {
      ctx.beginPath()
      ctx.rect(left, bodyTop, width, bodyHeight)
      ctx.clip()
    }
    if(shown / (width * scaleX) >= 1) {
      // a point or more to each column
      const peaks = peaksOf(this.soundData)
      // a column, in playlist x
      const columnWidth = 1 / scaleX

      p.noStroke()
      paint.fill(p, 'clipWave')
      ctx.beginPath()
      for(let column = first; column < last; column++) {
        // the points either side of the column too, since the line from
        // each passes through it
        const range = peakRange(peaks, Math.floor(pointAt(column)), Math.ceil(pointAt(column + 1)))
        if(range) {
          const [low, high] = range
          ctx.rect(
            (column - offsetX) / scaleX,
            y(low) - lineWidth / 2,
            columnWidth,
            y(high) - y(low) + lineWidth,
          )
        }
      }
      ctx.fill()
    } else {
      // zoomed in: fewer points than columns. the line through the points on
      // screen, and the ones just off either side
      const start = Math.max(0, Math.floor(pointAt(first)))
      const end = Math.min(count - 1, Math.ceil(pointAt(last)))
      const pointX = (i: number) => left + (i - firstPoint) / pointsPerUnit

      paint.stroke(p, 'clipWave')
      p.noFill()
      ctx.beginPath()
      ctx.moveTo(pointX(start), y(this.soundData[start]))
      for(let i = start + 1; i <= end; i++) {
        ctx.lineTo(pointX(i), y(this.soundData[i]))
      }
      // stroked without the zoom, which would otherwise stretch the line's
      // width along with the timeline (y is only scaled by pixel density)
      ctx.setTransform(scaleY, 0, 0, scaleY, 0, 0)
      ctx.lineWidth = lineWidth
      ctx.stroke()
    }
    p.pop()
  }
}

export default Waveform
