import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../state/slices/playlistSlice'
import projectReducer from '../../state/slices/projectSlice'
import mixerReducer, { setName } from '../../state/slices/mixerSlice'
import windowReducer, { openTab } from '../../state/slices/windowSlice'
import effectsReducer from '../../state/slices/effectsSlice'
import sequencerReducer, { clearPendingNoteSends } from '../../state/slices/sequencerSlice'
import { resetGestures } from '../../state/gesture'
import { EffectSlot, Racks } from '../../types/effects'
import { Note, SequencerState } from '../../types/sequencer'
import { View } from '../../types/ui'
import PianoRoll from './PianoRoll'
import { ROW_HEIGHT, STEP_WIDTH, rowTop } from './draw'
import { SHORT_MS } from './audition'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

const note = (pitch: number, start: number, length = 4): Note => ({ pitch, start, length, velocity: 0.8 })

const synth = (id: number): EffectSlot => ({
  id, pluginId: `vst3:${id}`, name: 'Synth', vendor: 'Acme', bypassed: false, missing: false, hasEditor: true, kind: 'instrument',
})

let sequencer: SequencerState
let racks: Racks

const makeSequencer = (notes: Note[] = []): SequencerState => ({
  rows: [],
  patterns: [
    { id: 0, name: 'Verse', length: 16, steps: {}, notes },
    { id: 1, name: null, length: 32, steps: {} },
  ],
  folders: [],
})

const stopped = { playing: false, patternId: null, startTimeMs: null, beatIntervalMs: 500, beatsPerBar: 4 }

const setup = () => {
  const store = configureStore({
    reducer: {
      playlist: playlistReducer,
      project: projectReducer,
      mixer: mixerReducer,
      window: windowReducer,
      effects: effectsReducer,
      sequencer: sequencerReducer,
    },
  })
  store.dispatch(openTab(View.PianoRoll))
  render(<Provider store={store}><PianoRoll /></Provider>)
  return store
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command).map(([, args]) => args)
const lastNotes = () => calls('set_pattern_notes').at(-1)?.notes as Note[]

// the grid sits at the page's top-left, 600 by 400
const grid = () => screen.getByTestId('note-grid')
const at = (step: number, pitch: number) => {
  const el = grid()
  return {
    clientX: step * STEP_WIDTH - el.scrollLeft + 1,
    clientY: rowTop(pitch) - el.scrollTop + ROW_HEIGHT / 2,
    button: 0,
    pointerId: 1,
  }
}

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => 600 })
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => 400 })
  // jsdom has no canvas: nothing is drawn
  HTMLCanvasElement.prototype.getContext = (() => null) as never
})

afterAll(() => {
  delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientWidth
  delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientHeight
})

beforeEach(() => {
  resetGestures()
  clearPendingNoteSends()
  sequencer = makeSequencer()
  racks = [{ track: 2, slots: [synth(7)] }, { track: 5, slots: [synth(8)] }]
  mockInvoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
    if(command === 'get_sequencer') return sequencer
    if(command === 'get_effects') return racks
    if(command === 'get_playlist_transport') return stopped
    if(command === 'set_pattern_notes') {
      sequencer = { ...sequencer, patterns: sequencer.patterns.map(p => (p.id === args.patternId ? { ...p, notes: args.notes as Note[] } : p)) }
      return sequencer
    }
    if(command === 'add_pattern') return { id: 0, sequencer: makeSequencer() }
    return undefined
  })
})

describe('the Piano Roll', () => {
  it('offers to add a pattern when there’s none', async () => {
    sequencer = { rows: [], patterns: [], folders: [] }
    setup()
    fireEvent.click(await screen.findByRole('button', { name: 'New pattern' }))
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1 }]))
    expect(await screen.findByRole('combobox', { name: 'Pattern' })).toBeTruthy()
  })

  it('picks the pattern with the Sequencer, and the track its notes play on', async () => {
    const store = setup()
    act(() => { store.dispatch(setName({ trackNumber: 5, name: 'Bass' })) })

    const pattern = await screen.findByRole('combobox', { name: 'Pattern' }) as HTMLSelectElement
    expect(Array.from(pattern.options).map(o => o.text)).toEqual(['Verse', '2'])
    fireEvent.change(pattern, { target: { value: '1' } })
    expect(store.getState().sequencer.selectedPatternId).toBe(1)
    expect(screen.getByText('2 bars')).toBeTruthy()

    const playsOn = await screen.findByRole('combobox', { name: 'Plays on' }) as HTMLSelectElement
    await waitFor(() => expect(Array.from(playsOn.options).map(o => o.text)).toEqual(['Track 3', 'Bass']))
    expect(playsOn.value).toBe('2')
    fireEvent.change(playsOn, { target: { value: '5' } })

    fireEvent.click(screen.getByRole('button', { name: 'Play the pattern' }))
    await waitFor(() => expect(calls('t_toggle_pattern')).toEqual([{ patternId: 1, track: 5 }]))
  })

  it('hints how to hear notes when no track has an instrument', async () => {
    racks = []
    setup()
    expect(await screen.findByText('Drag an instrument onto a track in the Mixer to hear notes')).toBeTruthy()
    const playsOn = screen.getByRole('combobox', { name: 'Plays on' }) as HTMLSelectElement
    expect(Array.from(playsOn.options).map(o => o.text)).toEqual(['No instrument'])
  })

  it('draws a quarter note with a click, hearing its pitch briefly', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    setup()
    await screen.findByRole('combobox', { name: 'Pattern' })
    await waitFor(() => expect((screen.getByRole('combobox', { name: 'Plays on' }) as HTMLSelectElement).value).toBe('2'))

    fireEvent.pointerDown(grid(), at(3.5, 62))
    fireEvent.pointerUp(grid(), at(3.5, 62))

    expect(lastNotes()).toEqual([note(62, 3, 4)])
    expect(calls('play_note')).toEqual([{ track: 2, pitch: 62, velocity: 0.8 }])
    // let go: it stops
    expect(calls('stop_note')).toEqual([{ track: 2, pitch: 62 }])
    vi.advanceTimersByTime(SHORT_MS)
    expect(calls('stop_note')).toHaveLength(1)
    vi.useRealTimers()
  })

  it('draws a note as long as it’s dragged, and moves and resizes notes', async () => {
    sequencer = makeSequencer([note(60, 0, 2)])
    setup()
    await screen.findByRole('combobox', { name: 'Pattern' })

    // drawn from step 4 to the middle of step 6: 3 steps
    fireEvent.pointerDown(grid(), at(4, 64))
    fireEvent.pointerMove(grid(), at(6.5, 64))
    fireEvent.pointerUp(grid(), at(6.5, 64))
    await waitFor(() => expect(lastNotes()).toEqual([note(60, 0, 2), note(64, 4, 3)]))

    // the first moved along a step and up two semitones
    fireEvent.pointerDown(grid(), at(0.2, 60))
    fireEvent.pointerMove(grid(), at(1.2, 62))
    fireEvent.pointerUp(grid(), at(1.2, 62))
    await waitFor(() => expect(lastNotes()).toEqual([note(62, 1, 2), note(64, 4, 3)]))

    // and made a step longer from its right edge
    fireEvent.pointerDown(grid(), at(2.9, 62))
    fireEvent.pointerMove(grid(), at(3.9, 62))
    fireEvent.pointerUp(grid(), at(3.9, 62))
    await waitFor(() => expect(lastNotes()).toEqual([note(62, 1, 3), note(64, 4, 3)]))

    // a plain click now draws a note as long as the last one resized
    fireEvent.pointerDown(grid(), at(10, 70))
    fireEvent.pointerUp(grid(), at(10, 70))
    await waitFor(() => expect(lastNotes().at(-1)).toEqual(note(70, 10, 3)))
  })

  it('moves the whole selection, and deletes it with Delete', async () => {
    sequencer = makeSequencer([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2)])
    setup()
    await screen.findByRole('combobox', { name: 'Pattern' })

    fireEvent.pointerDown(grid(), at(0.5, 60))
    fireEvent.pointerUp(grid(), at(0.5, 60))
    fireEvent.pointerDown(grid(), { ...at(4.5, 64), shiftKey: true })
    fireEvent.pointerUp(grid(), at(4.5, 64))

    fireEvent.pointerDown(grid(), at(4.5, 64))
    fireEvent.pointerMove(grid(), at(6.5, 64))
    fireEvent.pointerUp(grid(), at(6.5, 64))
    await waitFor(() => expect(lastNotes()).toEqual([note(60, 2, 2), note(64, 6, 2), note(67, 8, 2)]))

    fireEvent.keyDown(window, { key: 'Delete' })
    await waitFor(() => expect(lastNotes()).toEqual([note(67, 8, 2)]))
  })

  it('selects all with ⌘A (⌃A here), and clears with Escape', async () => {
    sequencer = makeSequencer([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2)])
    setup()
    await screen.findByRole('combobox', { name: 'Pattern' })

    fireEvent.pointerDown(grid(), at(0.5, 60))
    fireEvent.pointerUp(grid(), at(0.5, 60))
    fireEvent.keyDown(window, { key: 'Escape' })
    fireEvent.keyDown(window, { key: 'Delete' })
    expect(calls('set_pattern_notes')).toEqual([])

    fireEvent.keyDown(window, { key: 'a', ctrlKey: true })
    fireEvent.keyDown(window, { key: 'Backspace' })
    await waitFor(() => expect(lastNotes()).toEqual([]))
  })

  it('selects what a shift-drag box touches', async () => {
    sequencer = makeSequencer([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2)])
    setup()
    await screen.findByRole('combobox', { name: 'Pattern' })

    fireEvent.pointerDown(grid(), { ...at(3, 70), shiftKey: true })
    fireEvent.pointerMove(grid(), at(9, 62))
    fireEvent.pointerUp(grid(), at(9, 62))
    // a box draws nothing
    expect(calls('set_pattern_notes')).toEqual([])

    fireEvent.keyDown(window, { key: 'Delete' })
    await waitFor(() => expect(lastNotes()).toEqual([note(60, 0, 2)]))
  })

  it('deletes a note with a right-click', async () => {
    sequencer = makeSequencer([note(60, 0, 2), note(64, 4, 2)])
    setup()
    await screen.findByRole('combobox', { name: 'Pattern' })
    fireEvent.contextMenu(grid(), at(4.5, 64))
    await waitFor(() => expect(lastNotes()).toEqual([note(60, 0, 2)]))
  })

  it('plays a key of the keyboard while it’s held', async () => {
    setup()
    await waitFor(() => expect((screen.getByRole('combobox', { name: 'Plays on' }) as HTMLSelectElement).value).toBe('2'))
    const keys = screen.getByTitle('Click a key to hear it')
    fireEvent.pointerDown(keys, { clientY: rowTop(60) - grid().scrollTop + 2, button: 0 })
    expect(calls('play_note')).toEqual([{ track: 2, pitch: 60, velocity: 0.8 }])
    fireEvent.pointerUp(keys)
    expect(calls('stop_note')).toEqual([{ track: 2, pitch: 60 }])
  })
})
