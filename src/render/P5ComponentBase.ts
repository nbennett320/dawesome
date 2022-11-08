import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'

abstract class P5ComponentBase<T extends SketchProps> {
  p: P5CanvasInstance<T>
  canvas: p5.Renderer

  constructor(
    p: P5CanvasInstance<T>,
    canvas: p5.Renderer
  ) {
    this.p = p
    this.canvas = canvas
  }

  abstract render(): void
}

export default P5ComponentBase
