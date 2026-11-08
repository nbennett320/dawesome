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
  onNodeDoubleClick: (id: number) => void
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

// mirrors SnapSetting in engine/src/event_dispatch.rs
export interface SnapSetting {
  enabled: boolean
  // length of one snap step, in beats
  beats: number
}

// a clip being dragged with the mouse
interface ClipDrag {
  item: PlaylistObject
  // where the clip was grabbed, relative to its top-left corner (x in
  // playlist x, y in pixels), so the same spot stays under the mouse
  grabX: number
  grabY: number
  // false until the mouse moves past the drag threshold, so a plain click
  // doesn't move the clip
  active: boolean
}

// a sample being dragged in from the sidebar. the mouse is outside the canvas
// when the drag starts, so PlaylistCanvas reports its position
interface SampleDrag {
  path: string
  // mouse position on the canvas
  x: number
  y: number
  // the playlist's top in the window, used for waveform y
  containerTop: number
  // waveform and duration (seconds), once loaded from the engine
  data: { soundData: number[], duration: number } | null
  // the clip drawn under the mouse, built in draw once the data has loaded
  preview: PlaylistObject | null
  // set if the waveform couldn't be loaded
  error: string | null
}

// a clip following the mouse, whichever kind of drag it came from
interface DraggedClip {
  item: PlaylistObject
  grabX: number
  grabY: number
  mouseX: number
  mouseY: number
}

// where a dragged clip would land if it were dropped right now
interface DropTarget {
  // the clip's left edge, in beats, before snapping (the engine snaps it)
  beat: number
  // where it will land after snapping, used for the phantom preview
  snappedBeat: number
  trackNumber: number
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

  clipDrag: ClipDrag | null = null
  sampleDrag: SampleDrag | null = null
  // refreshed from the engine whenever a drag starts
  snap: SnapSetting = { enabled: true, beats: 1 }

  // reassigned function definitions
  onNodeRightClick!: (id: number) => void
  onNodeDoubleClick!: (id: number) => void
  onNodeMove!: (
    item: PlaylistItem,
    dropBeat: number,
    trackNumber: number,
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

  syncSnapSetting = async () => {
    try {
      this.snap = await invoke<SnapSetting>('get_snap_setting', {})
    } catch(e) {
      console.error('could not get the snap setting:', e)
    }
  }

  // the clip under the mouse, if any (the last one drawn is on top)
  clipUnderMouse = (): PlaylistObject | null => {
    const hovered = this.playlistObjects.filter(item => item.mouseOver())

    return hovered.length ? hovered[hovered.length - 1] : null
  }

  // the track at a y position on the canvas, clamped to the first/last track
  trackAt = (y: number): number => {
    const tracks = this.playlistTracks
    if(!tracks.length) return -1

    const track = tracks.find(t => t.minHeight <= y && y <= t.maxHeight)
    if(track) return track.trackNumber

    return y < tracks[0].minHeight
      ? tracks[0].trackNumber
      : tracks[tracks.length - 1].trackNumber
  }

  // the same rule the engine uses: clamp to the playlist, then round to the
  // nearest snap step
  snapBeat = (beat: number): number => {
    const clamped = Math.min(Math.max(beat, 0), this.beatsDisplayed)
    const { enabled, beats } = this.snap

    return enabled && beats > 0 ? Math.round(clamped / beats) * beats : clamped
  }

  // where a dragged clip would land, given the mouse position on the canvas
  // and how far from the clip's left edge it's held (in playlist x)
  dropTarget = (mouseX: number, mouseY: number, grabX = 0): DropTarget => {
    const left = this.screenToPlaylistX(mouseX) - grabX
    const beat = Math.min(Math.max(this.xToBeat(left), 0), this.beatsDisplayed)

    return {
      beat,
      snappedBeat: this.snapBeat(beat),
      trackNumber: this.trackAt(mouseY),
    }
  }

  // a sample dragged from the sidebar is held by the middle of its label
  sampleGrabY = staticDefaults.PlaylistObject.labelHeight / 2

  // start or update the sidebar drag preview
  // - x, y: mouse position on the canvas
  // - containerTop: the playlist's top in the window, used for waveform y
  setSampleDrag = (path: string, x: number, y: number, containerTop: number) => {
    if(this.sampleDrag?.path === path) {
      this.sampleDrag.x = x
      this.sampleDrag.y = y
      return
    }

    this.sampleDrag = { path, x, y, containerTop, data: null, preview: null, error: null }
    this.#loadSamplePreview(path)
    this.syncSnapSetting()
  }

  clearSampleDrag = () => { this.sampleDrag = null }

  #loadSamplePreview = async (path: string) => {
    try {
      const [soundData, duration] =
        await invoke<[number[], number]>('get_sample_preview', { samplePath: path })
      // the drag may have ended, or moved on to another sample, while loading
      if(this.sampleDrag?.path !== path) return

      this.sampleDrag.data = { soundData, duration }
    } catch(e) {
      console.error('could not preview sample:', path, e)
      if(this.sampleDrag?.path === path) this.sampleDrag.error = String(e)
    }
  }

  // build the sidebar drag preview with the p5 instance that's drawing, so it
  // can't end up drawing onto a canvas that was replaced (e.g. by a remount)
  #samplePreview = (
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
  ): PlaylistObject | null => {
    const drag = this.sampleDrag
    if(!drag?.data) return null
    if(drag.preview) return drag.preview

    const { path, containerTop, data: { soundData, duration } } = drag
    drag.preview = new PlaylistObject(p, canvas, this, {
      currentScale: this.currentScale,
      soundData,
      duration,
      timelineWidth: this.width,
      timelineHeight: staticDefaults.timelineHeight,
      trackNumber: 0,
      trackHeight: staticDefaults.trackHeight,
      playlistItem: {
        id: -1,
        path,
        startBeat: 0,
        trackNumber: 0,
        // same vertical offset added clips get (see addToPlaylist)
        pixelOffset: {
          x: 0, y: containerTop, left: 0, top: containerTop,
          right: 0, bottom: 0, xOffset: 0, yOffset: 0,
        },
      },
    })

    return drag.preview
  }

  // what the sidebar drag is doing, for the debug window
  #sampleDragStatus = (): string => {
    const drag = this.sampleDrag
    if(!drag) return 'none'
    if(drag.error) return `error: ${drag.error}`
    if(!drag.data) return 'loading'

    return `at ${Math.round(drag.x)},${Math.round(drag.y)}`
  }

  // the clip currently following the mouse, if any
  draggedClip = (
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
  ): DraggedClip | null => {
    const { clipDrag, sampleDrag } = this

    if(clipDrag?.active) {
      const { item, grabX, grabY } = clipDrag
      return { item, grabX, grabY, mouseX: p.mouseX, mouseY: p.mouseY }
    }

    const preview = this.#samplePreview(p, canvas)
    if(sampleDrag && preview) {
      return {
        item: preview,
        grabX: 0,
        grabY: this.sampleGrabY,
        mouseX: sampleDrag.x,
        mouseY: sampleDrag.y,
      }
    }

    return null
  }

  // draw the snapped landing spot, and the dragged clip pinned to the mouse
  // called inside the zoom/pan transform
  drawClipDrag = (p: P5CanvasInstance<CanvasProps>, canvas: p5.Renderer) => {
    const drag = this.draggedClip(p, canvas)
    if(!drag) return

    // keep the preview's zoom in step with the other clips
    drag.item.currentScale = this.currentScale

    const target = this.dropTarget(drag.mouseX, drag.mouseY, drag.grabX)
    const { left, top, width, height } = drag.item.boundingBox()

    // phantom: where the clip will land (after snapping, when snap is on)
    const track = this.playlistTracks.find(t => t.trackNumber === target.trackNumber)
    if(track) {
      const ctx = p.drawingContext as CanvasRenderingContext2D
      p.push()
      ctx.setLineDash([4, 3])
      p.strokeWeight(1)
      p.stroke(14, 165, 233)
      p.fill(125, 211, 252, 255*.25)
      p.rect(this.beatToX(target.snappedBeat), track.minHeight, width, height, 3)
      ctx.setLineDash([])
      p.pop()
    }

    // the clip itself, following the mouse
    p.push()
    p.translate(
      this.screenToPlaylistX(drag.mouseX) - drag.grabX - left,
      drag.mouseY - drag.grabY - top,
    )
    drag.item.render()
    p.pop()
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
    p.setup = () => {
      const { width, height, trackCount } = this

      if(!width || !height) {
        console.error(`Invalid canvas dimensions.\nwidth: ${width}, height: ${height}`)
      }

      canvas = p.createCanvas(width, height)
      this.canvas = canvas
      this.syncSnapSetting()
      p.frameRate(60)
      p.noStroke()

      // handle mouse press, and pick up a clip if one was pressed on
      canvas.mousePressed((ev: MouseEvent) => {
        if(ev.button === 0) {
          mousePressedX = p.mouseX
          mousePressedY = p.mouseY
          this.isMouseDown = true

          const item = this.clipUnderMouse()
          if(item) {
            const { left, top } = item.boundingBox()
            this.clipDrag = {
              item,
              grabX: this.screenToPlaylistX(p.mouseX) - left,
              grabY: p.mouseY - top,
              active: false,
            }
            this.syncSnapSetting()
          }
        }
      })

      // releasing anywhere in the window ends a press. a clip released over
      // the canvas has already been dropped by canvas.mouseReleased (element
      // listeners run before window listeners), so a drag still in progress
      // here was released outside the canvas and is cancelled
      p.mouseReleased = () => {
        mousePressedX = null
        mousePressedY = null
        this.isMouseDown = false
        this.isMouseDragged = false
        this.clipDrag = null
      }

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

      // drop a dragged clip where the phantom shows
      canvas.mouseReleased(() => {
        const drag = this.clipDrag
        const target = this.dropTarget(p.mouseX, p.mouseY, drag?.grabX)

        if(drag?.active && target.trackNumber >= 0) {
          // send the unsnapped beat, the engine owns snapping
          this.onNodeMove(drag.item.playlistItem, target.beat, target.trackNumber)
        }

        mousePressedX = null
        mousePressedY = null
        this.isMouseDown = false
        this.isMouseDragged = false
        this.clipDrag = null
      })

      // handle mouse drag
      canvas.mouseMoved((ev: MouseEvent) => {
        if(this.isMouseDown) {
          const dist = p.dist(mousePressedX ?? 0, mousePressedY ?? 0, p.mouseX, p.mouseY)
          if(dist > staticDefaults.mouseDragDetectionThreshold) {
            this.isMouseDragged = true

            if(this.clipDrag) {
              // dragging a clip, not panning the playlist
              this.clipDrag.active = true
            } else {
              this.transformX -= (p.mouseX - ev.offsetX)
              this.transformY -= (p.mouseY - ev.offsetY)
            }
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

      // double clicking a clip inspects it in the Sample tab
      this.canvasEl?.addEventListener('dblclick', () => {
        const item = this.clipUnderMouse()
        if(item) this.onNodeDoubleClick(item.playlistItem.id)
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
      this.onNodeDoubleClick = props.onNodeDoubleClick

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

      // render the clip being dragged on top of everything else
      this.drawClipDrag(p, canvas)
      
      p.pop()

      if(this.clipDrag?.active || this.sampleDrag) {
        p.cursor('grabbing')
      } else if(this.clipUnderMouse()) {
        p.cursor(p.HAND)
      } else {
        p.cursor(p.ARROW)
      }
      
      // render debug window
      if(this.debugMode) {
        // top-right, below the timeline: the bottom of the canvas can be
        // hidden behind the bottom of the playlist pane
        const debugWinWidth = 220
        const debugWinHeight = 130
        const debugWinTop = staticDefaults.timelineHeight + 10
        p.noStroke()
        p.fill(255,187,153,255*.6)
        p.rect(
          p.width - debugWinWidth, 
          debugWinTop,
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
          [
            `fps: ${fps}`,
            `scale: ${scale}`,
            `mousedown: ${this.isMouseDown}`,
            `dragging: ${this.isMouseDragged}`,
            `nodeCount: ${nodeCount}`,
            `trackCount: ${this.trackCount}`,
            `playing: ${this.playing}`,
            `sample drag: ${this.#sampleDragStatus()}`,
          ].join('\n'),
          p.width - debugWinWidth + 6,
          debugWinTop + 14,
        )
      }
    }
  }
}
