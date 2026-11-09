import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { RootState } from 'state/store'
import { WindowPane, SplitDirection, View, viewLabels } from '../../types/ui'
import * as paneTree from '../paneTree'

export interface WindowState {
  playlist: boolean
  sidebar: boolean
  devicePreferences: boolean
  windowPane: WindowPane
  // the pane last clicked in, where opened tabs go
  focusedPaneId: string
  // whether the Transport tab shows the cassette, or just its keys
  cassetteVisible: boolean
  // whether the cassette shows its tape pulled out across the heads
  tapePathVisible: boolean
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
      tabs: [
        { label: viewLabels[View.Playlist], component: View.Playlist },
        { label: viewLabels[View.SamplePreview], component: View.SamplePreview },
        { label: viewLabels[View.Mixer], component: View.Mixer },
        { label: viewLabels[View.Transport], component: View.Transport },
        { label: viewLabels[View.Sequencer], component: View.Sequencer },
      ],
      activeView: View.Playlist,
    },
  },
  focusedPaneId: 'root',
  cassetteVisible: true,
  tapePathVisible: false,
} as WindowState

// ids for panes created by splitting
let nextPaneId = 1
const newPaneId = () => `pane-${nextPaneId++}`

// keep focus on a pane that still exists after the layout changes
const fixFocus = (state: WindowState) => {
  const { root } = state.windowPane

  if(!paneTree.findPane(root, state.focusedPaneId)) {
    state.focusedPaneId = paneTree.leaves(root)[0].id
  }
}

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
    setCassetteVisible: (state, action: PayloadAction<boolean>) => {
      state.cassetteVisible = action.payload
    },
    setTapePathVisible: (state, action: PayloadAction<boolean>) => {
      state.tapePathVisible = action.payload
    },
    focusPane: (state, action: PayloadAction<string>) => {
      state.focusedPaneId = action.payload
    },
    selectTab: (state, action: PayloadAction<{ paneId: string, view: View }>) => {
      const { paneId, view } = action.payload
      state.windowPane.root = paneTree.selectTab(state.windowPane.root, paneId, view)
      state.focusedPaneId = paneId
    },
    closeTab: (state, action: PayloadAction<View>) => {
      state.windowPane.root = paneTree.closeTab(state.windowPane.root, action.payload)
      fixFocus(state)
    },
    moveTab: (
      state,
      action: PayloadAction<{ view: View, paneId: string, index?: number }>,
    ) => {
      const { view, paneId, index } = action.payload
      state.windowPane.root = paneTree.moveTab(state.windowPane.root, view, paneId, index)
      state.focusedPaneId = paneId
      fixFocus(state)
    },
    splitPane: {
      reducer: (
        state,
        action: PayloadAction<{
          paneId: string
          view: View
          direction: SplitDirection
          newPaneId: string
          splitId: string
          before: boolean
        }>,
      ) => {
        const { paneId, view, direction, newPaneId: id, splitId, before } = action.payload
        const root = paneTree.splitPane(
          state.windowPane.root, paneId, view, direction, id, splitId, before)

        if(root === state.windowPane.root) return

        state.windowPane.root = root
        state.focusedPaneId = id
      },
      // the new pane's ids are made here, so the reducer stays pure
      // - before: put the new pane left of (or above) the split one
      prepare: (paneId: string, view: View, direction: SplitDirection, before = false) => ({
        payload: {
          paneId,
          view,
          direction,
          newPaneId: newPaneId(),
          splitId: newPaneId(),
          before,
        },
      }),
    },
    // open a view in the focused pane, or select it where it's already open
    openTab: (state, action: PayloadAction<View>) => {
      const view = action.payload
      const { root } = state.windowPane
      const open = paneTree.paneWithView(root, view)

      if(open) {
        state.windowPane.root = paneTree.selectTab(root, open.id, view)
        state.focusedPaneId = open.id
        return
      }

      fixFocus(state)
      state.windowPane.root = paneTree.addTab(
        root,
        state.focusedPaneId,
        { label: viewLabels[view], component: view },
      )
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
// end show device preferences methods

// start window layout methods
export const {
  focusPane,
  setCassetteVisible,
  setTapePathVisible,
  selectTab,
  closeTab,
  moveTab,
  splitPane,
  openTab,
} = windowSlice.actions

export const selectWindowPane = (state: RootState) => state.window.windowPane.root

export const selectFocusedPaneId = (state: RootState) => state.window.focusedPaneId

// the view showing in the focused pane, e.g. for keys that act on it
export const selectFocusedView = (state: RootState): View | undefined =>
  paneTree.findPane(state.window.windowPane.root, state.window.focusedPaneId)?.activeView

export const selectCassetteVisible = (state: RootState) => state.window.cassetteVisible

export const selectTapePathVisible = (state: RootState) => state.window.tapePathVisible
// end window layout methods

// export root reducer for this slice
export default windowSlice.reducer
