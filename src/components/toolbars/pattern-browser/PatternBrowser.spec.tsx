import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../../state/slices/playlistSlice'
import projectReducer from '../../../state/slices/projectSlice'
import windowReducer, { selectFocusedView } from '../../../state/slices/windowSlice'
import sequencerReducer from '../../../state/slices/sequencerSlice'
import { SequencerState } from '../../../types/sequencer'
import { pattern, sampleRow, sequencerOf, steps, synthRow } from '../../../types/sequencer.fixture'
import { View } from '../../../types/ui'
import PatternBrowser from './PatternBrowser'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

// Beat and Fill (steps on the sample row), Verse and Chorus (notes on the
// synth row), and two folders
const sequencer: SequencerState = sequencerOf({
  rows: [sampleRow(0), synthRow(1)],
  patterns: [
    pattern(0, { name: 'Beat', notes: { 0: steps(0) } }),
    pattern(1, { name: 'Verse', notes: { 1: steps(0) } }),
    pattern(2, { name: 'Fill', folder: 10 }),
    pattern(3, { name: 'Chorus', notes: { 1: steps(4) }, folder: 11 }),
  ],
  folders: [{ id: 10, name: 'Fills' }, { id: 11, name: 'Choruses' }],
})

const setup = async () => {
  const store = configureStore({
    reducer: { playlist: playlistReducer, project: projectReducer, window: windowReducer, sequencer: sequencerReducer },
  })
  render(<Provider store={store}><PatternBrowser /></Provider>)
  await act(async () => {})
  return store
}

const rows = () => screen.getAllByRole('treeitem').map(row => row.textContent ?? '')

beforeEach(() => {
  mockInvoke.mockReset()
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 'get_sequencer') return sequencer
    if(command === 'add_pattern_folder') return { id: 12, sequencer }
    return undefined
  })
})

describe('the pattern browser', () => {
  it('lists every pattern and folder', async () => {
    await setup()
    expect(screen.getByRole('tree', { name: 'Patterns' })).toBeTruthy()
    expect(rows().some(row => row.startsWith('Fills'))).toBe(true)
    expect(rows().some(row => row.startsWith('Choruses'))).toBe(true)
    expect(rows().some(row => row.startsWith('Beat'))).toBe(true)
    expect(rows().some(row => row.startsWith('Verse'))).toBe(true)
  })

  it('shows a pattern in the sequencer tab that shows its notes', async () => {
    const store = await setup()
    fireEvent.click(screen.getByText('Beat'))
    expect(selectFocusedView(store.getState() as never)).toBe(View.Sequencer)
    expect(store.getState().sequencer.selected).toBe(0)

    fireEvent.click(screen.getByText('Verse'))
    expect(selectFocusedView(store.getState() as never)).toBe(View.NoteSequencer)
    expect(store.getState().sequencer.selected).toBe(1)
  })

  it('searches every pattern', async () => {
    await setup()
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'e' } })
    expect(screen.getByText('Verse')).toBeTruthy()
    expect(screen.getByText('Beat')).toBeTruthy()
    expect(screen.queryByText('Fill')).toBeNull()
  })

  it('makes folders', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'New folder' }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      'add_pattern_folder', { name: 'New folder', parent: null }))
  })
})
