import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../state/slices/playlistSlice'
import projectReducer from '../../state/slices/projectSlice'
import mixerReducer from '../../state/slices/mixerSlice'
import windowReducer, { openTab, selectFocusedView } from '../../state/slices/windowSlice'
import effectsReducer from '../../state/slices/effectsSlice'
import sequencerReducer, { clearPendingNoteSends, selectPattern } from '../../state/slices/sequencerSlice'
import pianoRollReducer, {
  setPianoRollSnapNote, setPianoRollTool, togglePianoRollSnap,
} from '../../state/slices/pianoRollSlice'
import { resetGestures } from '../../state/gesture'
import { clearNoteClipboard, runNoteCommandFromMenu } from '../../state/noteCommands'
import { Tool } from '../../editing/tools'
import { Note, SequencerState, Synth } from '../../types/sequencer'
import { View } from '../../types/ui'
import PianoRoll from './PianoRoll'
import { DEFAULT_STEP_WIDTH, ROW_HEIGHT, rowTop } from './draw'
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

const makeStore = () => configureStore({
  reducer: {
    playlist: playlistReducer,
    project: projectReducer,
    mixer: mixerReducer,
    window: windowReducer,
    effects: effectsReducer,
    sequencer: sequencerReducer,
    pianoRoll: pianoRollReducer,
  },
})

let store: ReturnType<typeof makeStore>

const setup = (tool: Tool = 'draw') => {
  store = makeStore()
  store.dispatch(openTab(View.PianoRoll))
  store.dispatch(setPianoRollTool(tool))
  render(<Provider store={store}><PianoRoll /></Provider>)
  return store
}

// the grid open on pattern 0, with these notes for synth 7
const open = async (notes: Note[] = [], tool: Tool = 'draw') => {
  sequencer = makeSequencer(notes)
  setup(tool)
  await screen.findByRole('combobox', { name: 'Pattern' })
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command).map(([, args]) => args)
const lastNotes = () => calls('set_pattern_notes').at(-1)?.notes as Note[]
const lastSynth = () => calls('set_pattern_notes').at(-1)?.synthId as number

// the grid sits at the page's top-left, 600 by 400
const grid = () => screen.getByTestId('note-grid')
// how wide a step is just now: the grid's width over pattern 0's 16 steps
const stepWidth = () => parseFloat((grid().firstElementChild as HTMLElement).style.width) / 16
const at = (step: number, pitch: number, keys: { shiftKey?: boolean, altKey?: boolean } = {}) => {
  const el = grid()
  return {
    clientX: step * stepWidth() - el.scrollLeft,
    clientY: rowTop(pitch) - el.scrollTop + ROW_HEIGHT / 2,
    button: 0,
    pointerId: 1,
    ...keys,
  }
}

type Spot = ReturnType<typeof at>

const click = (spot: Spot) => {
  fireEvent.pointerDown(grid(), spot)
  fireEvent.pointerUp(grid(), spot)
}

const drag = (from: Spot, to: Spot) => {
  fireEvent.pointerDown(grid(), from)
  fireEvent.pointerMove(grid(), to)
  fireEvent.pointerUp(grid(), to)
}

const key = (k: string, mods: { ctrlKey?: boolean, shiftKey?: boolean } = {}) => fireEvent.keyDown(window, { key: k, ...mods })

// the notes left after deleting the selection: which were selected
const deleteSelection = async (expected: Note[]) => {
  key('Delete')
  await waitFor(() => expect(lastNotes()).toEqual(expected))
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
  clearNoteClipboard()
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
    setup()

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
    await waitFor(() => expect(calls('toggle_pattern')).toEqual([{ patternId: 1 }]))
  })

  it('sends to the Note Sequencer when there’s no synth yet', async () => {
    sequencer = { ...makeSequencer(), synths: [] }
    setup()
    expect(await screen.findByText(/No synth yet/)).toBeTruthy()
    expect(screen.queryByRole('combobox', { name: 'Synth' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Note Sequencer' }))
    expect(selectFocusedView(store.getState() as never)).toBe(View.NoteSequencer)
  })

  it('has the playlist’s tray: Select, Draw and Cut, with Draw to start, and snap', async () => {
    await open()
    const pressed = (name: string) => screen.getByRole('button', { name }).getAttribute('aria-pressed')
    expect(pressed('Draw (D)')).toBe('true')
    expect(pressed('Select (S)')).toBe('false')
    expect(pressed('Cut (C)')).toBe('false')

    key('c')
    expect(store.getState().pianoRoll.tool).toBe('cut')
    fireEvent.click(screen.getByRole('button', { name: 'Select (S)' }))
    expect(store.getState().pianoRoll.tool).toBe('select')

    expect(store.getState().pianoRoll.snapEnabled).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: 'Snap to grid' }))
    expect(store.getState().pianoRoll.snapEnabled).toBe(false)
    const amount = screen.getByRole('combobox', { name: 'Piano Roll snap amount' }) as HTMLSelectElement
    expect(amount.value).toBe('SixteenthNote')
    fireEvent.change(amount, { target: { value: 'QuarterNote' } })
    expect(store.getState().pianoRoll.snapNote).toBe('QuarterNote')
  })

  describe('the Draw tool', () => {
    it('draws a quarter note with a click, hearing its pitch briefly on the synth', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true })
      await open()

      click(at(3.5, 62))

      expect(lastNotes()).toEqual([note(62, 3, 4)])
      expect(lastSynth()).toBe(7)
      expect(calls('play_note')).toEqual([{ synthId: 7, pitch: 62, velocity: 0.8 }])
      // let go: it stops
      expect(calls('stop_note')).toEqual([{ synthId: 7, pitch: 62 }])
      vi.advanceTimersByTime(SHORT_MS)
      expect(calls('stop_note')).toHaveLength(1)
      vi.useRealTimers()
    })

    it('draws a note as long as it’s dragged, and a click then draws one as long', async () => {
      await open([note(60, 0, 2)])

      // drawn from step 4 to step 6.6: to the nearest step, 3 long
      drag(at(4.5, 64), at(6.6, 64))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 0, 2), note(64, 4, 3)]))

      click(at(10.2, 70))
      await waitFor(() => expect(lastNotes().at(-1)).toEqual(note(70, 10, 3)))
    })

    it('snaps where a note starts and ends, unless snap is off or Option is held', async () => {
      await open()
      act(() => { store.dispatch(setPianoRollSnapNote('QuarterNote')) })

      // starts on the beat at or before the press, and ends on the nearest
      drag(at(5.5, 60), at(9.6, 60))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 4, 4)]))

      // Option held: on the step
      click(at(9.5, 62, { altKey: true }))
      await waitFor(() => expect(lastNotes().at(-1)).toEqual(note(62, 9, 4)))

      // snap off: on the step too
      act(() => { store.dispatch(togglePianoRollSnap()) })
      click(at(13.5, 64))
      await waitFor(() => expect(lastNotes().at(-1)).toEqual(note(64, 13, 3)))
    })

    it('takes the note being drawn to the key under the mouse, hearing it there', async () => {
      await open()

      // straight down a key: same start and length, on the new key
      fireEvent.pointerDown(grid(), at(4.5, 64))
      fireEvent.pointerMove(grid(), at(4.5, 63))
      await waitFor(() => expect(lastNotes()).toEqual([note(63, 4, 4)]))
      expect(calls('play_note').map(args => args.pitch)).toEqual([64, 63])
      // still on that key: not heard again
      fireEvent.pointerMove(grid(), at(4.6, 63))
      expect(calls('play_note')).toHaveLength(2)
      fireEvent.pointerUp(grid(), at(4.6, 63))
      await waitFor(() => expect(lastNotes()).toEqual([note(63, 4, 4)]))

      // diagonally: the key and the length together
      drag(at(8.5, 60), at(10.6, 62))
      await waitFor(() => expect(lastNotes().at(-1)).toEqual(note(62, 8, 3)))
    })

    it('moves and trims notes as Select does', async () => {
      await open([note(60, 2, 4)])
      drag(at(4, 60), at(6.2, 62))
      await waitFor(() => expect(lastNotes()).toEqual([note(62, 4, 4)]))
    })
  })

  describe('the Select tool', () => {
    it('moves a note once dragged past the drag distance, hearing each new pitch', async () => {
      await open([note(60, 2, 2), note(64, 8, 2)], 'select')

      // a nudge isn't a drag
      drag(at(3, 60), at(3.3, 60))
      expect(calls('set_pattern_notes')).toEqual([])

      drag(at(3, 60), at(4.2, 62))
      await waitFor(() => expect(lastNotes()).toEqual([note(62, 3, 2), note(64, 8, 2)]))
      expect(calls('play_note').map(args => args.pitch)).toEqual([60, 60, 62])
    })

    it('carries a note with the mouse, landing on the key nearest where it has it', async () => {
      await open([note(60, 2, 2)], 'select')
      // pressed near the bottom of its key
      const from = { ...at(3, 60), clientY: rowTop(60) - grid().scrollTop + ROW_HEIGHT - 2 }
      const by = (steps: number, px: number) => ({ ...from, clientX: from.clientX + steps * stepWidth(), clientY: from.clientY + px })

      fireEvent.pointerDown(grid(), from)
      // the mouse just over into the key below: the note's still mostly on
      // its own, and lands there
      fireEvent.pointerMove(grid(), by(1.2, 4))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 3, 2)]))
      // most of the way down: it lands on the key below, and is heard there
      fireEvent.pointerMove(grid(), by(1.2, 9))
      await waitFor(() => expect(lastNotes()).toEqual([note(59, 3, 2)]))
      expect(calls('play_note').at(-1)?.pitch).toBe(59)
      fireEvent.pointerUp(grid(), by(1.2, 9))
      await waitFor(() => expect(lastNotes()).toEqual([note(59, 3, 2)]))
    })

    it('moves the whole selection, and deletes it with Delete', async () => {
      await open([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2)], 'select')

      click(at(1, 60))
      click(at(5, 64, { shiftKey: true }))

      drag(at(5, 64), at(7.2, 64))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 2, 2), note(64, 6, 2), note(67, 8, 2)]))

      await deleteSelection([note(67, 8, 2)])
    })

    it('selects just the note pressed when it isn’t selected', async () => {
      await open([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2)], 'select')
      click(at(1, 60))
      click(at(9, 67, { shiftKey: true }))
      click(at(5, 64))
      await deleteSelection([note(60, 0, 2), note(67, 8, 2)])
    })

    it('trims a note by either edge, and the same edge of the rest of the selection', async () => {
      await open([note(60, 4, 4), note(64, 4, 2)], 'select')

      // the right edge, a step's last few pixels
      drag(at(7.9, 60), at(9.9, 60))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 4, 6), note(64, 4, 2)]))

      // the left edge
      drag(at(4.1, 60), at(2.1, 60))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 2, 8), note(64, 4, 2)]))

      // both selected: the other's left edge moves as far
      key('a', { ctrlKey: true })
      drag(at(2.1, 60), at(3.1, 60))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 3, 7), note(64, 5, 1)]))
    })

    it('draws a note as long as the last one trimmed', async () => {
      await open([note(60, 4, 4)], 'select')
      drag(at(7.9, 60), at(9.9, 60))
      act(() => { store.dispatch(setPianoRollTool('draw')) })
      click(at(0.5, 70))
      await waitFor(() => expect(lastNotes().at(-1)).toEqual(note(70, 0, 6)))
    })

    it('pans with a drag on empty space, and clears the selection with a click there', async () => {
      await open([note(60, 0, 2)], 'select')
      click(at(1, 60))

      const before = { left: grid().scrollLeft, top: grid().scrollTop }
      drag(at(10, 50), at(8, 48))
      expect(grid().scrollLeft).toBe(before.left + 2 * DEFAULT_STEP_WIDTH)
      expect(grid().scrollTop).toBe(before.top - 2 * ROW_HEIGHT)
      // panning drew nothing, and left the selection
      expect(calls('set_pattern_notes')).toEqual([])
      await deleteSelection([])
    })

    it('clears the selection with a click on empty space', async () => {
      await open([note(60, 0, 2)], 'select')
      click(at(1, 60))
      click(at(10, 50))
      key('Delete')
      expect(calls('set_pattern_notes')).toEqual([])
    })

    it('puts the notes back with Escape mid-drag', async () => {
      await open([note(60, 0, 2)], 'select')
      fireEvent.pointerDown(grid(), at(1, 60))
      fireEvent.pointerMove(grid(), at(4, 62))
      await waitFor(() => expect(lastNotes()).toEqual([note(62, 3, 2)]))
      key('Escape')
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 0, 2)]))
      fireEvent.pointerUp(grid(), at(4, 62))
      expect(lastNotes()).toEqual([note(60, 0, 2)])
    })
  })

  describe('shift', () => {
    it('shift+click adds a note to the selection, or takes it out, with any tool', async () => {
      await open([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2)], 'cut')
      click(at(1, 60, { shiftKey: true }))
      click(at(9, 67, { shiftKey: true }))
      click(at(1, 60, { shiftKey: true }))
      // nothing was cut
      expect(calls('set_pattern_notes')).toEqual([])
      await deleteSelection([note(60, 0, 2), note(64, 4, 2)])
    })

    it('selects what a shift-drag box touches, adding to the selection', async () => {
      await open([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2), note(72, 0, 2)])
      click(at(1, 72, { shiftKey: true }))

      drag(at(3, 70, { shiftKey: true }), at(9, 62, { shiftKey: true }))
      // a box draws nothing
      expect(calls('set_pattern_notes')).toEqual([])
      await deleteSelection([note(60, 0, 2)])
    })

    it('draws a box from a press on a note, rather than moving it', async () => {
      await open([note(60, 0, 2), note(64, 4, 2)], 'select')
      drag(at(1, 60, { shiftKey: true }), at(5, 64, { shiftKey: true }))
      expect(calls('set_pattern_notes')).toEqual([])
      await deleteSelection([])
    })

    it('is a click when let go within the drag distance', async () => {
      await open([note(60, 0, 2), note(64, 4, 2)], 'select')
      drag(at(1, 60, { shiftKey: true }), at(1.3, 60, { shiftKey: true }))
      await deleteSelection([note(64, 4, 2)])
    })
  })

  describe('the Cut tool', () => {
    it('cuts a note in two with a click, the same pitch and velocity, one change', async () => {
      await open([{ ...note(60, 0, 4), velocity: 0.5 }], 'cut')
      click(at(2.4, 60))
      expect(calls('set_pattern_notes')).toHaveLength(1)
      expect(lastNotes()).toEqual([{ ...note(60, 0, 2), velocity: 0.5 }, { ...note(60, 2, 2), velocity: 0.5 }])
    })

    it('cuts the rest of the selection under the cut too, and keeps the halves selected', async () => {
      await open([note(60, 0, 4), note(64, 0, 4), note(67, 0, 4)], 'cut')
      click(at(1, 60, { shiftKey: true }))
      click(at(1, 64, { shiftKey: true }))
      click(at(2.6, 60))
      await waitFor(() => expect(lastNotes()).toEqual([
        note(60, 0, 3), note(64, 0, 3), note(67, 0, 4), note(60, 3, 1), note(64, 3, 1),
      ]))
      await deleteSelection([note(67, 0, 4)])
    })

    it('doesn’t cut when the press is dragged away, and pans on empty space', async () => {
      await open([note(60, 0, 4)], 'cut')
      drag(at(2.4, 60), at(2.4, 50))
      drag(at(10, 50), at(8, 50))
      expect(calls('set_pattern_notes')).toEqual([])
      expect(grid().scrollLeft).toBe(2 * DEFAULT_STEP_WIDTH)
    })

    it('shows the razor cursor', async () => {
      await open([note(60, 0, 4)], 'cut')
      fireEvent.pointerMove(grid(), at(2.4, 60))
      expect(grid().style.cursor).toContain('razor')
    })
  })

  describe('the commands', () => {
    it('copies and pastes at the mouse, the copies selected', async () => {
      await open([note(60, 0, 2), note(64, 2, 2)], 'select')
      key('a', { ctrlKey: true })
      key('c', { ctrlKey: true })

      fireEvent.pointerMove(grid(), at(8.5, 50))
      key('v', { ctrlKey: true })
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 0, 2), note(64, 2, 2), note(60, 8, 2), note(64, 10, 2)]))
      await deleteSelection([note(60, 0, 2), note(64, 2, 2)])
    })

    it('pastes at the first snap line in view with the mouse away, and keeps the clipboard across patterns', async () => {
      await open([note(60, 1, 2)], 'select')
      key('a', { ctrlKey: true })
      key('x', { ctrlKey: true })
      await waitFor(() => expect(lastNotes()).toEqual([]))

      act(() => { store.dispatch(selectPattern(1)) })
      act(() => { store.dispatch(setPianoRollSnapNote('QuarterNote')) })
      grid().scrollLeft = 5 * DEFAULT_STEP_WIDTH
      fireEvent.pointerLeave(grid())
      key('v', { ctrlKey: true })
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 8, 2)]))
      expect(calls('set_pattern_notes').at(-1)?.patternId).toBe(1)
    })

    it('takes the system’s copy and paste events, acting on a shortcut once', async () => {
      await open([note(60, 0, 2)], 'select')
      key('a', { ctrlKey: true })
      fireEvent.copy(document)
      fireEvent.pointerMove(grid(), at(4.5, 50))
      key('v', { ctrlKey: true })
      fireEvent.paste(document)
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 0, 2), note(60, 4, 2)]))
      expect(calls('set_pattern_notes')).toHaveLength(1)
    })

    it('repeats the selection end to end with ⌘R (⌃R here), up to the pattern’s end', async () => {
      await open([note(60, 4, 2), note(64, 6, 2)], 'select')
      key('a', { ctrlKey: true })
      key('r', { ctrlKey: true })
      key('r', { ctrlKey: true })
      await waitFor(() => expect(lastNotes()).toEqual([
        note(60, 4, 2), note(64, 6, 2), note(60, 8, 2), note(64, 10, 2), note(60, 12, 2), note(64, 14, 2),
      ]))
      // nothing more fits
      key('r', { ctrlKey: true })
      expect(calls('set_pattern_notes')).toHaveLength(2)
    })

    it('repeats from the Edit menu, cutting short a copy that crosses the end', async () => {
      await open([note(60, 8, 6)], 'select')
      key('a', { ctrlKey: true })
      act(() => runNoteCommandFromMenu('repeat'))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 8, 6), note(60, 14, 2)]))
    })

    it('selects all with ⌘A (⌃A here), and clears with Escape', async () => {
      await open([note(60, 0, 2), note(64, 4, 2), note(67, 8, 2)])

      click(at(1, 60))
      key('Escape')
      key('Delete')
      expect(calls('set_pattern_notes')).toEqual([])

      key('a', { ctrlKey: true })
      key('Backspace')
      await waitFor(() => expect(lastNotes()).toEqual([]))
    })

    it('leave the notes alone when another view is focused, or mid-drag', async () => {
      await open([note(60, 0, 2)], 'select')
      act(() => { store.dispatch(openTab(View.Playlist)) })
      key('a', { ctrlKey: true })
      key('r', { ctrlKey: true })
      act(() => runNoteCommandFromMenu('select-all'))
      key('Delete')
      expect(calls('set_pattern_notes')).toEqual([])

      act(() => { store.dispatch(openTab(View.PianoRoll)) })
      click(at(1, 60))
      fireEvent.pointerDown(grid(), at(1, 60))
      key('r', { ctrlKey: true })
      key('Delete')
      fireEvent.pointerUp(grid(), at(1, 60))
      expect(calls('set_pattern_notes')).toEqual([])
    })

    it('forgets the selection in another pattern', async () => {
      await open([note(60, 0, 2)], 'select')
      key('a', { ctrlKey: true })
      act(() => { store.dispatch(selectPattern(1)) })
      act(() => { store.dispatch(selectPattern(0)) })
      key('Delete')
      expect(calls('set_pattern_notes')).toEqual([])
    })
  })

  describe('zoom and scroll', () => {
    it('zooms along time about the mouse with ⌘+wheel (⌃ here)', async () => {
      await open([], 'draw')
      fireEvent.wheel(grid(), { deltaY: -100, ctrlKey: true, clientX: 120, clientY: 50 })
      const width = stepWidth()
      expect(width).toBeCloseTo(DEFAULT_STEP_WIDTH * 1.035)
      // the step under the mouse (5) is still under it
      expect((grid().scrollLeft + 120) / width).toBeCloseTo(5)

      // and a press lands where the zoomed grid says
      click(at(7.5, 60))
      await waitFor(() => expect(lastNotes()).toEqual([note(60, 7, 4)]))
    })

    it('zooms with a pinch, from 6 to 96 pixels a step', async () => {
      await open()
      const pinch = (type: string, scale: number) => {
        const ev = new Event(type, { cancelable: true })
        Object.assign(ev, { scale, clientX: 0 })
        act(() => { grid().parentElement?.dispatchEvent(ev) })
        return ev
      }
      expect(pinch('gesturestart', 1).defaultPrevented).toBe(true)
      expect(pinch('gesturechange', 2).defaultPrevented).toBe(true)
      expect(stepWidth()).toBe(48)
      pinch('gesturechange', 20)
      expect(stepWidth()).toBe(96)
      pinch('gesturestart', 1)
      pinch('gesturechange', 0.01)
      expect(stepWidth()).toBe(6)
    })

    it('scrolls through the pitches with the wheel, and along time with shift', async () => {
      await open()
      const top = grid().scrollTop
      fireEvent.wheel(grid(), { deltaY: 30 })
      expect(grid().scrollTop).toBe(top + 30)
      fireEvent.wheel(grid(), { deltaY: 40, shiftKey: true })
      expect(grid().scrollLeft).toBe(40)
      expect(grid().scrollTop).toBe(top + 30)
      // over the keyboard too
      fireEvent.wheel(screen.getByTitle('Click a key to hear it'), { deltaX: 10 })
      expect(grid().scrollLeft).toBe(50)
    })
  })

  it('deletes a note with a right-click', async () => {
    await open([note(60, 0, 2), note(64, 4, 2)])
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
