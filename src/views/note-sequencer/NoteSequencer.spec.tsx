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
import appearanceReducer, { setPreferences } from '../../state/slices/appearanceSlice'
import sequencerReducer from '../../state/slices/sequencerSlice'
import { DragItem, beginSampleDrag, onSampleDrag } from '../../state/sampleDrag'
import { Note, SequencerState } from '../../types/sequencer'
import { pattern, sampleRow, sequencerOf, steps, synthRow } from '../../types/sequencer.fixture'
import { View } from '../../types/ui'
import NoteSequencer from './NoteSequencer'
import Sequencer from '../sequencer/Sequencer'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

const note = (pitch: number, start: number): Note => ({ pitch, start, length: 2, velocity: 0.8 })

// the kick (a sample row, 0), then Pad and Gone (synth rows 3 and 1, in
// slots 103 and 101). patterns Beat (the kick's first step), Verse (Pad's
// notes) and Chorus (empty, 4 bars)
const makeSequencer = (change: Partial<SequencerState> = {}): SequencerState => sequencerOf({
  rows: [sampleRow(0, { path: '/drums/kick.wav' }), synthRow(3, { name: 'Pad' }), synthRow(1, { name: 'Gone', missing: true })],
  patterns: [
    pattern(0, { name: 'Beat', notes: { 0: steps(0) } }),
    pattern(1, { name: 'Verse', notes: { 3: [note(60, 0), note(64, 4)] } }),
    pattern(2, { name: 'Chorus', length: 64 }),
  ],
  ...change,
})

const stopped = { playing: false, patternId: null, startTimeMs: null, beatIntervalMs: 500, beatsPerBar: 4 }

let sequencer: SequencerState

const makeStore = () => configureStore({
  reducer: {
    playlist: playlistReducer,
    project: projectReducer,
    mixer: mixerReducer,
    window: windowReducer,
    effects: effectsReducer,
    appearance: appearanceReducer,
    sequencer: sequencerReducer,
  },
})

const setup = async (View_: React.ComponentType = NoteSequencer, view = View.NoteSequencer) => {
  const store = makeStore()
  store.dispatch(openTab(view))
  const { container } = render(<Provider store={store}><View_ /></Provider>)
  // the sequencer is asked for when the tab opens
  await act(async () => {})
  return { store, container }
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command).map(([, args]) => args)

// a drag from the sidebar, let go over `target`
const dragOnto = async (item: DragItem, target: Element) => {
  document.elementFromPoint = () => target
  beginSampleDrag(item, { button: 0, clientX: 0, clientY: 0 })
  window.dispatchEvent(new MouseEvent('mousemove', { clientX: 40, clientY: 40 }))
  window.dispatchEvent(new MouseEvent('mouseup', { clientX: 40, clientY: 40 }))
  await act(async () => {})
}

beforeEach(() => {
  mockInvoke.mockReset()
  sequencer = makeSequencer()
  mockInvoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
    if(command === 'get_sequencer') return sequencer
    if(command === 'get_playlist_transport') return stopped
    if(command === 'add_synth_row') {
      sequencer = { ...sequencer, rows: [...sequencer.rows, synthRow(9, { name: 'Keys' })] }
      return { rowId: 9, sequencer }
    }
    if(command === 'remove_row') {
      sequencer = { ...sequencer, rows: sequencer.rows.filter(row => row.id !== args.rowId) }
      return sequencer
    }
    if(command === 'set_row_muted' || command === 'set_pattern_bars' || command === 'set_pattern_step') return sequencer
    if(command === 'add_pattern') {
      sequencer = { ...sequencer, patterns: [...sequencer.patterns, pattern(5)] }
      return { id: 5, sequencer }
    }
    return undefined
  })
})

describe('the Note Sequencer', () => {
  it('has a row per synth row, a missing one greyed, and every pattern along the top', async () => {
    await setup()

    expect(screen.getByRole('button', { name: 'Beat' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Chorus' })).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Verse' }))
    // no sample rows' steps, nor a Piano Roll key: the rows' notes open it
    expect(screen.queryByRole('button', { name: /step/ })).toBeNull()
    expect(screen.queryByText('kick.wav')).toBeNull()

    expect(screen.getByRole('button', { name: 'Pad notes' }).getAttribute('title')).toMatch(/^2 notes\./)
    expect(screen.getByText('Not installed')).toBeTruthy()
    expect((screen.getByText('Gone').closest('button') as HTMLButtonElement).disabled).toBe(true)
    expect(screen.getByRole('button', { name: 'Gone notes' }).getAttribute('title')).toMatch(/^No notes yet/)
  })

  it('opens the Piano Roll on a synth row’s notes in the pattern', async () => {
    const { store } = await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Chorus' }))
    fireEvent.click(screen.getByRole('button', { name: 'Gone notes' }))

    const state = store.getState()
    expect(selectFocusedView(state as never)).toBe(View.PianoRoll)
    expect(state.sequencer.selected).toBe(2)
    expect(state.sequencer.pianoRollRowId).toBe(1)
  })

  it('opens a synth’s own interface from its name, by its slot, where Preferences say', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Pad' }))
    await waitFor(() => expect(calls('open_effect_window')).toEqual([{ slotId: 103 }]))
  })

  it('mutes and removes a synth row, by its id', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Mute Pad' }))
    await waitFor(() => expect(calls('set_row_muted')).toEqual([{ rowId: 3, muted: true, gesture: null }]))

    fireEvent.click(screen.getByRole('button', { name: 'Remove Pad' }))
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Pad notes' })).toBeNull())
    expect(calls('remove_row')).toEqual([{ rowId: 3 }])
  })

  it('adds an instrument dropped anywhere on it, and nothing else', async () => {
    const { container } = await setup()
    await dragOnto({ kind: 'sample', path: '/x.wav' }, container.firstElementChild as Element)
    await dragOnto({ kind: 'effect', pluginId: 'vst3:verb', label: 'Verb' }, container.firstElementChild as Element)
    expect(mockInvoke.mock.calls.map(([command]) => command)).not.toContain('add_synth_row')

    // on the pattern strip, not just the rows
    await dragOnto({ kind: 'instrument', pluginId: 'vst3:keys', label: 'Keys' }, screen.getByRole('button', { name: 'Verse' }))
    expect(calls('add_synth_row')).toEqual([{ pluginId: 'vst3:keys' }])
    expect(await screen.findByRole('button', { name: 'Keys notes' })).toBeTruthy()
  })

  it('asks for an instrument while it has no synth rows', async () => {
    sequencer = makeSequencer({ rows: [sampleRow(0)] })
    await setup()
    expect(screen.getByText('Drag an instrument here')).toBeTruthy()
  })

  it('offers to make a pattern when there’s none', async () => {
    sequencer = makeSequencer({ patterns: [] })
    await setup()
    expect(screen.queryByRole('button', { name: 'Pad notes' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Make one' }))
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1 }]))
    expect(await screen.findByRole('button', { name: 'Pad notes' })).toBeTruthy()
  })

  it('sets a pattern’s length as a number of bars, 1 to 128', async () => {
    await setup()
    const bars = screen.getByRole('spinbutton', { name: 'Bars' }) as HTMLInputElement
    expect(bars.value).toBe('1')

    fireEvent.change(bars, { target: { value: '129' } })
    fireEvent.keyDown(bars, { key: 'Enter' })
    fireEvent.change(bars, { target: { value: '99' } })
    fireEvent.keyDown(bars, { key: 'Enter' })
    await waitFor(() => expect(calls('set_pattern_bars')).toEqual([{ patternId: 0, bars: 99 }]))
  })

  it('has no Play key of its own: the top bar plays its pattern', async () => {
    await setup()
    expect(screen.queryByRole('button', { name: 'Play the pattern' })).toBeNull()
  })

  it('lists every pattern under ▾, to pick one', async () => {
    const { store } = await setup()
    fireEvent.click(screen.getByRole('button', { name: 'All patterns' }))
    const items = screen.getAllByRole('menuitemradio')
    expect(items.map(i => i.textContent)).toEqual(['✓Beat', 'Verse', 'Chorus'])
    expect(items[0].getAttribute('aria-checked')).toBe('true')

    fireEvent.click(items[2])
    expect(store.getState().sequencer.selected).toBe(2)
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('searches the list, and Enter picks the first found', async () => {
    const { store } = await setup()
    fireEvent.click(screen.getByRole('button', { name: 'All patterns' }))
    const search = screen.getByRole('searchbox', { name: 'Search patterns' })
    fireEvent.change(search, { target: { value: 'cho' } })
    expect(screen.getAllByRole('menuitemradio').map(i => i.textContent)).toEqual(['Chorus'])
    fireEvent.change(search, { target: { value: 'zzz' } })
    expect(screen.getByText('No patterns match')).toBeTruthy()

    fireEvent.change(search, { target: { value: 'chorus' } })
    fireEvent.keyDown(search, { key: 'Enter' })
    expect(store.getState().sequencer.selected).toBe(2)
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('drags a pattern out of the list, closing it', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'All patterns' }))
    const dropped: DragItem[] = []
    const off = onSampleDrag(({ type, item }) => { if(type === 'drop') dropped.push(item) })

    fireEvent.mouseDown(screen.getByRole('menuitemradio', { name: 'Chorus' }), { button: 0, clientX: 10, clientY: 10 })
    act(() => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: 200, clientY: 300 })) })
    expect(screen.queryByRole('menu')).toBeNull()
    act(() => { window.dispatchEvent(new MouseEvent('mouseup', { clientX: 200, clientY: 300 })) })
    off()
    expect(dropped).toEqual([{ kind: 'pattern', patternId: 2, label: 'Chorus' }])
  })

  it('closes the list on Escape', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'All patterns' }))
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('menu')).toBeNull()
  })
})

describe('the Drum Sequencer', () => {
  it('shows every pattern, and the sample rows only, with no Piano Roll key', async () => {
    await setup(Sequencer, View.Sequencer)
    expect(screen.getByRole('button', { name: 'Beat' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Verse' })).toBeTruthy()
    expect(screen.getAllByRole('button', { name: /kick\.wav, step/ })).toHaveLength(16)
    expect(screen.queryByRole('button', { name: 'Pad notes' })).toBeNull()
    expect(screen.queryByRole('button', { name: /piano roll/i })).toBeNull()
    expect((screen.getByRole('spinbutton', { name: 'Bars' }) as HTMLInputElement).value).toBe('1')
  })

  it('makes patterns', async () => {
    await setup(Sequencer, View.Sequencer)
    fireEvent.click(screen.getByRole('button', { name: 'New pattern' }))
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1 }]))
  })

  it('lights a step straight away, as a note at the row’s step pitch', async () => {
    const { store } = await setup(Sequencer, View.Sequencer)
    const key = () => screen.getAllByRole('button', { name: /kick\.wav, step/ })[4]
    fireEvent.mouseDown(key(), { button: 0 })
    expect(key().getAttribute('aria-pressed')).toBe('true')
    expect(store.getState().sequencer.patterns[0].notes[0]).toEqual(steps(0, 4))
    expect(calls('set_pattern_step')).toEqual([{ patternId: 0, rowId: 0, step: 4, on: true, gesture: null }])
  })

  it('shows a sample row’s notes that aren’t plain steps as a picture, and changes nothing', async () => {
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Beat', notes: { 0: [...steps(0), note(60, 2.5)] } })] })
    await setup(Sequencer, View.Sequencer)
    expect(screen.queryByRole('button', { name: /kick\.wav, step/ })).toBeNull()
    expect(screen.getByRole('img', { name: 'kick.wav notes' })).toBeTruthy()
    fireEvent.click(screen.getByRole('img', { name: 'kick.wav notes' }))
    expect(mockInvoke.mock.calls.map(([command]) => command)).not.toContain('set_pattern_notes')
  })

  it('draws its step keys in the style the preferences pick', async () => {
    const { store, container } = await setup(Sequencer, View.Sequencer)
    const steps = () => container.querySelector('.seq-steps') as HTMLElement
    // a TR-909's by default
    expect(steps().dataset.keys).toBe('tr909')
    const lit = screen.getAllByRole('button', { name: /kick\.wav, step/ })
    expect(lit[0].classList.contains('is-lit')).toBe(true)
    expect(lit[1].classList.contains('is-lit')).toBe(false)
    // in beats of four, coloured by their place in the bar
    expect(Array.from(container.querySelectorAll('.seq-beat')).map(b => b.classList.contains(`is-beat-0`))).toEqual([true, false, false, false])

    act(() => { store.dispatch(setPreferences({ ...store.getState().appearance.preferences, stepKeys: 'tr808' })) })
    expect(steps().dataset.keys).toBe('tr808')
  })

  it('replaces the sample of a row a sample is dropped on', async () => {
    await setup(Sequencer, View.Sequencer)
    const step = screen.getAllByRole('button', { name: /kick\.wav, step/ })[3]
    await dragOnto({ kind: 'sample', path: '/drums/snare.wav' }, step)
    expect(calls('replace_row_sample')).toEqual([{ rowId: 0, path: '/drums/snare.wav' }])
    expect(calls('add_sample_row')).toEqual([])
  })

  it('adds a row for a sample dropped anywhere else', async () => {
    await setup(Sequencer, View.Sequencer)
    await dragOnto({ kind: 'sample', path: '/drums/snare.wav' }, screen.getByText('Drop more samples here to add rows'))
    expect(calls('add_sample_row')).toEqual([{ path: '/drums/snare.wav' }])
    expect(calls('replace_row_sample')).toEqual([])
  })

  it('shows which row a sample dragged over it would replace', async () => {
    await setup(Sequencer, View.Sequencer)
    const step = screen.getAllByRole('button', { name: /kick\.wav, step/ })[3]
    document.elementFromPoint = () => step
    beginSampleDrag({ kind: 'sample', path: '/drums/snare.wav' }, { button: 0, clientX: 0, clientY: 0 })
    act(() => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: 40, clientY: 40 })) })

    expect(screen.getByText('Replace sample with snare.wav')).toBeTruthy()
    expect(screen.getByText('Let go to replace kick.wav’s sample with snare.wav')).toBeTruthy()

    act(() => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })) })
    act(() => { window.dispatchEvent(new MouseEvent('mouseup', { clientX: 40, clientY: 40 })) })
    expect(screen.queryByText('Replace sample with snare.wav')).toBeNull()
  })
})
