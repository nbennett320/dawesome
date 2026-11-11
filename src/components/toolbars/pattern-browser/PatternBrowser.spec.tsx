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
import { PatternKind, SequencerState } from '../../../types/sequencer'
import { View } from '../../../types/ui'
import PatternBrowser from './PatternBrowser'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

// drum patterns Beat (no kind: an older project's) and Fill, note patterns
// Verse and Chorus, and a folder of each kind
const sequencer: SequencerState = {
  rows: [],
  synths: [],
  patterns: [
    { id: 0, name: 'Beat', length: 16, steps: {} },
    { id: 1, name: 'Verse', kind: 'note', length: 16, steps: {}, notes: {} },
    { id: 2, name: 'Fill', kind: 'drum', length: 16, steps: {}, folder: 10 },
    { id: 3, name: 'Chorus', kind: 'note', length: 16, steps: {}, notes: {}, folder: 11 },
  ],
  folders: [{ id: 10, name: 'Fills', kind: 'drum' }, { id: 11, name: 'Choruses', kind: 'note' }],
}

const setup = async (kind: PatternKind) => {
  const store = configureStore({
    reducer: { playlist: playlistReducer, project: projectReducer, window: windowReducer, sequencer: sequencerReducer },
  })
  render(<Provider store={store}><PatternBrowser kind={kind} /></Provider>)
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

describe('the pattern pages', () => {
  it('Drum Patterns lists drum patterns and folders, and shows one in the Drum Sequencer', async () => {
    const store = await setup('drum')
    expect(screen.getByRole('tree', { name: 'Drum patterns' })).toBeTruthy()
    expect(rows().some(row => row.startsWith('Fills'))).toBe(true)
    expect(rows().some(row => row.startsWith('Beat'))).toBe(true)
    expect(screen.queryByText('Verse')).toBeNull()
    expect(screen.queryByText('Choruses')).toBeNull()

    fireEvent.click(screen.getByText('Beat'))
    expect(selectFocusedView(store.getState() as never)).toBe(View.Sequencer)
    expect(store.getState().sequencer.selected.drum).toBe(0)
  })

  it('Note Patterns lists note patterns and folders, and shows one in the Note Sequencer', async () => {
    const store = await setup('note')
    expect(screen.getByRole('tree', { name: 'Note patterns' })).toBeTruthy()
    expect(rows().some(row => row.startsWith('Choruses'))).toBe(true)
    expect(rows().some(row => row.startsWith('Verse'))).toBe(true)
    expect(screen.queryByText('Beat')).toBeNull()
    expect(screen.queryByText('Fills')).toBeNull()

    fireEvent.click(screen.getByText('Verse'))
    expect(selectFocusedView(store.getState() as never)).toBe(View.NoteSequencer)
    expect(store.getState().sequencer.selected).toEqual({ drum: 0, note: 1 })
  })

  it('searches its own kind only', async () => {
    await setup('note')
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'e' } })
    expect(screen.getByText('Verse')).toBeTruthy()
    expect(screen.queryByText('Beat')).toBeNull()
  })

  it('makes folders of its own kind', async () => {
    await setup('note')
    fireEvent.click(screen.getByRole('button', { name: 'New folder' }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      'add_pattern_folder', { name: 'New folder', parent: null, kind: 'note' }))
  })
})
