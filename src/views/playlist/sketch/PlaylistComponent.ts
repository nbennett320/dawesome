
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import P5Component from '../../../render/P5Component'
import { CanvasProps, Renderer } from './index'

export interface PlaylistComponentProps {
  timelineWidth: number
  timelineHeight: number
  currentScale: number
}

abstract class PlaylistComponent extends P5Component<CanvasProps> {
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

  abstract render(): void
}

export default PlaylistComponent
