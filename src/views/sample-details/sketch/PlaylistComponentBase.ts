
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import P5ComponentBase from '../../../render/P5ComponentBase'
import { CanvasProps, Renderer } from './index'

export interface PlaylistComponentBaseProps {
  timelineWidth: number
  timelineHeight: number
  currentScale: number
}

abstract class PlaylistComponentBase extends P5ComponentBase<CanvasProps> {
  timelineWidth: number
  timelineHeight: number
  currentScale: number

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    renderer: Renderer,
    props: PlaylistComponentBaseProps,
  ) {
    super(p, canvas, renderer)
    this.timelineWidth = props.timelineWidth
    this.timelineHeight = props.timelineHeight
    this.currentScale = props.currentScale
  }

  abstract render(): void
}

export default PlaylistComponentBase
