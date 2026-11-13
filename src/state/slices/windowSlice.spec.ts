import reducer, {
  closeFocusedTab,
  closeTab,
  focusPane,
  keepPluginTabs,
  openPluginTab,
  openTab,
  pressSidebarPage,
  selectFocusedView,
  selectTab,
  selectTabShowing,
  setSidebar,
  splitPane,
  WindowState,
} from './windowSlice'
import { findPane, leaves } from '../paneTree'
import { SidebarPage, tabKey, View } from '../../types/ui'
import type { RootState } from '../store'

const initial = (): WindowState => reducer(undefined, { type: '@@init' })

describe('pressSidebarPage', () => {
  it('closes the sidebar when the page showing is pressed', () => {
    const state = reducer(initial(), pressSidebarPage(SidebarPage.Samples))
    expect(state.sidebar).toBe(false)
    expect(state.sidebarPage).toBe(SidebarPage.Samples)
  })

  it('switches page while open', () => {
    const state = reducer(initial(), pressSidebarPage(SidebarPage.Patterns))
    expect(state.sidebar).toBe(true)
    expect(state.sidebarPage).toBe(SidebarPage.Patterns)
  })

  it('opens on the page pressed while closed, not the one it was on', () => {
    const closed = reducer(initial(), setSidebar(false))
    const state = reducer(closed, pressSidebarPage(SidebarPage.Recordings))
    expect(state.sidebar).toBe(true)
    expect(state.sidebarPage).toBe(SidebarPage.Recordings)
  })

  it('opens again on the same page after closing it', () => {
    const closed = reducer(initial(), pressSidebarPage(SidebarPage.Samples))
    const state = reducer(closed, pressSidebarPage(SidebarPage.Samples))
    expect(state.sidebar).toBe(true)
    expect(state.sidebarPage).toBe(SidebarPage.Samples)
  })
})

describe('openTab', () => {
  it('opens the Mixdown tab, once', () => {
    const once = reducer(initial(), openTab(View.Mixdown))
    const twice = reducer(once, openTab(View.Mixdown))
    ;[once, twice].forEach(state => {
      const tabs = leaves(state.windowPane.root).flatMap(pane => pane.tabs ?? [])
      expect(tabs.filter(tab => tab.component === View.Mixdown)).toEqual([{ component: View.Mixdown }])
    })
  })
})

describe('closeFocusedTab', () => {
  it("closes the focused pane's selected tab, and selects its neighbour", () => {
    const opened = reducer(reducer(initial(), openTab(View.Mixer)), openTab(View.Mixdown))
    const state = reducer(opened, closeFocusedTab())
    const tabs = leaves(state.windowPane.root).flatMap(pane => pane.tabs ?? [])
    expect(tabs.some(tab => tab.component === View.Mixdown)).toBe(false)
    expect(tabs.some(tab => tab.component === View.Mixer)).toBe(true)
  })
})

describe('Preferences tab', () => {
  it('opens once', () => {
    const once = reducer(initial(), openTab(View.Preferences))
    const twice = reducer(once, openTab(View.Preferences))
    const tabs = leaves(twice.windowPane.root).flatMap(pane => pane.tabs ?? [])
    expect(tabs.filter(tab => tab.component === View.Preferences)).toEqual([{ component: View.Preferences }])
  })
})

describe('plugin tabs', () => {
  const PLUGIN_4 = tabKey(View.Plugin, 4)
  const PLUGIN_7 = tabKey(View.Plugin, 7)
  const root = (state: WindowState) => state.windowPane.root
  const as = (state: WindowState) => ({ window: state }) as unknown as RootState
  const pluginTabs = (state: WindowState) =>
    leaves(root(state)).flatMap(pane => pane.tabs).filter(tab => tab.component === View.Plugin)

  // the rack in a pane of its own, beside the first
  const withRack = () => {
    const opened = reducer(initial(), openTab(View.EffectsRack))
    return reducer(opened, splitPane('root', tabKey(View.EffectsRack), 'vertical'))
  }

  it('opens one per slot, in the pane asked for, or selects it where it’s open', () => {
    let state = withRack()
    const rackPane = state.focusedPaneId
    state = reducer(state, focusPane('root'))

    state = reducer(state, openPluginTab({ slotId: 4, paneId: rackPane }))
    state = reducer(state, openPluginTab({ slotId: 7, paneId: rackPane }))
    expect(findPane(root(state), rackPane)?.tabs.map(t => t.slotId)).toEqual([undefined, 4, 7])
    expect(state.focusedPaneId).toBe(rackPane)
    expect(selectTabShowing(PLUGIN_7)(as(state))).toBe(true)
    expect(selectTabShowing(PLUGIN_4)(as(state))).toBe(false)

    // opened again from elsewhere: selected, not opened twice
    state = reducer(state, focusPane('root'))
    state = reducer(state, openPluginTab({ slotId: 4 }))
    expect(pluginTabs(state)).toHaveLength(2)
    expect(selectTabShowing(PLUGIN_4)(as(state))).toBe(true)
    expect(state.focusedPaneId).toBe(rackPane)
  })

  it('goes in the focused pane when the one asked for is gone', () => {
    const state = reducer(initial(), openPluginTab({ slotId: 4, paneId: 'gone' }))
    expect(findPane(root(state), 'root')?.activeTab).toBe(PLUGIN_4)
  })

  it('closes and selects by key, leaving the other plugins’ tabs', () => {
    let state = reducer(initial(), openPluginTab({ slotId: 4 }))
    state = reducer(state, openPluginTab({ slotId: 7 }))
    state = reducer(state, selectTab({ paneId: 'root', key: PLUGIN_4 }))
    expect(selectTabShowing(PLUGIN_4)(as(state))).toBe(true)

    state = reducer(state, closeTab(PLUGIN_4))
    expect(pluginTabs(state)).toEqual([{ component: View.Plugin, slotId: 7 }])
  })

  it('closes the tabs of slots that went', () => {
    let state = withRack()
    const rackPane = state.focusedPaneId
    state = reducer(state, openPluginTab({ slotId: 4, paneId: 'root' }))
    state = reducer(state, openPluginTab({ slotId: 7, paneId: rackPane }))

    const same = reducer(state, keepPluginTabs([4, 7, 9]))
    expect(same).toBe(state)

    state = reducer(state, keepPluginTabs([7]))
    expect(pluginTabs(state)).toEqual([{ component: View.Plugin, slotId: 7 }])
    // ordinary tabs stay
    expect(leaves(root(state)).flatMap(pane => pane.tabs)).toHaveLength(6)
  })

  it('tells the focused pane’s view, a plugin’s tab or not', () => {
    let state = reducer(initial(), openPluginTab({ slotId: 4 }))
    expect(selectFocusedView(as(state))).toBe(View.Plugin)

    state = reducer(state, openTab(View.Mixer))
    expect(selectFocusedView(as(state))).toBe(View.Mixer)
    expect(selectTabShowing(tabKey(View.Mixer))(as(state))).toBe(true)
  })
})
