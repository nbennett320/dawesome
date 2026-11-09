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

      // listing the root again (e.g. StrictMode's second effect) replaces it
      state.browserSamples.root.children = rootChildren
      Object.entries({ ...sampleItems, ...dirItems } as Record<string, BrowserSampleItem>).forEach(([key, item]) => {
        state.browserSamples[key] = { ...item, children: state.browserSamples[key]?.children ?? item.children }
      })
    },
    addBrowserSamples: (state, action) => {
      const { sampleItems, dirItems, children, dirPath } = action.payload
      
      const dir = state.browserSamples[dirPath]
      if(!dir) return

      // listing a folder again replaces its children. subfolders already
      // opened keep what they listed
      dir.children = children
      dir.hasChildren = true
      Object.entries({ ...sampleItems, ...dirItems } as Record<string, BrowserSampleItem>).forEach(([key, item]) => {
        state.browserSamples[key] = { ...item, children: state.browserSamples[key]?.children ?? item.children }
      })
    }
  },
})

// start playlist samples methods
export const { setBrowserSamples, addBrowserSamples } = envSlice.actions

// the last part of a path, e.g. "./assets/drums" is "drums"
export const baseName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path

const byName = (a: BrowserSampleItem, b: BrowserSampleItem) =>
  a.label.localeCompare(b.label, undefined, { numeric: true, sensitivity: 'base' })

// the engine lists a directory as [sample names, sample paths, directory
// paths, directory paths]. items are keyed by path, since names repeat across
// folders. folders come first, then samples, each alphabetical
const toItems = ([samples, samplePaths, dirs, dirPaths]: [string[], string[], string[], string[]], where: string) => {
  if(samples.length !== samplePaths.length || dirs.length !== dirPaths.length) {
    // eslint-disable-next-line no-console
    console.error(`could not list ${where}: names and paths don't line up`)
  }

  const sampleItems: Record<string, BrowserSampleItem> = {}
  const dirItems: Record<string, BrowserSampleItem> = {}

  samplePaths.forEach((path, idx) => {
    sampleItems[path] = {
      index: path,
      canMove: false,
      hasChildren: false,
      children: [],
      data: samples[idx] ?? baseName(path),
      label: samples[idx] ?? baseName(path),
      path,
      canRename: false,
      itemType: BrowserItemTypes.Sample,
    }
  })

  dirPaths.forEach(path => {
    dirItems[path] = {
      index: path,
      canMove: false,
      hasChildren: true,
      children: [],
      data: baseName(path),
      label: baseName(path),
      path,
      canRename: false,
      itemType: BrowserItemTypes.Directory,
    }
  })

  const children = [
    ...Object.values(dirItems).sort(byName),
    ...Object.values(sampleItems).sort(byName),
  ].map(item => item.index)

  return { sampleItems, dirItems, children }
}

export const getBrowserRootSamples = () => async (dispatch: Dispatch) => {
  const listing = await invoke<[string[], string[], string[], string[]]>('get_sidebar_samples', {})
  const { sampleItems, dirItems, children } = toItems(listing, 'the samples folder')

  dispatch(setBrowserSamples({
    sampleItems,
    dirItems,
    rootChildren: children,
  }))
}

export const getDirectorySamples = (dirPath: string) => async (dispatch: Dispatch) => {
  const listing = await invoke<[string[], string[], string[], string[]]>('enumerate_directory', {
    dirPath,
  })
  const { sampleItems, dirItems, children } = toItems(listing, dirPath)

  dispatch(addBrowserSamples({
    sampleItems,
    dirItems,
    children,
    dirPath,
  }))
}

export const selectBrowserSamples = (state: RootState) => state.env.browserSamples
// end sidebar samples methods

export default envSlice.reducer
