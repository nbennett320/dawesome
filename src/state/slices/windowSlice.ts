import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { RootState } from 'state/store'

export interface WindowState {
  playlist: boolean
  sidebar: boolean
  devicePreferences: boolean
}

// todo: get initial state from backend,
// which parses dawesome.config file
const initialState = {
  playlist: true,
  sidebar: true,
  devicePreferences: false,
} as WindowState

export const windowSlice = createSlice({
  name: 'windowSlice',
  initialState,
  reducers: {
    setSidebar: (state, action) => {
      state.sidebar = action.payload
    },
    reduceShowDevicePreferences: (state, action) => {
      state.devicePreferences = action.payload
    },
  },
})

// start sidebar methods
export const { setSidebar } = windowSlice.actions

export const toggleSidebar = () => (
  dispatch: Dispatch, 
  getState: () => RootState
) => {
  const open = getState().window.sidebar
  dispatch(setSidebar(!open))
}

export const selectSidebar = (state: RootState) => state.window.sidebar
// end sidebar methods

// start show device methods
export const { reduceShowDevicePreferences } = windowSlice.actions

export const setShowDevicePreferences = (val: boolean) => async (dispatch: Dispatch) => {
  dispatch(reduceShowDevicePreferences(val))
}

export const selectDevicePreferences = (state: RootState) =>
  state.window.devicePreferences

// export root reducer for this slice
export default windowSlice.reducer
