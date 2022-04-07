import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'

export interface PlaylistTempoState {
  value: number
}

const initialState = {
  value: 120.
} as PlaylistTempoState

export const playlistTempoSlice = createSlice({
  name: 'playlistTempo',
  initialState,
  reducers: {
    reduceTempo: (state, action) => {
      state.value = action.payload
    }
  }
})

export const { reduceTempo } = playlistTempoSlice.actions

export const setPlaylistTempo = (val: number) => (dispatch: Dispatch) => {
  invoke('set_playlist_tempo', { val }).then(() => {
    dispatch(reduceTempo(val))
  })
}

// retrieve tempo from internal (rust) state
export const getPlaylistTempo = () => () => {
  invoke('get_playlist_playing', {}).then((data) => data as number)
}

// retrieve tempo from redux state
export const selectPlaylistTempo = (state: RootState) => state.playlistTempo

export default playlistTempoSlice.reducer
