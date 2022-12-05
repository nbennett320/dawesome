import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps, Renderer } from './index'
import { PlaylistItemPixelOffset } from '../../../types/playlist'

interface Props extends PlaylistComponentBaseProps {
  // id: number
  soundData: number[]
  trackNumber: number
  minHeight: number
  maxHeight: number
  trackHeight: number
  pixelOffset: PlaylistItemPixelOffset
}

class Waveform extends PlaylistComponentBase {
  // id: number
  soundData: number[]
  trackNumber: number
  pixelOffset: PlaylistItemPixelOffset
  height: number
  maxHeight: number
  minHeight: number
  trackHeight: number

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
    this.soundData = props.soundData
    this.trackNumber = props.trackNumber
    this.pixelOffset = props.pixelOffset
    // this.maxHeight = ((props.trackNumber + 1) * playlist.individualTrackHeight())
    // this.minHeight = this.maxHeight - this.height
    // const [min, max] = playlist.calculateTrackRange(props.trackNumber, props.pixelOffset)
    this.minHeight = props.minHeight
    this.maxHeight = props.maxHeight
    this.height = props.maxHeight - props.minHeight
    this.trackHeight = props.trackHeight
    // console.log(props.trackNumber, min, max, this.height, props.pixelOffset.y, props.pixelOffset.yOffset)
  }

  #getWidth = () => this.soundData.length / 2

  // to do: return actual width
  boundingBox = () => ({
    minHeight: this.minHeight,
    maxHeight: this.maxHeight,
    bottom: this.maxHeight,
    top: this.minHeight,
    left: this.pixelOffset.xOffset,
    right: this.#getWidth() + this.pixelOffset.xOffset,
    height: this.height,
    width: this.#getWidth(),
  })
  
  render = () => {
    const { xOffset, yOffset, y, top } = this.pixelOffset

    this.p.strokeWeight(1)
    this.p.stroke(0, 0, 0)
    for(let i = 0; i < this.soundData.length; i+=2) {
      const x0 = (this.soundData[i] * this.currentScale) + xOffset
      const y0 = this.minHeight + 40 + (this.trackHeight/2) + (this.soundData[i+1] * (this.height - 20)) - (y - yOffset)
      const x1 = (this.soundData[i+2] * this.currentScale) + xOffset
      const y1 = this.minHeight + 40 + (this.trackHeight/2) + (this.soundData[i+3] * (this.height - 20)) - (y - yOffset)
      this.p.line(x0, y0, x1, y1)
    }
  }
}

export default Waveform
