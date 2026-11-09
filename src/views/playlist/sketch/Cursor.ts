/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { Renderer, CanvasProps } from './index'
import { P5BoundingBox } from '../../../render/P5Component'

interface Props extends PlaylistComponentProps {}

class Cursor extends PlaylistComponent {
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
      top: 0,
      left: 0,
      right: 0,
      height: 0,
      width: 0,
      minHeight: 0,
      maxHeight: 0,
    }
  }

  // x position of the playhead, in the same units clips are drawn in
  #currentRatio = (): number | null => {
    const { transport, width } = this.renderer as Renderer

    if(!transport) return null

    const { startTimeMs, beatIntervalMs, loopBeats, beatsDisplayed, loopEnabled } = transport
    const elapsedMs = Math.max(0, Date.now() - startTimeMs)

    // the engine jumps back to the start after `loopBeats` beats
    const loopMs = beatIntervalMs * loopBeats
    const positionMs = loopEnabled && loopMs > 0 ? elapsedMs % loopMs : elapsedMs

    // the full playlist width spans `beatsDisplayed` beats
    const displayedMs = beatIntervalMs * beatsDisplayed
    const x = positionMs / displayedMs * width

    return x * this.currentScale
  }

  drawTriangle = () => {
    const ratio = this.#currentRatio()

    if(ratio === null) return

    this.p.triangle(0 + ratio, 0, 8+ ratio, 0, 0 + ratio, 8)
    this.p.stroke(1)
  }

  drawCursorLine = () => {
    const ratio = this.#currentRatio()

    if(ratio === null) return

    this.p.line(0 + ratio, 0, 0 + ratio, this.renderer.height)
  }

  render = () => {
    const { playing } = this.renderer as Renderer

    this.p.push()

    this.p.scale(1 / this.currentScale, 1)
    this.p.fill(playing ? 'limegreen' : 'darkorange')

    this.drawTriangle()
    this.drawCursorLine()

    this.p.pop()
  }
}

export default Cursor
