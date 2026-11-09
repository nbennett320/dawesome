import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItemPixelOffset } from '../../../types/playlist'
import { theme } from './theme'
import { Peaks, peakRange, peaksOfPairs } from './peaks'

interface Props extends PlaylistComponentProps {
  // id: number
  soundData: number[]
  duration: number
  startBeat: number
  trackNumber: number
  minHeight: number
  maxHeight: number
  trackHeight: number
  pixelOffset: PlaylistItemPixelOffset
}

class Waveform extends PlaylistComponent {
  soundData: number[]
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

  // the levels, summarised for drawing a pixel column at a time. built the
  // first time the clip is drawn
  #peaks: Peaks | null = null

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
  
  // the waveform is the line through the engine's points (soundData holds
  // [x, y] pairs: x is the point's index, y is 0..1), spread evenly across
  // the clip's width and filling the clip below its label
  //
  // a clip usually has many points to each pixel (about 2000 a second of
  // sound), so rather than a line per point, each column of pixels on screen
  // is filled from the lowest to the highest point the line passes through
  // there, all in one path. zoomed in past a point per pixel, the line is
  // traced through the points on screen. either way only what's on screen
  // is drawn, and it looks the same as a line per point
  render = () => {
    const { minHeight, height, labelHeight, p } = this
    const count = this.soundData.length / 2
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

    // the device x of point i, and the (fractional) point at device x
    const pointsPerUnit = count / width
    const deviceX = (i: number) => (left + i / pointsPerUnit) * scaleX + offsetX
    const pointAt = (x: number) => ((x - offsetX) / scaleX - left) * pointsPerUnit
    const y = (level: number) => bodyTop + level * bodyHeight

    // the columns on screen, left of the canvas's right edge
    const first = Math.max(0, Math.floor(deviceX(0)))
    const last = Math.min(canvasWidth, Math.ceil(deviceX(count - 1)))
    if(first >= last) return

    // the line is .75px wide, as it was when it was drawn a line per point
    const lineWidth = .75

    p.push()
    if(count / (width * scaleX) >= 1) {
      // a point or more to each column
      if(!this.#peaks) this.#peaks = peaksOfPairs(this.soundData)
      const peaks = this.#peaks
      // a column, in playlist x
      const columnWidth = 1 / scaleX

      p.noStroke()
      p.fill(...theme.clipWave)
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
            (high - low) * bodyHeight + lineWidth,
          )
        }
      }
      ctx.fill()
    } else {
      // zoomed in: fewer points than columns. the line through the points on
      // screen, and the ones just off either side
      const from = Math.max(0, Math.floor(pointAt(first)))
      const to = Math.min(count - 1, Math.ceil(pointAt(last)))
      const pointX = (i: number) => left + i / pointsPerUnit

      p.stroke(...theme.clipWave)
      p.noFill()
      ctx.beginPath()
      ctx.moveTo(pointX(from), y(this.soundData[2 * from + 1]))
      for(let i = from + 1; i <= to; i++) {
        ctx.lineTo(pointX(i), y(this.soundData[2 * i + 1]))
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
