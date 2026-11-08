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
  tempo: number
  beatsDisplayed: number
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
  debugMode: true,

  // defaults on Renderer construction, these will be reassigned
  // when the canvas renders
  height: 100,
  width: 200,
  maxPlaylistBeats: 32,
  trackCount: 4,
  tempo: 120,

  // class specific defaults
  PlaylistObject: {
    labelHeight: 20,
    tabHeight: 28,
  },
}

// mirrors PlaylistTransport in engine/src/event_dispatch.rs
export interface PlaylistTransport {
  startTimeMs: number
  beatIntervalMs: number
  loopBeats: number
  beatsDisplayed: number
  loopEnabled: boolean
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
  tempo: number = staticDefaults.tempo
  // the full canvas width (at scale 1) spans this many beats
  beatsDisplayed = 16
  debugMode: boolean = staticDefaults.debugMode

  currentScale = 1
  transformX = 0
  transformY = 0
  
  isMouseDown = false
  isMouseDragged = false
  mousePressedX: number | null = null
  mousePressedY: number | null = null

  playing: boolean = staticDefaults.playing
  // engine timing for the current run, used to draw the playback cursor
  transport: PlaylistTransport | null = null

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
      console.log("track data: min, max, dropy", min, max, dropY)
      
      if(min < dropY && dropY < max) {
        trackNumber = track.trackNumber
        break
      }
    }

    console.log("calculated trackNumber: ", trackNumber)
    return trackNumber
  }

  // fetch the engine's start time and beat length, so the cursor follows the
  // engine's beat loop instead of a frontend timer that starts at a different time
  syncTransport = async () => {
    try {
      const transport = await invoke<PlaylistTransport>('get_playlist_transport', {})
      // playback may have stopped while waiting on the engine
      if(this.playing) this.transport = transport
    } catch(e) {
      console.error('could not sync the playback cursor:', e)
    }
  }

  // clips, the grid, and the cursor are drawn in "playlist" x coordinates:
  // pixels at scale 1 with no pan, where the canvas width spans
  // `beatsDisplayed` beats. zoom and pan are applied on top when drawing
  // (translate(transformX) then scale(currentScale)), so positions stored in
  // beats stay correct when the canvas is resized, zoomed, or panned

  // convert an x position on the canvas (e.g. the mouse) to playlist x
  screenToPlaylistX = (screenX: number): number =>
    (screenX - this.transformX) / this.currentScale

  beatToX = (beat: number): number =>
    beat / this.beatsDisplayed * this.width

  xToBeat = (x: number): number =>
    this.width > 0 ? x / this.width * this.beatsDisplayed : 0

  // convert an x position on the canvas to beats from the playlist start
  screenXToBeat = (screenX: number): number =>
    this.xToBeat(this.screenToPlaylistX(screenX))

  // width, in playlist x, of something lasting `seconds`
  secondsToWidth = (seconds: number): number =>
    this.beatToX(seconds * this.tempo / 60)

  #handleRightClick = (ev: MouseEvent, p: P5CanvasInstance<CanvasProps>) => {
    console.log("handled right click", ev)

    this.playlistObjects.forEach(item => {
      if(item.mouseOver()) {
        const { mouseX, mouseY } = p
        item.onRightClick(ev, {
          mouseX,
          mouseY,
          target: item,
        })
      }
    })
  }

  // p5 sketch function
  sketch: Sketch<CanvasProps> = (p: P5CanvasInstance<CanvasProps>) => {
    // size, zoom, and pan are read from `this` (not copied into locals), so
    // the components that convert between screen and playlist coordinates
    // see the same values the sketch draws with
    let {
      canvas,
      mousePressedX,
      mousePressedY,
    } = this
    const { width, height, trackCount } = this

    if(!width || !height) {
      console.error(`Invalid canvas dimensions.\nwidth: ${width}, height: ${height}`)
    }

    p.setup = () => {
      canvas = p.createCanvas(width, height)
      p.frameRate(60)
      p.noStroke()

      // handle mouse press and dropping
      canvas.mousePressed((ev: MouseEvent) => {
        if(ev.button === 0) {
          console.log("started press")
          mousePressedX = p.mouseX
          mousePressedY = p.mouseY

          this.isMouseDown = true
          p.mouseReleased = () => {
            this.isMouseDown = false
            this.isMouseDragged = false
          }
        }
      })

      // handle zoom event
      canvas.mouseWheel((ev: WheelEvent) => {
        let scaleFactor = null
        if(ev?.deltaY < 0) {
          // zoom in
          scaleFactor = 1 + staticDefaults.zoomSensitivity
        } else if(ev?.deltaY > 0) {
          // zoom out
          scaleFactor = 1 - staticDefaults.zoomSensitivity
        } else {
          scaleFactor = 1
        }

        // limit scaling
        if(this.currentScale * scaleFactor < 1) return

        if(
          ev?.deltaY > 0 &&
          this.currentScale * scaleFactor - 1 < staticDefaults.zoomSensitivity
        ) {
          // handle fp rounding error
          scaleFactor = 1
          this.currentScale = 1
        }

        this.currentScale *= scaleFactor

        // zoom around the mouse
        this.transformX = p.mouseX - (p.mouseX * scaleFactor) + (this.transformX * scaleFactor)
        this.transformY = p.mouseY - (p.mouseY * scaleFactor) + (this.transformY * scaleFactor)

        // scall all playlist items
        this.playlistObjects.forEach(obj => {
          obj.currentScale = this.currentScale
        })

        console.log("currentScale: ", this.currentScale)
      })

      // handle click release
      canvas.mouseReleased(() => {
        mousePressedX = null
        mousePressedY = null
        this.isMouseDown = false
        this.isMouseDragged = false
      })

      // handle mouse drag
      canvas.mouseMoved((ev: MouseEvent) => {
        if(this.isMouseDown) {
          const dist = p.dist(mousePressedX ?? 0, mousePressedY ?? 0, p.mouseX, p.mouseY)
          if(dist > staticDefaults.mouseDragDetectionThreshold) {
            this.isMouseDragged = true
            this.transformX -= (p.mouseX - ev.offsetX)
            this.transformY -= (p.mouseY - ev.offsetY)
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
            currentScale: this.currentScale,
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

      this.height = props.height
      this.width = props.width
      this.maxPlaylistBeats = props.maxPlaylistBeats
      this.trackCount = props.trackCount
      this.tempo = props.tempo
      this.beatsDisplayed = props.beatsDisplayed
      const { width, height, trackCount, currentScale } = this

      this.onNodeRightClick = props.onNodeRightClick
      this.onNodeMove = props.onNodeMove

      // only (re)sync the cursor when playback starts or stops, other prop
      // updates (drops, moves, resizes) shouldn't reset it to the start
      if(props.playing && !this.playing) {
        this.syncTransport()
      } else if(!props.playing) {
        this.transport = null
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

      const { width, height, trackCount, currentScale, beatsDisplayed } = this

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

      this.transformX = p.constrain(this.transformX, width * (1 - currentScale), 0)
      this.transformY = p.constrain(this.transformY, height * (1 - currentScale), 0)

      p.push()
      p.translate(this.transformX, 1)
      p.scale(currentScale, 1)
      
      timeline.render()
      cursor.render()

      // render vertical grid lines
      const gridStroke = .2 * (1 - currentScale) > 0 ? .2 * (1 - currentScale) : .4
      p.strokeWeight(gridStroke)
      p.stroke(100, 100, 100)
      for(let i = 0; i < width; i += width/beatsDisplayed) {
        // p.line(i, 0, i, height)
        p.line(i, 0, i, trackCount * staticDefaults.trackHeight + staticDefaults.timelineHeight + 1)
      }

      // render playlist tracks
      this.playlistTracks.forEach(track => {
        track.render()
      })
      
      p.pop()
      
      // render debug window
      if(this.debugMode) {
        const debugWinWidth = 120
        const debugWinHeight = 200
        p.noStroke()
        p.fill(255,187,153,255*.6)
        p.rect(
          p.width - debugWinWidth, 
          p.height - debugWinHeight,
          debugWinWidth - 10, 
          debugWinHeight - 10
        )

        p.stroke(0,0,0)
        p.fill(0,0,0)
        p.strokeWeight(.1)
        p.textSize(11)
        p.textAlign(p.LEFT)
        p.textFont('Courier New')

        // debug info
        const fps = p.frameRate().toFixed(2)
        const scale = this.currentScale
        const nodeCount = this.playlistObjects.length
        p.text(
          `
          fps: ${fps}
          scale: ${scale}
          mousedown: ${this.isMouseDown}
          dragging: ${this.isMouseDragged}
          nodeCount: ${nodeCount}
          trackCount: ${this.trackCount}
          playing: ${this.playing}
          `,
          p.width - debugWinWidth - 11*6 + 11/2,
          p.height - debugWinHeight
        )
      }
    }
  }
}
