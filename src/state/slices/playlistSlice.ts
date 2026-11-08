import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'
import { PlaylistItem, PlaylistItemPixelOffset } from '../../types/playlist'
import { View } from '../../types/ui'
import {
  ClipSettings,
  clampClipSettings,
  defaultClipSettings,
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
  // how each clip is played back, by clip id. clips without an entry use
  // defaultClipSettings
  clipSettings: Record<number, ClipSettings>
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
    },
    setInspectedClip: (state, action: { type: string, payload: number | null }) => {
      state.inspectedClipId = action.payload
    },
    // change some of a clip's settings
    // - sampleSeconds: the sample's length before any speed change, to keep
    //   fades within the clip once the new pitch is applied
    updateClipSettings: (
      state,
      action: {
        type: string
        payload: { id: number, settings: Partial<ClipSettings>, sampleSeconds?: number }
      },
    ) => {
      const { id, settings, sampleSeconds } = action.payload
      const merged = { ...(state.clipSettings[id] ?? defaultClipSettings), ...settings }
      const played = sampleSeconds === undefined ? undefined : playedDuration(merged, sampleSeconds)

      state.clipSettings[id] = clampClipSettings(merged, played)
    },
    resetClipSettings: (state, action: { type: string, payload: number }) => {
      delete state.clipSettings[action.payload]
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
export const inspectClip = (id: number) => (dispatch: Dispatch) => {
  dispatch(setInspectedClip(id))
  dispatch(openTab(View.SamplePreview))
}

export const selectInspectedClip = (state: RootState): PlaylistItem | undefined => {
  const id = state.playlist.inspectedClipId
  return state.playlist.playlistItems.find(item => item.id === id)
}
// end inspected clip methods

// start clip settings methods
// (kept in the frontend for now, the engine will apply them next)
export const { updateClipSettings, resetClipSettings } = playlistSlice.actions

export const selectClipSettings = (id: number | undefined) =>
  (state: RootState): ClipSettings =>
    (id === undefined ? undefined : state.playlist.clipSettings[id]) ?? defaultClipSettings
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
