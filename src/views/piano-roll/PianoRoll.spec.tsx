import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../state/slices/playlistSlice'
import projectReducer from '../../state/slices/projectSlice'
import mixerReducer from '../../state/slices/mixerSlice'
import windowReducer, { openTab, selectFocusedView } from '../../state/slices/windowSlice'
import effectsReducer from '../../state/slices/effectsSlice'
import sequencerReducer, { clearPendingNoteSends } from '../../state/slices/sequencerSlice'
import { resetGestures } from '../../state/gesture'
import { Note, SequencerState, Synth } from '../../types/sequencer'
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

const synth = (id: number, name: string): Synth => ({
  id, pluginId: `vst3:${id}`, name, vendor: 'Acme', volume: 1, muted: false, missing: false, hasEditor: true,
})

let sequencer: SequencerState

// note patterns 0 and 1 (and a drum pattern, 2, which isn't offered), with
// synth 7's notes in 0, and synth 8's in 1
const makeSequencer = (notes: Note[] = []): SequencerState => ({
  rows: [],
  synths: [synth(7, 'Pad'), synth(8, 'Bass')],
  patterns: [
    { id: 0, name: 'Verse', kind: 'note', length: 16, steps: {}, notes: { 7: notes } },
    { id: 1, name: null, kind: 'note', length: 32, steps: {}, notes: { 8: [note(40, 0)] } },
    { id: 2, name: 'Beat', length: 16, steps: {} },
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
const lastSynth = () => calls('set_pattern_notes').at(-1)?.synthId as number

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
  mockInvoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
    if(command === 'get_sequencer') return sequencer
    if(command === 'get_playlist_transport') return stopped
    if(command === 'set_pattern_notes') {
      sequencer = {
        ...sequencer,
        patterns: sequencer.patterns.map(p => (p.id === args.patternId
          ? { ...p, notes: { ...p.notes, [args.synthId as number]: args.notes as Note[] } }
          : p)),
      }
      return sequencer
    }
    if(command === 'add_pattern') return { id: 0, sequencer: makeSequencer() }
    return undefined
  })
})

describe('the Piano Roll', () => {
  it('offers to add a note pattern when there’s none', async () => {
    sequencer = { ...makeSequencer(), patterns: [makeSequencer().patterns[2]] }
    setup()
    fireEvent.click(await screen.findByRole('button', { name: 'New pattern' }))
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1, kind: 'note' }]))
    expect(await screen.findByRole('combobox', { name: 'Pattern' })).toBeTruthy()
  })

  it('picks the note pattern and the synth whose notes it edits, and plays the whole pattern', async () => {
    const store = setup()

    const pattern = await screen.findByRole('combobox', { name: 'Pattern' }) as HTMLSelectElement
    expect(Array.from(pattern.options).map(o => o.text)).toEqual(['Verse', '2'])
    fireEvent.change(pattern, { target: { value: '1' } })
    expect(store.getState().sequencer.selected.note).toBe(1)
    expect(screen.getByText('2 bars')).toBeTruthy()

    const synthPicker = screen.getByRole('combobox', { name: 'Synth' }) as HTMLSelectElement
    expect(Array.from(synthPicker.options).map(o => o.text)).toEqual(['Pad', 'Bass'])
    expect(synthPicker.value).toBe('7')
    fireEvent.change(synthPicker, { target: { value: '8' } })
    expect(store.getState().sequencer.pianoRollSynthId).toBe(8)

    // Bass's note is under the mouse now: right-click removes it
    fireEvent.contextMenu(grid(), at(0.5, 40))
    await waitFor(() => expect(lastNotes()).toEqual([]))
    expect(lastSynth()).toBe(8)

    fireEvent.click(screen.getByRole('button', { name: 'Play the pattern' }))
    await waitFor(() => expect(calls('t_toggle_pattern')).toEqual([{ patternId: 1 }]))
  })

  it('sends to the Note Sequencer when there’s no synth yet', async () => {
    sequencer = { ...makeSequencer(), synths: [] }
    const store = setup()
    expect(await screen.findByText(/No synth yet/)).toBeTruthy()
    expect(screen.queryByRole('combobox', { name: 'Synth' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Note Sequencer' }))
    expect(selectFocusedView(store.getState() as never)).toBe(View.NoteSequencer)
  })

  it('draws a quarter note with a click, hearing its pitch briefly on the synth', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    setup()
    await screen.findByRole('combobox', { name: 'Pattern' })

    fireEvent.pointerDown(grid(), at(3.5, 62))
    fireEvent.pointerUp(grid(), at(3.5, 62))

    expect(lastNotes()).toEqual([note(62, 3, 4)])
    expect(lastSynth()).toBe(7)
    expect(calls('play_note')).toEqual([{ synthId: 7, pitch: 62, velocity: 0.8 }])
    // let go: it stops
    expect(calls('stop_note')).toEqual([{ synthId: 7, pitch: 62 }])
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

  it('plays a key of the keyboard on the synth while it’s held', async () => {
    setup()
    await screen.findByRole('combobox', { name: 'Synth' })
    const keys = screen.getByTitle('Click a key to hear it')
    fireEvent.pointerDown(keys, { clientY: rowTop(60) - grid().scrollTop + 2, button: 0 })
    expect(calls('play_note')).toEqual([{ synthId: 7, pitch: 60, velocity: 0.8 }])
    fireEvent.pointerUp(keys)
    expect(calls('stop_note')).toEqual([{ synthId: 7, pitch: 60 }])
  })
})
