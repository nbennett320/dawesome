import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import { RootState } from 'state/store'
import { BrowserSampleItem, BrowserItemTypes } from '../../types/sampleBrowser'

// a folder of samples, as Preferences › File Paths and the Samples browser
// list it (see engine/src/daw/library.rs)
export type SamplePath = {
  name: string
  path: string
  // the ones Dawesome comes with, a folder the user added, or the open
  // project's samples folder
  kind: 'bundled' | 'user' | 'project'
  // whether it's there (an added folder can be moved or deleted)
  found: boolean
}

export interface EnvState {
  browserSamples: Record<string, BrowserSampleItem>
  samplePaths: SamplePath[]
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
  samplePaths: [],
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

      // listing the root again (e.g. StrictMode's second effect) replaces it.
      // what was listed in a folder that's gone from it goes too, so search
      // doesn't find it
      state.browserSamples.root.children = rootChildren
      const roots = rootChildren as string[]
      Object.keys(state.browserSamples).forEach(key => {
        if(key !== 'root' && !roots.some(root => key === root || key.startsWith(`${root}/`))) {
          delete state.browserSamples[key]
        }
      })
      Object.entries({ ...sampleItems, ...dirItems } as Record<string, BrowserSampleItem>).forEach(([key, item]) => {
        state.browserSamples[key] = { ...item, children: state.browserSamples[key]?.children ?? item.children }
      })
    },
    setSamplePaths: (state, action: { payload: SamplePath[] }) => {
      state.samplePaths = action.payload
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
export const { setBrowserSamples, addBrowserSamples, setSamplePaths } = envSlice.actions

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

// the Samples browser's top rows: a folder for each folder of samples that's
// there, in File Paths' order, named as it lists them
const showRoots = (dispatch: Dispatch, paths: SamplePath[]) => {
  const dirItems: Record<string, BrowserSampleItem> = {}
  const found = paths.filter(path => path.found)
  found.forEach(({ name, path }) => {
    dirItems[path] = {
      index: path,
      canMove: false,
      hasChildren: true,
      children: [],
      data: name,
      label: name,
      path,
      canRename: false,
      itemType: BrowserItemTypes.Directory,
    }
  })

  dispatch(setSamplePaths(paths))
  dispatch(setBrowserSamples({
    sampleItems: {},
    dirItems,
    rootChildren: found.map(({ path }) => path),
  }))
}

// list the folders of samples: on starting, and when the project (whose
// samples folder is listed) changes
export const getBrowserRootSamples = () => async (dispatch: Dispatch) => {
  showRoots(dispatch, await invoke<SamplePath[]>('sample_paths'))
}

// add a folder of samples (File Paths' Add Folder…). rejects with why it
// can't be, e.g. it's already listed
export const addSamplePath = (path: string) => async (dispatch: Dispatch) => {
  showRoots(dispatch, await invoke<SamplePath[]>('add_sample_path', { path }))
}

// take a folder out of File Paths. nothing on disk changes
export const removeSamplePath = (path: string) => async (dispatch: Dispatch) => {
  showRoots(dispatch, await invoke<SamplePath[]>('remove_sample_path', { path }))
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
export const selectSamplePaths = (state: RootState) => state.env.samplePaths
// end sidebar samples methods

export default envSlice.reducer
