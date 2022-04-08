import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'

export interface PlaylistState {
  playing: boolean
  tempo: number
  runtime?: string | null
  metronomeEnabled: boolean
}

// todo: get initial state from backend,
// which parses dawesome.config file
const initialState = {
  playing: false,
  tempo: 120,
  runtime: null,
  metronomeEnabled: true,
} as PlaylistState

export const playlistSlice = createSlice({
  name: 'metronomeEnabled',
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
  },
})

// start play/pause methods
export const { setPlaying } = playlistSlice.actions

export const togglePlay = () => async (dispatch: Dispatch) => {
  await invoke<void>('toggle_playlist', {})
  const playing = await invoke<boolean>('get_playlist_playing', {})
  dispatch(setPlaying(playing))
}

export const selectPlaylistPlaying = (state: RootState) => state.playlist.playing
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

export const selectPlaylistRuntime = (state: RootState) => state.playlist.runtime
// end rutime methods

// start metronome enable/disable methods
export const { setMetronomeEnabled } = playlistSlice.actions

export const toggleMetronome = () => async (dispatch: Dispatch) => {
  await invoke<void>('toggle_metronome_enabled', {})
  const enabled = await invoke<boolean>('get_metronome_enabled', {})
  dispatch(setMetronomeEnabled(enabled))
}

export const selectMetronomeEnabled = (state: RootState) => state.playlist.metronomeEnabled
// end metronome enable/disable methods

// export root reducer for this slice
export default playlistSlice.reducer
