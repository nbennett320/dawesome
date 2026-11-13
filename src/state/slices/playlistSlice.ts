import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import { RootState } from 'state/store'
import { PlaylistItem, PlaylistItemPixelOffset } from '../../types/playlist'
import { Tool } from '../../editing/tools'
import { SnapNote } from '../../editing/snap'
import { View } from '../../types/ui'
import {
  ClipInfo,
  ClipOverride,
  ClipSettings,
  ClipWindow,
  clampClipSettings,
  defaultClipSettings,
  isDefaultClipSettings,
  playbackRate,
  windowedDuration,
} from '../../types/clip'
import {
  ChangedClips, LoadedClip, LoadedPatternClip, LoadedProject, Joined, MissingEffect, MissingSample, MovedClip, Placed, ProjectView, SavedProject, splitMissing,
} from '../../types/project'
import { currentGesture } from '../gesture'
import { DEFAULT_PLAYLIST_BEATS, Wind } from '../../types/transport'
import { SequencerState, SynthInstrument } from '../../types/sequencer'
import { MixerSettings } from '../../types/mixer'
import { openTab } from './windowSlice'

// mirrors daw::timing::Note in the engine
export type { SnapNote }

// mirrors SnapSetting in engine/src/event_dispatch.rs
interface SnapSetting {
  enabled: boolean
  note: SnapNote
  beats: number
}

export interface PlaylistState {
  playing: boolean
  // counts transport commands (play, pause, stop, seek), so views that follow
  // the playhead know to ask the engine where it is again
  transportRevision: number
  // the Transport tab's rewind or fast forward being held, so every counter
  // follows the tape while it winds
  wind: Wind | null
  tempo: number
  metronomeEnabled: boolean,
  loopEnabled: boolean,
  snapEnabled: boolean,
  snapNote: SnapNote,
  // the playlist's mouse tool. not saved with the project
  tool: Tool
  // the playlist grid's step at its zoom, in beats (see adaptiveStep). not
  // saved with the project
  gridStep: number
  playlistItems: Array<PlaylistItem>
  // counts projects opened (or started), so the playlist goes back to how
  // it opens: its first bars
  viewRevision: number
  // the clips an undo or redo changed, or a paste made, for the playlist to
  // select. the count goes up each time, so the same clips can be selected again
  travelled: { count: number, ids: number[] }
  // how many clips the engine's clipboard holds: whether there's anything to paste
  clipboardCount: number
  // the clip shown in the Sample tab
  inspectedClipId: number | null
  // how each clip is played back, by clip id, as last loaded from or sent to
  // the engine. clips without an entry use defaultClipSettings
  clipSettings: Record<number, ClipSettings>
  // each clip's sample length in seconds, before any speed change, by clip id
  clipSampleDurations: Record<number, number>
  // the part of its sample or pattern each trimmed clip plays, by clip id.
  // clips without an entry play all of it
  clipWindows: Record<number, ClipWindow>
  ui: {
    // the playlist's length in beats, from the engine (see PLAYLIST_BARS)
    maxBeatsDisplayed: number,
    trackCount: number,
  },
}

// todo: get initial state from backend,
// which parses dawesome.config file
const initialState = {
  playing: false,
  transportRevision: 0,
  wind: null,
  tempo: 120,
  metronomeEnabled: true,
  loopEnabled: true,
  snapEnabled: true,
  snapNote: 'QuarterNote',
  // not saved with the project
  tool: 'select',
  gridStep: 1,
  playlistItems: [],
  viewRevision: 0,
  travelled: { count: 0, ids: [] },
  clipboardCount: 0,
  inspectedClipId: null,
  clipSettings: {},
  clipSampleDurations: {},
  clipWindows: {},
  ui: {
    maxBeatsDisplayed: DEFAULT_PLAYLIST_BEATS,
    trackCount: 4,
  },
} as PlaylistState

// a clip that wasn't dropped with the mouse (loaded, or a pattern clip),
// so there's no drop position to debug
const NO_PIXEL_OFFSET: PlaylistItemPixelOffset = {
  x: 0, y: 0, left: 0, top: 0, right: 0, bottom: 0, xOffset: 0, yOffset: 0,
}

// the playlist's item for a clip the engine sent, built as a loaded project's are
const sampleItem = (clip: LoadedClip): PlaylistItem => ({
  id: clip.id,
  path: clip.path,
  startBeat: clip.startBeat,
  trackNumber: clip.trackNumber,
  pixelOffset: { ...NO_PIXEL_OFFSET },
})

const patternItem = (clip: LoadedPatternClip): PlaylistItem => ({
  id: clip.id,
  path: '',
  patternId: clip.patternId,
  startBeat: clip.startBeat,
  trackNumber: clip.trackNumber,
  pixelOffset: { ...NO_PIXEL_OFFSET },
})

// pattern clips painted with the Draw tool, as the engine's
// paint_pattern_clips returns them (Painted in
// engine/src/commands/sequencer.rs): the clips made, in order, the pattern
// they play (made by the paint when none was given), the sequencer, the
// mixer, and how many tracks there are now
export interface PaintedClips {
  clips: LoadedPatternClip[]
  patternId: number
  sequencer: SequencerState
  mixer: MixerSettings
  trackCount: number
}

// keep a clip's window as the engine has it: an entry when it's trimmed, none
// when it plays all of its source
const setWindow = (state: PlaylistState, id: number, window: ClipWindow | null | undefined) => {
  if(window) state.clipWindows[id] = window
  else delete state.clipWindows[id]
}

export const playlistSlice = createSlice({
  name: 'playlistSlice',
  initialState,
  reducers: {
    setWind: (state, action: PayloadAction<Wind | null>) => {
      state.wind = action.payload
    },
    bumpTransportRevision: (state) => {
      state.transportRevision += 1
    },
    setPlaying: (state, action) => {
      state.playing = action.payload
    },
    reduceTempo: (state, action) => {
      state.tempo = action.payload
    },
    setMetronomeEnabled: (state, action) => {
      state.metronomeEnabled = action.payload
    },
    setLoopEnabled: (state, action) => {
      state.loopEnabled = action.payload
    },
    addPlaylistItem: (state, action) => {
      const item = {
        id: action.payload.id,
        path: action.payload.path,
        startBeat: action.payload.startBeat,
        trackNumber: action.payload.trackNumber,
        pixelOffset: action.payload.pixelOffset,
      } as PlaylistItem
      if(action.payload.patternId !== undefined) item.patternId = action.payload.patternId
      state.playlistItems.push(item)
    },
    // clips whose samples moved (takes brought into the project's folder)
    reduceMovedClips: (state, action: PayloadAction<MovedClip[]>) => {
      action.payload.forEach(({ id, path }) => {
        const item = state.playlistItems.find(i => i.id === id)
        if(item) item.path = path
      })
    },
    movePlaylistItem: (state, action) => {
      const item = state.playlistItems.find(i => i.id === action.payload.id)
      if(!item) return

      // the rest (its sample, or its pattern) stays as it was
      item.startBeat = action.payload.startBeat
      item.trackNumber = action.payload.trackNumber
      item.pixelOffset = action.payload.pixelOffset
    },
    // clips moved together (a dragged selection), placed where the engine put them
    movePlaylistItems: (state, action: { type: string, payload: MovedNode[] }) => {
      const moved = new Map(action.payload.map(m => [m.id, m]))
      state.playlistItems.forEach(item => {
        const m = moved.get(item.id)
        if(!m) return
        item.startBeat = m.startBeat
        item.trackNumber = m.trackNumber
      })
    },
    // pattern clips replaced by a pattern clip per sample (see
    // explodePatternClips in sequencerSlice)
    reduceExploded: (state, action: PayloadAction<{ removed: number[], clips: LoadedPatternClip[] }>) => {
      const removed = new Set(action.payload.removed)
      state.playlistItems = state.playlistItems.filter(item => !removed.has(item.id))
      removed.forEach(id => { delete state.clipWindows[id] })
      // each plays the part of its pattern that the exploded clip played
      action.payload.clips.forEach(clip => {
        state.playlistItems.push(patternItem(clip))
        setWindow(state, clip.id, clip.window)
      })
    },
    // pattern clips painted with the Draw tool (see paintPatternClips). the
    // sequencer and the mixer take theirs from it too
    reducePainted: (state, action: PayloadAction<PaintedClips>) => {
      const { clips, trackCount } = action.payload
      clips.forEach(clip => {
        state.playlistItems.push(patternItem(clip))
        setWindow(state, clip.id, clip.window)
      })
      state.ui.trackCount = trackCount
    },
    setClipboardCount: (state, action: PayloadAction<number>) => {
      state.clipboardCount = action.payload
    },
    // the playlist selects these clips, as it does the ones an undo changed
    selectClips: (state, action: PayloadAction<number[]>) => {
      state.travelled = { count: state.travelled.count + 1, ids: action.payload }
    },
    removePlaylistItem: (state, action) => {
      state.playlistItems = state.playlistItems.filter(e => e.id !== action.payload.id)

      if(state.inspectedClipId === action.payload.id) {
        state.inspectedClipId = null
      }

      delete state.clipSettings[action.payload.id]
      delete state.clipSampleDurations[action.payload.id]
      delete state.clipWindows[action.payload.id]
    },
    setInspectedClip: (state, action: { type: string, payload: number | null }) => {
      state.inspectedClipId = action.payload
    },
    // a clip's settings and length, as the engine has them
    // (a clip with no window, such as one just recorded, can leave it out)
    setClipInfo: (
      state,
      action: { type: string, payload: { id: number } & Omit<ClipInfo, 'window'> & { window?: ClipWindow | null } },
    ) => {
      const { id, settings, sampleDuration, window } = action.payload
      state.clipSettings[id] = settings
      state.clipSampleDurations[id] = sampleDuration
      setWindow(state, id, window)
    },
    // clips as they are after a trim or a cut: the ones the playlist has are
    // updated where they are, and new ones (the second half of a cut) added
    reduceChangedClips: (state, action: PayloadAction<ChangedClips>) => {
      const { clips, patternClips } = action.payload

      const place = (item: PlaylistItem) => {
        const have = state.playlistItems.find(i => i.id === item.id)
        if(!have) {
          state.playlistItems.push(item)
          return
        }
        have.startBeat = item.startBeat
        have.trackNumber = item.trackNumber
        have.path = item.path
        // a deep copy turns a pattern clip into a sample clip: no stale pattern
        if(item.patternId !== undefined) have.patternId = item.patternId
        else delete have.patternId
      }

      clips.forEach(clip => {
        place(sampleItem(clip))
        state.clipSettings[clip.id] = clip.settings
        state.clipSampleDurations[clip.id] = clip.sampleDuration
        setWindow(state, clip.id, clip.window)
      })
      patternClips.forEach(clip => {
        place(patternItem(clip))
        setWindow(state, clip.id, clip.window)
      })
    },
    // change some of a clip's settings, kept in range like the engine does
    reduceClipSettings: (
      state,
      action: { type: string, payload: { id: number, settings: Partial<ClipSettings> } },
    ) => {
      const { id, settings } = action.payload
      const merged = { ...(state.clipSettings[id] ?? defaultClipSettings), ...settings }
      const sampleSeconds = state.clipSampleDurations[id]
      const played = sampleSeconds === undefined
        ? undefined
        : windowedDuration(merged, sampleSeconds, state.clipWindows[id])

      state.clipSettings[id] = clampClipSettings(merged, played)
    },
    // the project as it is after an undo or redo: every clip, with its
    // settings. clips keep their ids, so the playlist keeps the waveforms it
    // has, and the transport isn't touched
    reduceProjectView: (
      state,
      action: PayloadAction<{ project: ProjectView, selected: number[] | null }>,
    ) => {
      const { project, selected } = action.payload

      state.tempo = project.tempo
      state.playlistItems = [
        ...project.clips.map(clip => ({
          id: clip.id,
          path: clip.path,
          startBeat: clip.startBeat,
          trackNumber: clip.trackNumber,
          pixelOffset: { ...NO_PIXEL_OFFSET },
        })),
        ...project.patternClips.map(clip => ({
          id: clip.id,
          path: '',
          patternId: clip.patternId,
          startBeat: clip.startBeat,
          trackNumber: clip.trackNumber,
          pixelOffset: { ...NO_PIXEL_OFFSET },
        })),
      ]

      state.clipSettings = {}
      state.clipSampleDurations = {}
      state.clipWindows = {}
      project.clips.forEach(clip => {
        state.clipSettings[clip.id] = clip.settings
        state.clipSampleDurations[clip.id] = clip.sampleDuration
        setWindow(state, clip.id, clip.window)
      })
      project.patternClips.forEach(clip => setWindow(state, clip.id, clip.window))

      if(!state.playlistItems.some(item => item.id === state.inspectedClipId)) {
        state.inspectedClipId = null
      }
      if(selected) state.travelled = { count: state.travelled.count + 1, ids: selected }
    },
    setPlaylistUI: (state, action) => {
      state.ui = action.payload
    },
    setTrackCount: (state, action: PayloadAction<number>) => {
      state.ui.trackCount = action.payload
    },
    setSnapEnabled: (state, action) => {
      state.snapEnabled = action.payload
    },
    setSnapNote: (state, action) => {
      state.snapNote = action.payload
    },
    setTool: (state, action: PayloadAction<Tool>) => {
      state.tool = action.payload
    },
    setGridStep: (state, action: PayloadAction<number>) => {
      state.gridStep = action.payload
    },
    // replace the playlist with a project the engine just loaded (or a new,
    // empty one). the engine has already stopped playback
    reduceLoadedProject: (state, action: { type: string, payload: LoadedProject }) => {
      const project = action.payload

      state.playing = false
      state.tempo = project.tempo
      state.metronomeEnabled = project.metronomeEnabled
      state.loopEnabled = project.loopEnabled
      state.snapEnabled = project.snapEnabled
      state.snapNote = project.snapNote
      // the tool isn't saved: a project opens with Select
      state.tool = 'select'
      state.inspectedClipId = null
      // the engine empties its clipboard on New and Open
      state.clipboardCount = 0
      state.viewRevision += 1

      state.playlistItems = project.clips.map(clip => ({
        id: clip.id,
        path: clip.path,
        startBeat: clip.startBeat,
        trackNumber: clip.trackNumber,
        // only used to debug where a clip was dropped
        pixelOffset: { ...NO_PIXEL_OFFSET },
      }))
      // older projects, and the engine's tests' fixtures, have no patterns
      ;(project.patternClips ?? []).forEach(clip => {
        state.playlistItems.push({
          id: clip.id,
          path: '',
          patternId: clip.patternId,
          startBeat: clip.startBeat,
          trackNumber: clip.trackNumber,
          pixelOffset: { ...NO_PIXEL_OFFSET },
        })
      })

      state.clipSettings = {}
      state.clipSampleDurations = {}
      state.clipWindows = {}
      project.clips.forEach(clip => {
        state.clipSettings[clip.id] = clip.settings
        state.clipSampleDurations[clip.id] = clip.sampleDuration
        setWindow(state, clip.id, clip.window)
      })
      ;(project.patternClips ?? []).forEach(clip => setWindow(state, clip.id, clip.window))
    },
  },
})

// start play/pause methods
export const { setPlaying } = playlistSlice.actions

// whether the engine is playing, after a transport command
export const syncPlayingFrom = (dispatch: Dispatch) =>
  invoke<boolean>('get_playlist_playing', {})
    .then(playing => {
      dispatch(setPlaying(playing))
      dispatch(playlistSlice.actions.bumpTransportRevision())
    })

export const togglePlay = () => async (dispatch: Dispatch) => {
  await invoke<void>('toggle_playlist', {})
  await syncPlayingFrom(dispatch)
}

const transportError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// the transport's keys: play from the playhead, pause there, stop and go back
// to the start
export const playPlaylist = () => (dispatch: Dispatch) =>
  invoke<void>('play_playlist', {}).then(() => syncPlayingFrom(dispatch)).catch(transportError('play'))

export const pausePlaylist = () => (dispatch: Dispatch) =>
  invoke<void>('pause_playlist', {}).then(() => syncPlayingFrom(dispatch)).catch(transportError('pause'))

export const stopPlaylist = () => (dispatch: Dispatch) =>
  invoke<void>('stop_playlist', {}).then(() => syncPlayingFrom(dispatch)).catch(transportError('stop'))

// move the playhead to a beat (playing carries on from there)
export const seekPlaylist = (beat: number) => (dispatch: Dispatch) =>
  invoke<number>('seek_playlist', { beat })
    .then(() => syncPlayingFrom(dispatch))
    .catch(transportError(`move to beat ${beat}`))

// catch the engine stopping by itself, at the end of the song
export const syncPlaying = () => (dispatch: Dispatch, getState: () => RootState) =>
  invoke<boolean>('get_playlist_playing', {})
    .then(playing => {
      if(playing !== getState().playlist.playing) dispatch(setPlaying(playing))
    })
    .catch(transportError('check whether the playlist is playing'))

export const selectPlaylistPlaying = (state: RootState) =>
  state.playlist.playing

export const selectTransportRevision = (state: RootState) =>
  state.playlist.transportRevision

export const { setWind } = playlistSlice.actions

export const selectWind = (state: RootState) => state.playlist.wind
// end play/pause methods

// start tempo methods
export const { reduceTempo } = playlistSlice.actions

// the tempos the engine takes (see daw::timing::valid_tempo)
export const validTempo = (tempo: number): boolean =>
  Number.isFinite(tempo) && tempo >= 1 && tempo <= 1000

// change the tempo. clips and the playhead keep their beats, and playback
// carries on from the beat it had got to. a tempo that makes no sense isn't
// sent, and the engine refuses any while recording. resolves to whether the
// tempo was changed
// - gesture: the visit to the tempo field this is part of, if any. every
//   tempo typed during one visit is undone as one step
export const setPlaylistTempo = (val: number, gesture: number | null = null) => async (
  dispatch: Dispatch,
): Promise<boolean> => {
  if(!validTempo(val)) return false

  try {
    const tempo = await invoke<number>('set_playlist_tempo', { val, gesture })
    dispatch(reduceTempo(tempo))
    // the playhead's time, and the beat's length, have changed
    await syncPlayingFrom(dispatch)
    return true
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error(`could not set the tempo to ${val}:`, e)
    return false
  }
}

// retrieve tempo from redux state
export const selectPlaylistTempo = (state: RootState) => state.playlist.tempo
// end tempo methods

// start metronome enable/disable methods
export const { setMetronomeEnabled } = playlistSlice.actions

export const toggleMetronome = () => async (dispatch: Dispatch) => {
  await invoke<void>('toggle_metronome_enabled', {})
  const enabled = await invoke<boolean>('get_metronome_enabled', {})
  dispatch(setMetronomeEnabled(enabled))
}

export const selectMetronomeEnabled = (state: RootState) =>
  state.playlist.metronomeEnabled
// end metronome enable/disable methods

// start loop enable/disable methods
export const { setLoopEnabled } = playlistSlice.actions

export const toggleLoop = () => async (dispatch: Dispatch) => {
  await invoke<void>('toggle_loop_enabled', {})
  const enabled = await invoke<boolean>('get_loop_enabled', {})
  dispatch(setLoopEnabled(enabled))
}

export const selectLoopEnabled = (state: RootState) =>
  state.playlist.loopEnabled
// end loop methods

// start playlist item methods
export const { 
  addPlaylistItem, 
  movePlaylistItem,
  movePlaylistItems,
  removePlaylistItem
} = playlistSlice.actions

// the engine has as many tracks as the highest one with a clip on it (and
// at least its default number). ask it after clips are added, moved or
// removed
export const refreshTrackCount = () => async (dispatch: Dispatch) => {
  try {
    const [, , , trackCount] = await invoke<[number, number, number, number]>('get_playlist_data', {})
    dispatch(playlistSlice.actions.setTrackCount(trackCount))
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error('could not get the track count:', e)
  }
}

// dropBeat: where the clip was dropped, in beats from the playlist start
// (see Renderer.screenXToBeat, which accounts for zoom and pan)
export const addToPlaylist = (
  path: string, 
  trackNumber: number,
  dropBeat: number,
  pixelOffset: Omit<PlaylistItemPixelOffset, 'xOffset' | 'yOffset'>,
) => async (dispatch: Dispatch) => {
  const dropX = pixelOffset.x - pixelOffset.left
  const dropY = pixelOffset.y - pixelOffset.top
  // startBeat is where the engine actually placed the clip (after snapping)
  const { id, startBeat } = await invoke<{ id: number, startBeat: number }>('add_audiograph_node', {
    samplePath: path,
    trackNumber,
    dropBeat,
  })

  dispatch(addPlaylistItem({
    id,
    path,
    startBeat,
    trackNumber,
    pixelOffset: {
      ...pixelOffset,
      xOffset: dropX,
      yOffset: dropY,
    },
  } as PlaylistItem))

  // a clip dropped below the last track made a new one
  await refreshTrackCount()(dispatch)

  // its sample's length, which the playlist needs to trim it (and to show
  // it at the right length once its pitch changes)
  try {
    const info = await invoke<ClipInfo>('get_clip', { id })
    dispatch(playlistSlice.actions.setClipInfo({ id, ...info }))
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error(`could not load clip ${id}:`, e)
  }
}

// an empty track below the last one (the plus under the tracks). it can be
// undone
export const addTrack = () => async (dispatch: Dispatch) => {
  const trackCount = await invoke<number>('add_track', {})
  dispatch(playlistSlice.actions.setTrackCount(trackCount))
}

// place a pattern in the playlist (dragged from the Sequencer's pattern
// strip). the clip stays linked to the pattern, and is as long as the
// pattern is now
export const addPatternToPlaylist = (
  patternId: number,
  trackNumber: number,
  dropBeat: number,
) => async (dispatch: Dispatch) => {
  const clip = await invoke<LoadedPatternClip>('add_pattern_clip', {
    patternId,
    trackNumber,
    dropBeat,
  })

  dispatch(playlistSlice.actions.reduceChangedClips({ clips: [], patternClips: [clip] }))
  await refreshTrackCount()(dispatch)
}

// a clip as it is now, to put back if a move fails
const placedNow = (getState: () => RootState, ids: number[]): MovedNode[] =>
  getState().playlist.playlistItems
    .filter(item => ids.includes(item.id))
    .map(({ id, startBeat, trackNumber }) => ({ id, startBeat, trackNumber }))

// move a clip. it's moved at once to `shownBeat` (where the playlist showed
// it landing), as the engine can take a moment to answer (a pattern on a
// new track waits for its synths' plugins), then to where the engine put it
export const moveNodeInPlaylist = (
  id: number,
  path: string,
  trackNumber: number,
  dropBeat: number,
  pixelOffset: Omit<PlaylistItemPixelOffset, 'xOffset' | 'yOffset'>,
  shownBeat: number = dropBeat,
) => async (dispatch: Dispatch, getState: () => RootState) => {
  // relative to the playlist's top-left, same as addToPlaylist
  const dropX = pixelOffset.x - pixelOffset.left
  const dropY = pixelOffset.y - pixelOffset.top
  const place = (startBeat: number, track: number) => dispatch(movePlaylistItem({
    id,
    path,
    startBeat,
    trackNumber: track,
    pixelOffset: {
      ...pixelOffset,
      xOffset: dropX,
      yOffset: dropY,
    },
  } as PlaylistItem))

  const before = placedNow(getState, [id])
  place(shownBeat, trackNumber)

  try {
    // where the engine actually placed the clip (after snapping), in beats
    const startBeat = await invoke<number>('move_audiograph_node', {
      id,
      trackNumber,
      dropBeat,
    })
    place(startBeat, trackNumber)
  } catch(e) {
    // it didn't move: back where it was
    dispatch(movePlaylistItems(before))
    throw e
  }

  // moved below the last track (a new track), or off the last track
  await refreshTrackCount()(dispatch)
}

// where a clip in a moved group ended up. mirrors MovedNode in
// engine/src/event_dispatch.rs
export interface MovedNode {
  id: number
  startBeat: number
  trackNumber: number
}

// move selected clips together: the grabbed one (anchorId) to dropBeat on
// trackNumber, and the rest by as much. the engine snaps and keeps them in
// the playlist. as moveNodeInPlaylist, they're moved at once to where the
// playlist showed them landing (`shown`), then to where the engine put them
export const moveNodesInPlaylist = (
  ids: number[],
  anchorId: number,
  dropBeat: number,
  trackNumber: number,
  shown: MovedNode[] = [],
) => async (dispatch: Dispatch, getState: () => RootState) => {
  const before = placedNow(getState, ids)
  dispatch(movePlaylistItems(shown))

  try {
    const moved = await invoke<MovedNode[]>('move_audiograph_nodes', {
      ids,
      anchorId,
      dropBeat,
      trackNumber,
    })
    dispatch(movePlaylistItems(moved))
  } catch(e) {
    dispatch(movePlaylistItems(before))
    throw e
  }

  // moved onto the new-track area, or off the last track
  await refreshTrackCount()(dispatch)
}

// remove several clips (a selection)
export const removeManyFromPlaylist = (ids: number[]) => async (dispatch: Dispatch) => {
  // together: all of them go, or (if the engine refuses) none, and undoing
  // brings them all back at once
  await invoke('remove_audiograph_nodes', { ids })
  ids.forEach(id => dispatch(removePlaylistItem({ id })))
  // the last clips on the bottom tracks may have gone
  await refreshTrackCount()(dispatch)
}


export const selectPlaylistItems = (state: RootState) => state.playlist.playlistItems
// end playlist item methods

// start inspected clip methods
export const { setInspectedClip } = playlistSlice.actions

// show a clip in the Sample tab, opening the tab if it was closed
export const inspectClip = (id: number) => async (dispatch: Dispatch) => {
  dispatch(setInspectedClip(id))
  dispatch(openTab(View.SamplePreview))

  try {
    const info = await invoke<ClipInfo>('get_clip', { id })
    dispatch(playlistSlice.actions.setClipInfo({ id, ...info }))
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error(`could not load clip ${id}:`, e)
  }
}

export const selectInspectedClip = (state: RootState): PlaylistItem | undefined => {
  const id = state.playlist.inspectedClipId
  return state.playlist.playlistItems.find(item => item.id === id)
}
// end inspected clip methods

// start clip settings methods
// the frontend shows changes straight away and sends them to the engine,
// which applies them the next time the clip starts playing

// how often a clip's settings are sent while a control is being dragged
const SEND_SETTINGS_EVERY_MS = 50

// per clip: settings waiting to be sent, while a send was recent
const queuedSettings = new Map<number, ClipSettings | null>()

// per clip: the gesture (a drag of a knob or a fade's handle) its settings
// were last changed in, if any. sent with them, however late they go, so
// one drag is undone as one step
const settingsGesture = new Map<number, number | null>()

const sendSettingsNow = (id: number, settings: ClipSettings) =>
  invoke<ClipInfo>('set_clip_settings', { id, settings, gesture: settingsGesture.get(id) ?? null })
    // eslint-disable-next-line no-console
    .catch(e => console.error(`could not update clip ${id}:`, e))

// send a clip's settings, at most every SEND_SETTINGS_EVERY_MS. the latest
// settings are always sent last
const sendSettings = (id: number, settings: ClipSettings) => {
  if(queuedSettings.has(id)) {
    queuedSettings.set(id, settings)
    return
  }

  sendSettingsNow(id, settings)
  queuedSettings.set(id, null)

  const flush = () => {
    const queued = queuedSettings.get(id)

    if(queued) {
      sendSettingsNow(id, queued)
      queuedSettings.set(id, null)
      setTimeout(flush, SEND_SETTINGS_EVERY_MS)
    } else {
      queuedSettings.delete(id)
    }
  }

  setTimeout(flush, SEND_SETTINGS_EVERY_MS)
}

export const selectClipWindow = (id?: number) =>
  (state: RootState): ClipWindow | null =>
    (id === undefined ? undefined : state.playlist.clipWindows[id]) ?? null

export const selectClipSettings = (id: number | undefined) =>
  (state: RootState): ClipSettings =>
    (id === undefined ? undefined : state.playlist.clipSettings[id]) ?? defaultClipSettings

// how the playlist should show each clip whose settings were loaded or
// changed, by clip id. builds a new object every call, so don't pass it to
// useAppSelector
export const selectClipOverrides = (state: RootState): Record<number, ClipOverride> => {
  const { clipSettings, clipSampleDurations, clipWindows } = state.playlist
  const overrides: Record<number, ClipOverride> = {}

  Object.entries(clipSettings).forEach(([key, settings]) => {
    const id = Number(key)
    const sampleSeconds = clipSampleDurations[id]

    overrides[id] = {
      playedDuration: sampleSeconds === undefined
        ? undefined
        : windowedDuration(settings, sampleSeconds, clipWindows[id]),
      modified: !isDefaultClipSettings(settings),
      sampleDuration: sampleSeconds,
      rate: playbackRate(settings),
    }
  })

  return overrides
}

// the clip's sample length in seconds, before any speed change, once loaded
export const selectClipSampleDuration = (id: number | undefined) =>
  (state: RootState): number | undefined =>
    (id === undefined ? undefined : state.playlist.clipSampleDurations[id])

// change some of a clip's settings
export const updateClipSettings = (id: number, settings: Partial<ClipSettings>) => (
  dispatch: Dispatch,
  getState: () => RootState,
) => {
  dispatch(playlistSlice.actions.reduceClipSettings({ id, settings }))
  settingsGesture.set(id, currentGesture())
  sendSettings(id, selectClipSettings(id)(getState()))
}

export const resetClipSettings = (id: number) => (dispatch: Dispatch, getState: () => RootState) =>
  updateClipSettings(id, defaultClipSettings)(dispatch, getState)

// trim clips: each plays the part of its sample (seconds) or pattern (steps)
// from start to end, null for to the end. the engine keeps the sound where it
// is, and returns the clips as they are now. null if it refused
export const setClipWindows = (
  edits: { id: number, start: number, end: number | null }[],
) => async (dispatch: Dispatch): Promise<ChangedClips | null> => {
  try {
    const changed = await invoke<ChangedClips>('set_clip_windows', { edits })
    dispatch(playlistSlice.actions.reduceChangedClips(changed))
    await refreshTrackCount()(dispatch)
    return changed
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error('could not trim the clips:', e)
    return null
  }
}

// stretch sample clips, like tape: each is given the pitch that makes it the
// length it was dragged to, with its start or its end staying where it is.
// the engine returns the clips as they are now. null if it refused
export const stretchClips = (
  edits: { id: number, pitch: number, keep: 'start' | 'end' }[],
) => async (dispatch: Dispatch): Promise<ChangedClips | null> => {
  try {
    const changed = await invoke<ChangedClips>('stretch_clips', { edits })
    dispatch(playlistSlice.actions.reduceChangedClips(changed))
    await refreshTrackCount()(dispatch)
    return changed
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error('could not stretch the clips:', e)
    return null
  }
}

// cut clips in two at a beat. the second halves are new clips, which the
// engine returns with the first halves. null if it refused
export const splitClips = (ids: number[], beat: number) => async (
  dispatch: Dispatch,
): Promise<ChangedClips | null> => {
  try {
    const changed = await invoke<ChangedClips>('split_clips', { ids, beat })
    dispatch(playlistSlice.actions.reduceChangedClips(changed))
    await refreshTrackCount()(dispatch)
    return changed
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error(`could not cut the clips at beat ${beat}:`, e)
    return null
  }
}

export const { setClipboardCount, selectClips, reducePainted } = playlistSlice.actions

// paint clips of a pattern along a track with the Draw tool, from startBeat
// to endBeat (already snapped): end to end, the last cut short at the end.
// with no pattern (null), a new empty pattern long enough to hold the
// drag, and one clip of it. one step to undo. a track below the last makes
// one. null if the engine refused
export const paintPatternClips = (
  patternId: number | null,
  trackNumber: number,
  startBeat: number,
  endBeat: number,
) => async (dispatch: Dispatch): Promise<PaintedClips | null> => {
  try {
    const painted = await invoke<PaintedClips>('paint_pattern_clips', { patternId, trackNumber, startBeat, endBeat })
    dispatch(reducePainted(painted))
    return painted
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error('could not paint the pattern:', e)
    return null
  }
}

export const selectClipboardCount = (state: RootState) => state.playlist.clipboardCount

// copy clips to the engine's clipboard, which the system's is left out of
export const copyClips = (ids: number[]) => async (dispatch: Dispatch): Promise<void> => {
  if(!ids.length) return
  try {
    const count = await invoke<number>('copy_clips', { ids })
    dispatch(playlistSlice.actions.setClipboardCount(count))
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error('could not copy the clips:', e)
  }
}

// copy clips, and take them out of the playlist
export const cutClips = (ids: number[]) => async (dispatch: Dispatch): Promise<void> => {
  if(!ids.length) return
  try {
    const count = await invoke<number>('cut_clips', { ids })
    ids.forEach(id => dispatch(playlistSlice.actions.removePlaylistItem({ id })))
    dispatch(playlistSlice.actions.setClipboardCount(count))
    await refreshTrackCount()(dispatch)
  } catch(e) {
    // eslint-disable-next-line @typescript-eslint/no-use-before-define
    await notifyError(dispatch, `The clips couldn't be cut: ${e}`)
  }
}

// tell the user something in the notice banner. loaded when needed: the
// project slice pulls in the file dialogs, which this slice otherwise doesn't need
const notify = async (dispatch: Dispatch, message: string) => {
  const { setProjectNotice } = await import('./projectSlice')
  dispatch(setProjectNotice({ kind: 'info', message }))
}

// tell the user something went wrong, in the notice banner's error style
const notifyError = async (dispatch: Dispatch, message: string) => {
  const { setProjectNotice } = await import('./projectSlice')
  dispatch(setProjectNotice({ kind: 'error', message }))
}

// the Clips page lists its folder again: deep copy and join wrote files there.
// loaded when needed: the recording slice imports this one
const tookNewFiles = async (dispatch: Dispatch) => {
  const { takesChanged } = await import('./recordingSlice')
  dispatch(takesChanged())
}

// mirrors NEEDS_SAVING in engine/src/bounce.rs: what the engine says when a
// file has to be written and the project has never been saved
export const NEEDS_SAVING = 'The project has to be saved first, so its clips have somewhere to go.'

// show a wait cursor only if the work is slow, so a quick one shows nothing
export const withWaitCursor = async <T,>(
  work: Promise<T>,
  setBusy: (busy: boolean) => void,
): Promise<T> => {
  let busy = false
  const timer = setTimeout(() => {
    busy = true
    setBusy(true)
  }, 500)
  try {
    return await work
  } finally {
    clearTimeout(timer)
    if(busy) setBusy(false)
  }
}

// save the project (the Save dialog opens if it never was), and say whether
// it has a file now: the user may have cancelled
const saveFirst = async (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<boolean> => {
  const { runProjectCommand } = await import('./projectSlice')
  await runProjectCommand('save')(dispatch, getState)
  return !!getState().project.path
}

// run a command that writes files, saving the project first if the engine
// says it must. null if it failed or the save was cancelled. `report` says
// what to do with a failure's sentence
const writingClips = async <T,>(
  dispatch: Dispatch,
  getState: () => RootState,
  setBusy: (busy: boolean) => void,
  command: string,
  ids: number[],
  report: (message: string) => Promise<void>,
): Promise<T | null> => {
  const attempt = () => withWaitCursor(invoke<T>(command, { ids }), setBusy)
  try {
    return await attempt()
  } catch(e) {
    if(String(e) !== NEEDS_SAVING) {
      await report(String(e))
      return null
    }
  }

  if(!await saveFirst(dispatch, getState)) return null
  try {
    return await attempt()
  } catch(e) {
    await report(String(e))
    return null
  }
}

// write each clip out as a file of its own, and switch it to that. the clips
// keep their ids and places. all or nothing; the engine says why if it can't
export const deepCopyClips = (ids: number[], setBusy: (busy: boolean) => void = () => {}) => async (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<ChangedClips | null> => {
  if(!ids.length) return null
  const changed = await writingClips<ChangedClips>(
    dispatch, getState, setBusy, 'deep_copy_clips', ids, message => notifyError(dispatch, message),
  )
  if(!changed) return null

  dispatch(playlistSlice.actions.reduceChangedClips(changed))
  await tookNewFiles(dispatch)
  await refreshTrackCount()(dispatch)
  return changed
}

// join clips into one. the engine refuses, with a sentence starting
// 'Join needs', a join that doesn't apply: that's information, not a failure
export const joinClips = (ids: number[], setBusy: (busy: boolean) => void = () => {}) => async (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<Joined | null> => {
  if(ids.length < 2) return null
  const joined = await writingClips<Joined>(
    dispatch, getState, setBusy, 'join_clips', ids,
    message => message.startsWith('Join needs') ? notify(dispatch, message) : notifyError(dispatch, message),
  )
  if(!joined) return null

  joined.removed.forEach(id => dispatch(playlistSlice.actions.removePlaylistItem({ id })))
  dispatch(playlistSlice.actions.reduceChangedClips(joined))
  if(joined.sequencer) {
    // loaded when needed: the sequencer slice imports this one
    const { setSequencer } = await import('./sequencerSlice')
    dispatch(setSequencer(joined.sequencer))
  }
  await tookNewFiles(dispatch)
  const kept = [...joined.clips, ...joined.patternClips].map(clip => clip.id)
  if(kept.length) dispatch(playlistSlice.actions.selectClips(kept))
  await refreshTrackCount()(dispatch)
  return joined
}

// show the clips the engine placed, select them, and say if any were left out
const reducePlaced = async (dispatch: Dispatch, placed: Placed) => {
  dispatch(playlistSlice.actions.reduceChangedClips(placed))
  const ids = [...placed.clips, ...placed.patternClips].map(clip => clip.id)
  if(ids.length) dispatch(playlistSlice.actions.selectClips(ids))
  await refreshTrackCount()(dispatch)

  if(placed.leftOut > 0) {
    await notify(dispatch, placed.leftOut === 1
      ? '1 clip was left out: its pattern no longer exists.'
      : `${placed.leftOut} clips were left out: their patterns no longer exist.`)
  }
}

// paste the clipboard so its earliest clip starts at a beat: on the tracks
// the clips came from, or with the topmost one on `track`. the engine
// refuses, with a sentence for the user, if any would start past the end
export const pasteClips = (beat: number, track: number | null = null) => async (
  dispatch: Dispatch,
): Promise<Placed | null> => {
  try {
    const placed = await invoke<Placed>('paste_clips', { beat, track })
    await reducePlaced(dispatch, placed)
    return placed
  } catch(e) {
    await notify(dispatch, String(e))
    return null
  }
}

// duplicate clips so the copies start where the originals' span ends
export const repeatClips = (ids: number[]) => async (
  dispatch: Dispatch,
): Promise<Placed | null> => {
  if(!ids.length) return null
  try {
    const placed = await invoke<Placed>('repeat_clips', { ids })
    await reducePlaced(dispatch, placed)
    return placed
  } catch(e) {
    await notify(dispatch, String(e))
    return null
  }
}

// play a clip on its own, with its current settings
export const auditionClip = (id: number) => async (_: Dispatch, getState: () => RootState) => {
  // make sure the engine has the latest settings first
  await sendSettingsNow(id, selectClipSettings(id)(getState()))
  await invoke('preview_node', { id })
}
// end clip settings methods

// start playlist ui methods
export const {
  setPlaylistUI,
} = playlistSlice.actions

// the playlist's length and tracks. (the engine's first and third numbers
// aren't used)
export const initPlaylist = () => async (dispatch: Dispatch) => {
  const [, maxBeatsDisplayed, , trackCount] = await invoke<[number, number, number, number]>('get_playlist_data', {})
  dispatch(setPlaylistUI({ maxBeatsDisplayed, trackCount }))

  // the engine owns the snap defaults
  const snap = await invoke<SnapSetting>('get_snap_setting', {})
  dispatch(playlistSlice.actions.setSnapEnabled(snap.enabled))
  dispatch(playlistSlice.actions.setSnapNote(snap.note))
}

export const selectPlaylistUI = (state: RootState) => state.playlist.ui
// end playlist ui methods

// start loop enable/disable methods
export const { setSnapEnabled } = playlistSlice.actions

export const toggleSnap = () => async (dispatch: Dispatch) => {
  await invoke<void>('toggle_snap_enabled', {})
  const enabled = await invoke<boolean>('get_snap_enabled', {})
  dispatch(setSnapEnabled(enabled))
}

export const selectSnapEnabled = (state: RootState) =>
  state.playlist.snapEnabled

// set the note value clips snap to
export const setSnapSubdivision = (note: SnapNote) => async (dispatch: Dispatch) => {
  const snap = await invoke<SnapSetting>('set_snap_subdivision', { note })
  dispatch(playlistSlice.actions.setSnapNote(snap.note))
}

export const selectSnapNote = (state: RootState) =>
  state.playlist.snapNote
// end snap methods

// start project methods
// the engine reads and writes project files; the frontend then shows
// whatever it loaded

export const { setTool } = playlistSlice.actions

export const selectTool = (state: RootState) => state.playlist.tool

export const { setGridStep } = playlistSlice.actions

export const selectGridStep = (state: RootState) => state.playlist.gridStep

export const { reduceLoadedProject, reduceProjectView } = playlistSlice.actions

// settings changed a moment ago may still be waiting to be sent (see
// sendSettings). send them now, so the engine has them: before saving, or
// before undoing, which mustn't be overtaken by a late change
export const flushClipSettings = (getState: () => RootState): Promise<unknown> =>
  Promise.all(Array.from(queuedSettings.keys()).map(id => {
    queuedSettings.set(id, null)
    return sendSettingsNow(id, selectClipSettings(id)(getState()))
  }))

// settings still waiting to be sent no longer apply (the project was
// replaced, or an undo put the clips' settings back)
export const dropQueuedClipSettings = (): void => {
  queuedSettings.clear()
}

// what a project couldn't find when it was loaded, and what it left out
export interface LoadedMissing {
  samples: MissingSample[]
  effects: MissingEffect[]
  synths: SynthInstrument[]
  dropped: string[]
}

// show a project the engine just loaded. returns the samples and plugins it
// couldn't find
const showLoadedProject = async (
  dispatch: Dispatch,
  project: LoadedProject,
): Promise<LoadedMissing> => {
  // settings still waiting to be sent belong to clips that are gone
  queuedSettings.clear()
  settingsGesture.clear()
  dispatch(reduceLoadedProject(project))
  // the playlist's length depends on its clips
  await initPlaylist()(dispatch)

  const { effects, synths } = splitMissing(project.missingEffects ?? [], project.sequencer?.rows ?? [])
  return { samples: project.missing, effects, synths, dropped: project.dropped ?? [] }
}

// save the playlist to a project file. saved somewhere new (a first save, or
// Save As), the project gets a folder of its own, and the file it's saved to
// is returned
export const saveProject = (path: string, somewhereNew: boolean) => async (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<SavedProject> => {
  // clips changed in the last moment may not have been sent yet. send their
  // latest settings, so the file has them
  await flushClipSettings(getState)

  const saved = await invoke<SavedProject>('save_project', { path, somewhereNew })
  dispatch(playlistSlice.actions.reduceMovedClips(saved.movedClips))
  return saved
}

// replace the playlist with a project file. returns the samples it couldn't
// find, which were left out, and the plugins it couldn't load. if the file can't be opened, this throws and
// the playlist is unchanged
export const openProject = (path: string) => async (dispatch: Dispatch): Promise<LoadedMissing> => {
  const project = await invoke<LoadedProject>('load_project', { path })
  return showLoadedProject(dispatch, project)
}

// replace the playlist with an empty project
export const newProject = () => async (dispatch: Dispatch): Promise<void> => {
  const project = await invoke<LoadedProject>('new_project', {})
  await showLoadedProject(dispatch, project)
}
// end project methods

// export root reducer for this slice
export default playlistSlice.reducer
