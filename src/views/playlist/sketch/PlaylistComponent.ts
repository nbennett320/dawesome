
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import P5Component from '../../../render/P5Component'
import { CanvasProps, Renderer } from './index'

export interface PlaylistComponentProps {
  timelineWidth: number
  timelineHeight: number
  currentScale: number
}

abstract class PlaylistComponent<S = Record<string, unknown>> extends P5Component<CanvasProps, S> {
  timelineWidth: number
  timelineHeight: number
  currentScale: number

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: PlaylistComponentProps,
  ) {
    super(p, canvas, playlist)
    this.timelineWidth = props.timelineWidth
    this.timelineHeight = props.timelineHeight
    this.currentScale = props.currentScale
  }

  get playlist() { return this.renderer }

  // hit-test against the mouse in playlist x and y, since components are
  // drawn inside the zoom/pan/scroll transform but p.mouseX/Y are in canvas
  // pixels
  mouseOver = (): boolean => {
    const { top, left, bottom, right } = this.boundingBox()
    const mouseX = (this.renderer as Renderer).screenToPlaylistX(this.p.mouseX)
    const mouseY = (this.renderer as Renderer).screenToPlaylistY(this.p.mouseY)

    return (
      left <= mouseX && mouseX <= right &&
      top <= mouseY && mouseY <= bottom
    )
  }

  abstract render(): void
}

export default PlaylistComponent
