import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import playlistReducer from '../../state/slices/playlistSlice'
import tapeReducer, { setMachine } from '../../state/slices/tapeSlice'
import recordingReducer, { setRecording } from '../../state/slices/recordingSlice'
import { clearPendingMixerSends } from '../../state/slices/mixerSlice'
import { defaultTapeMachine } from '../../types/tape'
import { applyTheme } from '../../theme/apply'
import { slateLightTheme } from '../../theme/fixture'
import PitchKnob from './PitchKnob'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

beforeAll(() => applyTheme(slateLightTheme))
beforeEach(() => {
  vi.resetAllMocks()
  clearPendingMixerSends()
  mockInvoke.mockImplementation(() => Promise.resolve(undefined))
})
afterEach(cleanup)

const knob = () => {
  const store = configureStore({ reducer: { playlist: playlistReducer, tape: tapeReducer, recording: recordingReducer } })
  render(<Provider store={store}><PitchKnob /></Provider>)
  return store
}

const pitch = () => screen.getByRole('slider', { name: 'Pitch' })
const dimmed = () => pitch().closest('.transition-opacity')!.className.includes('opacity-50')

// a drag of the knob, `px` up from where it starts
const drag = (px: number) => {
  const el = pitch()
  el.setPointerCapture = () => {}
  fireEvent.pointerDown(el, { button: 0, clientY: 100, pointerId: 1 })
  fireEvent.pointerMove(el, { clientY: 100 - px, pointerId: 1 })
  fireEvent.pointerUp(el, { pointerId: 1 })
}

describe('the PITCH knob', () => {
  it('starts at its click, dimmed while the tape is off, and can still be set', () => {
    const store = knob()
    expect(pitch().getAttribute('aria-valuenow')).toBe('0')
    expect(screen.getByText('0%')).toBeTruthy()
    expect(dimmed()).toBe(true)
    fireEvent.keyDown(pitch(), { key: 'ArrowUp' })
    expect(store.getState().tape.machine.pitch).toBe(0.5)
    expect(mockInvoke).toHaveBeenLastCalledWith('set_tape_machine', {
      machine: { ...defaultTapeMachine, pitch: 0.5 }, gesture: null,
    })

    act(() => { store.dispatch(setMachine({ enabled: true })) })
    expect(dimmed()).toBe(false)
  })

  it('reaches 12 % either way, and clicks into place near 0', () => {
    const store = knob()
    fireEvent.keyDown(pitch(), { key: 'End' })
    expect(store.getState().tape.machine.pitch).toBe(12)
    fireEvent.keyDown(pitch(), { key: 'Home' })
    expect(store.getState().tape.machine.pitch).toBe(-12)
    fireEvent.doubleClick(pitch())
    expect(store.getState().tape.machine.pitch).toBe(0)

    // 180 px is the whole 24 %: 3 px is 0.4 %, inside the click
    drag(3)
    expect(store.getState().tape.machine.pitch).toBe(0)
    // 15 px is 2 %
    drag(15)
    expect(store.getState().tape.machine.pitch).toBe(2)
  })

  it('holds still while recording', () => {
    const store = knob()
    act(() => { store.dispatch(setRecording(true)) })
    expect(pitch().getAttribute('aria-disabled')).toBe('true')
    fireEvent.keyDown(pitch(), { key: 'ArrowUp' })
    expect(store.getState().tape.machine.pitch).toBe(0)
  })
})
