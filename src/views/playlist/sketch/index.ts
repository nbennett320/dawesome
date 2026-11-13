/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
  Sketch,
} from 'react-p5-wrapper'
import { invoke } from '@tauri-apps/api/core'
import Timeline from './Timeline'
import Cursor from './Cursor'
import PlaylistTrack from './PlaylistTrack'
import PlaylistObject from './PlaylistObject'
import RendererBase from '../../../render/RendererBase'
import { ClipOverride, ClipWindow } from '../../../types/clip'
import {
  PlaylistItem,
  PlaylistItemPixelOffset,
  PlaylistWindow
} from '../../../types/playlist'
import { CutItem, canCut, cutPoint, cutTargets } from '../../../editing/cut'
import { onThemeChange } from '../../../theme/apply'
import { paint, patternColors, theme, trackPaint } from './theme'
import { painter } from '../../../theme/canvas'
import { LIP, LipLook, drawLip, fitName, inBox, lipBox, lipWidth, onLed } from './lip'
import { trackColorKey, defaultTrackColor, trackLabel } from '../../../types/mixer'
import { DEFAULT_BEATS_PER_BAR, DEFAULT_PLAYLIST_BEATS, PlaylistTransport } from '../../../types/transport'
import { positionAt } from '../../transport/tape'
import {
  Lasso, ShiftPress, itemAt, keepExisting, lassoFrom, lassoRect, lassoSelection, shiftRelease, startsLasso,
  toggleSelected,
} from '../../../editing/lasso'
import { groupLanding } from '../../../editing/move'
import { snapDown } from '../../../editing/snap'
import { drawCutLine, drawLasso } from '../../../editing/look'
import { Tool } from '../../../editing/tools'
import { DRAG_THRESHOLD_PX, wheelIntent, zoomAround } from '../../../editing/view'
import type { MovedNode as Landed } from '../../../state/slices/playlistSlice'
import { PlaceableItem as DragItem, dragItemKey } from '../../../state/sampleDrag'
import { Pattern, SequencerState, patternSeconds } from '../../../types/sequencer'
import { decodeLevels } from '../../../util/waveform'
import { gridSteps, maxScale, openingScale, panForWidth, visibleBeats } from './grid'
import { lineLevel, tileStep } from '../../../editing/beatGrid'
import {
  Edge, MAX_PITCH, Snapping, Span, StretchClip, TrimClip, edgeAt, patternTrimClip, pitchOf, sampleTrimClip,
  snapPoint, stretchAll, stretchSet, trimAll, trimmedWindow,
} from './edges'
import { paintedSpans } from './paint'
import { clamp } from '../../../util/helpers'

export interface CanvasProps extends SketchProps {
  height: number
  width: number
  playing: boolean,
  tempo: number
  beatsDisplayed: number
  trackCount: number
  // colors given to tracks, by track number (see mixerSlice). tracks not
  // listed are their default color
  trackColors: Record<number, number>
  // names given to tracks, by track number (see mixerSlice). tracks not
  // listed are "Track n"
  trackNames: Record<number, string>
  // each track's mute and solo in the mixer (tracks not listed are neither),
  // shown on its lip
  trackMixes: Record<number, { mute: boolean, solo: boolean }>
  // a lip's LED was clicked (mute) or shift+clicked (solo): set it to this
  onTrackMute: (trackNumber: number, mute: boolean) => void
  onTrackSolo: (trackNumber: number, solo: boolean) => void
  // a lip was right-clicked, at clientX, clientY in the window
  onLipRightClick: (trackNumber: number, clientX: number, clientY: number) => void
  playlistObjects: PlaylistItem[]
  onItemDrop: (pw: PlaylistWindow) => void
  // a clip was right-clicked, at clientX, clientY in the window
  onNodeRightClick: (id: number, clientX: number, clientY: number) => void
  // empty space over the tracks was right-clicked, at this beat and track
  onEmptyRightClick: (beat: number, track: number, clientX: number, clientY: number) => void
  onNodeMove: () => void
  // several selected clips were dragged together (see Renderer.onNodesMove)
  onNodesMove: (ids: number[], anchorId: number, dropBeat: number, trackNumber: number, shown: Landed[]) => void
  onNodeDoubleClick: (id: number) => void
  // see PlaylistState.transportRevision
  transportRevision: number
  // whether the playlist loops (the ruler marks where the loop ends)
  loopEnabled: boolean
  // move the playhead to a beat (clicking or dragging in the ruler)
  onSeek: (beat: number) => void
  // clips' edges were dragged (see Renderer.onClipsTrim)
  onClipsTrim: (edits: WindowEdit[]) => Promise<unknown>
  // and with shift held (see Renderer.onClipsStretch)
  onClipsStretch: (edits: StretchEdit[]) => Promise<unknown>
  // clips were clicked with the Cut tool (see Renderer.onClipsCut)
  onClipsCut: (ids: number[], beat: number) => Promise<unknown>
  // clips were painted with the Draw tool (see Renderer.onPaint)
  onPaint: (patternId: number | null, trackNumber: number, startBeat: number, endBeat: number) => Promise<unknown>
  // the plus below the last track was clicked (see Renderer.onAddTrack)
  onAddTrack: () => Promise<unknown>
  // the grid's step changed, in beats
  onGridStep?: (beats: number) => void
}

// how the Cut tool's cursor is drawn: a razor blade, its middle on the
// spot, or a crosshair where the webview won't show that
const RAZOR_CURSOR = "url('/cursors/razor.svg') 12 12, crosshair"

// a sample clip to stretch: the pitch that makes it the length it was
// dragged to, and the edge that stays where it is, as the engine's
// stretch_clips takes it
export interface StretchEdit {
  id: number
  pitch: number
  keep: 'start' | 'end'
}

// the part of its sample (seconds) or pattern (steps) a clip is to play
// after a trim, as the engine's set_clip_windows takes it
export type WindowEdit = { id: number } & ClipWindow

// where a clip is drawn, and what of its source it shows
export interface ClipView {
  startBeat: number
  // how long it is
  seconds: number
  // the part of its source it shows, and the source's whole length: in
  // seconds of the sample, or steps of the pattern (where a pattern clip
  // starts and ends in its pattern, which can be past its end: it loops)
  from: number
  to: number
  length: number
}

export const staticDefaults = {
  mouseDragDetectionThreshold: DRAG_THRESHOLD_PX,
  timelineHeight: 24,
  trackHeight: 100,
  playing: false,
  debugMode: false,

  // defaults on Renderer construction, these will be reassigned
  // when the canvas renders
  height: 100,
  width: 200,
  trackCount: 4,
  tempo: 120,

  // class specific defaults
  PlaylistObject: {
    labelHeight: 20,
    tabHeight: 28,
  },
}

// the top of a track, in playlist y
export const trackTop = (trackNumber: number, trackHeight: number = staticDefaults.trackHeight): number =>
  trackHeight * trackNumber + trackNumber * .3 + staticDefaults.timelineHeight

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

// a lane's label is printed in the four-track controls' colors (see lip.ts)
const fourtrackPaint = painter('fourtrack')

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

// an edge of a clip being dragged, to trim it (and the same edge of the
// other selected clips, when it's one of them), or with shift held when it
// was pressed, to stretch it
interface EdgeDrag {
  edge: Edge
  grabbedId: number
  // set when shift was held at the press, whatever it does afterwards: the
  // sample clips to stretch, the grabbed one first (none for a pattern
  // clip, which can't be stretched: the drag does nothing)
  stretch: StretchClip[] | null
  // the grabbed clip, then the clips trimmed with it, as they were when
  // the edge was pressed
  clips: TrimClip[]
  // how far the edge was from the mouse when it was pressed, in beats, so
  // it doesn't jump to the mouse
  grabBeats: number
  // where it was pressed, on the canvas
  pressX: number
  pressY: number
  // false until the mouse moves past the drag threshold, so a click on an
  // edge trims nothing
  active: boolean
}

// clips being painted along a lane with the Draw tool, while the mouse is
// held. in beats, already snapped
interface Paint {
  track: number
  // where the press was, snapped down to the line at or before it
  start: number
  // where the mouse is, snapped to the nearest line
  end: number
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
  trackCount: number = staticDefaults.trackCount
  trackColors: Record<number, number> = {}
  trackMixes: Record<number, { mute: boolean, solo: boolean }> = {}
  onTrackMute!: (trackNumber: number, mute: boolean) => void
  onTrackSolo!: (trackNumber: number, solo: boolean) => void
  onLipRightClick!: (trackNumber: number, clientX: number, clientY: number) => void
  trackNames: Record<number, string> = {}
  // each name as its lip shows it, and the lip's width, by name: measured
  // once per name, not every frame
  #lipLabels = new Map<string, { text: string, width: number }>()
  tempo: number = staticDefaults.tempo
  // the playlist's length in beats: the full canvas width (at scale 1)
  // spans this many
  beatsDisplayed = DEFAULT_PLAYLIST_BEATS
  debugMode: boolean = staticDefaults.debugMode

  // the playlist opens zoomed in on its first bars (see openingScale)
  currentScale = openingScale(DEFAULT_PLAYLIST_BEATS)
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
  
  // resizing (dragging a pane's divider) keeps the beat at the left edge
  // where it was, rather than panning the timeline
  setWidth = (width: number) => {
    this.transformX = panForWidth(this.transformX, this.width, width)
    this.width = width
  }

  isMouseDown = false
  isMouseDragged = false
  mousePressedX: number | null = null
  mousePressedY: number | null = null

  playing: boolean = staticDefaults.playing
  // where the engine's playhead is, and its timing while playing, used to
  // draw the playback cursor
  transport: PlaylistTransport | null = null
  transportRevision: number | null = null
  // the time signature's beats to the bar, which the grid and ruler count
  // bars in
  get beatsPerBar(): number { return this.transport?.beatsPerBar || DEFAULT_BEATS_PER_BAR }
  loopEnabled: boolean | null = null
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
  edgeDrag: EdgeDrag | null = null
  // something slow is being written: the mouse shows the wait cursor
  busy = false
  setBusy = (busy: boolean) => {
    this.busy = busy
    this.wake()
  }
  // where the clips being trimmed start and end: while an edge is dragged,
  // and after it's let go until the engine's answer has been drawn, so the
  // clips don't flick back to where they were in between
  #trimSpans: Map<number, Span> | null = null
  // those clips as they were when the edge was pressed, which the spans
  // are measured against (the clips themselves change as the engine's
  // answer arrives: their windows first, their starts a moment later)
  #trimClips: Map<number, TrimClip> | null = null
  // and how fast each clip being stretched would play, by id
  #stretchRates: Map<number, number> | null = null
  // ends that wait (see #endEdgeDrag), and whether it's the clips being
  // built again that it waits for (a clip's start moved) rather than just
  // their windows arriving
  #settleTrim: (() => void) | null = null
  #settleOnRebuild = false
  // the part of its sample or pattern each trimmed clip plays, by id
  clipWindows: Record<number, ClipWindow> = {}
  // the edge under the mouse that a press would grab, and its clip's id
  #hoverEdge: { id: number, edge: Edge } | null = null
  // whether Option is held: an edge being dragged doesn't snap
  #altHeld = false
  // what the mouse does to clips: move and trim them, or cut them
  tool: Tool = 'select'
  // a clip pressed with the Cut tool, and where: let go there, it's cut
  #cutPress: { x: number, y: number, id: number } | null = null
  // the cut a click would make now, with the Cut tool: the clips it would
  // cut, the one under the mouse first, and the beat
  #cutHover: { ids: number[], beat: number } | null = null
  // clips being painted with the Draw tool, while the mouse is held
  #paint: Paint | null = null
  // the pattern the Draw tool paints: the one last selected in a
  // sequencer. null paints a new empty pattern
  paintPatternId: number | null = null

  // the sequencer's rows and patterns, which pattern clips draw. read every
  // frame, so editing a pattern shows straight away without rebuilding
  sequencer: SequencerState = { rows: [], patterns: [], folders: [] }

  // the clips picked with shift+click or a shift+drag box, by id
  selectedIds: Set<number> = new Set()
  // the box being dragged out with shift held, if any
  lasso: Lasso | null = null
  // a press with shift held that hasn't moved far enough to be a box yet:
  // let go where it is, it's a click (see #endShiftPress)
  #shiftPress: ShiftPress | null = null
  // whether the press was on empty track space (not a clip, the ruler or
  // the scrollbar), so a click there without dragging clears the selection
  #pressedOnEmpty = false
  // refreshed from the engine whenever a drag starts
  snap: SnapSetting = { enabled: true, beats: 1 }

  // reassigned function definitions
  onNodeRightClick!: (id: number, clientX: number, clientY: number) => void
  onEmptyRightClick!: (beat: number, track: number, clientX: number, clientY: number) => void
  onNodeDoubleClick!: (id: number) => void
  // a clip was dropped at dropBeat (unsnapped) on trackNumber. `shownBeat`
  // is where its phantom showed it landing, which it's drawn at until the
  // engine says where it went
  onNodeMove!: (
    item: PlaylistItem,
    dropBeat: number,
    trackNumber: number,
    shownBeat: number,
  ) => void
  // the grabbed clip (anchorId) was dropped at dropBeat (unsnapped) on
  // trackNumber, and every clip in ids moves by as much. `shown` is where
  // their phantoms showed them landing (as onNodeMove's shownBeat)
  onNodesMove!: (ids: number[], anchorId: number, dropBeat: number, trackNumber: number, shown: Landed[]) => void
  // clips' edges were dragged: each clip in `edits` is to play that part of
  // its sample or pattern. resolves to what the engine changed, or nothing
  // if it refused
  onClipsTrim!: (edits: WindowEdit[]) => Promise<unknown>
  // clips' edges were dragged with shift held: each clip in `edits` is to
  // be stretched to that pitch
  onClipsStretch!: (edits: StretchEdit[]) => Promise<unknown>
  // clips were clicked with the Cut tool: each clip in `ids` is to be cut
  // in two at `beat`
  onClipsCut!: (ids: number[], beat: number) => Promise<unknown>
  // clips were painted with the Draw tool: clips of `patternId` (null for a
  // new pattern) on trackNumber from startBeat to endBeat, snapped
  onPaint!: (patternId: number | null, trackNumber: number, startBeat: number, endBeat: number) => Promise<unknown>
  // the plus below the last track was clicked: an empty track goes under it
  onAddTrack!: () => Promise<unknown>
  // the grid's step changed, in beats (see adaptiveStep): for the readout
  // by the snap control
  onGridStep?: (beats: number) => void
  #gridStep = 0
  #reportGridStep = (beats: number) => {
    if(beats === this.#gridStep) return
    this.#gridStep = beats
    this.onGridStep?.(beats)
  }

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

      // the loop's length changed while playing: the engine takes it up on
      // its next beat (and counts the playhead from there), so ask again
      // once it has
      const before = this.transport
      if(transport.playing && before && before.loopBeats !== transport.loopBeats) {
        if(this.#loopSync) clearTimeout(this.#loopSync)
        this.#loopSync = setTimeout(() => {
          this.#loopSync = null
          this.syncTransport()
        }, transport.beatIntervalMs + 250)
      }

      this.transport = transport
      // a released seek has reached the engine
      if(!this.seekDragging) this.seekBeat = null
    } catch(e) {
      console.error('could not sync the playback cursor:', e)
    }
    this.wake()
  }

  #loopSync: ReturnType<typeof setTimeout> | null = null

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

  // back to how the playlist opens: its first bars, from the top (a project
  // was opened, or the playlist's length changed)
  resetView = () => {
    this.currentScale = openingScale(this.beatsDisplayed)
    this.transformX = 0
    this.scrollY = 0
    this.playlistObjects.forEach(obj => {
      obj.currentScale = this.currentScale
    })
    this.wake()
  }

  // where the loop ends, in beats, while the playlist loops
  loopEndBeat = (): number | null => {
    const t = this.transport
    return t && t.loopEnabled && t.loopBeats > 0 ? t.loopBeats : null
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

  // select just these clips (the ones an undo or redo changed). ones that
  // aren't in the playlist yet are kept once they are (see keepExisting)
  select = (ids: number[]) => {
    this.selectedIds = new Set(ids)
    this.wake()
  }

  // where a paste goes: the playhead, snapped like everything else
  pasteBeat = (): number | null => {
    const beat = this.playheadBeat()
    return beat === null ? null : this.snapBeat(beat)
  }

  // the selected clips' ids, clearing the selection (to delete them). none
  // while clips are being dragged, a box drawn, or a shift press is down
  // (whose clip would be toggled on release, after it was gone)
  takeSelection = (): number[] => {
    if(this.clipDrag?.active || this.lasso || this.#shiftPress || this.edgeDrag || this.#paint) return []
    const ids = [...this.selectedIds]
    this.clearSelection()
    return ids
  }

  // the mouse, in playlist coordinates
  #mousePoint = (p: P5CanvasInstance<CanvasProps>) => ({
    x: this.screenToPlaylistX(p.mouseX),
    y: this.screenToPlaylistY(p.mouseY),
  })

  // a shift press becomes a selection box once the mouse has moved far
  // enough from it, wherever it started: on empty space or on a clip. run
  // every frame, as the mouse may be outside the canvas by then
  #followShiftPress = (p: P5CanvasInstance<CanvasProps>) => {
    const press = this.#shiftPress
    if(!press || this.lasso) return
    if(!startsLasso(press, { x: p.mouseX, y: p.mouseY }, staticDefaults.mouseDragDetectionThreshold)) return

    this.lasso = lassoFrom(press, this.selectedIds)
    this.#shiftPress = null
  }

  // every clip's id and box, for working out what a box selects
  #clipBoxes = () =>
    this.playlistObjects.map(clip => ({ id: clip.playlistItem.id, box: clip.boundingBox() }))

  // a shift press was let go. one that never became a box was a click:
  // on a clip, it adds the clip to the selection, or takes it out (see
  // shiftRelease, which also covers a drag let go before a frame was drawn)
  #endShiftPress = (p: P5CanvasInstance<CanvasProps>) => {
    const press = this.#shiftPress
    if(!press) return
    this.#shiftPress = null
    this.selectedIds = shiftRelease(
      press,
      { screen: { x: p.mouseX, y: p.mouseY }, point: this.#mousePoint(p) },
      staticDefaults.mouseDragDetectionThreshold,
      this.selectedIds,
      this.#clipBoxes(),
    )
  }

  // the window lost the keyboard and mouse mid-press (another app came to
  // the front), so the release may never arrive: drop a pending shift press
  // and a box being drawn, keeping what's selected
  #dropPress = () => {
    this.#altHeld = false
    this.#cutPress = null
    if(!this.#shiftPress && !this.lasso && !this.edgeDrag && !this.#paint) return
    this.#shiftPress = null
    this.#paint = null
    this.lasso = null
    this.#cancelEdgeDrag()
    this.wake()
  }

  // Option, held or let go, changes whether a dragged edge snaps
  #followAlt = (ev: KeyboardEvent) => {
    if(this.#altHeld === ev.altKey) return
    this.#altHeld = ev.altKey
    // (the Cut tool's line moves too, and a paint's end)
    if(this.edgeDrag || this.#paint || this.tool === 'cut') this.wake()
  }

  // what an edge being dragged, or a cut, snaps to just now
  #snapping = (): Snapping => ({
    enabled: this.snap.enabled,
    every: this.snap.beats,
    // the playhead too, while it isn't moving
    playhead: this.playing ? null : this.playheadBeat(),
    bypass: this.#altHeld,
  })

  // ---- the Cut tool ----

  // (called on every store update, so only a change does anything)
  setTool = (tool: Tool) => {
    if(tool === this.tool) return
    this.tool = tool
    // whatever the other tool was in the middle of is dropped
    this.#cancelEdgeDrag()
    this.clipDrag = null
    this.#cutPress = null
    this.#paint = null
    this.syncSnapSetting()
    this.wake()
  }

  // ---- the Draw tool ----

  // what the Draw tool paints (sequencer.lastSelected)
  // (called on every store update, so only a change does anything)
  setPaintPattern = (patternId: number | null) => {
    if(patternId === this.paintPatternId) return
    this.paintPatternId = patternId
    if(this.#paint) this.wake()
  }

  // the beat at canvas x `screenX`, kept to the playlist
  #beatAt = (screenX: number): number =>
    clamp(this.screenXToBeat(screenX), 0, this.beatsDisplayed)

  // a press on empty lane space with the Draw tool, at canvas x, y: a paint
  // starts there, on the line at or before the mouse
  #startPaint = (x: number, y: number) => {
    const track = this.trackAt(this.screenToPlaylistY(y))
    if(track < 0) return
    const start = snapDown(this.#beatAt(x), this.#snapping())
    this.#paint = { track, start, end: start }
    this.syncSnapSetting()
  }

  // the paint's end follows the mouse along its lane, snapped. run every
  // frame, as the mouse may be outside the canvas
  #followPaint = (p: P5CanvasInstance<CanvasProps>) => {
    if(!this.#paint) return
    this.#paint.end = snapPoint(this.#beatAt(p.mouseX), this.#snapping())
  }

  // the clips the paint would lay down if let go now (see paintedSpans):
  // the pattern's, or one of the pattern it would make
  #paintSpans = (painting: Paint): Span[] => {
    const pattern = this.paintPatternId === null ? undefined : this.patternOf(this.paintPatternId)
    return paintedSpans(painting.start, painting.end, pattern ? pattern.length : null, this.beatsPerBar)
  }

  // the mouse came up: the clips are painted where their phantoms were,
  // wherever it was let go
  #endPaint = (p: P5CanvasInstance<CanvasProps>) => {
    const painting = this.#paint
    if(!painting) return
    // (once more, to where the mouse was let go)
    this.#followPaint(p)
    this.#paint = null
    // a pattern that's gone paints a new one, as its phantom showed
    const pattern = this.paintPatternId !== null && this.patternOf(this.paintPatternId) ? this.paintPatternId : null
    this.onPaint(pattern, painting.track, painting.start, painting.end)
    this.wake()
  }

  // where each clip of the paint will be, outlined like a dragged clip's
  // landing. called inside the zoom/pan transform
  #drawPaint = (p: P5CanvasInstance<CanvasProps>) => {
    const painting = this.#paint
    if(!painting) return
    const laneTop = this.#laneTop(painting.track)
    if(laneTop === null) return

    const ctx = p.drawingContext as CanvasRenderingContext2D
    p.push()
    ctx.setLineDash([4, 3])
    p.strokeWeight(1)
    paint.stroke(p, 'accent')
    paint.fill(p, 'accentSoft', 0.25)
    // in screen pixels across, so zoom doesn't stretch the outline or its dashes
    p.scale(1 / this.currentScale, 1)
    this.#paintSpans(painting).forEach(({ start, end }) => {
      const left = this.beatToX(start) * this.currentScale
      const width = this.beatToX(end - start) * this.currentScale
      p.rect(left, laneTop + 2, width, staticDefaults.trackHeight - 4, 3)
    })
    ctx.setLineDash([])
    p.pop()
  }

  // every clip that can be cut, as far as cutting goes
  #cutClips = (): CutItem[] => this.playlistObjects.flatMap(obj => {
    const clip = this.trimClipOf(obj)
    return clip ? [clip] : []
  })

  // the cut a click at the mouse would make: on the clip under it, where
  // the mouse is after snapping, and through every other selected clip
  // there when that clip is one of the selection. null where there's no
  // clip, or the clip can't be cut
  #cutAt = (p: P5CanvasInstance<CanvasProps>): { ids: number[], beat: number } | null => {
    const item = this.clipUnderMouse()
    const clip = item ? this.trimClipOf(item) : null
    if(!clip) return null

    const beat = cutPoint(clip, this.screenXToBeat(p.mouseX), this.#snapping())
    if(beat === null) return null
    const ids = cutTargets(this.#cutClips(), clip.id, this.selectedIds, beat)
    return ids.length ? { ids, beat } : null
  }

  // a press with the Cut tool was let go: where it went down (give or take
  // the drag distance), it's a click, and cuts. dragged away, it's nothing
  #endCutPress = (p: P5CanvasInstance<CanvasProps>) => {
    const press = this.#cutPress
    if(!press) return
    this.#cutPress = null
    if(p.dist(press.x, press.y, p.mouseX, p.mouseY) > staticDefaults.mouseDragDetectionThreshold) return

    const cut = this.#cutAt(p)
    // (the clip that was pressed, not one the mouse slid onto)
    if(cut && cut.ids[0] === press.id) this.onClipsCut(cut.ids, cut.beat)
  }

  // a dashed line down each clip a click would cut, where it would be cut.
  // called inside the zoom/pan transform
  #drawCutLine = (p: P5CanvasInstance<CanvasProps>) => {
    const cut = this.#cutHover
    if(!cut) return

    const ctx = p.drawingContext as CanvasRenderingContext2D
    const x = this.beatToX(cut.beat)
    this.playlistObjects
      .filter(obj => cut.ids.includes(obj.playlistItem.id))
      .forEach(obj => {
        const { top, bottom } = obj.boundingBox()
        // (x is stretched by the zoom, so the line's width is given without it)
        drawCutLine(ctx, x, top + 2, bottom - 2, 1.5 / this.currentScale)
      })
  }

  // the playhead's beat, if a clip can be cut there ("Split at playhead"):
  // unsnapped, wherever the playhead is now, playing or not
  splitBeatAtPlayhead = (id: number): number | null => {
    const obj = this.playlistObjects.find(o => o.playlistItem.id === id)
    const clip = obj ? this.trimClipOf(obj) : null
    const beat = this.playheadBeat()
    return clip && beat !== null && canCut(clip, beat) ? beat : null
  }

  // ---- trimming: dragging a clip's edge ----

  // a beat's length, in seconds
  #beatSeconds = (): number => (this.tempo > 0 ? 60 / this.tempo : 0)

  // a clip, as far as trimming goes: where its edges are, and how far they
  // can go. null for a sample clip whose length hasn't loaded yet, and a
  // pattern clip whose pattern is gone
  trimClipOf = (obj: PlaylistObject): TrimClip | null => {
    const { id, startBeat, patternId } = obj.playlistItem
    const window = this.clipWindows[id]
    if(patternId !== undefined) {
      const pattern = this.patternOf(patternId)
      if(!pattern) return null
      // (the engine gives every pattern clip a window: until it has, the
      // clip is all of its pattern, as a new clip is)
      const own = window ?? { start: 0, end: pattern.length }
      return patternTrimClip(id, startBeat, own, pattern.length, this.beatsDisplayed)
    }

    const override = this.clipOverrides[id]
    const beatSeconds = this.#beatSeconds()
    if(!override?.sampleDuration || !beatSeconds) return null
    return sampleTrimClip(id, startBeat, override.sampleDuration, override.rate ?? 1, window, beatSeconds)
  }

  // where a clip is drawn and what of its source it shows: as it is, or as
  // the trim being dragged would leave it
  viewOf = (obj: PlaylistObject): ClipView | null => {
    const { id } = obj.playlistItem
    // while its edge is dragged (and until that's settled), a clip is
    // measured as it was when the edge was pressed
    const clip = this.#trimClips?.get(id) ?? this.trimClipOf(obj)
    if(!clip) return null

    const span = this.#trimSpans?.get(id) ?? clip
    // a clip being stretched shows what it showed, squeezed or spread out
    const shows = this.#stretchRates?.has(id) ? clip : span
    const { perBeat } = clip
    return {
      startBeat: span.start,
      seconds: (span.end - span.start) * this.#beatSeconds(),
      from: (shows.start - clip.sourceStart) * perBeat,
      to: (shows.end - clip.sourceStart) * perBeat,
      length: clip.sourceLength,
    }
  }

  // show each trimmed clip's part of its sample or pattern
  // (called on every store update, so only a change wakes the playlist)
  setClipWindows = (windows: Record<number, ClipWindow>) => {
    if(windows === this.clipWindows) return
    this.clipWindows = windows
    // the engine's answer to a trim: unless a clip's start moved too (the
    // clips are built again for that), it's on screen now
    if(!this.#settleOnRebuild) this.#settleTrim?.()
    // a clip's length changes where the loop ends
    this.syncTransport()
    this.wake()
  }

  // the edge of a clip under the mouse, where a press grabs the edge rather
  // than the clip
  edgeUnderMouse = (): { item: PlaylistObject, edge: Edge } | null => {
    const p = this.#p
    const item = this.clipUnderMouse()
    if(!p || !item || !this.trimClipOf(item)) return null

    const { left, width } = item.boundingBox()
    const edge = edgeAt(
      (this.screenToPlaylistX(p.mouseX) - left) * this.currentScale,
      width * this.currentScale,
    )
    return edge ? { item, edge } : null
  }

  // the edge of a clip that has a bar drawn on it: the one a press would
  // grab, or the ones being dragged
  edgeBarOn = (id: number): Edge | null => {
    const drag = this.edgeDrag
    if(drag) return (drag.stretch ?? drag.clips).some(clip => clip.id === id) ? drag.edge : null
    return this.#hoverEdge?.id === id ? this.#hoverEdge.edge : null
  }

  // an edge was pressed. a selected clip's edge brings the same edge of the
  // rest of the selection with it
  // - stretch: shift was held, so the drag stretches the sample clips
  //   rather than trimming
  #startEdgeDrag = (p: P5CanvasInstance<CanvasProps>, item: PlaylistObject, edge: Edge, stretch = false) => {
    // an earlier trim still waiting for the engine: it's done with
    this.#settleTrim?.()

    const others = this.selectedIds.has(item.playlistItem.id)
      ? this.playlistObjects.filter(clip => clip !== item && this.selectedIds.has(clip.playlistItem.id))
      : []
    const clips = [item, ...others]
      .map(this.trimClipOf)
      .filter((clip): clip is TrimClip => clip !== null)
    const grabbed = clips.find(clip => clip.id === item.playlistItem.id)
    if(!grabbed) return

    // pattern clips are left as they are by a stretch (see stretchSet)
    const isPattern = (id: number) =>
      this.playlistObjects.find(o => o.playlistItem.id === id)?.playlistItem.patternId !== undefined
    const stretching = !stretch ? null : stretchSet(clips.map(clip => ({
      id: clip.id,
      start: clip.start,
      end: clip.end,
      rate: this.clipOverrides[clip.id]?.rate ?? 1,
      minLength: clip.minLength,
      pattern: isPattern(clip.id),
    })), grabbed.id)

    const edgeBeat = edge === 'left' ? grabbed.start : grabbed.end
    this.edgeDrag = {
      edge,
      grabbedId: grabbed.id,
      stretch: stretching,
      clips,
      grabBeats: edgeBeat - this.screenXToBeat(p.mouseX),
      pressX: p.mouseX,
      pressY: p.mouseY,
      active: false,
    }
    this.syncSnapSetting()
  }

  // move the dragged edge to the mouse: onto the grid when snap is on, or
  // the playhead when it's nearer and not moving, unless Option is held.
  // run every frame, as the mouse may be outside the canvas
  #followEdgeDrag = (p: P5CanvasInstance<CanvasProps>) => {
    const drag = this.edgeDrag
    if(!drag) return
    if(!drag.active) {
      if(p.dist(drag.pressX, drag.pressY, p.mouseX, p.mouseY) <= staticDefaults.mouseDragDetectionThreshold) return
      drag.active = true
    }

    const target = snapPoint(this.screenXToBeat(p.mouseX) + drag.grabBeats, this.#snapping())
    this.#trimClips = new Map(drag.clips.map(clip => [clip.id, clip]))
    if(drag.stretch) {
      const spans = stretchAll(drag.stretch, drag.grabbedId, drag.edge, target)
      this.#trimSpans = new Map(spans.map(({ id, start, end }) => [id, { start, end }]))
      this.#stretchRates = new Map(spans.map(({ id, rate }) => [id, rate]))
      return
    }

    this.#trimSpans = new Map(
      trimAll(drag.clips, drag.grabbedId, drag.edge, target).map(({ id, ...span }) => [id, span]),
    )
  }

  // the clips are drawn as they are again
  #clearTrimPreview = () => {
    this.#trimSpans = null
    this.#trimClips = null
    this.#stretchRates = null
  }

  // the mouse came up: the clips are trimmed to where their edges were
  // dragged, in one go
  #endEdgeDrag = () => {
    const drag = this.edgeDrag
    if(!drag) return
    // (once more, to where the mouse was let go: a quick drag can end
    // before a frame has been drawn)
    if(this.#p) this.#followEdgeDrag(this.#p)
    this.edgeDrag = null

    // shift+click on an edge, without dragging, is shift+click on the clip
    if(drag.stretch && !drag.active) {
      if(this.playlistObjects.some(o => o.playlistItem.id === drag.grabbedId)) {
        this.selectedIds = toggleSelected(this.selectedIds, drag.grabbedId)
      }
      this.#clearTrimPreview()
      this.wake()
      return
    }

    const spans = this.#trimSpans
    const rates = this.#stretchRates
    // an edge that didn't move leaves its clip alone
    const edits: WindowEdit[] = drag.stretch || !drag.active || !spans ? [] : drag.clips.flatMap(clip => {
      const span = spans.get(clip.id)
      if(!span || (span.start === clip.start && span.end === clip.end)) return []
      return [{ id: clip.id, ...trimmedWindow(clip, span, drag.edge, this.clipWindows[clip.id]) }]
    })
    const stretches: StretchEdit[] = !drag.stretch || !rates ? [] : drag.stretch.flatMap(clip => {
      const rate = rates.get(clip.id)
      if(rate === undefined || Math.abs(rate - clip.rate) < 1e-9) return []
      const pitch = clamp(pitchOf(rate), -MAX_PITCH, MAX_PITCH)
      return [{ id: clip.id, pitch, keep: drag.edge === 'left' ? 'end' as const : 'start' as const }]
    })
    if(!edits.length && !stretches.length) {
      this.#clearTrimPreview()
      this.wake()
      return
    }

    // the clips stay as they were dragged until the engine's answer is on
    // screen (see setClipWindows and updateWithProps), or it refuses
    const settle = () => {
      if(this.#settleTrim !== settle) return
      this.#settleTrim = null
      if(this.#trimSpans === spans) this.#clearTrimPreview()
      this.wake()
    }
    this.#settleTrim = settle
    // a left edge moves its clip's start, which the clips are built again for
    this.#settleOnRebuild = drag.edge === 'left'
    const sent = stretches.length ? this.onClipsStretch(stretches) : this.onClipsTrim(edits)
    sent.then(changed => { if(!changed) settle() }, settle)
    // (in case its answer changes nothing that's watched for)
    setTimeout(settle, 1_000)
  }

  // Escape, or the window losing the mouse: the clips go back as they were
  #cancelEdgeDrag = () => {
    if(!this.edgeDrag) return
    this.edgeDrag = null
    this.#clearTrimPreview()
    this.wake()
  }

  // stretch the box to the mouse, and select what it touches. run every
  // frame, so the selection follows when the tracks scroll under the box
  #followLasso = (p: P5CanvasInstance<CanvasProps>) => {
    if(!this.lasso) return
    this.lasso.end = this.#mousePoint(p)
    this.selectedIds = lassoSelection(this.lasso, this.#clipBoxes())
  }

  // the selection box: a dashed sky outline over a faint fill, drawn in
  // canvas pixels so zoom doesn't stretch the dashes
  #drawLasso = (p: P5CanvasInstance<CanvasProps>) => {
    if(!this.lasso) return
    const { left, top, width, height } = lassoRect(this.lasso)
    const x = this.transformX + left * this.currentScale
    const y = top - this.scrollY + 1

    drawLasso(p.drawingContext as CanvasRenderingContext2D, x, y, width * this.currentScale, height)
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
      || this.#shiftPress !== null
      || this.edgeDrag !== null
      || this.#paint !== null
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
    // and under a lane's lip
    if(p && (p.mouseY < staticDefaults.timelineHeight || this.overScrollbar(p.mouseX, p.mouseY)
      || this.lipAt(p.mouseX, p.mouseY) !== null)) {
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
    const clamped = clamp(beat, 0, this.beatsDisplayed)
    const { enabled, beats } = this.snap

    return enabled && beats > 0 ? Math.round(clamped / beats) * beats : clamped
  }

  // where a dragged clip would land, given the mouse position on the canvas
  // and how far from the clip's left edge it's held (in playlist x)
  dropTarget = (mouseX: number, mouseY: number, grabX = 0): DropTarget => {
    const left = this.screenToPlaylistX(mouseX) - grabX
    const beat = clamp(this.xToBeat(left), 0, this.beatsDisplayed)

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
    // the engine's answer to a stretch (see #endEdgeDrag)
    if(!this.#settleOnRebuild) this.#settleTrim?.()
    this.wake()

    // a clip's length changes where the loop ends. asked once the settings
    // have reached the engine (they're sent a little behind, see sendSettings)
    if(this.#overrideSync) clearTimeout(this.#overrideSync)
    this.#overrideSync = setTimeout(() => {
      this.#overrideSync = null
      this.syncTransport()
    }, 150)
  }

  #overrideSync: ReturnType<typeof setTimeout> | null = null

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
    paint.stroke(p, 'accent')
    paint.fill(p, 'accentSoft', 0.25)
    // in screen pixels across, so zoom doesn't stretch the outline or its dashes
    p.scale(1 / this.currentScale, 1)
    this.landings(drag).forEach(({ item, beat, track }) => {
      const laneTop = this.#laneTop(track)
      if(laneTop === null) return
      const { width, height } = item.boundingBox()
      p.rect(this.beatToX(beat) * this.currentScale, laneTop + 2, width * this.currentScale, height - 4, 3)
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
    this.scrollY = clamp(y, 0, this.maxScrollY()) || 0
  }

  // the vertical scrollbar, in canvas pixels. null when every track fits
  scrollbar = () => {
    const max = this.maxScrollY()
    if(max <= 0) return null

    const top = staticDefaults.timelineHeight
    const trackLength = this.height - top
    // the share of the tracks that's visible
    const visible = trackLength / (this.contentHeight() - top)
    const thumbHeight = clamp(trackLength * visible, SCROLLBAR.minThumbHeight, trackLength)
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
    const depth = (into: number) => clamp(into / zone, 0, 1)

    if(y < top + zone) this.scrollTo(this.scrollY - maxSpeed * depth(top + zone - y))
    else if(y > this.height - zone) this.scrollTo(this.scrollY + maxSpeed * depth(y - (this.height - zone)))
  }

  // the plus in the area below the last track, in canvas pixels (matching
  // the tracks' translate in draw). centerd in what's visible, so it doesn't
  // stretch with zoom or scroll away
  #newTrackPlus = () => {
    const top = this.newTrackAreaTop() - this.scrollY + 1
    return { top, cx: this.width / 2, cy: top + NEW_TRACK_AREA_HEIGHT / 2, radius: 10 }
  }

  // whether a point on the canvas is over the plus (which adds a track when
  // clicked). a little past its circle, as it's small
  overNewTrackPlus = (x: number, y: number): boolean => {
    const { cx, cy, radius } = this.#newTrackPlus()
    return y >= staticDefaults.timelineHeight && Math.hypot(x - cx, y - cy) <= radius + 4
  }

  // the area below the last track, with a plus: drop a clip here for a new
  // track, or click the plus for an empty one. highlighted while a dragged
  // clip would land there, and the plus while the mouse is over it
  #drawNewTrackArea = (p: P5CanvasInstance<CanvasProps>, canvas: p5.Renderer) => {
    const { top, cx, cy, radius } = this.#newTrackPlus()
    if(top > this.height) return

    const drag = this.draggedClip(p, canvas)
    const targeted = (!!drag && this.landings(drag).some(landing => landing.track === this.trackCount))
      || this.#paint?.track === this.trackCount
    const hovered = !targeted && !drag && !this.isMouseDown && this.overNewTrackPlus(p.mouseX, p.mouseY)

    const h = NEW_TRACK_AREA_HEIGHT
    const arm = 5

    p.push()
    if(targeted) {
      p.noStroke()
      paint.fill(p, 'accentSoft', 64 / 255)
      p.rect(0, top, this.width, h)
      paint.stroke(p, 'accent')
    } else {
      paint.stroke(p, hovered ? 'accent' : 'idle')
    }

    p.strokeWeight(1.5)
    p.noFill()
    p.circle(cx, cy, radius * 2)
    p.line(cx - arm, cy, cx + arm, cy)
    p.line(cx, cy - arm, cx, cy + arm)

    if(targeted) {
      p.noStroke()
      paint.fill(p, 'accent')
      p.textFont(theme.font)
      p.textSize(11)
      p.textAlign(p.LEFT, p.CENTER)
      p.text('New track', cx + radius + 6, cy)
    }
    p.pop()
  }

  // each lane's lip, pinned to the right edge (left of the scrollbar when
  // that's shown), for lanes at least partly below the ruler. in canvas
  // pixels, matching the tracks' translate in draw
  #lipBoxes = () => {
    const ctx = this.#p?.drawingContext as CanvasRenderingContext2D | undefined
    if(!ctx) return []
    const right = this.width - (this.scrollbar() ? SCROLLBAR.hitWidth : 0)
    const lips = []
    for(let n = 0; n < this.trackCount; n++) {
      const label = this.#lipLabel(ctx, this.trackNames[n] ?? trackLabel(n))
      const box = lipBox(
        trackTop(n) - this.scrollY + 1, staticDefaults.trackHeight, right, label.width, staticDefaults.timelineHeight,
      )
      if(box.y > this.height || box.y + box.h < staticDefaults.timelineHeight) continue
      lips.push({ track: n, box, text: label.text })
    }
    return lips
  }

  #lipLabel = (ctx: CanvasRenderingContext2D, name: string) => {
    let label = this.#lipLabels.get(name)
    if(!label) {
      ctx.save()
      ctx.font = LIP.font
      ctx.letterSpacing = LIP.letterSpacing
      const text = fitName(ctx, name.toUpperCase())
      label = { text, width: lipWidth(ctx.measureText(text).width) }
      ctx.restore()
      this.#lipLabels.set(name, label)
    }
    return label
  }

  // the track whose lip is at a point on the canvas, if any. the ruler
  // covers the lips that scroll up under it
  lipAt = (x: number, y: number): number | null => {
    if(y < staticDefaults.timelineHeight) return null
    return this.#lipBoxes().find(lip => inBox(lip.box, x, y))?.track ?? null
  }

  // the track whose lip's LED is at a point on the canvas, if any
  ledAt = (x: number, y: number): number | null => {
    if(y < staticDefaults.timelineHeight) return null
    return this.#lipBoxes().find(lip => onLed(lip.box, x, y))?.track ?? null
  }

  // a track's color (see TRACK_COLORS), as [r, g, b, a]
  trackRgba = (track: number) =>
    trackPaint.rgba(trackColorKey(this.trackColors[track] ?? defaultTrackColor(track)))

  // a pattern clip's colors on a track, in the track's color
  patternRgba = (track: number) =>
    patternColors(this.trackRgba(track), paint.rgba('clip'), paint.rgba('clipText'))

  // a track's state in the mixer, as its lip shows it
  #ring = (track: number): LipLook['ring'] => {
    const mix = this.trackMixes[track]
    if(mix?.mute) return 'mute'
    if(mix?.solo) return 'solo'
    const anySolo = Object.values(this.trackMixes).some(m => m.solo)
    return anySolo ? 'silenced' : null
  }

  // a lip's LED toggles its track's mute (a click) or solo (a shift+click)
  #toggleMute = (track: number) => this.onTrackMute?.(track, !this.trackMixes[track]?.mute)
  #toggleSolo = (track: number) => this.onTrackSolo?.(track, !this.trackMixes[track]?.solo)

  // the tracks the playhead is playing a clip on, unless the mixer silences
  // them. lights their lips' LEDs
  #soundingTracks = (): Set<number> => {
    const sounding = new Set<number>()
    const beat = this.playing ? this.playheadBeat() : null
    if(beat === null) return sounding

    const x = this.beatToX(beat)
    this.playlistObjects.forEach(clip => {
      const track = clip.playlistItem.trackNumber
      const ring = this.#ring(track)
      if(ring === 'mute' || ring === 'silenced') return
      const { left, right } = clip.boundingBox()
      if(x >= left && x < right) sounding.add(track)
    })
    return sounding
  }

  // over the clips, and under the ruler, which covers a lip as its lane
  // scrolls up under it
  #drawTrackLips = (p: P5CanvasInstance<CanvasProps>) => {
    const ctx = p.drawingContext as CanvasRenderingContext2D
    const sounding = this.#soundingTracks()
    const base = {
      ink: fourtrackPaint.rgba('inkSoft'),
      inkQuiet: fourtrackPaint.rgba('inkFaint'),
      well: fourtrackPaint.rgba('tray'),
      panel: fourtrackPaint.rgba('panel'),
      mute: fourtrackPaint.rgba('ledAmberOn'),
      solo: fourtrackPaint.rgba('ledLimeOn'),
    }
    // (lanes aren't shaded: the lines between them set them apart, so a
    // lip sits on the plain background)
    const lane = paint.rgba('background')

    this.#lipBoxes().forEach(({ track, box, text }) => {
      const led = this.trackRgba(track)
      drawLip(ctx, box, text, { ...base, lane, led }, { lit: sounding.has(track), ring: this.#ring(track) })
    })
  }

  #drawScrollbar = (p: P5CanvasInstance<CanvasProps>) => {
    const bar = this.scrollbar()
    if(!bar) return

    const active = this.scrollbarGrab !== null || this.overScrollbar(p.mouseX, p.mouseY)
    const { width, margin } = SCROLLBAR

    p.push()
    p.noStroke()
    paint.fill(p, 'scrollbar', (active ? 140 : 80) / 255)
    p.rect(this.width - width - margin, bar.thumbTop + 2, width, bar.thumbHeight - 4, width / 2)
    p.pop()
  }

  // zoom the timeline by `factor`, keeping the point at canvas x `aroundX`
  // in place. never zooms out past the whole playlist, or in past a 1/16
  // note grid (see maxScale)
  #zoomBy = (factor: number, aroundX: number) => {
    const most = maxScale(this.width, this.beatsDisplayed)
    const zoom = zoomAround({ scale: this.currentScale, pan: this.transformX }, factor, aroundX, 1, most)
    this.currentScale = zoom.scale
    this.transformX = zoom.pan

    this.playlistObjects.forEach(obj => {
      obj.currentScale = this.currentScale
    })
  }

  // pull the zoom back within maxScale, keeping the beat at the left edge.
  // the limit depends on the canvas width and the playlist's length, so a
  // resize or a new project can leave the zoom past it without any zooming
  #clampZoom = () => {
    const most = maxScale(this.width, this.beatsDisplayed)
    if(!(this.currentScale > most)) return
    this.transformX *= most / this.currentScale
    this.currentScale = most
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

  // whether something on the playlist is being dragged, or pressed: keys
  // that change the clips wait until it's let go
  isDragging = (): boolean =>
    !!this.edgeDrag || !!this.clipDrag?.active || !!this.lasso || this.#shiftPress !== null || this.#paint !== null

  #handleRightClick = (ev: MouseEvent, p: P5CanvasInstance<CanvasProps>) => {
    // not in the middle of dragging an edge, or painting
    if(this.edgeDrag || this.#paint) return

    // the clip on top, as for clicks (not one hidden under the timeline)
    const item = this.clipUnderMouse()
    if(item) {
      const { mouseX, mouseY } = p
      item.onRightClick(ev, {
        mouseX,
        mouseY,
        target: item,
      })
      return
    }

    if(overRuler(p.mouseY) || this.overScrollbar(p.mouseX, p.mouseY)) return
    // a lip has its own menu
    const lip = this.lipAt(p.mouseX, p.mouseY)
    if(lip !== null) {
      this.onLipRightClick?.(lip, ev.clientX, ev.clientY)
      return
    }
    const track = this.trackAt(this.screenToPlaylistY(p.mouseY))
    if(track < 0) return
    this.onEmptyRightClick(this.snapBeat(this.screenXToBeat(p.mouseX)), track, ev.clientX, ev.clientY)
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
      // a press whose release went to another app is dropped
      window.addEventListener('blur', this.#dropPress)
      window.addEventListener('keydown', this.#followAlt)
      window.addEventListener('keyup', this.#followAlt)
      // labels drawn before the font loaded are drawn again in it
      document.fonts?.ready.then(this.wake)
      // colours are read as they're drawn, so a theme change only needs a
      // redraw. stopped when the instance is removed. a closure of its own, so
      // removing an old instance can't unsubscribe a newer one
      const offTheme = onThemeChange(() => this.wake())
      const remove = p.remove.bind(p)
      p.remove = () => {
        offTheme()
        remove()
      }
      this.wake()

      // handle mouse press, and pick up a clip if one was pressed on
      canvas.mousePressed((ev: MouseEvent) => {
        this.#altHeld = ev.altKey
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

          // a lane's lip is its own control: the press doesn't reach the
          // clips or the lane under it. its LED mutes the track, or with
          // shift held, solos it
          // (from the event: p5 hasn't caught its own mouse position up
          // with this press yet)
          const led = this.ledAt(ev.offsetX, ev.offsetY)
          if(led !== null) {
            if(ev.shiftKey) this.#toggleSolo(led)
            else this.#toggleMute(led)
            return
          }
          if(this.lipAt(ev.offsetX, ev.offsetY) !== null) return

          // the plus below the last track adds an empty track, whatever the tool
          if(this.overNewTrackPlus(ev.offsetX, ev.offsetY)) {
            void this.onAddTrack?.()
            return
          }

          const item = this.clipUnderMouse()

          // shift+click a clip adds it to the selection, or takes it out.
          // shift+drag draws a box to select clips with, wherever it starts
          // (which it is isn't known until the mouse moves or lets go)
          if(ev.shiftKey) {
            // on a clip's edge, a drag stretches the clip (and a click is
            // still a click on the clip). not with the Cut tool, which
            // leaves edges alone (Draw is Select on a clip)
            const edge = item && this.tool !== 'cut' ? this.edgeUnderMouse() : null
            if(edge) {
              this.#startEdgeDrag(p, edge.item, edge.edge, true)
              return
            }

            // from the event: p5 hasn't caught its own mouse position up
            // with this press yet
            const point = { x: this.screenToPlaylistX(ev.offsetX), y: this.screenToPlaylistY(ev.offsetY) }
            this.#shiftPress = {
              screen: { x: ev.offsetX, y: ev.offsetY },
              point,
              itemId: itemAt(point, this.#clipBoxes()),
            }
            return
          }

          // the Cut tool: a click on a clip cuts it, selected or not. clips
          // aren't moved or trimmed (empty space still pans, below)
          if(this.tool === 'cut' && item) {
            this.#cutPress = { x: p.mouseX, y: p.mouseY, id: item.playlistItem.id }
            this.syncSnapSetting()
            return
          }

          // the Draw tool on empty lane space (the new-track area too) paints
          // clips along the lane, rather than panning. on a clip, it's Select
          if(this.tool === 'draw' && !item) {
            // (from the event: p5 hasn't caught its own mouse position up
            // with this press yet)
            this.#startPaint(ev.offsetX, ev.offsetY)
            return
          }

          // just inside either end of a clip, the press grabs that edge, to
          // trim the clip. (as for picking a clip up: one that isn't
          // selected leaves the selection)
          const grabbed = item ? this.edgeUnderMouse() : null
          if(grabbed) {
            if(!this.selectedIds.has(grabbed.item.playlistItem.id)) this.clearSelection()
            this.#startEdgeDrag(p, grabbed.item, grabbed.edge)
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
      p.mouseReleased = (ev?: MouseEvent) => {
        // another button let go while an edge is dragged (or a paint) isn't
        // the end of it
        if((this.edgeDrag || this.#paint) && ev && ev.button !== 0) return
        this.#endSeek()
        // (painted wherever the mouse is let go: its end stays in the playlist)
        this.#endPaint(p)
        // (a trim is applied wherever the mouse is let go)
        this.#endEdgeDrag()
        // (a cut needs the mouse still on its clip: over the canvas, where
        // canvas.mouseReleased has dealt with it)
        this.#cutPress = null
        this.#endShiftPress(p)
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

        const wheel = wheelIntent(ev, this.height)
        if(wheel.kind === 'zoom') {
          this.#zoomBy(wheel.factor, p.mouseX)
          return
        }

        // kept in range when drawn
        this.transformX -= wheel.dx
        this.scrollTo(this.scrollY + wheel.dy)
      })

      // drop a dragged clip where the phantom shows
      canvas.mouseReleased((ev?: MouseEvent) => {
        if((this.edgeDrag || this.#paint) && ev && ev.button !== 0) return
        this.#endEdgeDrag()
        this.#endPaint(p)
        if(!ev || ev.button === 0) this.#endCutPress(p)
        const drag = this.clipDrag
        const target = this.dropTarget(p.mouseX, p.mouseY, drag?.grabX)

        if(drag?.active && target.trackNumber >= 0) {
          // send the unsnapped beat, the engine owns snapping. the clips are
          // drawn where their phantoms were until it answers
          if(drag.group.length) {
            const ids = [drag.item, ...drag.group].map(clip => clip.playlistItem.id)
            const shown = this.landings({ ...drag, mouseX: p.mouseX, mouseY: p.mouseY })
              .map(({ item, beat, track }) => ({ id: item.playlistItem.id, startBeat: beat, trackNumber: track }))
            this.onNodesMove(ids, drag.item.playlistItem.id, target.beat, target.trackNumber, shown)
          } else {
            this.onNodeMove(drag.item.playlistItem, target.beat, target.trackNumber, target.snappedBeat)
          }
        }

        // a click on empty space (not a pan) clears the selection
        if(this.#pressedOnEmpty && !this.isMouseDragged) this.clearSelection()
        this.#pressedOnEmpty = false
        this.#endShiftPress(p)
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
        // (as well as the keys: Option may have gone down in another app)
        this.#altHeld = ev.altKey
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
        // an edge being dragged: the clips go back, and that's all
        if(this.edgeDrag) {
          this.#cancelEdgeDrag()
          return
        }
        // a paint: nothing's painted, and that's all
        if(this.#paint) {
          this.#paint = null
          this.wake()
          return
        }
        this.#shiftPress = null
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
        // (with the Cut tool, two clicks are two cuts, not opening the clip)
        if(this.tool === 'cut') return
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
      if(props.width > 0) this.setWidth(props.width)
      const tracksChanged = props.trackCount !== this.trackCount
      this.trackCount = props.trackCount
      this.trackColors = props.trackColors ?? {}
      this.trackMixes = props.trackMixes ?? {}
      this.onTrackMute = props.onTrackMute
      this.onTrackSolo = props.onTrackSolo
      this.onLipRightClick = props.onLipRightClick
      this.trackNames = props.trackNames ?? {}
      // clips' lengths in beats change with the tempo, so where the loop ends
      if(props.tempo !== this.tempo) this.syncTransport()
      this.tempo = props.tempo
      if(props.beatsDisplayed > 0 && props.beatsDisplayed !== this.beatsDisplayed) {
        this.beatsDisplayed = props.beatsDisplayed
        this.resetView()
      }
      const { width, height, trackCount, currentScale } = this

      this.onNodeRightClick = props.onNodeRightClick
      this.onEmptyRightClick = props.onEmptyRightClick
      this.onNodeMove = props.onNodeMove
      this.onNodesMove = props.onNodesMove
      this.onNodeDoubleClick = props.onNodeDoubleClick
      this.onSeek = props.onSeek
      this.onClipsTrim = props.onClipsTrim
      this.onClipsStretch = props.onClipsStretch
      this.onClipsCut = props.onClipsCut
      this.onPaint = props.onPaint
      this.onAddTrack = props.onAddTrack
      if(props.onGridStep !== this.onGridStep) {
        this.onGridStep = props.onGridStep
        // (so a new listener hears the step there is now)
        this.#gridStep = 0
      }

      // ask the engine where the playhead is when playback starts or stops,
      // or after any other transport command. other prop updates (drops,
      // moves, resizes) don't move it
      if(
        props.playing !== this.playing
        || props.transportRevision !== this.transportRevision
        || props.loopEnabled !== this.loopEnabled
      ) {
        this.transportRevision = props.transportRevision
        this.loopEnabled = props.loopEnabled
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
      // the clips changed, so where the loop ends may have too
      if(this.#builtItems !== null && props.playlistObjects !== this.#builtItems) this.syncTransport()
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
        // as its window says (see viewOf), or its pattern until it has one
        if(item.patternId !== undefined) {
          return { wf: new Float32Array(0), dur: this.patternSecondsOf(item.patternId) }
        }
        // (the same clip can come to play another file: a deep copy, a
        // join, or undoing one. its waveform is asked for again then)
        const known = current.get(item.id)
        if(known && known.playlistItem.path === item.path && known.playlistItem.patternId === undefined) {
          return { wf: known.soundData, dur: known.duration }
        }

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
        // the clips are as the engine has them: a trim that was waiting
        // for that is on screen
        this.#settleTrim?.()
        this.wake()
      })
    }

    // render p5 canvas
    p.draw = () => {
      if(!canvas) return

      // nothing's changed for a while: this is the last frame until
      // something does (see wake)
      if(!this.#animating() && performance.now() - this.#lastWake > IDLE_MS) p.noLoop()

      this.#clampZoom()
      const { width, trackCount, currentScale, beatsDisplayed, beatsPerBar } = this

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
      
      paint.background(p, 'background')

      this.transformX = p.constrain(this.transformX, width * (1 - currentScale), 0)
      // NaN never recovers through constrain, and breaks every clip's
      // hit-test (see screenToPlaylistX), so start over from no pan
      if(!Number.isFinite(this.transformX)) this.transformX = 0

      this.#followScrollbarDrag(p)
      this.#autoScroll(p)
      // kept in range: the canvas or the track count may have changed size
      this.scrollTo(this.scrollY)
      this.#followShiftPress(p)
      this.#followLasso(p)
      this.#followEdgeDrag(p)
      this.#followPaint(p)
      // the edge a press would grab, which its clip marks with a bar (not
      // while something else is being dragged or the mouse is held)
      const idle = !this.edgeDrag && !this.clipDrag && !this.sampleDrag && !this.lasso && !this.#paint
        && !this.isMouseDown && !this.seekDragging && this.scrollbarGrab === null
      const overTracks = idle && !overRuler(p.mouseY) && !this.overScrollbar(p.mouseX, p.mouseY)
        && this.lipAt(p.mouseX, p.mouseY) === null
      // (Draw is Select on a clip)
      const hover = overTracks && this.tool !== 'cut' ? this.edgeUnderMouse() : null
      this.#hoverEdge = hover ? { id: hover.item.playlistItem.id, edge: hover.edge } : null
      // and with the Cut tool, where a click would cut
      // (not with shift held, when a click selects rather than cuts)
      this.#cutHover = overTracks && this.tool === 'cut' && !p.keyIsDown(p.SHIFT) ? this.#cutAt(p) : null

      // the tracks, scrolled
      p.push()
      p.translate(this.transformX, 1 - this.scrollY)
      p.scale(currentScale, 1)

      // the grid follows the zoom (see adaptiveStep): its lines stay about
      // GRID_GAP pixels apart, splitting beats into finer notes zoomed in and
      // thinning to every few bars zoomed out. only what's on screen is drawn
      const gridBottom = trackTop(trackCount)
      const pixelsPerBeat = width * currentScale / beatsDisplayed
      const { lineEvery } = gridSteps(pixelsPerBeat, beatsPerBar)
      this.#reportGridStep(lineEvery)
      const [firstBeat, lastBeat] = visibleBeats(pixelsPerBeat, this.transformX, width, beatsDisplayed)
      const beatX = (beat: number) => beat * width / beatsDisplayed

      // every other band of a few lines is shaded, so where you are along
      // time reads at a glance whatever the zoom
      const tile = tileStep(lineEvery, beatsPerBar)
      p.noStroke()
      paint.fill(p, 'gridTile')
      for(let n = Math.floor(firstBeat / tile); n * tile < lastBeat; n++) {
        if(n % 2 === 1) p.rect(beatX(n * tile), 0, beatX(tile), gridBottom)
      }

      // the lines, 1px wide whatever the zoom: strongest on bars, then
      // beats, then (faint) the notes between. counted in steps, so a fine
      // step doesn't drift
      p.strokeWeight(1 / currentScale)
      for(let n = Math.ceil(firstBeat / lineEvery); n * lineEvery < lastBeat; n++) {
        const beat = n * lineEvery
        const level = lineLevel(beat, beatsPerBar)
        if(level === 2) paint.stroke(p, 'barLine')
        else paint.stroke(p, 'beatLine', level === 1 ? 1 : 0.55)
        const x = beatX(beat)
        p.line(x, 0, x, gridBottom)
      }

      // render playlist tracks
      this.playlistTracks.forEach(track => {
        track.render()
      })

      // render the clip being dragged on top of the other clips
      this.drawClipDrag(p, canvas)
      this.#drawCutLine(p)
      this.#drawPaint(p)
      
      p.pop()

      this.#drawTrackLips(p)
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

      if(this.busy) {
        p.cursor('wait')
      } else if(this.edgeDrag) {
        p.cursor('ew-resize')
      } else if(this.lasso || this.#paint) {
        p.cursor(p.CROSS)
      } else if(this.scrollbarGrab !== null || this.overScrollbar(p.mouseX, p.mouseY)) {
        p.cursor(p.ARROW)
      } else if(!this.clipDrag?.active && !this.sampleDrag && this.lipAt(p.mouseX, p.mouseY) !== null) {
        // the LED can be clicked
        p.cursor(this.ledAt(p.mouseX, p.mouseY) === null ? p.ARROW : p.HAND)
      } else if(this.seekDragging || (overRuler(p.mouseY) && !this.clipDrag?.active && !this.sampleDrag)) {
        p.cursor(p.HAND)
      } else if(this.#hoverEdge) {
        p.cursor('ew-resize')
      } else if(!this.clipDrag?.active && !this.sampleDrag && this.overNewTrackPlus(p.mouseX, p.mouseY)) {
        p.cursor(p.HAND)
      } else if(this.clipDrag?.active || this.sampleDrag) {
        p.cursor('grabbing')
      } else if(this.tool === 'cut') {
        p.cursor(RAZOR_CURSOR)
      } else if(this.clipUnderMouse()) {
        p.cursor(p.HAND)
      } else if(this.tool === 'draw') {
        // (empty lane space, where a press paints)
        p.cursor(p.CROSS)
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
        paint.fill(p, 'debug', 0.6)
        p.rect(
          p.width - debugWinWidth, 
          debugWinTop,
          debugWinWidth - 10, 
          debugWinHeight - 10
        )

        paint.stroke(p, 'debugText')
        paint.fill(p, 'debugText')
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
