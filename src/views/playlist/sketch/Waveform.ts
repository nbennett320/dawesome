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
    const { minHeight, height, labelHeight } = this
    const left = this.#getX()

    // the waveform fills the clip below its label. points are 0..1 from the
    // engine. (this used to subtract the playlist's distance from the top of
    // the window, taken from where the clip was dropped, so clips loaded from
    // a project, which were never dropped, were drawn a track too low)
    const bodyTop = minHeight + labelHeight
    const bodyHeight = height - labelHeight
    const pointY = (i: number) => bodyTop + this.soundData[i] * bodyHeight

    // no `* currentScale` here: the canvas is already scaled when this draws
    this.p.strokeWeight(.3)
    this.p.stroke(0, 0, 0)
    for(let i = 0; i < this.soundData.length - 2; i+=2) {
      this.p.line(this.pointX(i, left), pointY(i+1), this.pointX(i+2, left), pointY(i+3))
    }
  }
}

export default Waveform
