import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import Waveform from './Waveform'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItem } from '../../../types/playlist'
import { P5BoundingBox } from '../../../render/P5Component'

interface Props extends PlaylistComponentProps {
  playlistItem: PlaylistItem
  soundData: number[]
  duration: number
  trackNumber: number
  trackHeight: number
}

class PlaylistObject extends PlaylistComponent {
  playlistItem: PlaylistItem
  soundData: number[] = []
  duration: number
  trackNumber: number
  trackHeight: number = staticDefaults.trackHeight
  labelHeight: number = staticDefaults.PlaylistObject.labelHeight
  minHeight: number
  maxHeight: number
  waveform: Waveform
  isDragging = false

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
        trackNumber: this.playlistItem.trackNumber,
        pixelOffset: this.playlistItem.pixelOffset,
        minHeight: this.minHeight,
        maxHeight: this.maxHeight,
        trackHeight: this.trackHeight,
      }
    )
  }

  boundingBox = (): P5BoundingBox => this.waveform.boundingBox()

  // return true if mouse is over the particular playlist object
  // isMouseOver = (): boolean => {
  //   const { mouseX, mouseY } = this.p
  //   const { top, left, bottom, right } = this.waveform.boundingBox()

  //   return (
  //     left <= mouseX && mouseX <= right &&
  //     top <= mouseY && mouseY <= bottom
  //   )
  // }

  // call a function when the mouse is over this component
  // onMouseOver = (fn: () => void) => {
  //   if(this.isMouseOver()) {
  //     fn()
  //   }
  // }

  // // call a function when clicking on this component
  // onClick = (
  //   fn: (
  //     ev: MouseEvent,
  //     data: {
  //       playlistObject: PlaylistObject
  //       mouseX: number
  //       mouseY: number
  //     }
  //   ) => void
  // ) => {
  //   this.onMouseOver(() => {
  //     this.canvas.mouseClicked((ev: MouseEvent) => {
  //       const { mouseX, mouseY } = this.p

  //       fn(ev, { 
  //         mouseX, 
  //         mouseY,
  //         playlistObject: this,
  //       })
  //     })
  //   })
  // }

  // call a function on dragging an item
  onDrag = (
    dragFn: () => void,
    dropFn: () => void,
  ) => {
    this.onMouseOver(() => {
      if((this.renderer as Renderer).isMouseDragged) {
        this.canvas.mouseReleased(() => {
          this.isDragging = false
          console.log("it dropped")
          dropFn()
        })

        this.isDragging = true
      }
    })

    dragFn()
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
    this.p.text(
      this.playlistItem.path,
      (left + 1) * this.currentScale,
      top + 6,
      width * this.currentScale - 1,
      this.labelHeight,
    )

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
      this.p.strokeWeight(2)
      this.p.stroke(255,187,153)
      this.p.line(pixelOffset.xOffset, pixelOffset.yOffset, pixelOffset.xOffset + waveform.boundingBox().width, pixelOffset.yOffset)

    }
  }
}

export default PlaylistObject
