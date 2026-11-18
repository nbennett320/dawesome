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

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command)
// where the playhead's let go (a scrub's end moves it)
const seeks = () => calls('scrub_end')

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
    if(name === 'scrub_end') return Promise.resolve(args?.beat)
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

  it('follows a drag on screen, scrubbing the song along with it, and moves the playhead once, when it\'s let go', async () => {
    await renderTimeline()

    fireEvent.pointerDown(strip(), at(42))
    // 42px is 1.68s: the scrub begins there
    expect(calls('scrub_begin')).toEqual([['scrub_begin', { atMs: 1_680 }]])
    fireEvent.pointerMove(strip(), at(103))
    fireEvent.pointerMove(strip(), at(204))
    // 204px is 8.16s: shown where the mouse is, unsnapped, and scrubbed to
    expect(screen.getByText('0:08.1 · bar 5.1')).toBeTruthy()
    await waitFor(() => expect(calls('scrub_to').at(-1)).toEqual(['scrub_to', { atMs: 8_160 }]))
    expect(seeks()).toHaveLength(0)
    expect(calls('seek_playlist')).toHaveLength(0)

    fireEvent.pointerUp(strip(), at(204))
    // snapped to the nearest quarter note: beat 16.32 lands on 16
    await waitFor(() => expect(seeks()).toEqual([['scrub_end', { beat: 16 }]]))
  })

  it('snaps to the playlist\'s snap setting', async () => {
    const store = await renderTimeline()
    act(() => { store.dispatch(playlistSlice.actions.setSnapNote('WholeNote')) })

    fireEvent.pointerDown(strip(), at(130))
    fireEvent.pointerUp(strip(), at(130))
    // 10.4 beats in: the nearest whole note is beat 12
    await waitFor(() => expect(seeks()).toEqual([['scrub_end', { beat: 12 }]]))
  })

  it('goes nowhere when the drag is taken away: back where it was', async () => {
    await renderTimeline()

    fireEvent.pointerDown(strip(), at(100))
    fireEvent.pointerCancel(strip(), at(100))
    fireEvent.pointerUp(strip(), at(100))
    await waitFor(() => expect(seeks()).toEqual([['scrub_end', { beat: 0 }]]))
  })
})
