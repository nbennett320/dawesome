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
    if(command === 'list_recordings') return listing
    if(command === 'create_recording_folder') return 'New folder'
    return undefined
  })
})

describe('the recordings pages', () => {
  it('lists the folders and takes of its library', async () => {
    await setup('clips')
    expect(mockInvoke).toHaveBeenCalledWith('list_recordings', { library: 'clips' })
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
      'create_recording_folder', { parent: '', name: 'New folder', library: 'recordings' }))
  })

  it('makes a folder inside the folder last clicked', async () => {
    await setup()
    fireEvent.click(screen.getByText('Vocals'))
    fireEvent.click(screen.getByRole('button', { name: 'New folder' }))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      'create_recording_folder', { parent: 'Vocals', name: 'New folder', library: 'recordings' }))
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
    expect(mockInvoke).not.toHaveBeenCalledWith('delete_recording_folder', expect.anything())
  })

  it('moves a folder to the Trash once the user agrees', async () => {
    await setup()
    fireEvent.contextMenu(screen.getByText('Vocals'))
    mockAsk.mockResolvedValue(true)
    fireEvent.click(screen.getByText('Move to Trash'))
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      'delete_recording_folder', { folder: 'Vocals', library: 'recordings' }))
  })
})

describe('previewing a take', () => {
  // how long each take is, as the engine answers once it's playing
  const lengths: Record<string, number> = {
    '/p/recordings/Guitar.wav': 3,
    '/p/recordings/Riff.wav': 3,
    '/p/recordings/Bass.wav': 2,
    '/p/recordings/Solo.wav': 5,
  }
  // the takes the engine lists, which a rename changes
  let shown: RecordingsListing

  const row = (name: string) => screen.getByRole('treeitem', { name: new RegExp(name) })
  const stopButtons = () => screen.queryAllByRole('button', { name: 'Stop preview' })
  const dots = (name: string) => row(name).querySelectorAll('[aria-label="In use"], [aria-label="Not used"]')

  // press a take's row, and let the engine answer
  const hear = async (name: string) => {
    fireEvent.mouseDown(row(name), { button: 0 })
    await act(async () => {})
  }

  beforeEach(() => {
    vi.useFakeTimers()
    shown = { takes: [take('Guitar', ''), take('Bass', ''), take('Solo', '')], folders: [] }
    mockInvoke.mockImplementation(async (command: string, args?: { path?: string, name?: string }) => {
      if(command === 'list_recordings') return shown
      if(command === 'preview_sample') return lengths[args?.path ?? '']
      if(command === 'rename_recording') {
        const path = `/p/recordings/${args?.name}.wav`
        shown = { ...shown, takes: shown.takes.map(t => (t.path === args?.path ? { ...t, path, name: args?.name ?? '' } : t)) }
        return { path, movedClips: [], sequencer: null, problem: null }
      }
      return undefined
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('shows a stop button in place of the dot while it plays, if it’s longer than 2 seconds', async () => {
    await setup()
    expect(stopButtons()).toHaveLength(0)
    expect(dots('Guitar')).toHaveLength(1)

    await hear('Guitar')
    expect(mockInvoke).toHaveBeenCalledWith('preview_sample', { path: '/p/recordings/Guitar.wav' })
    expect(stopButtons()).toHaveLength(1)
    expect(row('Guitar').contains(stopButtons()[0])).toBe(true)
    expect(dots('Guitar')).toHaveLength(0)

    // and hides it once the take's over, and the dot's back
    await act(async () => { vi.advanceTimersByTime(2_999) })
    expect(stopButtons()).toHaveLength(1)
    await act(async () => { vi.advanceTimersByTime(1) })
    expect(stopButtons()).toHaveLength(0)
    expect(dots('Guitar')).toHaveLength(1)
  })

  it('shows no stop button for a take of 2 seconds or less', async () => {
    await setup()
    await hear('Bass')
    expect(mockInvoke).toHaveBeenCalledWith('preview_sample', { path: '/p/recordings/Bass.wav' })
    expect(stopButtons()).toHaveLength(0)
  })

  it('stops the preview without playing it again', async () => {
    await setup()
    await hear('Guitar')
    mockInvoke.mockClear()

    fireEvent.mouseDown(stopButtons()[0], { button: 0 })
    await act(async () => {})
    expect(mockInvoke).toHaveBeenCalledWith('stop_previews')
    expect(mockInvoke).not.toHaveBeenCalledWith('preview_sample', expect.anything())
    expect(stopButtons()).toHaveLength(0)
  })

  it('stops with Escape on the row', async () => {
    await setup()
    await hear('Guitar')

    fireEvent.keyDown(row('Guitar'), { key: 'Escape' })
    expect(mockInvoke).toHaveBeenCalledWith('stop_previews')
    expect(stopButtons()).toHaveLength(0)
  })

  it('moves the button to the take heard next, whose length it keeps', async () => {
    await setup()
    await hear('Guitar')
    await hear('Solo')
    expect(stopButtons()).toHaveLength(1)
    expect(row('Solo').contains(stopButtons()[0])).toBe(true)

    // (Guitar's 3 seconds don't end Solo's 5)
    await act(async () => { vi.advanceTimersByTime(3_000) })
    expect(stopButtons()).toHaveLength(1)
    await act(async () => { vi.advanceTimersByTime(2_000) })
    expect(stopButtons()).toHaveLength(0)
  })

  it('keeps the button on a take renamed while it plays', async () => {
    await setup()
    await hear('Guitar')

    fireEvent.contextMenu(row('Guitar'))
    fireEvent.click(screen.getByText('Rename'))
    const input = screen.getByRole('textbox', { name: 'Recording name' })
    fireEvent.change(input, { target: { value: 'Riff' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    await act(async () => {})

    expect(mockInvoke).toHaveBeenCalledWith('rename_recording', expect.objectContaining({ path: '/p/recordings/Guitar.wav', name: 'Riff' }))
    expect(stopButtons()).toHaveLength(1)
    expect(row('Riff').contains(stopButtons()[0])).toBe(true)
  })

  it('does the same on the Clips page', async () => {
    await setup('clips')
    await hear('Guitar')
    expect(stopButtons()).toHaveLength(1)
    fireEvent.mouseDown(stopButtons()[0], { button: 0 })
    await act(async () => {})
    expect(mockInvoke).toHaveBeenCalledWith('stop_previews')
    expect(stopButtons()).toHaveLength(0)
  })
})
