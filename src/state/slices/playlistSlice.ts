import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'
import { PlaylistItem, PlaylistItemPixelOffset } from '../../types/playlist'
import { View } from '../../types/ui'
import {
  ClipInfo,
  ClipOverride,
  ClipSettings,
  clampClipSettings,
  defaultClipSettings,
  isDefaultClipSettings,
  playedDuration,
} from '../../types/clip'
import { openTab } from './windowSlice'

// mirrors daw::timing::Note in the engine
export type SnapNote =
  | 'WholeNote'
  | 'HalfNote'
  | 'QuarterNote'
  | 'EighthNote'
  | 'SixteenthNote'

// mirrors SnapSetting in engine/src/event_dispatch.rs
interface SnapSetting {
  enabled: boolean
  note: SnapNote
  beats: number
}

export interface PlaylistState {
  playing: boolean
  tempo: number
  runtime?: string | null
  currentBeat?: number | null
  metronomeEnabled: boolean,
  loopEnabled: boolean,
  snapEnabled: boolean,
  snapNote: SnapNote,
  playlistItems: Array<PlaylistItem>
  // the clip shown in the Sample tab
  inspectedClipId: number | null
  // how each clip is played back, by clip id, as last loaded from or sent to
  // the engine. clips without an entry use defaultClipSettings
  clipSettings: Record<number, ClipSettings>
  // each clip's sample length in seconds, before any speed change, by clip id
  clipSampleDurations: Record<number, number>
  ui: {
    maxPlaylistBeats: number,
    maxBeatsDisplayed: number,
    maxPlaylistDuration: number,
    trackCount: number,
  },
}

// todo: get initial state from backend,
// which parses dawesome.config file
const initialState = {
  playing: false,
  tempo: 120,
  runtime: null,
  currentBeat: null,
  metronomeEnabled: true,
  loopEnabled: true,
  snapEnabled: true,
  snapNote: 'QuarterNote',
  playlistItems: [],
  inspectedClipId: null,
  clipSettings: {},
  clipSampleDurations: {},
  ui: {
    maxPlaylistBeats: 16,
    maxBeatsDisplayed: 16,
    maxPlaylistDuration: 120*2,
    trackCount: 4,
  },
} as PlaylistState

export const playlistSlice = createSlice({
  name: 'playlistSlice',
  initialState,
  reducers: {
    setPlaying: (state, action) => {
      state.playing = action.payload
    },
    reduceTempo: (state, action) => {
      state.tempo = action.payload
    },
    setRuntime: (state, action) => {
      state.runtime = action.payload
    },
    setBeatCount: (state, action) => {
      state.currentBeat = action.payload
    },
    setMetronomeEnabled: (state, action) => {
      state.metronomeEnabled = action.payload
    },
    setLoopEnabled: (state, action) => {
      state.loopEnabled = action.payload
    },
    addPlaylistItem: (state, action) => {
      state.playlistItems.push({
        id: action.payload.id,
        path: action.payload.path,
        startBeat: action.payload.startBeat,
        trackNumber: action.payload.trackNumber,
        pixelOffset: action.payload.pixelOffset,
      } as PlaylistItem)
    },
    movePlaylistItem: (state, action) => {
      for(let i = 0; i < state.playlistItems.length; i++) {
        const item = state.playlistItems[i]

        if(item.id === action.payload.id) {
          state.playlistItems[i] = {
            id: action.payload.id,
            path: action.payload.path,
            startBeat: action.payload.startBeat,
            trackNumber: action.payload.trackNumber,
            pixelOffset: action.payload.pixelOffset,
          } as PlaylistItem

          break
        }
      }
    },
    removePlaylistItem: (state, action) => {
      state.playlistItems = state.playlistItems.filter(e => e.id !== action.payload.id)

      if(state.inspectedClipId === action.payload.id) {
        state.inspectedClipId = null
      }

      delete state.clipSettings[action.payload.id]
      delete state.clipSampleDurations[action.payload.id]
    },
    setInspectedClip: (state, action: { type: string, payload: number | null }) => {
      state.inspectedClipId = action.payload
    },
    // a clip's settings and length, as the engine has them
    setClipInfo: (state, action: { type: string, payload: { id: number } & ClipInfo }) => {
      const { id, settings, sampleDuration } = action.payload
      state.clipSettings[id] = settings
      state.clipSampleDurations[id] = sampleDuration
    },
    // change some of a clip's settings, kept in range like the engine does
    reduceClipSettings: (
      state,
      action: { type: string, payload: { id: number, settings: Partial<ClipSettings> } },
    ) => {
      const { id, settings } = action.payload
      const merged = { ...(state.clipSettings[id] ?? defaultClipSettings), ...settings }
      const sampleSeconds = state.clipSampleDurations[id]
      const played = sampleSeconds === undefined ? undefined : playedDuration(merged, sampleSeconds)

      state.clipSettings[id] = clampClipSettings(merged, played)
    },
    setPlaylistUI: (state, action) => {
      state.ui = action.payload
    },
    setSnapEnabled: (state, action) => {
      state.snapEnabled = action.payload
    },
    setSnapNote: (state, action) => {
      state.snapNote = action.payload
    },
  },
})

// start play/pause methods
export const { setPlaying } = playlistSlice.actions

export const togglePlay = () => async (dispatch: Dispatch) => {
  invoke<void>('t_toggle_playlist', {})
  const playing = await invoke<boolean>('get_playlist_playing', {})
  dispatch(setPlaying(playing))
}

export const selectPlaylistPlaying = (state: RootState) =>
  state.playlist.playing
// end play/pause methods

// start tempo methods
export const { reduceTempo } = playlistSlice.actions

export const setPlaylistTempo = (val: number) => async (dispatch: Dispatch) => {
  await invoke<void>('set_playlist_tempo', { val })
  dispatch(reduceTempo(val))
}

// retrieve tempo from internal (rust) state
export const getPlaylistTempo = () => async (): Promise<boolean> => {
  const playing = await invoke<boolean>('get_playlist_playing', {})
  return playing
}

// retrieve tempo from redux state
export const selectPlaylistTempo = (state: RootState) => state.playlist.tempo
// end tempo methods

// start runtime methods
export const { setRuntime } = playlistSlice.actions

export const fetchPlaylistRuntime = () => async (dispatch: Dispatch) => {
  const runtime = await invoke<string>('get_playlist_runtime_formatted', {})
  dispatch(setRuntime(runtime))
}

export const selectPlaylistRuntime = (state: RootState) =>
  state.playlist.runtime
// end rutime methods

// start beat count methods
export const { setBeatCount } = playlistSlice.actions

export const fetchPlaylistBeatCount = () => async (dispatch: Dispatch) => {
  const count = await invoke<string>('get_playlist_beat_count', {})
  dispatch(setBeatCount(count))
}

export const selectPlaylistBeatCount = (state: RootState) =>
  state.playlist.currentBeat
// end rutime methods

// start metronome enable/disable methods
export const { setMetronomeEnabled } = playlistSlice.actions

export const toggleMetronome = () => async (dispatch: Dispatch) => {
  await invoke<void>('t_toggle_metronome_enabled', {})
  const enabled = await invoke<boolean>('get_metronome_enabled', {})
  dispatch(setMetronomeEnabled(enabled))
}

export const selectMetronomeEnabled = (state: RootState) =>
  state.playlist.metronomeEnabled
// end metronome enable/disable methods

// start loop enable/disable methods
export const { setLoopEnabled } = playlistSlice.actions

export const toggleLoop = () => async (dispatch: Dispatch) => {
  await invoke<void>('t_toggle_loop_enabled', {})
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
  removePlaylistItem
} = playlistSlice.actions

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
  const { id, startBeat } = await invoke<{ id: number, startBeat: number }>('t_add_audiograph_node', {
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
}

export const moveNodeInPlaylist = (
  id: number,
  path: string,
  trackNumber: number,
  dropBeat: number,
  pixelOffset: Omit<PlaylistItemPixelOffset, 'xOffset' | 'yOffset'>,
) => async (dispatch: Dispatch) => {
  // relative to the playlist's top-left, same as addToPlaylist
  const dropX = pixelOffset.x - pixelOffset.left
  const dropY = pixelOffset.y - pixelOffset.top
  // where the engine actually placed the clip (after snapping), in beats
  const startBeat = await invoke<number>('move_audiograph_node', {
    id,
    trackNumber,
    dropBeat,
  })

  dispatch(movePlaylistItem({
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
}

export const removeFromPlaylist = (id: number) => async (dispatch: Dispatch) => {
  await invoke('remove_audiograph_node', {
    id
  })

  dispatch(removePlaylistItem({
    id
  }))
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

const sendSettingsNow = (id: number, settings: ClipSettings) =>
  invoke<ClipInfo>('set_clip_settings', { id, settings })
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

export const selectClipSettings = (id: number | undefined) =>
  (state: RootState): ClipSettings =>
    (id === undefined ? undefined : state.playlist.clipSettings[id]) ?? defaultClipSettings

// how the playlist should show each clip whose settings were loaded or
// changed, by clip id. builds a new object every call, so don't pass it to
// useAppSelector
export const selectClipOverrides = (state: RootState): Record<number, ClipOverride> => {
  const { clipSettings, clipSampleDurations } = state.playlist
  const overrides: Record<number, ClipOverride> = {}

  Object.entries(clipSettings).forEach(([key, settings]) => {
    const id = Number(key)
    const sampleSeconds = clipSampleDurations[id]

    overrides[id] = {
      playedDuration: sampleSeconds === undefined
        ? undefined
        : playedDuration(settings, sampleSeconds),
      modified: !isDefaultClipSettings(settings),
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
  sendSettings(id, selectClipSettings(id)(getState()))
}

export const resetClipSettings = (id: number) => (dispatch: Dispatch, getState: () => RootState) =>
  updateClipSettings(id, defaultClipSettings)(dispatch, getState)

// play a clip on its own, with its current settings
export const auditionClip = (id: number) => async (_: Dispatch, getState: () => RootState) => {
  // make sure the engine has the latest settings first
  await sendSettingsNow(id, selectClipSettings(id)(getState()))
  await invoke('t_preview_node', { id })
}
// end clip settings methods

// start playlist ui methods
export const {
  setPlaylistUI,
} = playlistSlice.actions

export const initPlaylist = () => async (dispatch: Dispatch) => {
  const [
    maxPlaylistBeats,
    maxBeatsDisplayed,
    maxPlaylistDuration,
    trackCount
  ] = await invoke<[number, number, number, number]>('get_playlist_data', {})
  
  dispatch(setPlaylistUI({
    maxPlaylistBeats,
    maxBeatsDisplayed,
    maxPlaylistDuration,
    trackCount,
  }))

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

// export root reducer for this slice
export default playlistSlice.reducer
