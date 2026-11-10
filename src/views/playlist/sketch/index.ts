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
import { ClipOverride } from '../../../types/clip'
import {
  PlaylistItem,
  PlaylistItemPixelOffset,
  PlaylistWindow
} from '../../../types/playlist'
import { theme } from './theme'
import { PlaylistTransport } from '../../../types/transport'
import { positionAt } from '../../transport/tape'
import { Lasso, groupLanding, keepExisting, lassoRect, lassoSelection, toggleSelected } from './selection'
import { PlaceableItem as DragItem, dragItemKey } from '../../../state/sampleDrag'
import { Pattern, SequencerState, patternSeconds } from '../../../types/sequencer'
import { decodeLevels } from '../../../util/waveform'

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
  // a clip was right-clicked, at clientX, clientY in the window
  onNodeRightClick: (id: number, clientX: number, clientY: number) => void
  onNodeMove: () => void
  // several selected clips were dragged together (see Renderer.onNodesMove)
  onNodesMove: (ids: number[], anchorId: number, dropBeat: number, trackNumber: number) => void
  onNodeDoubleClick: (id: number) => void
  // see PlaylistState.transportRevision
  transportRevision: number
  // move the playhead to a beat (clicking or dragging in the ruler)
  onSeek: (beat: number) => void
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
  trackCount: 4,
  tempo: 120,

  // class specific defaults
  PlaylistObject: {
    labelHeight: 20,
    tabHeight: 28,
  },
}

// the top of a track, in playlist y. matches PlaylistTrack's layout
const trackTop = (trackNumber: number): number =>
  staticDefaults.trackHeight * trackNumber + trackNumber * .3 + staticDefaults.timelineHeight

// room below the last track: dropping a clip there puts it on a new track
const NEW_TRACK_AREA_HEIGHT = staticDefaults.trackHeight / 2

// the vertical scrollbar, drawn over the right edge of the tracks
const SCROLLBAR = {
  width: 6,
  // from the right edge, where a press grabs the scrollbar
  hitWidth: 14,
  // gap between the thumb and the canvas's right edge
  margin: 3,
  minThumbHeight: 24,
}

// dragging a clip near the top or bottom of the tracks scrolls them
const AUTO_SCROLL = {
  // how far from the edge it starts, in pixels
  zone: 32,
  // pixels per frame, at the very edge
  maxSpeed: 12,
}

// a wheel event's delta in lines (a mouse wheel on some systems), in pixels
// how long the playlist keeps drawing after something changes (see
// Renderer.wake), so whatever follows from it (a hover, a scroll settling)
// is drawn too
const IDLE_MS = 500

const WHEEL_LINE_PIXELS = 16
// how much zoom one pixel of ⌘+wheel gives, before limiting to
// staticDefaults.zoomSensitivity per event
const WHEEL_ZOOM_PER_PIXEL = 0.01

// WebKit (the macOS webview) reports trackpad pinches as gesture events,
// not ⌘+wheel. not in the DOM typings
interface GestureEvent extends UIEvent {
  scale: number
}

export type { PlaylistTransport }

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
  // the other selected clips, dragged along with it. empty for one clip
  group: PlaylistObject[]
}

// a sample being dragged in from the sidebar, or a pattern from the
// Sequencer. the mouse is outside the canvas when the drag starts, so
// PlaylistCanvas reports its position
interface SampleDrag {
  item: DragItem
  // see dragItemKey
  key: string
  // mouse position on the canvas
  x: number
  y: number
  // the playlist's top in the window, used for waveform y
  containerTop: number
  // waveform and duration (seconds), once loaded from the engine
  data: { soundData: Float32Array, duration: number } | null
  // the clip drawn under the mouse, built in draw once the data has loaded
  preview: PlaylistObject | null
  // set if the waveform couldn't be loaded
  error: string | null
}

// a clip following the mouse, whichever kind of drag it came from
interface DraggedClip {
  item: PlaylistObject
  // the other clips following it (a dragged selection)
  group: PlaylistObject[]
  grabX: number
  grabY: number
  mouseX: number
  mouseY: number
}

// where one clip of a drag will land, for its phantom
interface Landing {
  item: PlaylistObject
  beat: number
  track: number
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
  const [wf, dur] = await invoke<[string, number]>('get_node_data', { id })

  return {
    wf: decodeLevels(wf),
    dur,
  }
}

// whether a canvas y position is in the ruler, along the top
const overRuler = (y: number): boolean => y >= 0 && y < staticDefaults.timelineHeight

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
  // how far the tracks are scrolled down, in pixels. the timeline stays at
  // the top. zoom only stretches x, so this isn't scaled
  scrollY = 0
  // while the scrollbar's thumb is dragged: how far below the thumb's top
  // it's held, in pixels
  scrollbarGrab: number | null = null
  // the p5 instance drawing, for the mouse position
  #p: P5CanvasInstance<CanvasProps> | null = null
  // whether the playlist can be seen. hidden behind another tab, it stops
  // drawing (see setVisible)
  #visible = true
  
  isMouseDown = false
  isMouseDragged = false
  mousePressedX: number | null = null
  mousePressedY: number | null = null

  playing: boolean = staticDefaults.playing
  // where the engine's playhead is, and its timing while playing, used to
  // draw the playback cursor
  transport: PlaylistTransport | null = null
  transportRevision: number | null = null
  // counts transport fetches, so a slow one can't replace a newer one's
  #transportFetch = 0
  // the beat the ruler is being dragged to, or was just released at (shown
  // until the engine confirms it)
  seekBeat: number | null = null
  seekDragging = false
  #seekTimeout: ReturnType<typeof setTimeout> | null = null
  onSeek!: (beat: number) => void

  playlistTracks: Array<PlaylistTrack> = []
  playlistObjects: Array<PlaylistObject> = []

  // counts rebuilds, so a slow one can't replace the clips a newer one built
  #rebuildGeneration = 0
  // the playlist's items the clips were last built from. the clips are only
  // rebuilt when these change (or the number of tracks does), not when the
  // playlist is seeked, resized, or its tempo changes
  #builtItems: PlaylistItem[] | null = null
  // waveforms being fetched for new clips, so a rebuild that starts before
  // another's finished doesn't ask the engine for the same one again
  #fetching = new Map<number, ReturnType<typeof fetchNodeData>>()

  clipDrag: ClipDrag | null = null
  sampleDrag: SampleDrag | null = null

  // the Sequencer's rows and patterns, which pattern clips draw. read every
  // frame, so editing a pattern shows straight away without rebuilding
  sequencer: SequencerState = { rows: [], patterns: [], folders: [] }

  // the clips picked with shift+click or a shift+drag box, by id
  selectedIds: Set<number> = new Set()
  // the box being dragged out with shift held, if any
  lasso: Lasso | null = null
  // whether the press was on empty track space (not a clip, the ruler or
  // the scrollbar), so a click there without dragging clears the selection
  #pressedOnEmpty = false
  // refreshed from the engine whenever a drag starts
  snap: SnapSetting = { enabled: true, beats: 1 }

  // reassigned function definitions
  onNodeRightClick!: (id: number, clientX: number, clientY: number) => void
  onNodeDoubleClick!: (id: number) => void
  onNodeMove!: (
    item: PlaylistItem,
    dropBeat: number,
    trackNumber: number,
  ) => void
  // the grabbed clip (anchorId) was dropped at dropBeat (unsnapped) on
  // trackNumber, and every clip in ids moves by as much
  onNodesMove!: (ids: number[], anchorId: number, dropBeat: number, trackNumber: number) => void

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

    let trackNumber = -1
    for(let i = 0; i < this.playlistTracks.length; i++) {
      const track = this.playlistTracks[i]
      const [min, max] = [track.minHeight, track.maxHeight]

      if(min < dropY && dropY < max) {
        trackNumber = track.trackNumber
        break
      }
    }

    return trackNumber
  }

  // fetch the engine's start time and beat length, so the cursor follows the
  // engine's beat loop instead of a frontend timer that starts at a different time
  syncTransport = async () => {
    this.#transportFetch += 1
    const fetch = this.#transportFetch

    try {
      const transport = await invoke<PlaylistTransport>('get_playlist_transport', {})
      if(fetch !== this.#transportFetch) return

      this.transport = transport
      // a released seek has reached the engine
      if(!this.seekDragging) this.seekBeat = null
    } catch(e) {
      console.error('could not sync the playback cursor:', e)
    }
    this.wake()
  }

  // the playhead, in beats: following the engine's run while playing, or
  // where the ruler's being dragged to
  playheadBeat = (): number | null => {
    if(this.seekBeat !== null) return this.seekBeat

    const t = this.transport
    if(!t || t.beatIntervalMs <= 0) return null
    return positionAt(t, Date.now()) / t.beatIntervalMs
  }

  // the beat under the mouse in the ruler, snapped like clips are
  rulerBeat = (screenX: number): number => this.snapBeat(this.screenXToBeat(screenX))

  #startSeek = (screenX: number) => {
    this.seekDragging = true
    this.seekBeat = this.rulerBeat(screenX)
  }

  // tell the engine where the ruler was released. the playhead stays there
  // until the engine's transport confirms it (or gives up after a moment)
  #endSeek = () => {
    if(!this.seekDragging || this.seekBeat === null) return
    this.seekDragging = false

    const beat = this.seekBeat
    this.onSeek?.(beat)

    if(this.#seekTimeout) clearTimeout(this.#seekTimeout)
    this.#seekTimeout = setTimeout(() => {
      if(!this.seekDragging) this.seekBeat = null
      this.wake()
    }, 1500)
  }

  syncSnapSetting = async () => {
    try {
      this.snap = await invoke<SnapSetting>('get_snap_setting', {})
    } catch(e) {
      console.error('could not get the snap setting:', e)
    }
    this.wake()
  }

  clearSelection = () => {
    this.selectedIds = new Set()
    this.wake()
  }

  // the selected clips' ids, clearing the selection (to delete them). none
  // while clips are being dragged or a box drawn
  takeSelection = (): number[] => {
    if(this.clipDrag?.active || this.lasso) return []
    const ids = [...this.selectedIds]
    this.clearSelection()
    return ids
  }

  // the mouse, in playlist coordinates
  #mousePoint = (p: P5CanvasInstance<CanvasProps>) => ({
    x: this.screenToPlaylistX(p.mouseX),
    y: this.screenToPlaylistY(p.mouseY),
  })

  // start a selection box where the mouse was pressed
  #startLasso = (p: P5CanvasInstance<CanvasProps>) => {
    const start = this.#mousePoint(p)
    this.lasso = { start, end: start, base: new Set(this.selectedIds) }
  }

  // stretch the box to the mouse, and select what it touches. run every
  // frame, so the selection follows when the tracks scroll under the box
  #followLasso = (p: P5CanvasInstance<CanvasProps>) => {
    if(!this.lasso) return
    this.lasso.end = this.#mousePoint(p)
    this.selectedIds = lassoSelection(
      this.lasso,
      this.playlistObjects.map(clip => ({ id: clip.playlistItem.id, box: clip.boundingBox() })),
    )
  }

  // the selection box: a dashed sky outline over a faint fill, drawn in
  // canvas pixels so zoom doesn't stretch the dashes
  #drawLasso = (p: P5CanvasInstance<CanvasProps>) => {
    if(!this.lasso) return
    const { left, top, width, height } = lassoRect(this.lasso)
    const x = this.transformX + left * this.currentScale
    const y = top - this.scrollY + 1

    p.push()
    p.fill(...theme.lassoFill, 255 * .08)
    p.stroke(...theme.selectedBorder)
    p.strokeWeight(1)
    const ctx = p.drawingContext as CanvasRenderingContext2D
    ctx.setLineDash([4, 3])
    p.rect(x, y, width * this.currentScale, height)
    ctx.setLineDash([])
    p.pop()
  }

  // the playlist draws only while something on it can change: while it's
  // playing, while something's being dragged, and for a moment after
  // anything else (the mouse, a prop update, data arriving). otherwise the
  // last frame stays, and drawing costs nothing
  //
  // when wake was last called (performance.now(), ms)
  #lastWake = 0

  // the p5 instance drawing, unless it's been removed (its canvas is gone):
  // started again, it would draw on unseen
  #liveP = () => {
    const p = this.#p
    const ctx = p?.drawingContext as CanvasRenderingContext2D | undefined
    return p && ctx?.canvas.isConnected ? p : null
  }

  // something changed: draw it, and keep drawing for a moment (see IDLE_MS)
  wake = () => {
    this.#lastWake = performance.now()
    if(this.#visible) this.#liveP()?.loop()
  }

  // whether the next frame may differ from the last without anything waking
  // the playlist: the playhead moves, or something follows the mouse
  #animating = (): boolean => {
    const t = this.transport
    return this.playing
      || !!(t?.playing && t.patternId === null)
      || this.isMouseDown
      || this.clipDrag !== null
      || this.sampleDrag !== null
      || this.lasso !== null
      || this.seekDragging
      || this.scrollbarGrab !== null
  }

  // stop drawing while the playlist can't be seen (its tab isn't selected),
  // and start again, from however things are by then, once it can
  setVisible = (visible: boolean) => {
    if(visible === this.#visible) return
    this.#visible = visible

    // (a new p5 instance starts as this says, in setup)
    const p = this.#liveP()
    if(visible) this.wake()
    else p?.noLoop()
  }

  // the clip under the mouse, if any (the last one drawn is on top)
  clipUnderMouse = (): PlaylistObject | null => {
    // clips scrolled up under the timeline, or under the scrollbar, are hidden
    const p = this.#p
    if(p && (p.mouseY < staticDefaults.timelineHeight || this.overScrollbar(p.mouseX, p.mouseY))) {
      return null
    }

    const hovered = this.playlistObjects.filter(item => item.mouseOver())

    return hovered.length ? hovered[hovered.length - 1] : null
  }

  // the track at a y position in playlist y, clamped to the first track.
  // the area below the last track (and anything under it) is a new track,
  // numbered trackCount, which the engine creates when a clip lands on it
  trackAt = (y: number): number => {
    const tracks = this.playlistTracks
    if(!tracks.length) return -1
    if(y >= this.newTrackAreaTop()) return this.trackCount

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
      trackNumber: this.trackAt(this.screenToPlaylistY(mouseY)),
    }
  }

  // a sample dragged from the sidebar is held by the middle of its label
  sampleGrabY = staticDefaults.PlaylistObject.labelHeight / 2

  // start or update the sidebar drag preview
  // - x, y: mouse position on the canvas
  // - containerTop: the playlist's top in the window, used for waveform y
  setSampleDrag = (item: DragItem, x: number, y: number, containerTop: number) => {
    this.wake()
    const key = dragItemKey(item)
    if(this.sampleDrag?.key === key) {
      this.sampleDrag.x = x
      this.sampleDrag.y = y
      return
    }

    this.sampleDrag = { item, key, x, y, containerTop, data: null, preview: null, error: null }
    if(item.kind === 'sample') {
      this.#loadSamplePreview(item.path, key)
    } else {
      // a pattern clip has no waveform: it draws its pattern (see PlaylistObject)
      this.sampleDrag.data = { soundData: new Float32Array(0), duration: this.patternSecondsOf(item.patternId) }
    }
    this.syncSnapSetting()
  }

  // the pattern a pattern clip plays, if it still exists
  patternOf = (patternId: number): Pattern | undefined =>
    this.sequencer.patterns.find(p => p.id === patternId)

  // how long a pattern plays for at the playlist's tempo, in seconds
  patternSecondsOf = (patternId: number): number => {
    const pattern = this.patternOf(patternId)
    return pattern ? patternSeconds(pattern, this.tempo) : 0
  }

  // (pattern clips draw their pattern)
  // (called on every store update, so only a change wakes the playlist)
  setSequencer = (sequencer: SequencerState) => {
    if(sequencer === this.sequencer) return
    this.sequencer = sequencer
    this.wake()
  }

  clearSampleDrag = () => {
    this.sampleDrag = null
    this.wake()
  }

  // clips whose settings changed in the Sample tab, by id. kept here so
  // clips rebuilt on later prop updates get them too
  clipOverrides: Record<number, ClipOverride> = {}

  // show each clip's played length (which changes with its pitch) and
  // whether it was modified, without rebuilding the clips
  setClipOverrides = (overrides: Record<number, ClipOverride>) => {
    this.clipOverrides = overrides
    this.playlistObjects.forEach(this.#applyClipOverride)
    this.wake()
  }

  #applyClipOverride = (obj: PlaylistObject) => {
    const override = this.clipOverrides[obj.playlistItem.id]

    obj.modified = !!override?.modified
    // newer than the length fetched when the clip was built, which can lag
    // behind while settings are being sent to the engine
    if(override?.playedDuration !== undefined) obj.setDuration(override.playedDuration)
  }

  #loadSamplePreview = async (path: string, key: string) => {
    try {
      const [encoded, duration] =
        await invoke<[string, number]>('get_sample_preview', { samplePath: path })
      // the drag may have ended, or moved on to another sample, while loading
      if(this.sampleDrag?.key !== key) return

      this.sampleDrag.data = { soundData: decodeLevels(encoded), duration }
    } catch(e) {
      console.error('could not preview sample:', path, e)
      if(this.sampleDrag?.key === key) this.sampleDrag.error = String(e)
    }
    this.wake()
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

    const { item, containerTop, data: { soundData, duration } } = drag
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
        path: item.kind === 'sample' ? item.path : '',
        ...(item.kind === 'pattern' ? { patternId: item.patternId } : {}),
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
      const { item, group, grabX, grabY } = clipDrag
      return { item, group, grabX, grabY, mouseX: p.mouseX, mouseY: p.mouseY }
    }

    const preview = this.#samplePreview(p, canvas)
    if(sampleDrag && preview) {
      return {
        item: preview,
        group: [],
        grabX: 0,
        grabY: this.sampleGrabY,
        mouseX: sampleDrag.x,
        mouseY: sampleDrag.y,
      }
    }

    return null
  }

  // whether a clip is following the mouse, so its track doesn't draw it
  isDragged = (item: PlaylistObject): boolean => {
    const drag = this.clipDrag
    return !!drag?.active && (drag.item === item || drag.group.includes(item))
  }

  // where every clip in a drag will land if dropped now: just the dragged
  // clip, or the whole selection moved together the way the engine moves it
  landings = (drag: DraggedClip): Landing[] => {
    const target = this.dropTarget(drag.mouseX, drag.mouseY, drag.grabX)
    if(!drag.group.length) {
      return [{ item: drag.item, beat: target.snappedBeat, track: target.trackNumber }]
    }

    const clips = [drag.item, ...drag.group]
    const placed = clips.map(clip => ({
      id: clip.playlistItem.id,
      beat: clip.playlistItem.startBeat,
      track: clip.playlistItem.trackNumber,
    }))
    return groupLanding(placed, drag.item.playlistItem.id, target.snappedBeat, target.trackNumber, this.beatsDisplayed, this.trackCount)
      .map((landed, i) => ({ item: clips[i], beat: landed.beat, track: landed.track }))
  }

  // the top of a track's lane, in playlist y. a new track (see trackAt)
  // starts where the area below the last one does
  #laneTop = (trackNumber: number): number | null => {
    if(trackNumber === this.trackCount) return trackTop(this.trackCount)
    return this.playlistTracks.find(t => t.trackNumber === trackNumber)?.minHeight ?? null
  }

  // draw the snapped landing spots, and the dragged clips pinned to the mouse
  // called inside the zoom/pan transform
  drawClipDrag = (p: P5CanvasInstance<CanvasProps>, canvas: p5.Renderer) => {
    const drag = this.draggedClip(p, canvas)
    if(!drag) return

    // keep the preview's zoom in step with the other clips
    drag.item.currentScale = this.currentScale

    // phantoms: where the clips will land (after snapping, when snap is on)
    const ctx = p.drawingContext as CanvasRenderingContext2D
    p.push()
    ctx.setLineDash([4, 3])
    p.strokeWeight(1)
    p.stroke(...theme.accent)
    p.fill(...theme.accentSoft, 255*.25)
    this.landings(drag).forEach(({ item, beat, track }) => {
      const laneTop = this.#laneTop(track)
      if(laneTop === null) return
      const { width, height } = item.boundingBox()
      p.rect(this.beatToX(beat), laneTop + 2, width, height - 4, 3)
    })
    ctx.setLineDash([])
    p.pop()

    // the clips themselves, following the mouse. every clip moves as far as
    // the grabbed one, so the group keeps its shape
    const { left, top } = drag.item.boundingBox()
    p.push()
    p.translate(
      this.screenToPlaylistX(drag.mouseX) - drag.grabX - left,
      this.screenToPlaylistY(drag.mouseY) - drag.grabY - top,
    )
    drag.group.forEach(clip => clip.render())
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

  // convert a y position on the canvas (e.g. the mouse) to playlist y, which
  // tracks and clips are laid out in (see PlaylistTrack)
  screenToPlaylistY = (screenY: number): number => screenY + this.scrollY

  // the top of the area below the last track, for dropping onto a new track
  newTrackAreaTop = (): number => trackTop(this.trackCount)

  // the bottom of everything that scrolls, in playlist y
  contentHeight = (): number => this.newTrackAreaTop() + NEW_TRACK_AREA_HEIGHT

  // the furthest the tracks can scroll: 0 when they all fit
  maxScrollY = (): number => Math.max(0, this.contentHeight() - this.height)

  // scroll the tracks, kept in range
  scrollTo = (y: number) => {
    // `|| 0` also catches NaN, which would break every clip's hit-test
    this.scrollY = Math.min(Math.max(y, 0), this.maxScrollY()) || 0
  }

  // the vertical scrollbar, in canvas pixels. null when every track fits
  scrollbar = () => {
    const max = this.maxScrollY()
    if(max <= 0) return null

    const top = staticDefaults.timelineHeight
    const trackLength = this.height - top
    // the share of the tracks that's visible
    const visible = trackLength / (this.contentHeight() - top)
    const thumbHeight = Math.min(trackLength, Math.max(SCROLLBAR.minThumbHeight, trackLength * visible))
    const thumbTop = top + (trackLength - thumbHeight) * (this.scrollY / max)

    return { top, trackLength, thumbTop, thumbHeight, max }
  }

  // whether a point on the canvas is over the scrollbar (when it's shown)
  overScrollbar = (x: number, y: number): boolean => {
    const bar = this.scrollbar()
    return !!bar && x >= this.width - SCROLLBAR.hitWidth && x <= this.width && y >= bar.top
  }

  // while the thumb is dragged, keep it under the mouse
  #followScrollbarDrag = (p: P5CanvasInstance<CanvasProps>) => {
    const bar = this.scrollbar()
    if(this.scrollbarGrab === null || !bar) return

    const room = bar.trackLength - bar.thumbHeight
    if(room > 0) this.scrollTo((p.mouseY - this.scrollbarGrab - bar.top) / room * bar.max)
  }

  // dragging a clip (or a sample from the sidebar, or a selection box) near
  // the top or bottom of the tracks scrolls them, faster closer to the edge
  #autoScroll = (p: P5CanvasInstance<CanvasProps>) => {
    let y: number | null = null
    if(this.clipDrag?.active || this.lasso) y = p.mouseY
    else if(this.sampleDrag) y = this.sampleDrag.y
    if(y === null) return

    const { zone, maxSpeed } = AUTO_SCROLL
    const top = staticDefaults.timelineHeight
    const depth = (into: number) => Math.min(1, Math.max(0, into / zone))

    if(y < top + zone) this.scrollTo(this.scrollY - maxSpeed * depth(top + zone - y))
    else if(y > this.height - zone) this.scrollTo(this.scrollY + maxSpeed * depth(y - (this.height - zone)))
  }

  // the area below the last track, with a plus: drop a clip here for a new
  // track. highlighted while a dragged clip would land there
  #drawNewTrackArea = (p: P5CanvasInstance<CanvasProps>, canvas: p5.Renderer) => {
    // in canvas pixels, matching the tracks' translate in draw
    const top = this.newTrackAreaTop() - this.scrollY + 1
    if(top > this.height) return

    const drag = this.draggedClip(p, canvas)
    const targeted = !!drag
      && this.landings(drag).some(landing => landing.track === this.trackCount)

    const h = NEW_TRACK_AREA_HEIGHT
    // centred in what's visible, so it doesn't stretch with zoom or scroll away
    const cx = this.width / 2
    const cy = top + h / 2
    const radius = 10
    const arm = 5

    p.push()
    if(targeted) {
      p.noStroke()
      p.fill(...theme.accentSoft, 64)
      p.rect(0, top, this.width, h)
      p.stroke(...theme.accent)
    } else {
      p.stroke(...theme.idle)
    }

    p.strokeWeight(1.5)
    p.noFill()
    p.circle(cx, cy, radius * 2)
    p.line(cx - arm, cy, cx + arm, cy)
    p.line(cx, cy - arm, cx, cy + arm)

    if(targeted) {
      p.noStroke()
      p.fill(...theme.accent)
      p.textFont(theme.font)
      p.textSize(11)
      p.textAlign(p.LEFT, p.CENTER)
      p.text('New track', cx + radius + 6, cy)
    }
    p.pop()
  }

  #drawScrollbar = (p: P5CanvasInstance<CanvasProps>) => {
    const bar = this.scrollbar()
    if(!bar) return

    const active = this.scrollbarGrab !== null || this.overScrollbar(p.mouseX, p.mouseY)
    const { width, margin } = SCROLLBAR

    p.push()
    p.noStroke()
    p.fill(...theme.scrollbar, active ? 140 : 80)
    p.rect(this.width - width - margin, bar.thumbTop + 2, width, bar.thumbHeight - 4, width / 2)
    p.pop()
  }

  // zoom the timeline by `factor`, keeping the point at canvas x `aroundX`
  // in place. never zooms out past the whole playlist
  #zoomBy = (factor: number, aroundX: number) => {
    if(!Number.isFinite(factor) || factor <= 0) return

    const scale = Math.max(1, this.currentScale * factor)
    const applied = scale / this.currentScale
    this.currentScale = scale
    this.transformX = aroundX - (aroundX - this.transformX) * applied

    this.playlistObjects.forEach(obj => {
      obj.currentScale = this.currentScale
    })
  }

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
    // the clip on top, as for clicks (not one hidden under the timeline)
    const item = this.clipUnderMouse()
    if(item) {
      const { mouseX, mouseY } = p
      item.onRightClick(ev, {
        mouseX,
        mouseY,
        target: item,
      })
    }
  }

  // p5 sketch function
  sketch: Sketch<CanvasProps> = (p: P5CanvasInstance<CanvasProps>) => {
    // a new p5 instance (the playlist was remounted) builds its own clips
    this.#builtItems = null
    // size, zoom, and pan are read from `this` (not copied into locals), so
    // the components that convert between screen and playlist coordinates
    // see the same values the sketch draws with
    let {
      canvas,
      mousePressedX,
      mousePressedY,
    } = this
    this.#p = p

    p.setup = () => {
      const { width, height, trackCount } = this

      if(!width || !height) {
        console.error(`Invalid canvas dimensions.\nwidth: ${width}, height: ${height}`)
      }

      canvas = p.createCanvas(width, height)
      this.canvas = canvas
      this.syncSnapSetting()
      p.frameRate(60)
      if(!this.#visible) p.noLoop()
      p.noStroke()

      // anything done on the canvas shows (see wake). drags carry on past
      // its edges, and keep it drawing until they end (see #animating)
      const elt = canvas.elt as HTMLCanvasElement
      const wakeOn = ['pointerdown', 'pointermove', 'pointerup', 'wheel', 'dblclick', 'contextmenu', 'gesturechange']
      wakeOn.forEach(type => elt.addEventListener(type, this.wake, { passive: true }))
      // labels drawn before the font loaded are drawn again in it
      document.fonts?.ready.then(this.wake)
      this.wake()

      // handle mouse press, and pick up a clip if one was pressed on
      canvas.mousePressed((ev: MouseEvent) => {
        if(ev.button === 0) {
          // the scrollbar: grab the thumb, or jump to where the track was pressed
          const bar = this.scrollbar()
          if(bar && this.overScrollbar(p.mouseX, p.mouseY)) {
            const onThumb = p.mouseY >= bar.thumbTop && p.mouseY <= bar.thumbTop + bar.thumbHeight
            this.scrollbarGrab = onThumb ? p.mouseY - bar.thumbTop : bar.thumbHeight / 2
            this.#followScrollbarDrag(p)
            return
          }

          // the ruler moves the playhead, rather than panning
          if(overRuler(p.mouseY)) {
            this.#startSeek(p.mouseX)
            return
          }

          const item = this.clipUnderMouse()

          // shift+click a clip adds it to the selection, or takes it out.
          // shift+drag on empty space draws a box to select clips with
          if(ev.shiftKey) {
            if(item) this.selectedIds = toggleSelected(this.selectedIds, item.playlistItem.id)
            else this.#startLasso(p)
            return
          }

          mousePressedX = p.mouseX
          mousePressedY = p.mouseY
          this.isMouseDown = true
          this.#pressedOnEmpty = !item

          // picking up a clip that isn't selected leaves the selection
          if(item && !this.selectedIds.has(item.playlistItem.id)) this.clearSelection()

          if(item) {
            const { left, top } = item.boundingBox()
            this.clipDrag = {
              item,
              grabX: this.screenToPlaylistX(p.mouseX) - left,
              grabY: this.screenToPlaylistY(p.mouseY) - top,
              active: false,
              // a selected clip brings the rest of the selection with it
              group: this.playlistObjects.filter(clip =>
                clip !== item && this.selectedIds.has(clip.playlistItem.id)),
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
        this.#endSeek()
        this.lasso = null
        this.#pressedOnEmpty = false
        this.scrollbarGrab = null
        mousePressedX = null
        mousePressedY = null
        this.isMouseDown = false
        this.isMouseDragged = false
        this.clipDrag = null
        this.wake()
      }

      // the wheel or a two-finger swipe scrolls: up/down through the tracks,
      // left/right (or shift+wheel) along the timeline. ⌘+wheel zooms
      canvas.mouseWheel((ev: WheelEvent) => {
        ev.preventDefault()

        let unit = 1
        if(ev.deltaMode === WheelEvent.DOM_DELTA_LINE) unit = WHEEL_LINE_PIXELS
        else if(ev.deltaMode === WheelEvent.DOM_DELTA_PAGE) unit = this.height

        let dx = ev.deltaX * unit
        let dy = ev.deltaY * unit

        if(ev.metaKey || ev.ctrlKey) {
          const { zoomSensitivity } = staticDefaults
          const factor = Math.exp(-dy * WHEEL_ZOOM_PER_PIXEL)
          this.#zoomBy(Math.min(1 + zoomSensitivity, Math.max(1 - zoomSensitivity, factor)), p.mouseX)
          return
        }

        // a mouse wheel with shift held (some systems already swap these)
        if(ev.shiftKey && dx === 0) {
          dx = dy
          dy = 0
        }

        // kept in range when drawn
        this.transformX -= dx
        this.scrollTo(this.scrollY + dy)
      })

      // drop a dragged clip where the phantom shows
      canvas.mouseReleased(() => {
        const drag = this.clipDrag
        const target = this.dropTarget(p.mouseX, p.mouseY, drag?.grabX)

        if(drag?.active && target.trackNumber >= 0) {
          // send the unsnapped beat, the engine owns snapping
          if(drag.group.length) {
            const ids = [drag.item, ...drag.group].map(clip => clip.playlistItem.id)
            this.onNodesMove(ids, drag.item.playlistItem.id, target.beat, target.trackNumber)
          } else {
            this.onNodeMove(drag.item.playlistItem, target.beat, target.trackNumber)
          }
        }

        // a click on empty space (not a pan) clears the selection
        if(this.#pressedOnEmpty && !this.isMouseDragged) this.clearSelection()
        this.#pressedOnEmpty = false
        this.lasso = null

        mousePressedX = null
        mousePressedY = null
        this.isMouseDown = false
        this.isMouseDragged = false
        this.clipDrag = null
        this.scrollbarGrab = null
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
              this.scrollTo(this.scrollY + (p.mouseY - ev.offsetY))
            }
          }
        }
      })

      // Esc clears the selection (and drops a box being drawn)
      p.keyPressed = () => {
        if(p.keyCode !== p.ESCAPE) return
        this.lasso = null
        this.clearSelection()
        this.wake()
      }

      // handle right click
      this.canvasEl = document.getElementById(canvas.id())
      this.canvasEl?.addEventListener('contextmenu', (e) => {
        e.preventDefault()
        this.#handleRightClick(e, p)
      })

      // a trackpad pinch zooms (WebKit). preventing the default also stops
      // it zooming the whole window
      let pinchScale = 1
      this.canvasEl?.addEventListener('gesturestart', (e) => {
        e.preventDefault()
        pinchScale = 1
      })
      this.canvasEl?.addEventListener('gesturechange', (e) => {
        e.preventDefault()
        const { scale } = e as GestureEvent
        this.#zoomBy(scale / pinchScale, p.mouseX)
        pinchScale = scale
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
      if(!canvas) {
        console.error("this.canvas in the playlist Renderer was null when p.updatingWithProps was called.")
        return
      }
      this.wake()

      // the canvas's box can be unmeasured for a moment (e.g. while a split
      // pane remounts the playlist). keep the last good size: an undefined
      // width would turn the pan offset into NaN in draw
      if(props.height > 0) this.height = props.height
      if(props.width > 0) this.width = props.width
      this.maxPlaylistBeats = props.maxPlaylistBeats
      const tracksChanged = props.trackCount !== this.trackCount
      this.trackCount = props.trackCount
      this.tempo = props.tempo
      this.beatsDisplayed = props.beatsDisplayed
      const { width, height, trackCount, currentScale } = this

      this.onNodeRightClick = props.onNodeRightClick
      this.onNodeMove = props.onNodeMove
      this.onNodesMove = props.onNodesMove
      this.onNodeDoubleClick = props.onNodeDoubleClick
      this.onSeek = props.onSeek

      // ask the engine where the playhead is when playback starts or stops,
      // or after any other transport command. other prop updates (drops,
      // moves, resizes) don't move it
      if(props.playing !== this.playing || props.transportRevision !== this.transportRevision) {
        this.transportRevision = props.transportRevision
        this.syncTransport()
      }

      this.playing = props.playing

      // the canvas follows the playlist's box. clips work out where they are
      // from the canvas's size as they're drawn, but the track lines are as
      // wide as the timeline was when they were built
      if(p.width !== width || p.height !== height) {
        p.resizeCanvas(width, height)
        this.playlistTracks.forEach(track => { track.timelineWidth = width })
      }

      // the clips only need building again when they (or the tracks) change.
      // tempo, zoom, and size are read from the renderer as they're drawn
      if(props.playlistObjects === this.#builtItems && !tracksChanged) return
      this.#builtItems = props.playlistObjects

      // the clips are rebuilt once every clip's data has arrived. until then
      // the current clips stay, so a click while rebuilding still finds them
      this.#rebuildGeneration += 1
      const generation = this.#rebuildGeneration
      const built = canvas

      // clips already in the playlist keep the waveform they were built with,
      // so only new clips ask the engine for theirs. (ids are never reused,
      // and their length follows their settings, see #applyClipOverride)
      const current = new Map(this.playlistObjects.map(clip => [clip.playlistItem.id, clip]))

      // a clip's waveform and length (seconds)
      const clipData = async (item: PlaylistItem) => {
        // a pattern clip has no sample: it draws its pattern, and is as long
        // as the pattern is (see PlaylistObject)
        if(item.patternId !== undefined) {
          return { wf: new Float32Array(0), dur: this.patternSecondsOf(item.patternId) }
        }
        const known = current.get(item.id)
        if(known) return { wf: known.soundData, dur: known.duration }

        let fetching = this.#fetching.get(item.id)
        if(!fetching) {
          fetching = fetchNodeData(item.id).finally(() => this.#fetching.delete(item.id))
          this.#fetching.set(item.id, fetching)
        }
        return fetching
      }

      const buildClip = async (item: PlaylistItem): Promise<PlaylistObject | null> => {
        try {
          const { wf: soundData, dur: duration } = await clipData(item)
          const clip = new PlaylistObject(p, built, this, {
            currentScale,
            soundData,
            duration,
            timelineWidth: width,
            timelineHeight: staticDefaults.timelineHeight,
            trackNumber: item.trackNumber,
            trackHeight: staticDefaults.trackHeight,
            playlistItem: item,
          })
          this.#applyClipOverride(clip)
          return clip
        } catch(e) {
          console.error('could not load clip', item.id, e)
          return null
        }
      }

      Promise.all(props.playlistObjects.map(buildClip)).then(clips => {
        // a newer update started while these loaded, it has the final say
        if(generation !== this.#rebuildGeneration) return

        // in the playlist's order, not the order the data arrived in
        const newPlaylistObjects = clips.filter((c): c is PlaylistObject => c !== null)
        // zoom may have changed while loading
        newPlaylistObjects.forEach(obj => { obj.currentScale = this.currentScale })

        const newPlaylistTracks: Array<PlaylistTrack> = []
        for(let i = 0; i < trackCount; i++) {
          newPlaylistTracks.push(new PlaylistTrack(p, built, this, {
            currentScale: this.currentScale,
            trackCount,
            // the canvas may have been resized while loading
            timelineWidth: this.width,
            timelineHeight: staticDefaults.timelineHeight,
            trackNumber: i,
            trackHeight: staticDefaults.trackHeight,
            playlistObjects: newPlaylistObjects,
          }))
        }

        this.playlistObjects = newPlaylistObjects
        this.playlistTracks = newPlaylistTracks
        this.selectedIds = keepExisting(this.selectedIds, newPlaylistObjects.map(c => c.playlistItem.id))
        this.wake()
      })
    }

    // render p5 canvas
    p.draw = () => {
      if(!canvas) return

      // nothing's changed for a while: this is the last frame until
      // something does (see wake)
      if(!this.#animating() && performance.now() - this.#lastWake > IDLE_MS) p.noLoop()

      const { width, trackCount, currentScale, beatsDisplayed } = this

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
      
      p.background(...theme.background)

      this.transformX = p.constrain(this.transformX, width * (1 - currentScale), 0)
      // NaN never recovers through constrain, and breaks every clip's
      // hit-test (see screenToPlaylistX), so start over from no pan
      if(!Number.isFinite(this.transformX)) this.transformX = 0

      this.#followScrollbarDrag(p)
      this.#autoScroll(p)
      // kept in range: the canvas or the track count may have changed size
      this.scrollTo(this.scrollY)
      this.#followLasso(p)

      // the tracks, scrolled
      p.push()
      p.translate(this.transformX, 1 - this.scrollY)
      p.scale(currentScale, 1)

      // every other track lane is shaded a little
      p.noStroke()
      p.fill(...theme.laneAlt)
      for(let n = 1; n < trackCount; n += 2) {
        p.rect(0, trackTop(n), width, staticDefaults.trackHeight)
      }

      // render vertical grid lines: one per beat, darker on each bar. 1px
      // wide whatever the zoom
      p.strokeWeight(1 / currentScale)
      const gridBottom = trackTop(trackCount)
      for(let beat = 0; beat < beatsDisplayed; beat++) {
        const x = beat * width / beatsDisplayed
        p.stroke(...(beat % theme.beatsPerBar === 0 ? theme.barLine : theme.beatLine))
        p.line(x, 0, x, gridBottom)
      }

      // render playlist tracks
      this.playlistTracks.forEach(track => {
        track.render()
      })

      // render the clip being dragged on top of the other clips
      this.drawClipDrag(p, canvas)
      
      p.pop()

      this.#drawNewTrackArea(p, canvas)
      // under the ruler, which covers it when it reaches past the top
      this.#drawLasso(p)

      // the timeline and playhead stay at the top, over the scrolled tracks
      p.push()
      p.translate(this.transformX, 1)
      p.scale(currentScale, 1)
      timeline.render()
      p.pop()

      // follow a ruler drag, even outside the canvas
      if(this.seekDragging) this.seekBeat = this.rulerBeat(p.mouseX)
      cursor.render()

      this.#drawScrollbar(p)

      if(this.lasso) {
        p.cursor(p.CROSS)
      } else if(this.scrollbarGrab !== null || this.overScrollbar(p.mouseX, p.mouseY)) {
        p.cursor(p.ARROW)
      } else if(this.seekDragging || (overRuler(p.mouseY) && !this.clipDrag?.active && !this.sampleDrag)) {
        p.cursor(p.HAND)
      } else if(this.clipDrag?.active || this.sampleDrag) {
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
        const debugWinHeight = 157
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
            `scroll: ${Math.round(this.scrollY)} / ${Math.round(this.maxScrollY())}`,
            `sample drag: ${this.#sampleDragStatus()}`,
            `selected: ${this.selectedIds.size}${this.lasso ? ' (box)' : ''}`,
          ].join('\n'),
          p.width - debugWinWidth + 6,
          debugWinTop + 14,
        )
      }
    }
  }
}
