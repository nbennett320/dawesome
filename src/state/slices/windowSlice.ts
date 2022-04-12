import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { RootState } from 'state/store'

export interface WindowState {
  playlist: boolean
  devicePreferences: boolean
}

// todo: get initial state from backend,
// which parses dawesome.config file
const initialState = {
  playlist: true,
  devicePreferences: false
} as WindowState

export const windowSlice = createSlice({
  name: 'windowSlice',
  initialState,
  reducers: {
    reduceShowDevicePreferences: (state, action) => {
      state.devicePreferences = action.payload
    },
  },
})

// start play/pause methods
export const { reduceShowDevicePreferences } = windowSlice.actions

export const setShowDevicePreferences = (val: boolean) => async (dispatch: Dispatch) => {
  dispatch(reduceShowDevicePreferences(val))
}

export const selectDevicePreferences = (state: RootState) =>
  state.window.devicePreferences

// export root reducer for this slice
export default windowSlice.reducer
