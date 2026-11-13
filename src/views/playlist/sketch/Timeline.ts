import PlaylistComponent from './PlaylistComponent'
import { Renderer } from './index'
import { P5BoundingBox } from '../../../render/P5Component'
import { paint, theme } from './theme'
import { MIN_LABEL_GAP, gridSteps, rulerLabel, visibleBeats } from './grid'
import { lineLevel } from '../../../editing/beatGrid'

class Timeline extends PlaylistComponent {
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
    // a tick on each grid line (see adaptiveStep), and bar numbers that thin
    // out so they never overlap, with bar.beat on each beat when zoomed in
    // (see rulerLabel). only the beats on screen are drawn
    const { lineEvery, labelEvery } = gridSteps(beatWidth, theme.beatsPerBar)
    // (a number is drawn to the right of its beat, so one just off the left
    // edge may still show)
    const [first, last] = visibleBeats(beatWidth, renderer.transformX, this.timelineWidth, limit, MIN_LABEL_GAP)

    // unscaled, so the numbers and ticks don't stretch when zoomed
    p.push()
    p.scale(1 / this.currentScale, 1)
    // (counted in steps, so a fine step doesn't drift)
    for(let n = Math.ceil(first / lineEvery); n * lineEvery < last; n++) {
      const i = n * lineEvery
      const x = i * beatWidth
      const level = lineLevel(i, theme.beatsPerBar)

      // a tick up from the bottom: tallest on each bar, short between beats
      paint.stroke(p, 'rulerLine', level === 0 ? 0.6 : 1)
      p.strokeWeight(1)
      p.line(x, this.timelineHeight - [3, 5, 10][level], x, this.timelineHeight)

      // (labelEvery is whole beats, so a number is always on a beat or bar)
      if(i % labelEvery === 0) {
        const bar = level === 2
        p.noStroke()
        paint.fill(p, bar ? 'rulerBarText' : 'rulerText')
        p.textStyle(bar ? p.BOLD : p.NORMAL)
        p.text(rulerLabel(i, theme.beatsPerBar), x + 3, this.timelineHeight - 8)
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
    paint.stroke(p, 'loopEnd')
    p.strokeWeight(1.5)
    p.line(x, 2, x, this.timelineHeight)
    p.noStroke()
    paint.fill(p, 'loopEnd')
    // the flag points back along the loop
    p.triangle(x, 2, x - 7, 5.5, x, 9)
  }

  render() {
    const { p } = this
    p.noStroke()
    paint.fill(p, 'ruler')
    p.rect(0, -1, this.timelineWidth, this.timelineHeight + 1)
    this.drawNumbers()

    // a line under the ruler, 1px whatever the zoom
    p.push()
    p.scale(1 / this.currentScale, 1)
    paint.stroke(p, 'rulerLine')
    p.strokeWeight(1)
    p.line(0, this.timelineHeight - .5, this.timelineWidth * this.currentScale, this.timelineHeight - .5)
    p.pop()
  }
}

export default Timeline
