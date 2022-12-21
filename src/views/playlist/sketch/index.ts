/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
  Sketch,
} from 'react-p5-wrapper'
import { invoke } from '@tauri-apps/api'
import Timeline from './Timeline'
import Cursor from './Cursor'
import PlaylistTrack from './PlaylistTrack'
import PlaylistObject from './PlaylistObject'
import RendererBase from '../../../render/RendererBase'
import {
  PlaylistItem,
  PlaylistItemPixelOffset,
  PlaylistWindow
} from '../../../types/playlist'

export interface CanvasProps extends SketchProps {
  height: number
  width: number
  playing: boolean,
  maxPlaylistBeats: number
  trackCount: number
  playlistObjects: PlaylistItem[]
  onItemDrop: (pw: PlaylistWindow) => void
  onNodeRightClick: (id: number) => void
  onNodeMove: () => void
}

export const staticDefaults = {
  zoomSensitivity: .035,
  mouseDragDetectionThreshold: 10,
  timelineHeight: 24,
  trackHeight: 100,
  playing: false,
  debugMode: false,

  // defaults on Renderer construction, these will be reassigned
  // when the canvas renders
  height: 100,
  width: 200,
  maxPlaylistBeats: 32,
  trackCount: 5,

  // class specific defaults
  PlaylistObject: {
    labelHeight: 20,
  },
}

// get waveform data and duration of individual audio nodes
const fetchNodeData = async (id: number) => {
  const [wf, dur] = await invoke<[number[], number]>('get_node_data', { id })

  return {
    wf,
    dur,
  }
}

export class Renderer extends RendererBase {
  canvasEl = document.getElementById('playlist-canvas')
  height: number = staticDefaults.height
  width: number = staticDefaults.width
  maxPlaylistBeats: number = staticDefaults.maxPlaylistBeats
  trackCount: number = staticDefaults.trackCount
  debugMode: boolean = staticDefaults.debugMode

  currentScale = 1
  transformX = 0
  transformY = 0
  
  isMouseDragged = false
  mousePressedX: number | null = null
  mousePressedY: number | null = null

  playing: boolean = staticDefaults.playing
  playlistStart: number | null = null

  playlistTracks: Array<PlaylistTrack> = []
  playlistObjects: Array<PlaylistObject> = []

  // reassigned function definitions
  onNodeRightClick!: (id: number) => void
  onNodeMove!: (
    item: PlaylistItem,
    dropCoords: { x: number, y: number },
  ) => void

  // return the current zoom/scale
  getCurrentScale = (): number => this.currentScale

  // calculate the height of playlist tracks
  individualTrackHeight = (): number => 
    (this.height - staticDefaults.timelineHeight) / this.trackCount

  // calculate the min and max height of a particular track
  calculateTrackRange = (
    trackNumber: number,
    dropData: PlaylistItemPixelOffset,
  ): [number, number] => {
    if(trackNumber > this.trackCount) {
      console.error(`Invalid trackNumber passed to calculate track range: ${trackNumber}`)
    }

    const individualTrackHeight = this.individualTrackHeight()
    const playlistStart = dropData.top - staticDefaults.timelineHeight
    
    let min = -1
    let max = -1
    for(let i = 0; i < this.trackCount; i++) {
      min = i * individualTrackHeight + playlistStart
      max = (i+1) * individualTrackHeight + playlistStart

      if(i === trackNumber) {
        break
      }
    }

    return [min, max]
  }

  calculateTrackNumber = (dropData: PlaylistItemPixelOffset): number => {
    const dropY = dropData.y - dropData.top

    console.log("obj", this.playlistTracks, this)
    let trackNumber = -1
    for(let i = 0; i < this.playlistTracks.length; i++) {
      const track = this.playlistTracks[i]
      const [min, max] = [track.minHeight, track.maxHeight]
      console.log("min, max, dropy", min, max, dropY)
      
      if(min < dropY && dropY < max) {
        trackNumber = track.trackNumber
        break
      }
    }

    console.log("calculated trackNumber: ", trackNumber)
    return trackNumber
  }


  #handleRightClick = (ev: MouseEvent, p: P5CanvasInstance<CanvasProps>) => {
    console.log("handled right click", ev)

    this.playlistObjects.forEach(item => {
      if(item.isMouseOver()) {
        const { mouseX, mouseY } = p
        item.onRightClick(ev, {
          mouseX,
          mouseY,
          playlistObject: item,
        })
      }
    })
  }

  // p5 sketch function
  sketch: Sketch<CanvasProps> = (p: P5CanvasInstance<CanvasProps>) => {
    let {
      canvas,
      height,
      width,
      maxPlaylistBeats,
      trackCount,
      currentScale,
      transformX,
      transformY,
      mousePressedX,
      mousePressedY,
      // playlistTracks,
      // playlistObjects,
    } = this

    if(!width || !height) {
      console.error(`Invalid canvas dimensions.\nwidth: ${width}, height: ${height}`)
    }

    p.setup = () => {
      canvas = p.createCanvas(width, height)
      p.noStroke()

      // handle mouse press and dropping
      canvas.mousePressed((ev: MouseEvent) => {
        if(ev.button === 0) {
          console.log("started press")
          mousePressedX = p.mouseX
          mousePressedY = p.mouseY

          this.isMouseDragged = true
          p.mouseReleased = () => {
            this.isMouseDragged = false
          }
        }
      })

      // handle zoom event
      canvas.mouseWheel((ev: WheelEvent) => {
        let scaleFactor = null
        if(ev?.deltaY < 0) {
          scaleFactor = 1 + staticDefaults.zoomSensitivity
        } else if(ev?.deltaY > 0) {
          scaleFactor = 1 - staticDefaults.zoomSensitivity
        } else {
          scaleFactor = 1
        }

        // limit scaling
        if(currentScale * scaleFactor < 1) return

        currentScale *= scaleFactor
        this.currentScale *= scaleFactor
        transformX = p.mouseX - (p.mouseX * scaleFactor) + (transformX * scaleFactor)
        transformY = p.mouseY - (p.mouseY * scaleFactor) + (transformY * scaleFactor)

        // scall all playlist items
        this.playlistObjects.forEach(obj => {
          obj.currentScale = this.currentScale
        })

        console.log("currentScale: ", currentScale)
      })

      // handle click release
      canvas.mouseReleased(() => {
        mousePressedX = null
        mousePressedY = null
        this.isMouseDragged = false
      })

      // handle mouse drag
      canvas.mouseMoved((ev: MouseEvent) => {
        if(this.isMouseDragged) {
          const dist = p.dist(mousePressedX ?? 0, mousePressedY ?? 0, p.mouseX, p.mouseY)
          if(dist > staticDefaults.mouseDragDetectionThreshold) {
            this.isMouseDragged = true
            transformX -= (p.mouseX - ev.offsetX)
            transformY -= (p.mouseY - ev.offsetY)
          }
        }
      })

      canvas.drop((dropped) => {
        console.log("dropped this: ", dropped)
      })

      // handle right click
      this.canvasEl = document.getElementById(canvas.id())
      this.canvasEl?.addEventListener('contextmenu', (e) => {
        e.preventDefault()
        this.#handleRightClick(e, p)
      })

      const newPlaylistTracks: Array<PlaylistTrack> = []
      for(let i = 0; i < trackCount; i++) {
        if(!canvas) return

        const track = new PlaylistTrack(
          p,
          canvas,
          this,
          {
            currentScale,
            trackCount,
            timelineWidth: width,
            timelineHeight: height,
            trackNumber: i,
            trackHeight: staticDefaults.trackHeight,
            playlistObjects: [],
          }
        )

        newPlaylistTracks.push(track)
      }

      this.playlistTracks = newPlaylistTracks
    }

    // handle canvas recieved props
    p.updateWithProps = props => {
      console.log("updating with props", props)
      if(!canvas) {
        console.error("this.canvas in the playlist Renderer was null when p.updatingWithProps was called.")
        return
      }

      height = props.height
      width = props.width
      maxPlaylistBeats = props.maxPlaylistBeats
      trackCount = props.trackCount

      this.onNodeRightClick = props.onNodeRightClick
      this.onNodeMove = props.onNodeMove

      if(props.playing) {
        this.playlistStart = Date.now()
      } else {
        this.playlistStart = null
      }

      if(props.playing !== this.playing) {
        // save on resources if only `playing` prop changed
        this.playing = props.playing
        return
      }
      
      p.resizeCanvas(width, height)

      const newPlaylistObjects: Array<PlaylistObject> = []
      const newPlaylistTracks: Array<PlaylistTrack> = []

      props.playlistObjects.forEach(async (item) => {
        if(!canvas) return

        // const soundData = await fetchWaveformData(item.id)
        const { wf: soundData, dur: duration } = await fetchNodeData(item.id)
        const p5PlaylistObject = new PlaylistObject(
          p,
          canvas,
          this,
          {
            currentScale,
            soundData,
            duration,
            timelineWidth: width, 
            timelineHeight: staticDefaults.timelineHeight,
            trackNumber: item.trackNumber,
            trackHeight: staticDefaults.trackHeight,
            playlistItem: item,
          }
        )

        newPlaylistObjects.push(p5PlaylistObject)
      })

      for(let i = 0; i < trackCount; i++) {
        if(!canvas) return

        const track = new PlaylistTrack(
          p,
          canvas,
          this,
          {
            currentScale,
            trackCount,
            timelineWidth: width,
            timelineHeight: staticDefaults.timelineHeight,
            trackNumber: i,
            trackHeight: staticDefaults.trackHeight,
            playlistObjects: newPlaylistObjects,
          }
        )

        newPlaylistTracks.push(track)
      }

      this.playlistObjects = newPlaylistObjects
      this.playlistTracks = newPlaylistTracks

      console.log("objects to be rendered: ", this.playlistObjects, this.playlistTracks)
    }

    // render p5 canvas
    p.draw = () => {
      if(!canvas) return

      const timeline = new Timeline(
        p,
        canvas,
        this,
        {
          timelineWidth: width, 
          timelineHeight: staticDefaults.timelineHeight,
          currentScale,
        }
      )

      const cursor = new Cursor(
        p,
        canvas,
        this,
        {
          timelineWidth: width, 
          timelineHeight: staticDefaults.timelineHeight,
          currentScale,
        }
      )
      
      p.background(255, 255, 255)
      p.stroke(180, 180, 180)
      p.fill(255, 255, 255)

      transformX = p.constrain(transformX, width * (1 - currentScale), 0)
      transformY = p.constrain(transformY, height * (1 - currentScale), 0)

      p.push()
      p.translate(transformX, 1)
      p.scale(currentScale, 1)
      
      timeline.render()
      cursor.render()

      // render vertical grid lines
      const gridStroke = .2 * (1 - currentScale) > 0 ? .2 * (1 - currentScale) : .4
      p.strokeWeight(gridStroke)
      p.stroke(100, 100, 100)
      for(let i = 0; i < width; i += width/maxPlaylistBeats) {
        p.line(i, 0, i, height)
      }

      // render playlist tracks
      this.playlistTracks.forEach(track => {
        track.render()
      })
      
      p.pop()
    }
  }
}
