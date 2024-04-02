/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { CanvasProps, Renderer } from './index'
import { P5BoundingBox } from '../../../render/P5Component'

interface Props extends PlaylistComponentProps {}

class Timeline extends PlaylistComponent {
  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
  }

  boundingBox = (): P5BoundingBox => {
    return {
      bottom: 0,
      top: this.timelineHeight,
      left: 0,
      right: this.timelineWidth,
      height: this.timelineHeight,
      width: this.timelineWidth,
      minHeight: 0,
      maxHeight: this.timelineHeight,
    }
  }

  drawNumbers() {
    this.p.noStroke()
    this.p.fill('#222')
    this.p.textFont('Helvetica')

    const limit = 16

    for(let i = 0; i < limit; i++) {
      this.p.push()
      this.p.scale(1 / this.currentScale, 1)
      this.p.text(
        i+1, 
        (i * (this.timelineWidth * this.currentScale / limit) + 2), 
        this.timelineHeight - 2
      )
      this.p.pop()
    }
  }

  render() {
    this.p.fill('#ccc')
    this.p.rect(0, 0, this.timelineWidth, this.timelineHeight)
    this.drawNumbers()
  }
}

export default Timeline
