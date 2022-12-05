import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import Waveform from './Waveform'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItem } from '../../../types/playlist'

interface Props extends PlaylistComponentBaseProps {
  playlistItem: PlaylistItem
  soundData: number[]
  trackNumber: number
  trackHeight: number
}

class PlaylistObject extends PlaylistComponentBase {
  playlistItem: PlaylistItem
  soundData: number[] = []
  trackNumber: number
  trackHeight: number = staticDefaults.trackHeight
  minHeight: number
  maxHeight: number
  waveform: Waveform

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)
    this.playlistItem = props.playlistItem
    this.soundData = props.soundData
    this.trackNumber = props.trackNumber
    this.trackHeight = props?.trackHeight ?? staticDefaults.trackHeight
    this.minHeight = this.trackHeight * this.trackNumber + (this.trackNumber * .3)
    this.maxHeight = this.minHeight + this.trackHeight
    this.waveform = this.getWaveform()
  }

  getWaveform = (): Waveform => (
    new Waveform(
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
        minHeight: this.minHeight,
        maxHeight: this.maxHeight,
        trackHeight: this.trackHeight,
      }
    )
  )

  drawBoundingBox = () => {
    const {
      top,
      bottom,
      left,
      right,
      height,
      width,
    } = this.waveform.boundingBox()
    this.p.strokeWeight(1)
    this.p.stroke(125,211,252)
    this.p.fill(224,242,254)
    this.p.rect(left, top, width, height, 3, 3, 3, 3)
    // this.p.line(left, top, right, top)
    this.p.line(left, top + 20, right, top + 20)

    // render text
    this.p.noStroke()
    this.p.fill('#222')
    this.p.push()
    this.p.scale(1 / this.currentScale, 1)
    this.p.text(this.playlistItem.path, left + 3, top + 20 - 3)
    this.p.pop()
    // this.p.line(left, bottom, right, bottom)
    // this.p.line(left, top, left, bottom)
    // this.p.line(right, top, right, bottom)

  }

  render = () => {
    const { waveform } = this

    this.drawBoundingBox()

    // render waveform
    waveform.render()

    const { pixelOffset } = this.playlistItem
    this.p.strokeWeight(2)
    this.p.stroke(255,187,153)
    this.p.line(pixelOffset.xOffset, pixelOffset.yOffset, pixelOffset.xOffset+100, pixelOffset.yOffset)
  }
}

export default PlaylistObject
