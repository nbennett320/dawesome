import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import effectsReducer from '../../../state/slices/effectsSlice'
import { PluginCatalog, PluginInfo } from '../../../types/effects'
import EffectsBrowser, { POLL_MS } from './EffectsBrowser'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const mockInvoke = invoke as Mock

const plugin = (name: string, category: string, vendor = 'Acme'): PluginInfo => ({
  id: `vst3:${name}`,
  path: `/Library/Audio/Plug-Ins/VST3/${name}.vst3`,
  name,
  vendor,
  version: '1.0',
  kind: 'effect',
  category,
  subcategories: [category],
})

const catalog = (change: Partial<PluginCatalog> = {}): PluginCatalog => ({
  plugins: [plugin('Verb', 'Reverb', 'Tidewater'), plugin('Comp', 'Dynamics'), plugin('Gate', 'Dynamics')],
  failed: [],
  scanning: false,
  done: 0,
  total: 0,
  ...change,
})

let answers: PluginCatalog[]

const setup = async () => {
  const store = configureStore({ reducer: { effects: effectsReducer } })
  render(<Provider store={store}><EffectsBrowser /></Provider>)
  // the catalog is asked for straight away
  await act(async () => {})
}

beforeEach(() => {
  vi.resetAllMocks()
  answers = [catalog()]
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 'get_plugins') return answers.length > 1 ? answers.shift() : answers[0]
    return undefined
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('the effects browser', () => {
  it('shows categories as closed folders, and opens one', async () => {
    await setup()
    expect(mockInvoke).toHaveBeenCalledWith('get_plugins', {})

    const folders = screen.getAllByRole('treeitem').map(row => row.textContent)
    expect(folders).toEqual(['Dynamics2', 'Reverb1'])

    fireEvent.click(screen.getByRole('treeitem', { name: /Dynamics/ }))
    expect(screen.getByText('Comp')).toBeTruthy()
    expect(screen.getByText('Gate')).toBeTruthy()
    expect(screen.queryByText('Verb')).toBeNull()
  })

  it('searches by vendor, opening the folders with a match', async () => {
    await setup()
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search effects' }), { target: { value: 'tidewater' } })

    expect(screen.getByText('Verb')).toBeTruthy()
    expect(screen.getByText('Tidewater')).toBeTruthy()
    expect(screen.queryByText('Dynamics')).toBeNull()

    fireEvent.change(screen.getByRole('searchbox', { name: 'Search effects' }), { target: { value: 'zzz' } })
    expect(screen.getByText('No effects match.')).toBeTruthy()
  })

  it('says where it looked when there are none', async () => {
    answers = [catalog({ plugins: [] })]
    await setup()
    expect(screen.getByText('No VST3 effects found')).toBeTruthy()
    expect(screen.getByText('Looked in:')).toBeTruthy()
  })

  it('lists what couldn’t be read, closed', async () => {
    answers = [catalog({ failed: [{ path: '/x/Broken.vst3', reason: 'timed out' }] })]
    await setup()

    const toggle = screen.getByRole('button', { name: /Couldn’t be read \(1\)/ })
    expect(screen.queryByText('timed out')).toBeNull()
    fireEvent.click(toggle)
    expect(screen.getByText('Broken.vst3')).toBeTruthy()
    expect(screen.getByText('timed out')).toBeTruthy()
  })

  it('rescans everything, showing how it goes until it’s done', async () => {
    vi.useFakeTimers()
    await setup()

    answers = [catalog({ scanning: true, done: 0, total: 12 }), catalog({ scanning: true, done: 3, total: 12 }), catalog()]
    fireEvent.click(screen.getByRole('button', { name: 'Rescan plugins' }))
    await act(async () => {})
    expect(mockInvoke).toHaveBeenCalledWith('scan_plugins', { everything: true })
    expect(screen.getByText('Scanning 0 of 12…')).toBeTruthy()
    expect((screen.getByRole('button', { name: 'Rescan plugins' }) as HTMLButtonElement).disabled).toBe(true)

    await act(async () => { vi.advanceTimersByTime(POLL_MS) })
    expect(screen.getByText('Scanning 3 of 12…')).toBeTruthy()

    await act(async () => { vi.advanceTimersByTime(POLL_MS) })
    expect(screen.queryByRole('status')).toBeNull()

    // and stops asking
    const asked = mockInvoke.mock.calls.length
    await act(async () => { vi.advanceTimersByTime(POLL_MS * 3) })
    expect(mockInvoke.mock.calls.length).toBe(asked)
  })
})
