import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps, Renderer, staticDefaults } from './index'

interface Props extends PlaylistComponentBaseProps {
  trackNumber: number
  trackCount: number
  trackHeight?: number
}

class PlaylistTrack extends PlaylistComponentBase {
  trackNumber: number
  trackCount: number
  trackHeight: number = staticDefaults.trackHeight
  minHeight: number
  maxHeight: number

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
    this.trackNumber = props.trackNumber
    this.trackCount = props.trackCount
    this.trackHeight = props?.trackHeight ?? staticDefaults.trackHeight
    this.minHeight = this.trackHeight * this.trackNumber + (this.trackNumber * .3)
    this.maxHeight = this.minHeight + this.trackHeight
  }

  render = () => {
    const { 
      p,
      timelineWidth,
      minHeight,
      maxHeight,
    } = this

    p.stroke(0, 0, 0)
    p.strokeWeight(.3)
    p.line(0, minHeight, timelineWidth, minHeight)
    p.line(0, maxHeight+.3, timelineWidth, maxHeight+.3)
  }
}

export default PlaylistTrack
