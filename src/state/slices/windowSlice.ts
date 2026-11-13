import { createSlice, PayloadAction } from '@reduxjs/toolkit'
import { RootState } from 'state/store'
import {
  keyOf, PaneTab, SidebarPage, SplitDirection, TabKey, tabKey, View, viewLabels, WindowPane,
} from '../../types/ui'
import { pluginTabLabel } from '../../types/effects'
import { synthRowBySlot } from '../../types/sequencer'
import * as paneTree from '../paneTree'

export interface WindowState {
  playlist: boolean
  sidebar: boolean
  // the page the sidebar shows (kept while it's closed)
  sidebarPage: SidebarPage
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
  sidebarPage: SidebarPage.Samples,
  windowPane: {
    id: 'dawesome',
    root: {
      id: 'root',
      tabs: [
        { component: View.Playlist },
        // { component: View.SamplePreview },
        { component: View.Mixer },
        // { component: View.Transport },
        { component: View.PatternSequencer },
      ],
      activeTab: tabKey(View.Playlist),
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

// open a tab in a pane (the focused one when it's left out or gone), or
// select it where it's already open
const open = (state: WindowState, tab: PaneTab, paneId?: string) => {
  const { root } = state.windowPane
  const key = keyOf(tab)
  const already = paneTree.paneWithTab(root, key)

  if(already) {
    state.windowPane.root = paneTree.selectTab(root, already.id, key)
    state.focusedPaneId = already.id
    return
  }

  if(paneId && paneTree.findPane(root, paneId)) state.focusedPaneId = paneId
  fixFocus(state)
  state.windowPane.root = paneTree.addTab(root, state.focusedPaneId, tab)
}

export const windowSlice = createSlice({
  name: 'windowSlice',
  initialState,
  reducers: {
    setSidebar: (state, action: PayloadAction<boolean>) => {
      state.sidebar = action.payload
    },
    // a key on the sidebar's rail was pressed: pressing the page that's
    // showing closes the sidebar, any other opens it on that page
    pressSidebarPage: (state, action: PayloadAction<SidebarPage>) => {
      if(state.sidebar && state.sidebarPage === action.payload) {
        state.sidebar = false
        return
      }

      state.sidebar = true
      state.sidebarPage = action.payload
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
    selectTab: (state, action: PayloadAction<{ paneId: string, key: TabKey }>) => {
      const { paneId, key } = action.payload
      state.windowPane.root = paneTree.selectTab(state.windowPane.root, paneId, key)
      state.focusedPaneId = paneId
    },
    closeTab: (state, action: PayloadAction<TabKey>) => {
      state.windowPane.root = paneTree.closeTab(state.windowPane.root, action.payload)
      fixFocus(state)
    },
    // close the focused pane's selected tab (the command line's :q)
    closeFocusedTab: state => {
      const pane = paneTree.findPane(state.windowPane.root, state.focusedPaneId)
      if(!pane?.activeTab) return
      state.windowPane.root = paneTree.closeTab(state.windowPane.root, pane.activeTab)
      fixFocus(state)
    },
    // close the plugin tabs of slots that aren't listed (removed, or with
    // nothing to show)
    keepPluginTabs: (state, action: PayloadAction<number[]>) => {
      const keep = new Set(action.payload)
      const root = paneTree.closeTabsUnless(state.windowPane.root, tab =>
        tab.component !== View.Plugin || keep.has(tab.slotId as number))

      if(root === state.windowPane.root) return
      state.windowPane.root = root
      fixFocus(state)
    },
    moveTab: (
      state,
      action: PayloadAction<{ key: TabKey, paneId: string, index?: number }>,
    ) => {
      const { key, paneId, index } = action.payload
      state.windowPane.root = paneTree.moveTab(state.windowPane.root, key, paneId, index)
      state.focusedPaneId = paneId
      fixFocus(state)
    },
    splitPane: {
      reducer: (
        state,
        action: PayloadAction<{
          paneId: string
          key: TabKey
          direction: SplitDirection
          newPaneId: string
          splitId: string
          before: boolean
        }>,
      ) => {
        const { paneId, key, direction, newPaneId: id, splitId, before } = action.payload
        const root = paneTree.splitPane(
          state.windowPane.root, paneId, key, direction, id, splitId, before)

        if(root === state.windowPane.root) return

        state.windowPane.root = root
        state.focusedPaneId = id
      },
      // the new pane's ids are made here, so the reducer stays pure
      // - before: put the new pane left of (or above) the split one
      prepare: (paneId: string, key: TabKey, direction: SplitDirection, before = false) => ({
        payload: {
          paneId,
          key,
          direction,
          newPaneId: newPaneId(),
          splitId: newPaneId(),
          before,
        },
      }),
    },
    // open a view in the focused pane, or select it where it's already open
    openTab: (state, action: PayloadAction<View>) => {
      open(state, { component: action.payload })
    },
    // open a plugin's tab in `paneId` (or the focused pane), or select it
    // where it's already open
    openPluginTab: (state, action: PayloadAction<{ slotId: number, paneId?: string }>) => {
      const { slotId, paneId } = action.payload
      open(state, { component: View.Plugin, slotId }, paneId)
    },
  },
})

// start sidebar methods
export const { setSidebar, pressSidebarPage } = windowSlice.actions

export const selectSidebar = (state: RootState) => state.window.sidebar

export const selectSidebarPage = (state: RootState) => state.window.sidebarPage
// end sidebar methods

// start window layout methods
export const {
  focusPane,
  setCassetteVisible,
  setTapePathVisible,
  selectTab,
  closeTab,
  closeFocusedTab,
  moveTab,
  splitPane,
  openTab,
  openPluginTab,
  keepPluginTabs,
} = windowSlice.actions

export const selectWindowPane = (state: RootState) => state.window.windowPane.root

export const selectFocusedPaneId = (state: RootState) => state.window.focusedPaneId

// the view showing in the focused pane, e.g. for keys that act on it
export const selectFocusedView = (state: RootState): View | undefined => {
  const pane = paneTree.findPane(state.window.windowPane.root, state.window.focusedPaneId)
  return pane && paneTree.activeTabOf(pane)?.component
}

// whether a tab is open and the selected one of its pane, so it's on screen
// (the others stay mounted, but hidden)
export const selectTabShowing = (key: TabKey) => (state: RootState): boolean =>
  paneTree.paneWithTab(state.window.windowPane.root, key)?.activeTab === key

// a plugin's tab's name: an effect's is worked out from the racks each
// time, since it depends on the other effects (see pluginTabLabel). a
// synth row's is its synth's name, which is unique
const pluginLabel = (state: RootState, slotId: number): string =>
  pluginTabLabel(state.effects.racks, state.mixer.names, slotId)
  ?? synthRowBySlot(state.sequencer.rows, slotId)?.instrument.name
  ?? viewLabels[View.Plugin]

// a tab's name
export const selectTabLabel = (tab: PaneTab) => (state: RootState): string => (
  tab.component === View.Plugin ? pluginLabel(state, tab.slotId as number) : viewLabels[tab.component]
)

export const selectCassetteVisible =(state: RootState) => state.window.cassetteVisible

export const selectTapePathVisible = (state: RootState) => state.window.tapePathVisible
// end window layout methods

// export root reducer for this slice
export default windowSlice.reducer
