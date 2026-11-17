import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import playlistReducer from '../../state/slices/playlistSlice'
import tapeReducer from '../../state/slices/tapeSlice'
import { clearPendingMixerSends } from '../../state/slices/mixerSlice'
import { defaultTapeMachine } from '../../types/tape'
import { applyTheme } from '../../theme/apply'
import { slateLightTheme } from '../../theme/fixture'
import TapeMachinePanel from './TapeMachinePanel'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

beforeAll(() => applyTheme(slateLightTheme))
beforeEach(() => {
  vi.resetAllMocks()
  clearPendingMixerSends()
  // get_tape: the engine's tape, as it starts
  mockInvoke.mockImplementation((command: string) =>
    Promise.resolve(command === 'get_tape' ? { ...defaultTapeMachine } : undefined))
})
afterEach(cleanup)

const panel = async () => {
  const store = configureStore({ reducer: { playlist: playlistReducer, tape: tapeReducer } })
  render(<Provider store={store}><TapeMachinePanel /></Provider>)
  // the engine's tape, read as it opens
  await act(async () => { await Promise.resolve() })
  return store
}

const tapeKey = () => screen.getByRole('button', { name: 'Tape' })
const age = () => screen.getByRole('slider', { name: 'Tape age' })
// the controls the switch dims
const dimmed = () => age().closest('.transition-opacity')!.className.includes('opacity-50')

describe('the tape machine panel', () => {
  it('has every control, at its default', async () => {
    await panel()
    expect(mockInvoke).toHaveBeenCalledWith('get_tape', {})
    expect(tapeKey().getAttribute('aria-pressed')).toBe('false')
    expect((screen.getByRole('combobox', { name: 'Tape machine' }) as HTMLSelectElement).value).toBe('0')
    expect(screen.getByRole('option', { name: 'Deck 2' })).toBeTruthy()
    expect(age().getAttribute('aria-valuenow')).toBe('0')
    expect(screen.getByRole('slider', { name: 'Wow and flutter' }).getAttribute('aria-valuenow')).toBe('100')
    expect(screen.getByRole('slider', { name: 'Tape hiss' }).getAttribute('aria-valuenow')).toBe('100')
    expect(screen.getByRole('button', { name: 'dbx noise reduction' }).getAttribute('aria-pressed')).toBe('true')
    expect(screen.getByRole('slider', { name: 'Tape output' }).getAttribute('aria-valuenow')).toBe('0')
  })

  it('dims while the tape is off, but can still be set', async () => {
    const store = await panel()
    expect(dimmed()).toBe(true)
    fireEvent.keyDown(age(), { key: 'ArrowUp' })
    expect(store.getState().tape.machine.age).toBe(5)
    expect(mockInvoke).toHaveBeenLastCalledWith('set_tape_machine', {
      machine: { ...defaultTapeMachine, age: 5 }, gesture: null,
    })

    fireEvent.click(tapeKey())
    expect(store.getState().tape.machine.enabled).toBe(true)
    expect(dimmed()).toBe(false)
  })

  it('goes back to each default on a double-click', async () => {
    const store = await panel()
    const output = screen.getByRole('slider', { name: 'Tape output' })
    fireEvent.keyDown(output, { key: 'End' })
    expect(store.getState().tape.machine.outputDb).toBe(12)
    fireEvent.doubleClick(output)
    expect(store.getState().tape.machine.outputDb).toBe(0)

    const wobble = screen.getByRole('slider', { name: 'Wow and flutter' })
    fireEvent.keyDown(wobble, { key: 'Home' })
    fireEvent.doubleClick(wobble)
    expect(store.getState().tape.machine.wobble).toBe(100)
  })
})
