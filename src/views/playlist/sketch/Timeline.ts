/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { CanvasProps, Renderer } from './index'
import { P5BoundingBox } from '../../../render/P5Component'
import { theme } from './theme'
import { MIN_LABEL_GAP, gridSteps, visibleBeats } from './grid'

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

    const renderer = this.renderer as Renderer
    const limit = renderer.beatsDisplayed
    const beatWidth = this.timelineWidth * this.currentScale / limit
    // zoomed out, ticks and then numbers thin out so they never overlap,
    // and only the beats on screen are drawn
    const { lineEvery, labelEvery } = gridSteps(beatWidth, theme.beatsPerBar)
    // (a number is drawn to the right of its beat, so one just off the left
    // edge may still show)
    const [first, last] = visibleBeats(beatWidth, renderer.transformX, this.timelineWidth, limit, MIN_LABEL_GAP)

    // unscaled, so the numbers and ticks don't stretch when zoomed
    p.push()
    p.scale(1 / this.currentScale, 1)
    for(let i = Math.ceil(first / lineEvery) * lineEvery; i < last; i += lineEvery) {
      const x = i * beatWidth
      const bar = i % theme.beatsPerBar === 0

      // a tick up from the bottom, taller on each bar
      p.stroke(...theme.rulerLine)
      p.strokeWeight(1)
      p.line(x, this.timelineHeight - (bar ? 10 : 5), x, this.timelineHeight)

      if(i % labelEvery === 0) {
        p.noStroke()
        p.fill(...(bar ? theme.rulerBarText : theme.rulerText))
        p.textStyle(bar ? p.BOLD : p.NORMAL)
        p.text(i + 1, x + 3, this.timelineHeight - 8)
      }
    }
    p.textStyle(p.NORMAL)

    this.drawLoopEnd(beatWidth)
    p.pop()
  }

  // where the playlist jumps back to the start, while it loops: a small flag
  // at the loop's end. called unscaled, with a beat's width in pixels
  drawLoopEnd(beatWidth: number) {
    const { p } = this
    const beat = (this.renderer as Renderer).loopEndBeat()
    if(beat === null) return

    const x = beat * beatWidth
    p.stroke(...theme.loopEnd)
    p.strokeWeight(1.5)
    p.line(x, 2, x, this.timelineHeight)
    p.noStroke()
    p.fill(...theme.loopEnd)
    // the flag points back along the loop
    p.triangle(x, 2, x - 7, 5.5, x, 9)
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
