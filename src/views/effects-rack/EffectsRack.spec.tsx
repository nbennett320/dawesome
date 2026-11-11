import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../state/slices/playlistSlice'
import projectReducer from '../../state/slices/projectSlice'
import mixerReducer, { setName } from '../../state/slices/mixerSlice'
import windowReducer from '../../state/slices/windowSlice'
import effectsReducer, { setRacks, showRack } from '../../state/slices/effectsSlice'
import appearanceReducer, { defaultPreferences, setPreferences } from '../../state/slices/appearanceSlice'
import sequencerReducer from '../../state/slices/sequencerSlice'
import { paneWithTab } from '../../state/paneTree'
import { EffectSlot, Racks } from '../../types/effects'
import { tabKey, View } from '../../types/ui'
import EffectsRack from './EffectsRack'
import { gapAt, rackDrop } from './useRackDrop'
import { channelDrop } from '../mixer/useChannelDrop'
import { channelChoices } from './ChannelSelect'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

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

const makeStore = () => configureStore({
  reducer: {
    playlist: playlistReducer,
    project: projectReducer,
    mixer: mixerReducer,
    window: windowReducer,
    effects: effectsReducer,
    appearance: appearanceReducer,
    sequencer: sequencerReducer,
  },
})

const setup = () => {
  const store = makeStore()
  store.dispatch(setRacks(racks))
  store.dispatch(showRack(0))
  render(<Provider store={store}><EffectsRack /></Provider>)
  return store
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command)

beforeEach(() => {
  vi.resetAllMocks()
  vi.restoreAllMocks()
  racks = [
    { track: 0, slots: [slot(1), slot(2, { hasEditor: false }), slot(3, { missing: true }), slot(4, { name: 'Verb' })] },
    { track: null, slots: [slot(9, { name: 'Limiter' })] },
  ]
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 'get_effects') return racks
    if(command === 'set_effect_bypassed') return racks
    return undefined
  })
})

describe('the Effects Rack’s stack', () => {
  it('lists the channel’s effects in order, a missing one greyed and only removable', () => {
    setup()

    const rows = screen.getAllByRole('listitem')
    expect(rows.map(row => row.getAttribute('aria-label'))).toEqual(['Effect 1', 'Effect 2', 'Effect 3', 'Verb'])
    expect(screen.getByText('Not installed')).toBeTruthy()
    expect((screen.getByRole('button', { name: 'Effect 3 on' }) as HTMLButtonElement).disabled).toBe(true)
    expect(screen.queryByRole('button', { name: 'Open Effect 3' })).toBeNull()
    expect((screen.getByRole('button', { name: 'Remove Effect 3' }) as HTMLButtonElement).disabled).toBe(false)
    // without an interface of its own: no tab or pop out
    expect(screen.queryByRole('button', { name: 'Open Effect 2' })).toBeNull()
  })

  it('bypasses an effect', async () => {
    setup()
    fireEvent.click(screen.getByRole('button', { name: 'Effect 1 on' }))
    await waitFor(() => expect(calls('set_effect_bypassed')).toEqual([['set_effect_bypassed', { slotId: 1, bypassed: true }]]))
  })

  it('names the tracks as the mixer does, and shows another channel', () => {
    const store = setup()
    act(() => { store.dispatch(setName({ trackNumber: 1, name: 'Bass' })) })

    const select = screen.getByRole('combobox', { name: 'Channel' })
    expect(Array.from((select as HTMLSelectElement).options).map(o => o.text))
      .toEqual(['Track 1', 'Bass', 'Track 3', 'Track 4', 'Master'])

    fireEvent.change(select, { target: { value: 'master' } })
    expect(screen.getAllByRole('listitem').map(row => row.getAttribute('aria-label'))).toEqual(['Limiter'])
  })

  it('hints where to drop when a channel has none', () => {
    const store = setup()
    act(() => { store.dispatch(showRack(2)) })
    expect(screen.getByText('Drag an effect here')).toBeTruthy()
  })
})

describe('an effect’s own interface', () => {
  it('opens in a window of its own unless Preferences say a tab', async () => {
    const store = setup()
    // one key to open it, then the one to remove it
    const keys = screen.getAllByRole('listitem')[3].querySelectorAll('button')
    expect([...keys].slice(-2).map(key => key.getAttribute('aria-label'))).toEqual(['Open Verb', 'Remove Verb'])

    fireEvent.click(screen.getByRole('button', { name: 'Open Verb' }))
    await waitFor(() => expect(calls('open_effect_window')).toEqual([['open_effect_window', { slotId: 4 }]]))
    expect(paneWithTab(store.getState().window.windowPane.root, tabKey(View.Plugin, 4))).toBeUndefined()
  })

  it('opens in a tab of its own, in the rack’s pane, when Preferences say so', () => {
    const store = setup()
    act(() => { store.dispatch(setPreferences({ ...defaultPreferences, pluginView: 'tab' })) })
    fireEvent.click(screen.getByRole('button', { name: 'Open Verb' }))

    const root = store.getState().window.windowPane.root
    expect(paneWithTab(root, tabKey(View.Plugin, 4))?.id).toBe(paneWithTab(root, tabKey(View.EffectsRack))?.id)
    expect(calls('open_effect_window')).toEqual([])
    // the stack stays as it is
    expect(screen.getAllByRole('listitem')).toHaveLength(4)
  })
})

describe('dropping into the stack', () => {
  it('finds the gap between rows', () => {
    const mids = [10, 30, 50]
    expect(gapAt(mids, 0)).toBe(0)
    expect(gapAt(mids, 20)).toBe(1)
    expect(gapAt(mids, 49)).toBe(2)
    expect(gapAt(mids, 99)).toBe(3)
    expect(gapAt([], 5)).toBe(0)
  })

  it('inserts an effect at the gap, and moves a row to its place after the move', () => {
    const chain: Racks = [{ track: 0, slots: [slot(1), slot(2), slot(3)] }, { track: 1, slots: [slot(5)] }]
    expect(rackDrop(chain, 0, { kind: 'effect', pluginId: 'vst3:x', label: 'X' }, 2)).toEqual({ kind: 'add', index: 2 })
    // the first row dropped below the second
    expect(rackDrop(chain, 0, { kind: 'slot', slotId: 1, label: '' }, 2)).toEqual({ kind: 'move', index: 1 })
    expect(rackDrop(chain, 0, { kind: 'slot', slotId: 3, label: '' }, 0)).toEqual({ kind: 'move', index: 0 })
    // just where it is
    expect(rackDrop(chain, 0, { kind: 'slot', slotId: 2, label: '' }, 2)).toBeNull()
    // from another channel
    expect(rackDrop(chain, 0, { kind: 'slot', slotId: 5, label: '' }, 1)).toEqual({ kind: 'move', index: 1 })
    expect(rackDrop(chain, 0, { kind: 'slot', slotId: 99, label: '' }, 1)).toBeNull()
  })
})

describe('an instrument dropped on a mixer strip', () => {
  const instrument = { kind: 'instrument' as const, pluginId: 'vst3:synth', label: 'Synth' }
  const chains = (): Racks => [{ track: 0, slots: [slot(1), slot(2)] }, { track: null, slots: [slot(9)] }]

  it('starts a synth on a track, with or without effects, and the master refuses it', () => {
    const chain = chains()
    expect(channelDrop(chain, 0, instrument)).toEqual({ kind: 'synth' })
    expect(channelDrop(chain, 3, instrument)).toEqual({ kind: 'synth' })
    expect(channelDrop(chain, 'master', instrument)).toBeNull()
  })

  it('leaves effects as they were', () => {
    const chain = chains()
    expect(channelDrop(chain, 'master', { kind: 'effect', pluginId: 'vst3:x', label: 'X' })).toEqual({ kind: 'add' })
    expect(channelDrop(chain, 3, { kind: 'slot', slotId: 1, label: '' })).toEqual({ kind: 'move', index: 0 })
    expect(channelDrop(chain, 0, { kind: 'slot', slotId: 2, label: '' })).toBeNull()
  })
})

describe('channelChoices', () => {
  it('lists the mixer’s tracks, any further one with effects, and the master', () => {
    expect(channelChoices(2, {}, [], 0).map(c => c.value)).toEqual(['0', '1', 'master'])
    expect(channelChoices(2, { 0: 'Kick' }, [{ track: 3, slots: [] }], 'master').map(c => c.label))
      .toEqual(['Kick', 'Track 2', 'Track 3', 'Track 4', 'Master'])
  })
})
