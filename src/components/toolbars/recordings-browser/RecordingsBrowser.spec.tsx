import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog'
import playlistReducer from '../../../state/slices/playlistSlice'
import projectReducer from '../../../state/slices/projectSlice'
import recordingReducer, { setRecording } from '../../../state/slices/recordingSlice'
import sequencerReducer from '../../../state/slices/sequencerSlice'
import { Library, RecordingFile, RecordingsListing } from '../../../types/recordings'
import RecordingsBrowser from './RecordingsBrowser'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock
const mockAsk = ask as Mock

const take = (name: string, folder: string): RecordingFile => ({
  path: `/p/recordings/${folder ? `${folder}/` : ''}${name}.wav`, name, folder, durationSecs: 2, modifiedMs: null,
})

// Vocals/ holds Verse, and Guitar is at the top
const listing: RecordingsListing = {
  takes: [take('Guitar', ''), take('Verse', 'Vocals')],
  folders: ['Vocals'],
}

const setup = async (library: Library = 'recordings', recording = false) => {
  const store = configureStore({
    reducer: {
      playlist: playlistReducer, project: projectReducer, recording: recordingReducer, sequencer: sequencerReducer,
    },
  })
  if(recording) store.dispatch(setRecording(true))
  render(<Provider store={store}><RecordingsBrowser library={library} /></Provider>)
  await act(async () => {})
  return store
}

const names = () => screen.getAllByRole('treeitem').map(row => row.textContent ?? '')

beforeEach(() => {
  mockInvoke.mockReset()
  mockAsk.mockReset()
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 't_list_recordings') return listing
    if(command === 't_create_recording_folder') return 'New folder'
    return undefined
  })
})

describe('the recordings pages', () => {
  it('lists the folders and takes of its library', async () => {
    await setup('clips')
    expect(mockInvoke).toHaveBeenCalledWith('t_list_recordings', { library: 'clips' })
    expect(screen.getByRole('tree', { name: 'Clips' })).toBeTruthy()
    expect(names().some(row => row.startsWith('Vocals'))).toBe(true)
    expect(names().some(row => row.startsWith('Guitar'))).toBe(true)
  })

  it('opens and closes a folder, to show what is in it', async () => {
    await setup()
    // (which folders are open is kept while the app runs, so it may start open)
    const open = () => !!screen.queryByText('Verse')
    const before = open()
    fireEvent.click(screen.getByText('Vocals'))
    expect(open()).toBe(!before)
    fireEvent.click(screen.getByText('Vocals'))
    expect(open()).toBe(before)
  })

  it('searches the takes', async () => {
    await setup()
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'ver' } })
    expect(screen.getByText('Verse')).toBeTruthy()
    expect(screen.queryByText('Guitar')).toBeNull()
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'zzz' } })
    expect(screen.getByText('No recordings match.')).toBeTruthy()
  })

  it('makes a folder, with a name that is free', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'New folder' }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      't_create_recording_folder', { parent: '', name: 'New folder', library: 'recordings' }))
  })

  it('makes a folder inside the folder last clicked', async () => {
    await setup()
    fireEvent.click(screen.getByText('Vocals'))
    fireEvent.click(screen.getByRole('button', { name: 'New folder' }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      't_create_recording_folder', { parent: 'Vocals', name: 'New folder', library: 'recordings' }))
  })

  it('leaves the folders alone while recording', async () => {
    await setup('recordings', true)
    const button = screen.getByRole('button', { name: 'New folder' }) as HTMLButtonElement
    expect(button.disabled).toBe(true)
  })

  it('asks before moving a folder to the Trash, and does nothing if refused', async () => {
    await setup()
    fireEvent.contextMenu(screen.getByText('Vocals'))
    mockAsk.mockResolvedValue(false)
    fireEvent.click(screen.getByText('Move to Trash'))
    await waitFor(() => expect(mockAsk).toHaveBeenCalledWith(
      'Move "Vocals" and the 1 recording in it to the Trash?', 'Delete folder'))
    expect(mockInvoke).not.toHaveBeenCalledWith('t_delete_recording_folder', expect.anything())
  })

  it('moves a folder to the Trash once the user agrees', async () => {
    await setup()
    fireEvent.contextMenu(screen.getByText('Vocals'))
    mockAsk.mockResolvedValue(true)
    fireEvent.click(screen.getByText('Move to Trash'))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      't_delete_recording_folder', { folder: 'Vocals', library: 'recordings' }))
  })
})
