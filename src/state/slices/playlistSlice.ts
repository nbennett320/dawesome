import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'
import { PlaylistItem } from '../../types/playlist'

export interface PlaylistState {
  playing: boolean
  tempo: number
  runtime?: string | null
  metronomeEnabled: boolean,
  loopEnabled: boolean,
  playlistItems: Array<PlaylistItem>
}

// todo: get initial state from backend,
// which parses dawesome.config file
const initialState = {
  playing: false,
  tempo: 120,
  runtime: null,
  metronomeEnabled: true,
  loopEnabled: true,
  playlistItems: [],
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
        offset: action.payload.offset,
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
            offset: action.payload.offset,
            trackNumber: action.payload.trackNumber,
            pixelOffset: action.payload.pixelOffset,
          } as PlaylistItem

          break
        }
      }
    },
    removePlaylistItem: (state, action) => {
      state.playlistItems = state.playlistItems.filter(e => e.id !== action.payload.id)
    }
  },
})

// start play/pause methods
export const { setPlaying } = playlistSlice.actions

export const togglePlay = () => async (dispatch: Dispatch) => {
  invoke<void>('toggle_playlist', {})
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
// end metronome enable/disable methods

// start playlist item methods
export const { 
  addPlaylistItem, 
  movePlaylistItem,
  removePlaylistItem
} = playlistSlice.actions

export const addToPlaylist = (
  path: string, 
  offset: number, 
  trackNumber: number,
  pixelOffset: number,
) => async (dispatch: Dispatch) => {
  console.log("adding node:", path, offset, trackNumber)
  const id = await invoke<number>('add_audiograph_node', {
    samplePath: path,
    startOffset: offset,
    trackNumber,
  })

  console.log("id:",id)
  dispatch(addPlaylistItem({
    id,
    path,
    offset,
    trackNumber,
    pixelOffset,
  } as PlaylistItem))
}

export const moveNodeInPlaylist = (
  id: number,
  path: string,
  offset: number,
  trackNumber: number,
  pixelOffset: number,
) => async (dispatch: Dispatch) => {
  await invoke('move_audiograph_node', {
    id,
    startOffset: offset,
    trackNumber,
  })

  dispatch(movePlaylistItem({
    id,
    path,
    offset,
    trackNumber,
    pixelOffset,
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

// export root reducer for this slice
export default playlistSlice.reducer
