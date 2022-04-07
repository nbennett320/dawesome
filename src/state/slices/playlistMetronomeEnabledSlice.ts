import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'

export interface PlaylistMetronomeEnabledState {
  value: boolean
}

const initialState = {
  value: true
} as PlaylistMetronomeEnabledState

export const playlistMetronomeEnabled = createSlice({
  name: 'playlistTempo',
  initialState,
  reducers: {
    setEnabled: (state, action) => {
      state.value = action.payload
    }
  }
})

export const { setEnabled } = playlistMetronomeEnabled.actions

export const toggleMetronome = () => (dispatch: Dispatch) => {
  invoke('toggle_metronome_enabled', {}).then(() => {
    invoke('get_metronome_enabled', {}).then((data) => {
      dispatch(setEnabled(data as boolean))
    })
  })
}

export const selectMetronomeEnabled = (state: RootState) => state.playlistMetronomeEnabled

export default playlistMetronomeEnabled.reducer

