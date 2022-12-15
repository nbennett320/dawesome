/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { Renderer, CanvasProps } from './index'

interface Props extends PlaylistComponentBaseProps {}

class Cursor extends PlaylistComponentBase {
  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
  }

  #currentRatio = (): number | null => {
    const now = Date.now()
    const { playlistStart } = this.renderer as Renderer

    if(!playlistStart) return null

    const delta = now - playlistStart
    const ratio = delta / ((this.renderer as Renderer).maxPlaylistBeats / 4)
    return ratio * this.currentScale
  }

  drawTriangle = () => {
    const ratio = this.#currentRatio()

    if(!ratio) return

    this.p.triangle(0 + ratio, 0, 8+ ratio, 0, 0 + ratio, 8)
    this.p.stroke(1)
  }

  drawCursorLine = () => {
    const ratio = this.#currentRatio()

    if(!ratio) return

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
