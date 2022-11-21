import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps } from './index'

interface Props extends PlaylistComponentBaseProps {
  // id: number
  soundData: number[]
  trackNumber: number
  pixelOffset: number
}

class Waveform extends PlaylistComponentBase {
  // id: number
  soundData: number[]
  trackNumber: number
  pixelOffset: number
  height: number
  maxHeight: number
  minHeight: number

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    props: Props,
  ) {
    super(p, canvas, props)
    this.soundData = props.soundData
    this.trackNumber = props.trackNumber
    this.pixelOffset = props.pixelOffset * this.currentScale
    this.height = props.timelineHeight / props.trackNumber
    this.maxHeight = this.height * (props.trackNumber + 1)
    this.minHeight = this.maxHeight - this.height
  }
  
  render = () => {
    for(let i = 0; i < this.soundData.length; i+=2) {
      const x0 = (this.soundData[i] + this.pixelOffset) * this.currentScale
      const y0 = this.soundData[i+1] * this.height + this.maxHeight
      const x1 = (this.soundData[i+2] + this.pixelOffset) * this.currentScale
      const y1 = this.soundData[i+3] * this.height + this.maxHeight
      this.p.stroke(3)
      this.p.color('red')
      this.p.line(x0, y0, x1, y1)
    }
  }
}

export default Waveform
