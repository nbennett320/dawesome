/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps } from './index'

interface Props extends PlaylistComponentBaseProps {}

class Cursor extends PlaylistComponentBase {
  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    props: Props,
  ) {
    super(p, canvas, props)
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
