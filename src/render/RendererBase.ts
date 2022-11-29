import p5 from 'p5'
import { Sketch } from 'react-p5-wrapper'

abstract class RendererBase {
  canvas: p5.Renderer | null
  abstract height: number
  abstract width: number

  constructor() {
    this.canvas = null
  }

  setHeight = (height: number) => { this.height = height }
  setWidth = (width: number) => { this.width = width }
  
  // render the p5 sketch. 
  // warning: method should only be implemented, not called
  // because p5 will run this method
  abstract sketch: Sketch
}

export default RendererBase
