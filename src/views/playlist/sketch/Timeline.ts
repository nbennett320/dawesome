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
    const { beatsPerBar } = renderer
    const limit = renderer.beatsDisplayed
    const beatWidth = this.timelineWidth * this.currentScale / limit
    // a tick on each grid line (see adaptiveStep), and bar numbers that thin
    // out so they never overlap, with bar.beat on each beat when zoomed in
    // (see rulerLabel). only the beats on screen are drawn
    const { lineEvery, labelEvery } = gridSteps(beatWidth, beatsPerBar)
    // (a number is drawn to the right of its beat, so one just off the left
    // edge may still show)
    const [first, last] = visibleBeats(beatWidth, renderer.transformX, this.timelineWidth, limit, MIN_LABEL_GAP)

    // unscaled, so the numbers and ticks don't stretch when zoomed
    p.push()
    p.scale(1 / this.currentScale, 1)
    // (under the numbers, so they stay readable)
    this.drawLoopBand(beatWidth)
    // (counted in steps, so a fine step doesn't drift)
    for(let n = Math.ceil(first / lineEvery); n * lineEvery < last; n++) {
      const i = n * lineEvery
      const x = i * beatWidth
      const level = lineLevel(i, beatsPerBar)

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
        p.text(rulerLabel(i, beatsPerBar), x + 3, this.timelineHeight - 8)
      }
    }
    p.textStyle(p.NORMAL)

    this.drawLoopFlags(beatWidth)
    p.pop()
  }

  // whether the playlist loops: a region is still drawn while it doesn't,
  // dimmed
  #loopAlpha = (): number => ((this.renderer as Renderer).transport?.loopEnabled ? 1 : 0.4)

  // the loop region, if there is one: a band along the ruler. called
  // unscaled, with a beat's width in pixels
  drawLoopBand(beatWidth: number) {
    const { p } = this
    const { region } = (this.renderer as Renderer).loopMarks()
    if(!region) return

    p.noStroke()
    paint.fill(p, 'loopRegion', this.#loopAlpha())
    p.rect(region.start * beatWidth, 0, (region.end - region.start) * beatWidth, this.timelineHeight)
  }

  // where the playlist goes round, while it loops: a small flag at the
  // loop's end, pointing back along it. a region has one at its start too,
  // pointing in. called unscaled, with a beat's width in pixels
  drawLoopFlags(beatWidth: number) {
    const { region, flag } = (this.renderer as Renderer).loopMarks()
    if(region) {
      const alpha = this.#loopAlpha()
      this.#drawFlag(region.start * beatWidth, 1, alpha)
      this.#drawFlag(region.end * beatWidth, -1, alpha)
    } else if(flag !== null) {
      this.#drawFlag(flag * beatWidth, -1, 1)
    }
  }

  // a flag on a line at `x`, pointing right (1) or left (-1)
  #drawFlag(x: number, points: 1 | -1, alpha: number) {
    const { p } = this
    paint.stroke(p, 'loopEnd', alpha)
    p.strokeWeight(1.5)
    p.line(x, 2, x, this.timelineHeight)
    p.noStroke()
    paint.fill(p, 'loopEnd', alpha)
    p.triangle(x, 2, x + 7 * points, 5.5, x, 9)
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
