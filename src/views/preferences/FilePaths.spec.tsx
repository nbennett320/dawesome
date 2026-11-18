import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import envReducer, { SamplePath } from '../../state/slices/envSlice'
import projectReducer from '../../state/slices/projectSlice'
import FilePaths from './FilePaths'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
const mockInvoke = invoke as Mock
const mockOpen = open as Mock

const bundled: SamplePath = { name: 'Dawesome', path: '/app/samples', kind: 'bundled', found: true }
const drums: SamplePath = { name: 'Drums', path: '/Users/me/Drums', kind: 'user', found: true }
const gone: SamplePath = { name: 'Old', path: '/Volumes/USB/Old', kind: 'user', found: false }
const project: SamplePath = { name: 'This project', path: '/Users/me/song/samples', kind: 'project', found: true }

let listed: SamplePath[]

const setup = async () => {
  const store = configureStore({ reducer: { env: envReducer, project: projectReducer } })
  render(<Provider store={store}><FilePaths /></Provider>)
  await act(async () => {})
  return store
}

const rows = () => screen.getAllByRole('listitem')

beforeEach(() => {
  listed = [bundled, drums, gone, project]
  mockInvoke.mockReset()
  mockOpen.mockReset()
  mockInvoke.mockImplementation(async (command: string, args?: { path?: string }) => {
    if(command === 'sample_paths') return listed
    if(command === 'add_sample_path') {
      if(args?.path === '/Users/me/Drums/Kicks') throw 'that folder is inside Drums, which is already in File Paths'
      listed = [...listed, { name: 'Loops', path: args?.path ?? '', kind: 'user', found: true }]
      return listed
    }
    if(command === 'remove_sample_path') {
      listed = listed.filter(path => path.path !== args?.path)
      return listed
    }
    return undefined
  })
})

describe('File Paths', () => {
  it('lists every folder of samples, in order', async () => {
    await setup()
    expect(rows().map(row => row.textContent)).toEqual([
      expect.stringContaining('Dawesome'),
      expect.stringContaining('Drums'),
      expect.stringContaining('Old'),
      expect.stringContaining('This project'),
    ])
    expect(rows()[2].textContent).toContain('Not found')
  })

  it('only lets the user’s own folders be removed', async () => {
    await setup()
    expect(screen.getAllByRole('button', { name: /^Remove / }).map(b => b.getAttribute('aria-label')))
      .toEqual(['Remove Drums', 'Remove Old'])

    fireEvent.click(screen.getByRole('button', { name: 'Remove Old' }))
    await act(async () => {})
    expect(mockInvoke).toHaveBeenCalledWith('remove_sample_path', { path: '/Volumes/USB/Old' })
    expect(rows()).toHaveLength(3)
  })

  it('adds a folder picked in the folder picker', async () => {
    mockOpen.mockResolvedValue('/Users/me/Loops')
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Add Folder…' }))
    await act(async () => {})
    expect(mockOpen).toHaveBeenCalledWith(expect.objectContaining({ directory: true }))
    expect(mockInvoke).toHaveBeenCalledWith('add_sample_path', { path: '/Users/me/Loops' })
    expect(rows()[4].textContent).toContain('Loops')
  })

  it('does nothing when the picker is cancelled', async () => {
    mockOpen.mockResolvedValue(null)
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Add Folder…' }))
    await act(async () => {})
    expect(mockInvoke).not.toHaveBeenCalledWith('add_sample_path', expect.anything())
  })

  it('says why a folder can’t be added', async () => {
    mockOpen.mockResolvedValue('/Users/me/Drums/Kicks')
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Add Folder…' }))
    await act(async () => {})
    expect(screen.getByRole('alert').textContent).toContain('inside Drums')
    expect(rows()).toHaveLength(4)
  })
})
