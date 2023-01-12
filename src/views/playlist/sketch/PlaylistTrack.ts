import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps, Renderer, staticDefaults } from './index'
import PlaylistObject from './PlaylistObject'

interface Props extends PlaylistComponentBaseProps {
  trackNumber: number
  trackCount: number
  trackHeight?: number
  playlistObjects: Array<PlaylistObject>
}

class PlaylistTrack extends PlaylistComponentBase {
  trackNumber: number
  trackCount: number
  trackHeight: number = staticDefaults.trackHeight
  minHeight: number
  maxHeight: number
  playlistObjects: Array<PlaylistObject>

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
    this.minHeight = this.trackHeight * this.trackNumber + (this.trackNumber * .3) + staticDefaults.timelineHeight
    this.maxHeight = this.minHeight + this.trackHeight
    this.playlistObjects = props.playlistObjects
  }

  drawDraggingPlaylistItem = (item: PlaylistObject) => {
    const { 
      p,
      minHeight,
    } = this

    const dragX = p.pmouseX
    const dragY = p.pmouseY
    const {
      top,
      left,
      right,
      height,
      width,
    } = item.waveform.boundingBox()

    // draw bounding box
    this.p.strokeWeight(1)
    this.p.stroke(125,211,252)
    this.p.fill(224,242,254, 255*.3)
    this.p.rect(dragX, dragY, width, height, 3, 3, 3, 3)
    this.p.line(dragX, dragY + item.labelHeight, dragX + width, dragY + item.labelHeight)

    // draw item waveform
    const timePerPixel = (this.renderer as Renderer).timePerPixel()
    const { currentScale, trackHeight, labelHeight } = item.waveform
    const { xOffset, yOffset, y } = item.waveform.pixelOffset

    this.p.strokeWeight(.3)
    this.p.stroke(0, 0, 0, 255*.3)
    for(let i = 0; i < item.waveform.soundData.length; i+=2) {
      const x0 = (item.waveform.soundData[i] * currentScale * timePerPixel) + xOffset + dragX - left
      const y0 = minHeight + labelHeight*2 + (trackHeight/2) + (item.waveform.soundData[i+1] * (height - labelHeight)) - (y - yOffset) + dragY - top
      const x1 = (item.waveform.soundData[i+2] * currentScale * timePerPixel) + xOffset + dragX - left
      const y1 = minHeight + labelHeight*2 + (trackHeight/2) + (item.waveform.soundData[i+3] * (height - labelHeight)) - (y - yOffset) + dragY - top
      this.p.line(x0, y0, x1, y1)
    }

    // draw item text
    this.p.noStroke()
    this.p.fill(34,34,34,255*.3)
    this.p.push()
    this.p.scale(1 / this.currentScale, 1)
    this.p.textWrap(this.p.CHAR)
    this.p.text(
      item.playlistItem.path,
      (dragX + 1) * this.currentScale,
      dragY + 6,
      width * this.currentScale - 1,
      item.labelHeight,
    )
  }

  render = () => {
    const { 
      p,
      timelineWidth,
      minHeight,
      maxHeight,
      playlistObjects,
    } = this

    // render audio nodes in the playlist
    p.stroke(0, 0, 0)
    p.strokeWeight(.6)
    playlistObjects
      .filter(item => item.playlistItem.trackNumber === this.trackNumber)
      .forEach(item => {
        item.render()

        if(item.isMouseOver()) {
          p.cursor(p.HAND)
        } else {
          p.cursor(p.ARROW)
        }

        item.onClick((ev, data) => {
          if(p.mouseButton === p.LEFT) {
            // handle left click
            console.log("left clicked on item: ", item.playlistItem.path, ev)
          } 
        })

        // handle right click
        // right click function is called in Renderer.#handleRightClick
        // because p5's left/right click handle appears to not be working
        // in the webview canvas
        item.onRightClick = (ev, data) => {
          console.log("right clicked on item: ", item.playlistItem.path, ev)

          const { id } = item.playlistItem;
          (this.renderer as Renderer).onNodeRightClick(id);
          (this.renderer as Renderer).isMouseDragged = false
          item.isDragging = false

          // reset cursor to arrow after removing an item from the playlist
          setTimeout(() => {
            p.cursor(p.ARROW)
          })
        }

        item.onDoubleClick((ev, data) => {
          console.log("yay it double clicked", ev)
        })
        
        item.onDrag(() => {
          if(item.isDragging) {
            p.cursor('grabbing')
            this.drawDraggingPlaylistItem(item)
          }
        }, () => {
          p.cursor(p.ARROW)
          const dropCoords = {
            x: p.winMouseX,
            y: p.winMouseY,
          };

          (this.renderer as Renderer).onNodeMove(item.playlistItem, dropCoords)
        })
    })

    // render track lines
    p.stroke(0, 0, 0)
    p.strokeWeight(.3)
    p.line(0, minHeight, timelineWidth, minHeight)
    p.line(0, maxHeight+.3, timelineWidth, maxHeight+.3)
  }
}

export default PlaylistTrack
