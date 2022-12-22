/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps, Renderer } from './index'

interface Props extends PlaylistComponentBaseProps {}

class Timeline extends PlaylistComponentBase {
  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
  }

  drawNumbers() {
    this.p.noStroke()
    this.p.fill('#222')
    this.p.textFont('Helvetica')

    for(let i = 0; i < 32; i++) {
      this.p.push()
      this.p.scale(1 / this.currentScale, 1)
      this.p.text(
        i+1, 
        (i * (this.timelineWidth * this.currentScale / 32) + 2), 
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
