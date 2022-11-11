import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps } from './index'

interface Props extends PlaylistComponentBaseProps {
  // id: number
  soundData: number[]
  // pathd: string
  // viewBox: string
}

class Waveform extends PlaylistComponentBase {
  // id: number
  soundData: number[]

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    props: Props,
  ) {
    super(p, canvas, props)
    this.soundData = props.soundData
    // this.pathd = props.pathd
    // this.viewBox = props.viewBox
  }
  
  render = () => {
    console.log("rendering waveform", this.soundData)
    this.p.push()

    for(let i = 0; i < this.soundData.length; i+=2) {
      const x0 = this.soundData[i]
      const y0 = this.soundData[i+1]
      const x1 = this.soundData[i+2]
      const y1 = this.soundData[i+3]
      this.p.stroke(3)
      this.p.color('red')
      this.p.line(x0, y0 * 5 - 200, x1, y1 * 5 - 200)
    }

    // this.p.noLoop()
    
    this.p.pop()
  }
}

export default Waveform
