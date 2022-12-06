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
  labelHeight: number = staticDefaults.PlaylistObject.labelHeight
  minHeight: number
  maxHeight: number
  waveform: Waveform
  #mouseWasPressed = 0
  #mouseWasDoubleClicked = false
  #mousePendingDoubleClick = false

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
    this.p.line(left, top + this.labelHeight, right, top + this.labelHeight)

    // render text
    this.p.noStroke()
    this.p.fill('#222')
    this.p.push()
    this.p.scale(1 / this.currentScale, 1)
    this.p.text(this.playlistItem.path, left + 3, top + this.labelHeight - 3)
    this.p.pop()
  }

  // return true if mouse is over the particular playlist object
  isMouseOver = (): boolean => {
    const { mouseX, mouseY } = this.p
    const { top, left, bottom, right } = this.waveform.boundingBox()

    return (
      left <= mouseX && mouseX <= right &&
      top <= mouseY && mouseY <= bottom
    )
  }

  // call a function when the mouse is over this component
  onMouseOver = (fn: () => void) => {
    if(this.isMouseOver()) {
      fn()
    }
  }

  // call a function when clicking on this component
  onClick = (
    fn: (ev: {
      playlistObject: PlaylistObject
      mouseX: number
      mouseY: number
    }) => void
  ) => {
    if(this.isMouseOver() && this.p.mouseIsPressed && !this.#mouseWasPressed) {
      this.#mouseWasPressed += 1
      return
    }

    // on release
    if(this.isMouseOver() && !this.p.mouseIsPressed && this.#mouseWasPressed) {
      const { mouseX, mouseY } = this.p
      fn({ 
        playlistObject: this,
        mouseX, 
        mouseY,
      })

      this.#mousePendingDoubleClick = true
      setTimeout(() => {
        this.#mouseWasPressed = 0
        this.#mousePendingDoubleClick = false
      }, 200)
    }
  }

  // call a function when double clicking on this component
  onDoubleClick = (
    fn: (ev: {
      playlistObject: PlaylistObject
      mouseX: number
      mouseY: number
    }) => void
  ) => {
    let timeout: NodeJS.Timeout | null = null

    // initial click
    if(
      this.isMouseOver() && this.p.mouseIsPressed && 
      this.#mouseWasPressed && !this.#mouseWasDoubleClicked &&
      this.#mousePendingDoubleClick
    ) {
      this.#mouseWasPressed += 1
      this.#mouseWasDoubleClicked = true

      timeout = setTimeout(() => {
        this.#mouseWasDoubleClicked = false
      }, 200)
      return
    }

    // handle second click
    if(
      this.isMouseOver() && !this.p.mouseIsPressed &&
      this.#mouseWasPressed === 2 && this.#mouseWasDoubleClicked
    ) {
      const { mouseX, mouseY } = this.p
      fn({ 
        playlistObject: this,
        mouseX, 
        mouseY,
      })

      this.#mouseWasDoubleClicked = false
      if(timeout) { clearTimeout(timeout) }
    }
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
