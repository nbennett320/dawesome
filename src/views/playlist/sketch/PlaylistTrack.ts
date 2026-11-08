import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import { CanvasProps, Renderer, staticDefaults } from './index'
import PlaylistObject from './PlaylistObject'
import { P5BoundingBox, State as ComponentState } from '../../../render/P5Component'

interface Props extends PlaylistComponentProps {
  trackNumber: number
  trackCount: number
  trackHeight?: number
  playlistObjects: Array<PlaylistObject>
}

type State = ComponentState<{
  
}>

class PlaylistTrack extends PlaylistComponent<State> {
  trackNumber: number
  trackCount: number
  trackHeight: number = staticDefaults.trackHeight
  tabHeight: number = staticDefaults.PlaylistObject.tabHeight
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

  boundingBox = (): P5BoundingBox => {
    return {
      bottom: 0,
      top: 0,
      left: 0,
      right: 0,
      width: 0,
      height: 0,
      minHeight: this.minHeight,
      maxHeight: this.maxHeight,
    }
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
    // a clip being dragged is drawn by the Renderer, pinned to the mouse
    const { clipDrag } = this.renderer as Renderer
    const dragged = clipDrag?.active ? clipDrag.item : null

    playlistObjects
      .filter(item => item.playlistItem.trackNumber === this.trackNumber)
      .filter(item => item !== dragged)
      .forEach(item => {
        item.render()

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
          ;(this.renderer as Renderer).clipDrag = null

          // reset cursor to arrow after removing an item from the playlist
          setTimeout(() => {
            p.cursor(p.ARROW)
          })
        }

        // double click is handled by the Renderer, which finds the clip
        // under the mouse when the canvas is double clicked
    })

    // render track lines
    p.stroke(0, 0, 0)
    p.strokeWeight(.3)
    p.line(0, minHeight, timelineWidth, minHeight)
    p.line(0, maxHeight+.3, timelineWidth, maxHeight+.3)
  }
}

export default PlaylistTrack
