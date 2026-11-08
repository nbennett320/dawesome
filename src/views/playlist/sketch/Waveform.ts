import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItemPixelOffset } from '../../../types/playlist'

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

  // x of the i'th waveform point (soundData holds [x, y] pairs, where x is
  // the point's index), spread evenly across the clip's width
  pointX = (i: number, left: number) => {
    const points = this.soundData.length / 2

    return left + (points > 0 ? this.soundData[i] / points : 0) * this.#getWidth()
  }

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
  
  render = () => {
    const { minHeight, trackHeight, height, labelHeight, tabHeight } = this
    const { yOffset, y } = this.pixelOffset
    const left = this.#getX()

    // no `* currentScale` here: the canvas is already scaled when this draws
    this.p.strokeWeight(.3)
    this.p.stroke(0, 0, 0)
    for(let i = 0; i < this.soundData.length - 2; i+=2) {
      const x0 = this.pointX(i, left)
      const y0 = minHeight + labelHeight*2 + tabHeight + (trackHeight/2) + (this.soundData[i+1] * (height - labelHeight)) - (y - yOffset)
      const x1 = this.pointX(i+2, left)
      const y1 = minHeight + labelHeight*2 + tabHeight + (trackHeight/2) + (this.soundData[i+3] * (height - labelHeight)) - (y - yOffset)
      this.p.line(x0, y0, x1, y1)
    }
  }
}

export default Waveform
