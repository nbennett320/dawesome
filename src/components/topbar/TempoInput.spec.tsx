import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer, { reduceTempo } from '../../state/slices/playlistSlice'
import recordingReducer, { setRecording } from '../../state/slices/recordingSlice'
import sequencerReducer from '../../state/slices/sequencerSlice'
import { resetGestures } from '../../state/gesture'
import TempoInput, { FINE_PX_PER_BPM, PX_PER_BPM, draggedTempo, tempoWheels } from './TempoInput'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

describe('tempoWheels', () => {
  it('shows a whole tempo on three wheels', () => {
    expect(tempoWheels(120)).toBe('120')
    expect(tempoWheels(90)).toBe('090')
    expect(tempoWheels(1)).toBe('001')
    expect(tempoWheels(1000)).toBe('1000')
  })

  it('shows a tenth when there is one', () => {
    expect(tempoWheels(128.5)).toBe('128.5')
    expect(tempoWheels(99.96)).toBe('100')
  })
})

describe('draggedTempo', () => {
  it('is whole, and one the engine takes', () => {
    expect(draggedTempo(120.4)).toBe(120)
    expect(draggedTempo(120.6)).toBe(121)
    expect(draggedTempo(-3)).toBe(1)
    expect(draggedTempo(2000)).toBe(1000)
  })
})

describe('TempoInput', () => {
  // the tempos sent to the engine, with the gesture each was part of
  let sent: { val: number, gesture: number | null }[]

  const open = (tempo = 120) => {
    const store = configureStore({
      reducer: { playlist: playlistReducer, recording: recordingReducer, sequencer: sequencerReducer },
    })
    store.dispatch(reduceTempo(tempo))
    render(<Provider store={store}><TempoInput /></Provider>)
    return store
  }

  const dial = () => screen.getByRole('spinbutton', { name: 'Playlist tempo' })

  beforeEach(() => {
    sent = []
    resetGestures()
    HTMLElement.prototype.setPointerCapture = vi.fn()
    mockInvoke.mockReset()
    mockInvoke.mockImplementation(async (command: string, args?: { val: number, gesture: number | null }) => {
      if(command === 'set_playlist_tempo' && args) {
        sent.push(args)
        return args.val
      }
      if(command === 'get_playlist_playing') return false
      if(command === 'get_playlist_transport') return { playing: false, patternId: null, startTimeMs: null }
      return undefined
    })
  })

  it('shows the tempo on counter wheels', () => {
    open(95)
    expect(screen.getByRole('img', { name: 'Tempo 095' })).toBeInTheDocument()
    expect(dial()).toHaveAttribute('aria-valuenow', '95')
  })

  it('goes up when dragged up, and down when dragged down, one step to undo', async () => {
    const store = open(120)
    fireEvent.pointerDown(window, { clientY: 100 })
    fireEvent.pointerDown(dial(), { button: 0, clientY: 100 })
    fireEvent.pointerMove(dial(), { clientY: 100 - 10 * PX_PER_BPM })
    expect(dial()).toHaveAttribute('aria-valuenow', '130')
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(130))

    fireEvent.pointerMove(dial(), { clientY: 100 + 5 * PX_PER_BPM })
    fireEvent.pointerUp(dial(), { clientY: 100 + 5 * PX_PER_BPM })
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(115))

    // every tempo the drag went through was part of the one gesture
    expect(new Set(sent.map(s => s.gesture)).size).toBe(1)
    expect(sent[0].gesture).not.toBeNull()
  })

  it('moves finer with shift held', async () => {
    const store = open(120)
    fireEvent.pointerDown(dial(), { button: 0, clientY: 100 })
    fireEvent.pointerMove(dial(), { clientY: 100 - 2 * FINE_PX_PER_BPM, shiftKey: true })
    fireEvent.pointerUp(dial())
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(122))
  })

  it('sends the tempos a fast drag goes through in order, ending on the last', async () => {
    let release: () => void = () => {}
    const store = open(120)
    const plain = mockInvoke.getMockImplementation()!
    // the engine is slow to answer the first
    mockInvoke.mockImplementationOnce((command: string, args: { val: number, gesture: number | null }) =>
      new Promise(resolve => { release = () => resolve(plain(command, args)) }))

    fireEvent.pointerDown(dial(), { button: 0, clientY: 100 })
    fireEvent.pointerMove(dial(), { clientY: 100 - PX_PER_BPM })
    fireEvent.pointerMove(dial(), { clientY: 100 - 2 * PX_PER_BPM })
    fireEvent.pointerMove(dial(), { clientY: 100 - 3 * PX_PER_BPM })
    fireEvent.pointerUp(dial())
    expect(dial()).toHaveAttribute('aria-valuenow', '123')

    await act(async () => release())
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(123))
    // the one on its way, then only the latest
    expect(sent.map(s => s.val)).toEqual([121, 123])
  })

  it('takes a typed tempo after a double-click, on Enter', async () => {
    const store = open(120)
    fireEvent.doubleClick(dial())
    const field = screen.getByLabelText('Playlist tempo input')
    fireEvent.change(field, { target: { value: '87.5' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(87.5))
    expect(screen.getByRole('img', { name: 'Tempo 087.5' })).toBeInTheDocument()
  })

  it('takes a typed tempo when the field is left, and none on Escape', async () => {
    const store = open(120)
    fireEvent.doubleClick(dial())
    let field = screen.getByLabelText('Playlist tempo input')
    fireEvent.change(field, { target: { value: '140' } })
    fireEvent.blur(field)
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(140))

    fireEvent.doubleClick(dial())
    field = screen.getByLabelText('Playlist tempo input')
    fireEvent.change(field, { target: { value: '60' } })
    fireEvent.keyDown(field, { key: 'Escape' })
    expect(screen.queryByLabelText('Playlist tempo input')).toBeNull()
    expect(store.getState().playlist.tempo).toBe(140)
  })

  it('ignores a typed tempo the engine wouldn’t take', () => {
    open(120)
    fireEvent.doubleClick(dial())
    const field = screen.getByLabelText('Playlist tempo input')
    fireEvent.change(field, { target: { value: '0' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(sent).toEqual([])
  })

  it('steps with the arrow keys', async () => {
    const store = open(120)
    fireEvent.keyDown(dial(), { key: 'ArrowUp' })
    fireEvent.keyDown(dial(), { key: 'ArrowUp', shiftKey: true })
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(131))
    fireEvent.keyDown(dial(), { key: 'ArrowDown' })
    await waitFor(() => expect(store.getState().playlist.tempo).toBe(130))
  })

  it('can’t be changed while recording', () => {
    const store = open(120)
    act(() => { store.dispatch(setRecording(true)) })
    fireEvent.pointerDown(dial(), { button: 0, clientY: 100 })
    fireEvent.pointerMove(dial(), { clientY: 0 })
    fireEvent.doubleClick(dial())
    fireEvent.keyDown(dial(), { key: 'ArrowUp' })
    expect(screen.queryByLabelText('Playlist tempo input')).toBeNull()
    expect(dial()).toHaveAttribute('aria-disabled', 'true')
    expect(sent).toEqual([])
  })
})
