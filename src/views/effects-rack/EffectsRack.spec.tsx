import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../state/slices/playlistSlice'
import projectReducer from '../../state/slices/projectSlice'
import mixerReducer, { setName } from '../../state/slices/mixerSlice'
import windowReducer, { openTab } from '../../state/slices/windowSlice'
import effectsReducer, { expandSlot, setRacks, showRack } from '../../state/slices/effectsSlice'
import { EffectSlot, Racks } from '../../types/effects'
import { View } from '../../types/ui'
import EffectsRack from './EffectsRack'
import { gapAt, rackDrop, rackGap } from './useRackDrop'
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
let dockFails: string | null

const makeStore = () => configureStore({
  reducer: {
    playlist: playlistReducer,
    project: projectReducer,
    mixer: mixerReducer,
    window: windowReducer,
    effects: effectsReducer,
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

// the placeholder sits further left once it has the interface's size (it's
// centred), so the engine has to be told it moved
const rects = () => {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function rect(this: HTMLElement) {
    const placeholder = this.dataset.testid === 'docked-placeholder'
    const left = placeholder ? (this.style.width === '0px' ? 400 : 250) : 100
    const top = placeholder ? 200 : 60
    return { left, top, x: left, y: top, width: 0, height: 0, right: left, bottom: top, toJSON: () => ({}) } as DOMRect
  })
}

beforeEach(() => {
  vi.resetAllMocks()
  vi.restoreAllMocks()
  racks = [
    { track: 0, slots: [slot(1), slot(2, { hasEditor: false }), slot(3, { missing: true }), slot(4, { name: 'Verb' })] },
    { track: null, slots: [slot(9, { name: 'Limiter' })] },
  ]
  dockFails = null
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 'get_effects') return racks
    if(command === 'dock_effect') {
      if(dockFails) throw dockFails
      return { width: 300, height: 200, resizable: false }
    }
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
    expect(screen.queryByRole('button', { name: 'Show Effect 3 here' })).toBeNull()
    expect((screen.getByRole('button', { name: 'Remove Effect 3' }) as HTMLButtonElement).disabled).toBe(false)
    // without an interface of its own: no expand or pop out
    expect(screen.queryByRole('button', { name: 'Show Effect 2 here' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Open Effect 2 in a window' })).toBeNull()
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

describe('an expanded effect', () => {
  it('docks its interface where the placeholder is, and follows it', async () => {
    rects()
    setup()

    fireEvent.click(screen.getByRole('button', { name: 'Show Effect 1 here' }))

    expect(calls('dock_effect')).toEqual([[
      'dock_effect',
      { slotId: 1, clip: { x: 100, y: 60, width: 0, height: 0 }, x: 400, y: 200 },
    ]])
    // the placeholder takes the interface's size
    const placeholder = await screen.findByTestId('docked-placeholder')
    await waitFor(() => expect(placeholder.style.width).toBe('300px'))
    expect(placeholder.style.height).toBe('200px')
    // and the engine is told where it went
    await waitFor(() => expect(calls('move_docked_effect')).toEqual([[
      'move_docked_effect',
      { clip: { x: 100, y: 60, width: 0, height: 0 }, x: 250, y: 200 },
    ]]))

    // stepping only counts effects with an interface to show
    expect(screen.getByLabelText('Effect shown').textContent).toContain('1 / 2')
  })

  it('steps to the next effect, undocking the last', async () => {
    setup()
    fireEvent.click(screen.getByRole('button', { name: 'Show Effect 1 here' }))
    fireEvent.click(screen.getByRole('button', { name: 'Next effect' }))

    expect(screen.getByLabelText('Effect shown').textContent).toContain('2 / 2')
    expect((screen.getByRole('button', { name: 'Next effect' }) as HTMLButtonElement).disabled).toBe(true)
    const sent = mockInvoke.mock.calls.map(([name, args]) => (name === 'dock_effect' ? `dock ${args.slotId}` : name))
      .filter(name => name !== 'get_effects')
    expect(sent).toEqual(['dock 1', 'undock_effect', 'dock 4'])
  })

  it('collapses back to the stack, undocking', () => {
    setup()
    fireEvent.click(screen.getByRole('button', { name: 'Show Effect 1 here' }))
    fireEvent.click(screen.getByRole('button', { name: 'Back to the list' }))

    expect(calls('undock_effect')).toHaveLength(1)
    expect(screen.getAllByRole('listitem')).toHaveLength(4)
  })

  it('pops out into a window, and the tab goes back to the stack', async () => {
    setup()
    fireEvent.click(screen.getByRole('button', { name: 'Show Effect 1 here' }))
    fireEvent.click(screen.getByRole('button', { name: 'Open Effect 1 in a window' }))

    await waitFor(() => expect(calls('open_effect_window')).toEqual([['open_effect_window', { slotId: 1 }]]))
    expect(calls('undock_effect')).toHaveLength(1)
    expect(screen.getAllByRole('listitem')).toHaveLength(4)
  })

  it('another channel picked: undocked, and that channel’s stack', () => {
    setup()
    fireEvent.click(screen.getByRole('button', { name: 'Show Effect 1 here' }))
    fireEvent.change(screen.getByRole('combobox', { name: 'Channel' }), { target: { value: 'master' } })

    expect(calls('undock_effect')).toHaveLength(1)
    expect(screen.getAllByRole('listitem').map(row => row.getAttribute('aria-label'))).toEqual(['Limiter'])
  })

  it('removed (or undone away): undocked, back to the stack', () => {
    const store = setup()
    fireEvent.click(screen.getByRole('button', { name: 'Show Effect 1 here' }))
    act(() => { store.dispatch(setRacks([{ track: 0, slots: [slot(4)] }])) })

    expect(calls('undock_effect')).toHaveLength(1)
    expect(screen.getAllByRole('listitem')).toHaveLength(1)
  })

  it('only docked while the tab is showing', () => {
    const store = setup()
    act(() => { store.dispatch(expandSlot(1)) })
    expect(calls('dock_effect')).toHaveLength(1)

    // another tab selected in the pane
    act(() => { store.dispatch(openTab(View.Mixer)) })
    expect(calls('undock_effect')).toHaveLength(1)

    act(() => { store.dispatch(openTab(View.EffectsRack)) })
    expect(calls('dock_effect')).toHaveLength(2)
  })

  it('shows why it couldn’t be docked, and offers a window', async () => {
    dockFails = 'not supported on this system'
    setup()
    fireEvent.click(screen.getByRole('button', { name: 'Show Effect 1 here' }))

    expect(await screen.findByText(/not supported on this system/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Open in a window' }))
    await waitFor(() => expect(calls('open_effect_window')).toEqual([['open_effect_window', { slotId: 1 }]]))
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

describe('a track’s instrument', () => {
  const synth = (id: number, change: Partial<EffectSlot> = {}) =>
    slot(id, { name: `Synth ${id}`, kind: 'instrument', ...change })
  const chains = (): Racks => [
    { track: 0, slots: [synth(1), slot(2), slot(3)] },
    { track: 1, slots: [slot(5)] },
    { track: 2, slots: [synth(6)] },
    { track: null, slots: [slot(9)] },
  ]
  const instrument = { kind: 'effect' as const, pluginId: 'vst3:synth', label: 'Synth', instrument: true }
  const effect = { kind: 'effect' as const, pluginId: 'vst3:x', label: 'X' }

  it('is listed first, marked as the instrument, with the clips joining after it', () => {
    racks = chains()
    setup()

    const rows = screen.getAllByRole('listitem')
    expect(rows.map(row => row.getAttribute('aria-label'))).toEqual(['Synth 1', 'Effect 2', 'Effect 3'])
    expect(rows[0].getAttribute('aria-roledescription')).toBe('instrument')
    expect(rows[0].textContent).toContain('Instrument')
    expect(rows[1].textContent).not.toContain('Instrument')
    expect(screen.getByText('Notes ↓')).toBeTruthy()
    expect(screen.getByText('+ clips ↓')).toBeTruthy()
    expect(screen.getByText('Instrument, 2 effects')).toBeTruthy()
    // it can be switched off, shown and removed like an effect
    expect(screen.getByRole('button', { name: 'Synth 1 on' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Show Synth 1 here' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Remove Synth 1' })).toBeTruthy()
  })

  it('has no gap above it for an effect, and an instrument only goes first', () => {
    const chain = chains()
    expect(rackGap(chain, 0, effect, 0)).toBe(1)
    expect(rackGap(chain, 0, effect, 2)).toBe(2)
    expect(rackGap(chain, 1, effect, 0)).toBe(0)
    expect(rackGap(chain, 1, instrument, 1)).toBe(0)

    expect(rackDrop(chain, 0, effect, 0)).toEqual({ kind: 'add', index: 1 })
    // an effect row dragged to the top stays below the instrument
    expect(rackDrop(chain, 0, { kind: 'slot', slotId: 3, label: '' }, 0)).toEqual({ kind: 'move', index: 1 })
    expect(rackDrop(chain, 0, { kind: 'slot', slotId: 2, label: '' }, 0)).toBeNull()
  })

  it('from the sidebar, is added as a track’s instrument wherever it’s dropped, never on the master', () => {
    const chain = chains()
    expect(rackDrop(chain, 1, instrument, 1)).toEqual({ kind: 'add', index: null })
    expect(rackDrop(chain, 0, instrument, 3)).toEqual({ kind: 'add', index: null })
    expect(rackDrop(chain, 'master', instrument, 0)).toBeNull()
  })

  it('moves to another track that has none, never within its chain or to the master', () => {
    const chain = chains()
    const row = { kind: 'slot' as const, slotId: 1, label: '' }
    expect(rackDrop(chain, 0, row, 2)).toBeNull()
    expect(rackDrop(chain, 1, row, 1)).toEqual({ kind: 'move', index: 0 })
    expect(rackDrop(chain, 2, row, 0)).toBeNull()
    expect(rackDrop(chain, 'master', row, 0)).toBeNull()
  })

  it('dropped on a mixer strip: a track takes it, the master refuses it', () => {
    const chain = chains()
    expect(channelDrop(chain, 1, instrument)).toEqual({ kind: 'add' })
    expect(channelDrop(chain, 0, instrument)).toEqual({ kind: 'add' })
    expect(channelDrop(chain, 'master', instrument)).toBeNull()
    expect(channelDrop(chain, 'master', effect)).toEqual({ kind: 'add' })

    const row = { kind: 'slot' as const, slotId: 1, label: '' }
    expect(channelDrop(chain, 3, row)).toEqual({ kind: 'move', index: 0 })
    expect(channelDrop(chain, 0, row)).toBeNull()
    expect(channelDrop(chain, 2, row)).toBeNull()
    expect(channelDrop(chain, 'master', row)).toBeNull()
    // an effect still goes to the end
    expect(channelDrop(chain, 2, { kind: 'slot', slotId: 2, label: '' })).toEqual({ kind: 'move', index: 1 })
  })

  it('treats a slot without a kind (an older engine) as an effect', () => {
    const chain: Racks = [{ track: 0, slots: [slot(1), slot(2)] }]
    expect(rackGap(chain, 0, effect, 0)).toBe(0)
  })
})

describe('channelChoices', () => {
  it('lists the mixer’s tracks, any further one with effects, and the master', () => {
    expect(channelChoices(2, {}, [], 0).map(c => c.value)).toEqual(['0', '1', 'master'])
    expect(channelChoices(2, { 0: 'Kick' }, [{ track: 3, slots: [] }], 'master').map(c => c.label))
      .toEqual(['Kick', 'Track 2', 'Track 3', 'Track 4', 'Master'])
  })
})
