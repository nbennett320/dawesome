/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { CanvasProps, Renderer } from './index'
import { P5BoundingBox } from '../../../render/P5Component'
import { theme } from './theme'

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
    const { p } = this
    p.textFont(theme.font)
    p.textSize(10)
    p.textAlign(p.LEFT, p.BASELINE)

    const limit = (this.renderer as Renderer).beatsDisplayed
    const beatWidth = this.timelineWidth * this.currentScale / limit

    // unscaled, so the numbers and ticks don't stretch when zoomed
    p.push()
    p.scale(1 / this.currentScale, 1)
    for(let i = 0; i < limit; i++) {
      const x = i * beatWidth
      const bar = i % theme.beatsPerBar === 0

      // a tick up from the bottom, taller on each bar
      p.stroke(...theme.rulerLine)
      p.strokeWeight(1)
      p.line(x, this.timelineHeight - (bar ? 10 : 5), x, this.timelineHeight)

      p.noStroke()
      p.fill(...(bar ? theme.rulerBarText : theme.rulerText))
      p.textStyle(bar ? p.BOLD : p.NORMAL)
      p.text(i + 1, x + 3, this.timelineHeight - 8)
    }
    p.textStyle(p.NORMAL)
    p.pop()
  }

  render() {
    const { p } = this
    p.noStroke()
    p.fill(...theme.ruler)
    p.rect(0, -1, this.timelineWidth, this.timelineHeight + 1)
    this.drawNumbers()

    // a line under the ruler, 1px whatever the zoom
    p.push()
    p.scale(1 / this.currentScale, 1)
    p.stroke(...theme.rulerLine)
    p.strokeWeight(1)
    p.line(0, this.timelineHeight - .5, this.timelineWidth * this.currentScale, this.timelineHeight - .5)
    p.pop()
  }
}

export default Timeline
