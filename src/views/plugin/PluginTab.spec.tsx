import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../state/slices/playlistSlice'
import projectReducer from '../../state/slices/projectSlice'
import mixerReducer, { setName } from '../../state/slices/mixerSlice'
import windowReducer, {
  closeTab, openPluginTab, openTab, selectFocusedView, selectTabLabel,
} from '../../state/slices/windowSlice'
import effectsReducer, { setRacks } from '../../state/slices/effectsSlice'
import sequencerReducer, { setSequencer } from '../../state/slices/sequencerSlice'
import { paneWithTab } from '../../state/paneTree'
import { EditorInfo, EffectSlot, Racks } from '../../types/effects'
import { Row, SequencerState } from '../../types/sequencer'
import { sequencerOf, synthRow } from '../../types/sequencer.fixture'
import { tabKey, View } from '../../types/ui'
import PluginTab from './PluginTab'
import { usePluginTabsFollowRacks } from './usePluginTabsFollowRacks'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

// whether a pane's tab is being dragged
let dragging = false
vi.mock('../../components/panes/dragTab', () => ({ useDraggingTab: () => dragging }))

const mockInvoke = invoke as Mock

const slot = (id: number, change: Partial<EffectSlot> = {}): EffectSlot => ({
  id,
  pluginId: `vst3:${id}`,
  name: `Effect ${id}`,
  vendor: 'Acme',
  bypassed: false,
  missing: false,
  hasEditor: true,
  ...change,
})

let racks: Racks
let editor: EditorInfo
let dockFails: string | null
// the scroll area's size, padding and all
let area: { width: number, height: number }

const makeStore = () => configureStore({
  reducer: {
    playlist: playlistReducer,
    project: projectReducer,
    mixer: mixerReducer,
    window: windowReducer,
    effects: effectsReducer,
    sequencer: sequencerReducer,
  },
})

const Harness = ({ slotId }: { slotId: number }) => {
  usePluginTabsFollowRacks()
  return <PluginTab slotId={slotId} />
}

// slot 1's tab, open and selected
const setup = (slotId = 1) => {
  const store = makeStore()
  store.dispatch(setRacks(racks))
  store.dispatch(openPluginTab({ slotId }))
  const view = render(<Provider store={store}><Harness slotId={slotId} /></Provider>)
  return { store, ...view }
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command)
const sent = () => mockInvoke.mock.calls.map(([name]) => name)

// ResizeObserver isn't in jsdom: keep the callbacks, to call as if resized
let observers: Array<() => void>
class FakeResizeObserver {
  constructor(callback: () => void) { observers.push(callback) }
  observe() {}
  disconnect() {}
}
const resized = async () => {
  act(() => { observers.forEach(callback => callback()) })
  await new Promise(resolve => requestAnimationFrame(resolve))
}

beforeEach(() => {
  vi.resetAllMocks()
  vi.restoreAllMocks()
  racks = [
    { track: 0, slots: [slot(1), slot(2, { hasEditor: false })] },
    { track: null, slots: [slot(9, { name: 'Limiter' })] },
  ]
  editor = { width: 300, height: 200, resizable: false }
  dockFails = null
  area = { width: 616, height: 416 }
  dragging = false
  observers = []
  vi.stubGlobal('ResizeObserver', FakeResizeObserver)

  mockInvoke.mockImplementation(async (command: string, args: { width: number, height: number }) => {
    if(command === 'dock_effect') {
      if(dockFails) throw dockFails
      return editor
    }
    if(command === 'resize_docked_effect') return { ...editor, width: args.width, height: args.height }
    if(command === 'set_effect_bypassed') return racks
    return undefined
  })

  // the placeholder sits further left once it has the interface's size
  // (it's centerd), so the engine has to be told it moved
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function rect(this: HTMLElement) {
    const placeholder = this.dataset.testid === 'docked-placeholder'
    const left = placeholder ? (this.style.width === '0px' ? 400 : 250) : 100
    const top = placeholder ? 200 : 60
    return { left, top, x: left, y: top, width: 0, height: 0, right: left, bottom: top, toJSON: () => ({}) } as DOMRect
  })
  vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockImplementation(function width(this: HTMLElement) {
    return this.dataset.dockedEffect ? area.width : 0
  })
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function height(this: HTMLElement) {
    return this.dataset.dockedEffect ? area.height : 0
  })
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('a plugin’s tab', () => {
  it('docks the interface where the placeholder is, with the room there is, and follows it', async () => {
    setup()

    expect(calls('dock_effect')).toEqual([[
      'dock_effect',
      { slotId: 1, clip: { x: 100, y: 60, width: 0, height: 0 }, x: 400, y: 200, fill: { width: 600, height: 400 } },
    ]])
    // the placeholder takes the interface's size
    const placeholder = await screen.findByTestId('docked-placeholder')
    await waitFor(() => expect(placeholder.style.width).toBe('300px'))
    expect(placeholder.style.height).toBe('200px')
    // and the engine is told where it went
    await waitFor(() => expect(calls('move_docked_effect')).toEqual([[
      'move_docked_effect',
      { slotId: 1, clip: { x: 100, y: 60, width: 0, height: 0 }, x: 250, y: 200 },
    ]]))
  })

  it('gives a resizable interface the new room as the pane resizes, once each time it changes', async () => {
    editor = { width: 600, height: 400, resizable: true }
    setup()
    await waitFor(() => expect(screen.getByTestId('docked-placeholder').style.width).toBe('600px'))
    await resized()
    expect(calls('resize_docked_effect')).toEqual([])

    area = { width: 816, height: 516 }
    await resized()
    await resized()
    expect(calls('resize_docked_effect')).toEqual([['resize_docked_effect', { slotId: 1, width: 800, height: 500 }]])
    await waitFor(() => expect(screen.getByTestId('docked-placeholder').style.width).toBe('800px'))
  })

  it('leaves a fixed-size interface as it is when the pane resizes', async () => {
    setup()
    await waitFor(() => expect(screen.getByTestId('docked-placeholder').style.width).toBe('300px'))

    area = { width: 816, height: 516 }
    await resized()
    expect(calls('resize_docked_effect')).toEqual([])
  })

  it('is only docked while its tab is showing', () => {
    const { store } = setup()
    expect(calls('dock_effect')).toHaveLength(1)

    act(() => { store.dispatch(openTab(View.Mixer)) })
    expect(calls('undock_effect')).toEqual([['undock_effect', { slotId: 1 }]])

    act(() => { store.dispatch(openPluginTab({ slotId: 1 })) })
    expect(calls('dock_effect')).toHaveLength(2)

    act(() => { store.dispatch(closeTab(tabKey(View.Plugin, 1))) })
    expect(calls('undock_effect')).toHaveLength(2)
  })

  it('gets out of the way while a tab is dragged', () => {
    const { rerender, store } = setup()

    dragging = true
    rerender(<Provider store={store}><Harness slotId={1} /></Provider>)
    expect(sent().filter(name => name !== 'move_docked_effect')).toEqual(['dock_effect', 'undock_effect'])

    dragging = false
    rerender(<Provider store={store}><Harness slotId={1} /></Provider>)
    expect(calls('dock_effect')).toHaveLength(2)
  })

  it('has On, its channel, and pop out in its bar', async () => {
    const { store } = setup()
    act(() => { store.dispatch(setName({ trackNumber: 0, name: 'Drums' })) })

    fireEvent.click(screen.getByRole('button', { name: 'Effect 1 on' }))
    await waitFor(() => expect(calls('set_effect_bypassed')).toEqual([['set_effect_bypassed', { slotId: 1, bypassed: true }]]))

    // the channel's name opens the rack there, with its row pointed out
    act(() => { store.dispatch(openTab(View.Mixer)) })
    fireEvent.click(screen.getByRole('button', { name: 'Drums' }))
    const root = store.getState().window.windowPane.root
    expect(paneWithTab(root, tabKey(View.EffectsRack))?.activeTab).toBe(tabKey(View.EffectsRack))
    expect(store.getState().effects.rackChannel).toBe(0)
    expect(store.getState().effects.highlightSlotId).toBe(1)

    expect(screen.queryByRole('combobox')).toBeNull()
    expect(screen.queryByRole('button', { name: /next|previous/i })).toBeNull()
  })

  it('pops out into a window: its tab closes, undocking it', async () => {
    const { store } = setup()
    fireEvent.click(screen.getByRole('button', { name: 'Open Effect 1 in a window' }))

    await waitFor(() => expect(calls('open_effect_window')).toEqual([['open_effect_window', { slotId: 1 }]]))
    expect(paneWithTab(store.getState().window.windowPane.root, tabKey(View.Plugin, 1))).toBeUndefined()
    // (the engine undocks it first anyway, and a slot that isn't docked
    // ignores undock_effect)
    await waitFor(() => expect(calls('undock_effect')).toEqual([['undock_effect', { slotId: 1 }]]))
  })

  it('shows why it couldn’t be docked, and offers a window', async () => {
    dockFails = 'not supported on this system'
    setup()

    expect(await screen.findByText(/not supported on this system/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Open in a window' }))
    await waitFor(() => expect(calls('open_effect_window')).toEqual([['open_effect_window', { slotId: 1 }]]))
  })

  it('closes when its slot goes, or can no longer show anything', () => {
    const { store } = setup()
    const open = () => !!paneWithTab(store.getState().window.windowPane.root, tabKey(View.Plugin, 1))
    expect(open()).toBe(true)

    act(() => { store.dispatch(setRacks([{ track: 0, slots: [slot(1, { missing: true })] }])) })
    expect(open()).toBe(false)
    expect(calls('undock_effect')).toEqual([['undock_effect', { slotId: 1 }]])

    act(() => { store.dispatch(openPluginTab({ slotId: 9 })) })
    act(() => { store.dispatch(setRacks([])) })
    expect(paneWithTab(store.getState().window.windowPane.root, tabKey(View.Plugin, 9))).toBeUndefined()
  })
})

describe('a synth row’s tab', () => {
  // rows 2 (Pad) and 3, their plugins in slots 20 and 21: a tab goes by slot
  const pad = (muted = false) => synthRow(2, { slot: 20, name: 'Pad', muted })
  const other = (missing = false) => synthRow(3, { slot: 21, missing })
  const sequencer = (rows: Row[]): SequencerState => sequencerOf({ rows })

  const setupSynth = () => {
    const store = makeStore()
    store.dispatch(setRacks(racks))
    store.dispatch(setSequencer(sequencer([pad(), other()])))
    store.dispatch(openPluginTab({ slotId: 20 }))
    render(<Provider store={store}><Harness slotId={20} /></Provider>)
    return store
  }

  beforeEach(() => {
    const docked = mockInvoke.getMockImplementation()
    mockInvoke.mockImplementation(async (command: string, args: { rowId: number, muted: boolean }) => {
      if(command === 'set_row_muted') return { row: pad(args.muted) }
      return docked?.(command, args)
    })
  })

  it('is named after the synth, and docks its interface by its slot', async () => {
    const store = setupSynth()
    expect(selectTabLabel({ component: View.Plugin, slotId: 20 })(store.getState() as never)).toBe('Pad')
    // (not by the row's id)
    expect(selectTabLabel({ component: View.Plugin, slotId: 2 })(store.getState() as never)).not.toBe('Pad')
    await waitFor(() => expect(calls('dock_effect')[0]?.[1]).toMatchObject({ slotId: 20 }))
  })

  it('has mute, a key to the Pattern Sequencer, and pop out in its bar', async () => {
    const store = setupSynth()
    expect(screen.queryByRole('button', { name: 'Pad on' })).toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Mute Pad' }))
    await waitFor(() => expect(calls('set_row_muted')).toEqual([['set_row_muted', { rowId: 2, muted: true, gesture: null }]]))
    await waitFor(() => expect(store.getState().sequencer.rows[0].muted).toBe(true))

    fireEvent.click(screen.getByRole('button', { name: 'Pattern Sequencer' }))
    expect(selectFocusedView(store.getState() as never)).toBe(View.PatternSequencer)

    fireEvent.click(screen.getByRole('button', { name: 'Open Pad in a window' }))
    await waitFor(() => expect(calls('open_effect_window')).toEqual([['open_effect_window', { slotId: 20 }]]))
    expect(paneWithTab(store.getState().window.windowPane.root, tabKey(View.Plugin, 20))).toBeUndefined()
  })

  it('closes when the synth row goes, or its synth isn’t installed', () => {
    const store = setupSynth()
    const open = (id: number) => !!paneWithTab(store.getState().window.windowPane.root, tabKey(View.Plugin, id))
    act(() => { store.dispatch(openPluginTab({ slotId: 21 })) })
    expect(open(20) && open(21)).toBe(true)

    act(() => { store.dispatch(setSequencer(sequencer([other(true)]))) })
    expect(open(20)).toBe(false)
    expect(open(21)).toBe(false)
    // effects' tabs are left alone
    act(() => { store.dispatch(openPluginTab({ slotId: 1 })) })
    expect(open(1)).toBe(true)
  })
})
