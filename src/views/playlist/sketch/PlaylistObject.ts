import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import Waveform from './Waveform'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItem } from '../../../types/playlist'
import { P5BoundingBox, State as ComponentState } from '../../../render/P5Component'

interface Props extends PlaylistComponentProps {
  playlistItem: PlaylistItem
  soundData: number[]
  duration: number
  trackNumber: number
  trackHeight: number
}

type State = ComponentState<Record<string, never>>

// room at the end of the label for the modified dot, in screen pixels
const MODIFIED_DOT_SPACE = 12

class PlaylistObject extends PlaylistComponent<State> {
  playlistItem: PlaylistItem
  soundData: number[] = []
  duration: number
  trackNumber: number
  trackHeight: number = staticDefaults.trackHeight
  labelHeight: number = staticDefaults.PlaylistObject.labelHeight
  minHeight: number
  maxHeight: number
  waveform: Waveform
  // whether the clip's settings differ from the default, shown as a dot
  modified = false

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)

    this.playlistItem = props.playlistItem
    this.soundData = props.soundData
    this.duration = props.duration
    this.trackNumber = props.trackNumber
    this.trackHeight = props?.trackHeight ?? staticDefaults.trackHeight
    this.minHeight = this.trackHeight * this.trackNumber + (this.trackNumber * .3) + staticDefaults.timelineHeight
    this.maxHeight = this.minHeight + this.trackHeight
    this.waveform = new Waveform(
      this.p,
      this.canvas,
      playlist,
      {
        duration: this.duration,
        currentScale: this.currentScale,
        timelineHeight: this.timelineHeight,
        timelineWidth: this.timelineWidth,
        soundData: this.soundData,
        startBeat: this.playlistItem.startBeat,
        trackNumber: this.playlistItem.trackNumber,
        pixelOffset: this.playlistItem.pixelOffset,
        minHeight: this.minHeight,
        maxHeight: this.maxHeight,
        trackHeight: this.trackHeight,
      }
    )
  }

  boundingBox = (): P5BoundingBox => this.waveform.boundingBox()

  // how long the clip plays for, in seconds. changes with its pitch
  setDuration = (seconds: number) => {
    this.duration = seconds
    this.waveform.duration = seconds
  }

  // call function on right click
  // eslint-disable-next-line class-methods-use-this
  onDrop = (
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _ev: MouseEvent,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _data: {
      playlistObject: PlaylistObject
      mouseX: number
      mouseY: number
    }
  ) => {}

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
    this.p.fill(224,242,254, 255*.6)
    this.p.rect(left, top, width, height, 3, 3, 3, 3)
    this.p.line(left, top + this.labelHeight, right, top + this.labelHeight)

    // render text
    this.p.noStroke()
    this.p.fill('#222')
    this.p.push()
    this.p.scale(1 / this.currentScale, 1)
    this.p.textWrap(this.p.CHAR)
    // leave room for the modified dot
    const dotSpace = this.modified ? MODIFIED_DOT_SPACE : 0
    this.p.text(
      this.playlistItem.path,
      (left + 1) * this.currentScale,
      top + 6,
      width * this.currentScale - 1 - dotSpace,
      this.labelHeight,
    )

    // a dot at the end of the label: this clip's settings were changed
    // in the Sample tab. drawn unscaled, so it stays round when zoomed
    if(this.modified) {
      this.p.fill(217, 119, 6)
      this.p.circle(
        right * this.currentScale - MODIFIED_DOT_SPACE / 2,
        top + this.labelHeight / 2,
        6,
      )
    }

    if((this.renderer as Renderer)?.debugMode) {
      // text width debug
      this.p.fill(255,187,153,255*.3)
      this.p.rect(
        (left + 1) * this.currentScale,
        top + 6,
        width * this.currentScale - 1,
        this.labelHeight,
      )
    }

    this.p.pop()
  }

  render = () => {
    const { waveform } = this

    this.drawBoundingBox()

    // render waveform
    waveform.render()

    if((this.renderer as Renderer)?.debugMode) {
      // mouse drop height debug
      const { pixelOffset } = this.playlistItem
      const { left, width } = waveform.boundingBox()
      this.p.strokeWeight(2)
      this.p.stroke(255,187,153)
      this.p.line(left, pixelOffset.yOffset, left + width, pixelOffset.yOffset)

    }
  }
}

export default PlaylistObject
