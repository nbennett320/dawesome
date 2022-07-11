import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { RootState } from 'state/store'
import { BrowserSampleItem, BrowserItemTypes } from '../../types/sampleBrowser'

export interface EnvState {
  browserSamples: Record<string, BrowserSampleItem>
}

const initialState = {
  browserSamples: {
    root: {
      index: 'root',
      canMove: false,
      hasChildren: true,
      children: [],
      data: 'assets',
      label: './assets',
      path: './assets',
      canRename: false,
      itemType: BrowserItemTypes.Directory,
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
  const [samples, samplePaths, dirs, dirPaths] = 
    await invoke<[string[], string[], string[], string[]]>('get_sidebar_samples', {})
  const sampleItems: Record<string, BrowserSampleItem> = {}
  const dirItems: Record<string, BrowserSampleItem> = {}

  if(samples.length !== samplePaths.length) {
    // eslint-disable-next-line no-console
    console.error('Error getting samples: sample.length is not equal to samplePaths.length')
  }

  if(dirs.length !== dirPaths.length) {
    // eslint-disable-next-line no-console
    console.error('Error getting samples: dirs.length is not equal to dirPaths.length')
  }

  samples.forEach((sample, idx) => {
    sampleItems[sample] = {
      index: sample,
      canMove: false,
      hasChildren: false,
      children: [],
      data: sample,
      label: sample,
      path: samplePaths[idx],
      canRename: false,
      itemType: BrowserItemTypes.Sample,
    } as BrowserSampleItem
  })

  dirs.forEach((dir, idx) => {
    dirItems[dir] = {
      index: dir,
      canMove: false,
      hasChildren: false,
      children: [],
      data: dir,
      label: dir,
      path: dirPaths[idx],
      canRename: false,
      itemType: BrowserItemTypes.Directory,
    } as BrowserSampleItem
  })

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
