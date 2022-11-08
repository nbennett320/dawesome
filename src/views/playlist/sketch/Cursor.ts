/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'
import P5ComponentBase from '../../../render/P5ComponentBase'
import { CanvasProps } from './index'

class Cursor extends P5ComponentBase<CanvasProps> {
  timelineWidth: number
  timelineHeight: number

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    timelineWidth: number,
    timelineHeight: number,
  ) {
    super(p, canvas)
    this.timelineWidth = timelineWidth
    this.timelineHeight = timelineHeight
  }

  render = () => {
    this.p.background(220)
    this.p.fill('red')
    this.p.triangle(0, 0, 8, 0, 0, 8)
  }
}

export default Cursor
