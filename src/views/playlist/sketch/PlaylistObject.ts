import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import Waveform from './Waveform'
import { CanvasProps, Renderer } from './index'
import { PlaylistItem } from '../../../types/playlist'

interface Props extends PlaylistComponentBaseProps {
  playlistItem: PlaylistItem
  soundData: number[]
}

class PlaylistObject extends PlaylistComponentBase {
  playlistItem: PlaylistItem
  soundData: number[] = []

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
    this.playlistItem = props.playlistItem
    this.soundData = props.soundData
  }

  render = () => {
    const waveform = new Waveform(
      this.p,
      this.canvas,
      this.renderer,
      {
        currentScale: this.currentScale,
        timelineHeight: this.timelineHeight,
        timelineWidth: this.timelineWidth,
        soundData: this.soundData,
        trackNumber: this.playlistItem.trackNumber,
        pixelOffset: this.playlistItem.pixelOffset,
      }
    )

    waveform.render()

    const { pixelOffset } = this.playlistItem
    this.p.strokeWeight(2)
    this.p.stroke(255,187,153)
    this.p.line(pixelOffset.xOffset, pixelOffset.yOffset, pixelOffset.xOffset+100, pixelOffset.yOffset)
  }
}

export default PlaylistObject
