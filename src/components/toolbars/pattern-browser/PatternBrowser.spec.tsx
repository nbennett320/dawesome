import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
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

// Beat (steps on the sample row), Verse and Chorus (notes on the synth row),
// Both (on both), Fill (empty), and two folders
const sequencer: SequencerState = sequencerOf({
  rows: [sampleRow(0), synthRow(1)],
  patterns: [
    pattern(0, { name: 'Beat', notes: { 0: steps(0) } }),
    pattern(1, { name: 'Verse', notes: { 1: steps(0) } }),
    pattern(2, { name: 'Fill', folder: 10 }),
    pattern(3, { name: 'Chorus', notes: { 1: steps(4) }, folder: 11 }),
    pattern(4, { name: 'Both', notes: { 0: steps(0), 1: steps(2) } }),
  ],
  folders: [{ id: 10, name: 'Fills' }, { id: 11, name: 'Choruses' }],
})
// what the engine has (the sequencer above, unless a spec says otherwise)
let engineHas = sequencer

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
  engineHas = sequencer
  mockInvoke.mockReset()
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 'get_sequencer') return engineHas
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

  it('shows the one folder tree that old drum and note folders were merged into', async () => {
    // as the engine opens the v3 fixture: its note folder "Verse" became
    // "Verse 2", beside the drum folder "Verse"
    engineHas = sequencerOf({
      rows: [sampleRow(0), synthRow(3)],
      patterns: [
        pattern(0, { name: 'Loop 1', notes: { 0: steps(0) }, folder: 0 }),
        pattern(4, { name: 'Sequence 5', notes: { 3: steps(0) }, folder: 3 }),
      ],
      folders: [{ id: 0, name: 'Verse' }, { id: 1, name: 'Verse 2' }, { id: 3, name: 'Pads', parent: 1 }],
    })
    await setup()
    // (each folder's row ends with how many it holds, each pattern's with
    // its length)
    expect(rows()).toEqual(['Verse1', 'Verse 21'])
    fireEvent.click(screen.getByText('Verse 2'))
    fireEvent.click(screen.getByText('Pads'))
    fireEvent.click(screen.getByText('Verse'))
    expect(rows()).toEqual(['Verse1', 'Loop 11 bar', 'Verse 21', 'Pads1', 'Sequence 51 bar'])
  })

  it('marks what each pattern plays: samples, instruments, or both', async () => {
    await setup()
    const badges = (name: string) => within(screen.getByText(name).closest('[role="treeitem"]') as HTMLElement)
      .queryAllByRole('img').map(img => img.getAttribute('aria-label')).filter(label => label?.startsWith('Plays'))
    expect(badges('Beat')).toEqual(['Plays samples'])
    expect(badges('Verse')).toEqual(['Plays instruments'])
    expect(badges('Both')).toEqual(['Plays samples', 'Plays instruments'])
  })

  it('shows a pattern in the Pattern Sequencer', async () => {
    const store = await setup()
    fireEvent.click(screen.getByText('Beat'))
    expect(selectFocusedView(store.getState() as never)).toBe(View.PatternSequencer)
    expect(store.getState().sequencer.selected).toBe(0)

    fireEvent.click(screen.getByText('Verse'))
    expect(selectFocusedView(store.getState() as never)).toBe(View.PatternSequencer)
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
