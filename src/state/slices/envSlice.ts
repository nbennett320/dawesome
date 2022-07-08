import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'
import { BrowserSampleItem } from '../../types/sampleBrowser'

export interface EnvState {
  browserSamples: Record<string, BrowserSampleItem>
}

const initialState = {
  browserSamples: {
    root: {
      index: 'root',
      canMove: false,
      hasChildren: false,
      children: [],
      data: 'assets',
      canRename: false,
    },
  },
} as EnvState

export const envSlice = createSlice({
  name: 'envSlice',
  initialState,
  reducers: {
    setBrowserSamples: (
      state, 
      action,
    ) => {
      const { sampleItems, dirItems, rootChildren } = action.payload
      console.log("child keys: ", sampleItems, dirItems, rootChildren)
      state.browserSamples?.root.children.push(...rootChildren)

      state.browserSamples = {
        ...state.browserSamples,
        ...sampleItems,
        ...dirItems,
      }

      if(state.browserSamples?.root.children) {
        // remove duplicated keys
        state.browserSamples.root.children = [...new Set(state.browserSamples?.root.children)]
      }

      console.log("updated browserSamples: ", state.browserSamples)
    },
  },
})

// start playlist samples methods
export const { setBrowserSamples } = envSlice.actions

export const getBrowserSamples = () => async (dispatch: Dispatch) => {
  const [samples, dirs] = await invoke<[string[], string[]]>('get_sidebar_samples', {})

  const sampleItems = samples.map(sample => ({
    index: sample,
    canMove: false,
    hasChildren: false,
    children: [],
    data: sample,
    canRename: false,
  } as BrowserSampleItem))

  const dirItems = dirs.map(dir => ({
    index: dir,
    canMove: false,
    hasChildren: false,
    children: [],
    data: dir,
    canRename: false,
  } as BrowserSampleItem))

  const rootChildren = [...samples, ...dirs]

  dispatch(setBrowserSamples({ 
    sampleItems, 
    dirItems, 
    rootChildren,
  }))
}

export const selectBrowserSamples = (state: RootState) => state.env.browserSamples
// end sidebar samples methods

export default envSlice.reducer
