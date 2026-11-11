import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { Renderer, CanvasProps } from './index'
import { P5BoundingBox } from '../../../render/P5Component'
import { paint } from './theme'

interface Props extends PlaylistComponentProps {}

// the playhead: a line down through every track, stopping at the area below
// them (for dragging in a new track), with a pointer in the ruler. drawn in
// canvas pixels, outside the zoom/pan/scroll transform
class Cursor extends PlaylistComponent {
  // eslint-disable-next-line @typescript-eslint/no-useless-constructor
  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
  }

  // the playhead isn't clicked on (the ruler is, see Renderer)
  // eslint-disable-next-line class-methods-use-this
  boundingBox = (): P5BoundingBox => ({
    bottom: 0,
    top: 0,
    left: 0,
    right: 0,
    height: 0,
    width: 0,
    minHeight: 0,
    maxHeight: 0,
  })

  render = () => {
    const playlist = this.renderer as Renderer
    const beat = playlist.playheadBeat()
    if(beat === null) return

    const x = playlist.transformX + playlist.beatToX(beat) * playlist.currentScale
    // scrolled out of view
    if(x < -8 || x > this.p.width + 8) return

    // sky while playing or being moved, grey while stopped
    const colour = playlist.playing || playlist.seekBeat !== null
      ? 'playhead'
      : 'playheadStopped'

    this.p.push()
    paint.stroke(this.p, colour)
    this.p.strokeWeight(1.5)
    // the last track's bottom, scrolled (the tracks are drawn 1px down)
    const bottom = Math.min(this.p.height, playlist.newTrackAreaTop() + 1 - playlist.scrollY)
    this.p.line(x, 0, x, bottom)
    this.p.noStroke()
    paint.fill(this.p, colour)
    this.p.triangle(x - 5, 0, x + 5, 0, x, 8)
    this.p.pop()
  }
}

export default Cursor
