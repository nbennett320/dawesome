import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, render, screen } from '@testing-library/react'
import appearanceReducer from '../state/slices/appearanceSlice'
import { postInfoNote } from '../events/infoNote'
import InfoBar from './InfoBar'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const setup = () => {
  const store = configureStore({ reducer: { appearance: appearanceReducer } })
  render(<Provider store={store}><InfoBar /></Provider>)
}

describe('the info bar', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('shows a note for a moment, not as a warning', () => {
    setup()
    act(() => postInfoNote('Loop start · bar 9'))
    const note = screen.getByText('Loop start · bar 9')
    expect(note.className).toContain('text-ink-strong')
    expect(note.className).not.toContain('text-danger')

    act(() => { vi.advanceTimersByTime(3_500) })
    expect(screen.queryByText('Loop start · bar 9')).toBeNull()
  })

  it('shows a newer note in place of the last', () => {
    setup()
    act(() => postInfoNote('Loop start · bar 9'))
    act(() => postInfoNote('Song start · bar 1'))
    expect(screen.queryByText('Loop start · bar 9')).toBeNull()
    expect(screen.getByText('Song start · bar 1')).toBeTruthy()
  })
})
