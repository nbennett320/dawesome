/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'
import P5ComponentBase from '../../../render/P5ComponentBase'
import { CanvasProps } from './index'

export interface TimelineSketchProps extends SketchProps {
  height: number
  width: number
  timelineHeight: number
}

class Timeline extends P5ComponentBase<CanvasProps> {
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

  drawNumbers() {
    this.p.noStroke()
    this.p.fill('#222')

    for(let i = 0; i < 32; i++) {
      this.p.text(i+1, (i * (this.timelineWidth / 32) + 2), this.timelineHeight - 2)
    }
  }

  render() {
    this.p.fill('#ccc')
    this.p.rect(0, 0, this.timelineWidth, this.timelineHeight)
    this.drawNumbers()
  }
}

export default Timeline
