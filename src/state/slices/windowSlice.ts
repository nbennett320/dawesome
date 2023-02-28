import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { RootState } from 'state/store'
import { WindowPane, PaneTab, WindowNode, View } from '../../types/ui'

// util binary search
const findWindowNode = (
  root: WindowNode, 
  id: string,
  path = [],
): [WindowNode, string[]] | null => {
  if(root.id === id) return [root, path]
  if(root.child) return null

  if(root.left) {
    const res = findWindowNode(root.left, id)
    if(res) {
      const [left, p] = res
      return [left, p]
    }
  }

  if(root.right) {
    const res = findWindowNode(root.right, id)
    if(res) {
      const [right, p] = res
      return [right, p]
    }
  }

  return null
}

// const updateWindowNode = (
//   root: WindowNode,
//   current: WindowNode,
//   update: WindowNode,
//   path: string[],
//   id: string
// ):  => {
//   if(current.id === id) return root
//   if(current.child) return [null, null]

//   if(current.left) {
//     const left = updateWindowNode(
//       root,
//       current.left,
//       update,
//       [...path, current.id],
//       id
//     )

//     if(left) return left
//   }

//   if(root.right) {
//     const right = updateWindowNode(root, current.right, update, id)
//     if(right) return right
//   }

//   return null
// }

export interface WindowState {
  playlist: boolean
  sidebar: boolean
  devicePreferences: boolean
  windowPane: WindowPane
}

// todo: get initial state from backend,
// which parses dawesome.config file
const initialState = {
  playlist: true,
  sidebar: true,
  devicePreferences: false,
  windowPane: {
    id: 'dawesome',
    root: {
      id: 'root',
      child: View.Playlist,
      tabs: [
        {
          label: 'Playlist',
          index: 0,
          active: true,
          component: View.Playlist,
        },
        {
          label: 'Test',
          index: 1,
          active: false,
          component: View.Test
        }
      ]
    }
  }
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
    addTab: (
      state, 
      action: {
        type: string
        payload: {
          tab: PaneTab
          paneId: string
        }
      }
    ) => {
      const { windowPane } = state
      const res = findWindowNode(windowPane.root, action.payload.paneId)

      if(!res) return

      const [target, path] = res

      target.tabs.push(action.payload.tab)
      target.tabs.sort((a, b) => a.index - b.index)

      const updatedWindowPane = windowPane

      // add tab to window pane
      state.windowPane = action.payload

    }
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
// end show device preferences methods

// start window layout methods
export const addTab = (tab: PaneTab, paneId: string) => async (dispatch: Dispatch) => {

}

export const selectWindowPane = (state: RootState) => state.window.windowPane.root

// export root reducer for this slice
export default windowSlice.reducer
