import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'

export interface PlaylistRuntimeState {
  value: number | null
}

const initialState = {
  value: null
} as PlaylistRuntimeState

export const playlistRuntimeSlice = createSlice({
  name: 'playlistRuntime',
  initialState,
  reducers: {
    setRuntime: (state, action) => {
      state.value = action.payload
    }
  }
})

export const { setRuntime } = playlistRuntimeSlice.actions

export const fetchPlaylistRuntime = () => (dispatch: Dispatch) => {
  invoke('get_playlist_runtime_formatted', {}).then((data) => {
    dispatch(setRuntime(data as string))
  })
}

export const selectPlaylistRuntime = (state: RootState) => state.playlistRuntime

export default playlistRuntimeSlice.reducer
