import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer, { playlistSlice } from '../../state/slices/playlistSlice'
import sequencerReducer from '../../state/slices/sequencerSlice'
import { PlaylistTransport } from '../../types/transport'
import Timeline from './Timeline'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

// the strip is this wide, and its border this thick
const WIDTH = 400
const BORDER = 1

// 120bpm 4/4, 8 bars of song (16s): 50px a bar, 12.5px a beat
const transport: PlaylistTransport = {
  playing: false,
  startTimeMs: null,
  startOffsetMs: 0,
  positionMs: 0,
  lastPlayStartBeats: null,
  beatIntervalMs: 500,
  loopStartBeats: 0, loopEndBeats: 32, loopRegion: false, loopEndPinned: false,
  beatsDisplayed: 256,
  loopEnabled: false,
  songEndMs: 15_000,
  beatsPerBar: 4,
  patternId: null,
  patternLength: 0,
}

const makeStore = () => configureStore({
  reducer: { playlist: playlistReducer, sequencer: sequencerReducer },
})

class FixedResizeObserver {
  constructor(private callback: ResizeObserverCallback) {}

  observe() {
    this.callback([{ contentRect: { width: WIDTH, height: 80 } } as ResizeObserverEntry], this as never)
  }

  disconnect() {}

  unobserve() {}
}

const seeks = () => mockInvoke.mock.calls.filter(([name]) => name === 'seek_playlist')

const strip = () => screen.getByTestId('timeline-strip')

const at = (x: number) => ({ clientX: x + BORDER, button: 0, pointerId: 1 })

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', FixedResizeObserver)
  Element.prototype.setPointerCapture = vi.fn()
  Element.prototype.getBoundingClientRect = () => ({ left: 0 } as DOMRect)
  Object.defineProperty(HTMLElement.prototype, 'clientLeft', { configurable: true, get: () => BORDER })
  mockInvoke.mockReset()
  mockInvoke.mockImplementation((name: string, args?: { beat?: number }) => {
    if(name === 'get_playlist_transport') return Promise.resolve(transport)
    if(name === 'seek_playlist') return Promise.resolve(args?.beat)
    if(name === 'get_playlist_playing') return Promise.resolve(false)
    return Promise.resolve(null)
  })
})

afterEach(() => {
  vi.unstubAllGlobals()
})

const renderTimeline = async () => {
  const store = makeStore()
  render(<Provider store={store}><Timeline /></Provider>)
  await screen.findByText('0:16.0')
  return store
}

describe('Timeline', () => {
  it('runs from the start to the end of the last clip\'s bar', async () => {
    await renderTimeline()
    expect(screen.getByText('0:00.0')).toBeTruthy()
    expect(screen.getByText('0:16.0')).toBeTruthy()
  })

  it('follows a drag on screen, and tells the engine once, when it\'s let go', async () => {
    await renderTimeline()

    fireEvent.pointerDown(strip(), at(42))
    fireEvent.pointerMove(strip(), at(103))
    fireEvent.pointerMove(strip(), at(204))
    // 204px is 8.16s: shown where the mouse is, unsnapped
    expect(screen.getByText('0:08.1 · bar 5.1')).toBeTruthy()
    expect(seeks()).toHaveLength(0)

    fireEvent.pointerUp(strip(), at(204))
    // snapped to the nearest quarter note: beat 16.32 lands on 16
    await waitFor(() => expect(seeks()).toEqual([['seek_playlist', { beat: 16 }]]))
  })

  it('snaps to the playlist\'s snap setting', async () => {
    const store = await renderTimeline()
    act(() => { store.dispatch(playlistSlice.actions.setSnapNote('WholeNote')) })

    fireEvent.pointerDown(strip(), at(130))
    fireEvent.pointerUp(strip(), at(130))
    // 10.4 beats in: the nearest whole note is beat 12
    await waitFor(() => expect(seeks()).toEqual([['seek_playlist', { beat: 12 }]]))
  })

  it('goes nowhere when the drag is taken away', async () => {
    await renderTimeline()

    fireEvent.pointerDown(strip(), at(100))
    fireEvent.pointerCancel(strip(), at(100))
    fireEvent.pointerUp(strip(), at(100))
    expect(seeks()).toHaveLength(0)
  })
})
