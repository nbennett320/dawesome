/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import {
  P5CanvasInstance,
} from 'react-p5-wrapper'
import P5ComponentBase from '../../../render/P5ComponentBase'
import { CanvasProps } from './index'

class Cursor extends P5ComponentBase<CanvasProps> {
  timelineWidth: number
  timelineHeight: number
  currentScale: number

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    timelineWidth: number,
    timelineHeight: number,
    currentScale: number,
  ) {
    super(p, canvas)
    this.timelineWidth = timelineWidth
    this.timelineHeight = timelineHeight
    this.currentScale = currentScale
  }

  render = () => {
    this.p.push()
    this.p.scale(1 / this.currentScale, 1)
    this.p.fill('red')
    this.p.triangle(0, 0, 8, 0, 0, 8)
    this.p.line(0, 0, -1, -1 * this.timelineHeight)
    this.p.stroke(1)
    this.p.pop()
  }
}

export default Cursor
