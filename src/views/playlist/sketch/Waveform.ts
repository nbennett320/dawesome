import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItemPixelOffset } from '../../../types/playlist'

interface Props extends PlaylistComponentBaseProps {
  // id: number
  soundData: number[]
  duration: number
  trackNumber: number
  minHeight: number
  maxHeight: number
  trackHeight: number
  pixelOffset: PlaylistItemPixelOffset
}

class Waveform extends PlaylistComponentBase {
  soundData: number[]
  duration: number
  trackNumber: number
  pixelOffset: PlaylistItemPixelOffset
  height: number
  maxHeight: number
  minHeight: number
  trackHeight: number
  labelHeight: number = staticDefaults.PlaylistObject.labelHeight

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
    this.soundData = props.soundData
    this.duration = props.duration
    this.trackNumber = props.trackNumber
    this.pixelOffset = props.pixelOffset
    this.minHeight = props.minHeight
    this.maxHeight = props.maxHeight
    this.height = props.maxHeight - props.minHeight
    this.trackHeight = props.trackHeight
  }

  #getWidth = () => 
    (this.soundData.length / 2) * (this.renderer as Renderer).timePerPixel()

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
    const timePerPixel = (this.renderer as Renderer).timePerPixel()
    const { currentScale, minHeight, trackHeight, height, labelHeight } = this
    const { xOffset, yOffset, y } = this.pixelOffset

    this.p.strokeWeight(.3)
    this.p.stroke(0, 0, 0)
    for(let i = 0; i < this.soundData.length; i+=2) {
      const x0 = (this.soundData[i] * currentScale * timePerPixel) + xOffset
      const y0 = minHeight + labelHeight*2 + (trackHeight/2) + (this.soundData[i+1] * (height - labelHeight)) - (y - yOffset)
      const x1 = (this.soundData[i+2] * currentScale * timePerPixel) + xOffset
      const y1 = minHeight + labelHeight*2 + (trackHeight/2) + (this.soundData[i+3] * (height - labelHeight)) - (y - yOffset)
      this.p.line(x0, y0, x1, y1)
    }
  }
}

export default Waveform
