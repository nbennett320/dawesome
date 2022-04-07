import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'

export interface PlayPausePlaylistState {
  value: boolean
}

const initialState = {
  value: false
} as PlayPausePlaylistState

export const playPausePlaylistSlice = createSlice({
  name: 'playlistPlaying',
  initialState,
  reducers: {
    setPlaying: (state, action) => {
      state.value = action.payload
    }
  }
})

export const { setPlaying } = playPausePlaylistSlice.actions

export const togglePlay = () => (dispatch: Dispatch) => {
  invoke('toggle_playlist', {}).then(() => {
    invoke('get_playlist_playing', {}).then((data) => {
      dispatch(setPlaying(data as boolean))
    })
  })
}

export const selectPlaylistPlaying = (state: RootState) => state.playlistPlaying

export default playPausePlaylistSlice.reducer
