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
    this.minHeight = this.trackHeight * this.trackNumber + (this.trackNumber * .3)
    this.maxHeight = this.minHeight + this.trackHeight
    this.playlistObjects = props.playlistObjects
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
          if(ev.button === 0) {
            // handle left click
            console.log("left clicked on item: ", item.playlistItem.path, ev)
          } else if(ev.button === 2) {
            // handle right click
            ev.preventDefault()
            console.log("right clicked on item: ", item.playlistItem.path, ev)
          }
        })

        item.onDoubleClick((ev, data) => {
          console.log("yay it double clicked", ev)
        })
        
        item.onDrag(ev => {
          console.log("dragging!!!")
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
