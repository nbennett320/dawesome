import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import envReducer from '../../../state/slices/envSlice'
import projectReducer from '../../../state/slices/projectSlice'
import SampleBrowser from './SampleBrowser'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

// the folder of samples Dawesome comes with, as File Paths lists it
const samples = { name: 'Dawesome', path: '/s', kind: 'bundled', found: true }

// how long each sample plays, in seconds
const lengths: Record<string, number> = { '/s/Clap.wav': 2, '/s/Kick.wav': 0.5, '/s/Long.wav': 5, '/s/Shaker.aif': 3 }

const setup = async () => {
  const store = configureStore({ reducer: { env: envReducer, project: projectReducer } })
  render(<Provider store={store}><SampleBrowser /></Provider>)
  await act(async () => {})
}

const row = (name: string) => screen.getByRole('treeitem', { name: new RegExp(name) })
const stopButtons = () => screen.queryAllByRole('button', { name: 'Stop preview' })

// press a sample's row, and let the engine answer
const hear = async (name: string) => {
  fireEvent.mouseDown(row(name), { button: 0 })
  await act(async () => {})
}

beforeEach(() => {
  vi.useFakeTimers()
  mockInvoke.mockReset()
  mockInvoke.mockImplementation(async (command: string, args?: { path?: string }) => {
    if(command === 'sample_paths') return [samples]
    if(command === 'enumerate_directory') {
      return [['Clap', 'Kick', 'Long', 'Shaker'], ['/s/Clap.wav', '/s/Kick.wav', '/s/Long.wav', '/s/Shaker.aif'], [], []]
    }
    if(command === 'preview_sample') return lengths[args?.path ?? '']
    return undefined
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('previewing a sample', () => {
  it('shows a stop button on its row while it plays, if it’s longer than 2 seconds', async () => {
    await setup()
    expect(stopButtons()).toHaveLength(0)

    await hear('Shaker')
    expect(mockInvoke).toHaveBeenCalledWith('preview_sample', { path: '/s/Shaker.aif' })
    expect(stopButtons()).toHaveLength(1)
    expect(row('Shaker').contains(stopButtons()[0])).toBe(true)

    // and hides it once the sample's over
    await act(async () => { vi.advanceTimersByTime(2_999) })
    expect(stopButtons()).toHaveLength(1)
    await act(async () => { vi.advanceTimersByTime(1) })
    expect(stopButtons()).toHaveLength(0)
  })

  it('shows no stop button for a sample of 2 seconds or less', async () => {
    await setup()
    await hear('Kick')
    expect(mockInvoke).toHaveBeenCalledWith('preview_sample', { path: '/s/Kick.wav' })
    expect(stopButtons()).toHaveLength(0)
    await hear('Clap')
    expect(stopButtons()).toHaveLength(0)
  })

  it('stops the preview without playing it again', async () => {
    await setup()
    await hear('Shaker')
    mockInvoke.mockClear()

    fireEvent.mouseDown(stopButtons()[0], { button: 0 })
    await act(async () => {})
    expect(mockInvoke).toHaveBeenCalledWith('stop_previews')
    expect(mockInvoke).not.toHaveBeenCalledWith('preview_sample', expect.anything())
    expect(stopButtons()).toHaveLength(0)
  })

  it('stops with Escape on the row', async () => {
    await setup()
    await hear('Shaker')

    fireEvent.keyDown(row('Shaker'), { key: 'Escape' })
    expect(mockInvoke).toHaveBeenCalledWith('stop_previews')
    expect(stopButtons()).toHaveLength(0)
  })

  it('moves the button to the sample heard next, whose length it keeps', async () => {
    await setup()
    await hear('Shaker')
    await hear('Long')
    expect(row('Long').contains(stopButtons()[0])).toBe(true)
    expect(stopButtons()).toHaveLength(1)

    // the shaker's end doesn't hide the long sample's button
    await act(async () => { vi.advanceTimersByTime(3_000) })
    expect(stopButtons()).toHaveLength(1)
    await act(async () => { vi.advanceTimersByTime(2_000) })
    expect(stopButtons()).toHaveLength(0)
  })

  it('takes the button away when a short sample is heard next', async () => {
    await setup()
    await hear('Long')
    expect(stopButtons()).toHaveLength(1)
    await hear('Kick')
    expect(stopButtons()).toHaveLength(0)
  })

  it('shows no button if the sample couldn’t be played', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {})
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'sample_paths') return [samples]
      if(command === 'enumerate_directory') return [['Kick'], ['/s/Kick.wav'], [], []]
      if(command === 'preview_sample') throw new Error('could not decode')
      return undefined
    })
    await setup()
    await hear('Kick')
    expect(stopButtons()).toHaveLength(0)
  })
})

describe('folders of samples', () => {
  const drums = { name: 'Drums', path: '/me/Drums', kind: 'user', found: true }
  const gone = { name: 'Old', path: '/usb/Old', kind: 'user', found: false }
  const listings: Record<string, [string[], string[], string[], string[]]> = {
    '/s': [['Clap'], ['/s/Clap.wav'], [], []],
    '/me/Drums': [['Kick'], ['/me/Drums/Kick.wav'], [], []],
  }

  it('shows each one that’s there as a folder, open, named as File Paths names it', async () => {
    mockInvoke.mockImplementation(async (command: string, args?: { dirPath?: string }) => {
      if(command === 'sample_paths') return [samples, drums, gone]
      if(command === 'enumerate_directory') return listings[args?.dirPath ?? '']
      return undefined
    })
    await setup()
    const folders = screen.getAllByRole('treeitem').filter(item => item.hasAttribute('aria-expanded'))
    expect(folders.map(folder => folder.textContent)).toEqual([
      expect.stringContaining('Dawesome'),
      expect.stringContaining('Drums'),
    ])
    expect(folders.every(folder => folder.getAttribute('aria-expanded') === 'true')).toBe(true)
    expect(row('Clap')).toBeTruthy()
    expect(row('Kick')).toBeTruthy()
    expect(screen.queryByRole('treeitem', { name: /Old/ })).toBeNull()
  })
})
