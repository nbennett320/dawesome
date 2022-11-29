import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'
import RendererBase from './RendererBase'

abstract class P5ComponentBase<T extends SketchProps> {
  p: P5CanvasInstance<T>
  canvas: p5.Renderer
  renderer: RendererBase

  constructor(
    p: P5CanvasInstance<T>,
    canvas: p5.Renderer,
    renderer: RendererBase
  ) {
    this.p = p
    this.canvas = canvas
    this.renderer = renderer
  }

  abstract render(): void
}

export default P5ComponentBase
