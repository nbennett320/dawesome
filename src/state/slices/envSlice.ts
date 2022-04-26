import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'

export interface EnvState {
  playlistSamples: string[]
}

const initialState = {
  playlistSamples: [],
} as EnvState

export const envSlice = createSlice({
  name: 'envSlice',
  initialState,
  reducers: {
    setPlaylistSamples: (state, action) => {
      state.playlistSamples = action.payload
    },
  },
})

// start playlist samples methods
export const { setPlaylistSamples } = envSlice.actions

export const getPlaylistSamples = () => async (dispatch: Dispatch) => {
  const samples = await invoke<string[]>('get_sidebar_samples', {})
  dispatch(setPlaylistSamples(samples))
}

export const selectPlaylistSamples = (state: RootState) => state.env.playlistSamples
// end sidebar samples methods

export default envSlice.reducer
