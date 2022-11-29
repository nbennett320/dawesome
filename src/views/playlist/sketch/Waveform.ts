import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps, Renderer } from './index'
import { PlaylistItemPixelOffset } from '../../../types/playlist'

interface Props extends PlaylistComponentBaseProps {
  // id: number
  soundData: number[]
  trackNumber: number
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
    const [min, max] = playlist.calculateTrackRange(props.trackNumber, props.pixelOffset)
    this.minHeight = min
    this.maxHeight = max
    this.height = max - min
    // console.log(props.trackNumber, min, max, this.height, props.pixelOffset.y, props.pixelOffset.yOffset)
  }
  
  render = () => {
    const { xOffset, yOffset, y, top } = this.pixelOffset

    for(let i = 0; i < this.soundData.length; i+=2) {
      const x0 = (this.soundData[i] * this.currentScale) + xOffset
      const y0 = this.minHeight + (this.maxHeight/2) + (this.soundData[i+1] * (this.height/2)) - this.minHeight + (y - yOffset) * this.trackNumber
      const x1 = (this.soundData[i+2] * this.currentScale) + xOffset
      const y1 = this.minHeight + (this.maxHeight/2) + (this.soundData[i+3] * (this.height/2)) - this.minHeight + (y - yOffset) * this.trackNumber
      this.p.stroke(1)
      this.p.color('red')
      this.p.line(x0, y0, x1, y1)
    }
  }
}

export default Waveform
