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
import appearanceReducer from '../../state/slices/appearanceSlice'
import sequencerReducer from '../../state/slices/sequencerSlice'
import { DragItem, beginSampleDrag } from '../../state/sampleDrag'
import { Note, SequencerState, Synth } from '../../types/sequencer'
import { View } from '../../types/ui'
import NoteSequencer from './NoteSequencer'
import Sequencer from '../sequencer/Sequencer'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

const note = (pitch: number, start: number): Note => ({ pitch, start, length: 2, velocity: 0.8 })

const synth = (id: number, name: string, change: Partial<Synth> = {}): Synth => ({
  id, pluginId: `vst3:${id}`, name, vendor: 'Acme', volume: 1, muted: false, missing: false, hasEditor: true, ...change,
})

// a drum pattern (0) and two note patterns (1, 2), with Pad's notes in 1
const makeSequencer = (change: Partial<SequencerState> = {}): SequencerState => ({
  rows: [{ id: 0, path: '/drums/kick.wav', label: null, volume: 1, pan: 0, muted: false }],
  synths: [synth(7, 'Pad'), synth(8, 'Gone', { missing: true })],
  patterns: [
    { id: 0, name: 'Beat', length: 16, steps: { 0: [0] } },
    { id: 1, name: 'Verse', kind: 'note', length: 16, steps: {}, notes: { 7: [note(60, 0), note(64, 4)] } },
    { id: 2, name: 'Chorus', kind: 'note', length: 64, steps: {}, notes: {} },
  ],
  folders: [],
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
    if(command === 'add_synth') {
      sequencer = { ...sequencer, synths: [...sequencer.synths, synth(9, 'Keys')] }
      return { synthId: 9, sequencer }
    }
    if(command === 'remove_synth') {
      sequencer = { ...sequencer, synths: sequencer.synths.filter(s => s.id !== args.synthId) }
      return sequencer
    }
    if(command === 'set_synth_muted' || command === 'set_pattern_bars') return sequencer
    if(command === 'add_pattern') {
      sequencer = { ...sequencer, patterns: [...sequencer.patterns, { id: 5, name: null, kind: 'note', length: 16, steps: {}, notes: {} }] }
      return { id: 5, sequencer }
    }
    return undefined
  })
})

describe('the Note Sequencer', () => {
  it('has a row per synth, a missing one greyed, and only note patterns along the top', async () => {
    await setup()

    expect(screen.getByRole('button', { name: 'Verse' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Chorus' })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Beat' })).toBeNull()
    // no drum steps, nor a Piano Roll key: the rows' notes open it
    expect(screen.queryByRole('button', { name: /step/ })).toBeNull()

    expect(screen.getByRole('button', { name: 'Pad notes' }).getAttribute('title')).toMatch(/^2 notes\./)
    expect(screen.getByText('Not installed')).toBeTruthy()
    expect((screen.getByText('Gone').closest('button') as HTMLButtonElement).disabled).toBe(true)
    expect(screen.getByRole('button', { name: 'Gone notes' }).getAttribute('title')).toMatch(/^No notes yet/)
  })

  it('opens the Piano Roll on a synth’s notes in the pattern', async () => {
    const { store } = await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Chorus' }))
    fireEvent.click(screen.getByRole('button', { name: 'Gone notes' }))

    const state = store.getState()
    expect(selectFocusedView(state as never)).toBe(View.PianoRoll)
    expect(state.sequencer.selected.note).toBe(2)
    expect(state.sequencer.pianoRollSynthId).toBe(8)
  })

  it('opens a synth’s own interface from its name, where Preferences say', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Pad' }))
    await waitFor(() => expect(calls('open_effect_window')).toEqual([{ slotId: 7 }]))
  })

  it('mutes and removes a synth', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Mute Pad' }))
    await waitFor(() => expect(calls('set_synth_muted')).toEqual([{ synthId: 7, muted: true }]))

    fireEvent.click(screen.getByRole('button', { name: 'Remove Pad' }))
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Pad notes' })).toBeNull())
    expect(calls('remove_synth')).toEqual([{ synthId: 7 }])
  })

  it('adds an instrument dropped anywhere on it, and nothing else', async () => {
    const { container } = await setup()
    await dragOnto({ kind: 'sample', path: '/x.wav' }, container.firstElementChild as Element)
    await dragOnto({ kind: 'effect', pluginId: 'vst3:verb', label: 'Verb' }, container.firstElementChild as Element)
    expect(mockInvoke.mock.calls.map(([command]) => command)).not.toContain('add_synth')

    // on the pattern strip, not just the rows
    await dragOnto({ kind: 'instrument', pluginId: 'vst3:keys', label: 'Keys' }, screen.getByRole('button', { name: 'Verse' }))
    expect(calls('add_synth')).toEqual([{ pluginId: 'vst3:keys' }])
    expect(await screen.findByRole('button', { name: 'Keys notes' })).toBeTruthy()
  })

  it('asks for an instrument while it has no synths', async () => {
    sequencer = makeSequencer({ synths: [] })
    await setup()
    expect(screen.getByText('Drag an instrument here')).toBeTruthy()
  })

  it('offers to make a note pattern when there’s none', async () => {
    sequencer = makeSequencer({ patterns: [makeSequencer().patterns[0]] })
    await setup()
    expect(screen.queryByRole('button', { name: 'Pad notes' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Make one' }))
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1, kind: 'note' }]))
    expect(await screen.findByRole('button', { name: 'Pad notes' })).toBeTruthy()
  })

  it('sets a note pattern’s length as a number of bars, 1 to 64', async () => {
    await setup()
    const bars = screen.getByRole('spinbutton', { name: 'Bars' }) as HTMLInputElement
    expect(bars.value).toBe('1')

    fireEvent.change(bars, { target: { value: '99' } })
    fireEvent.keyDown(bars, { key: 'Enter' })
    fireEvent.change(bars, { target: { value: '12' } })
    fireEvent.keyDown(bars, { key: 'Enter' })
    await waitFor(() => expect(calls('set_pattern_bars')).toEqual([{ patternId: 1, bars: 12 }]))
  })

  it('plays and stops the pattern from its strip', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Play the pattern' }))
    await waitFor(() => expect(calls('toggle_pattern')).toEqual([{ patternId: 1 }]))
  })
})

describe('the Drum Sequencer', () => {
  it('shows drum patterns only, with 1, 2 or 4 bars and no Piano Roll key', async () => {
    await setup(Sequencer, View.Sequencer)
    expect(screen.getByRole('button', { name: 'Beat' })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Verse' })).toBeNull()
    expect(screen.getAllByRole('button', { name: /kick\.wav, step/ })).toHaveLength(16)
    expect(screen.queryByRole('button', { name: /piano roll/i })).toBeNull()
    expect(screen.queryByRole('spinbutton', { name: 'Bars' })).toBeNull()
    expect(screen.getByRole('button', { name: '4' })).toBeTruthy()
  })

  it('makes drum patterns', async () => {
    await setup(Sequencer, View.Sequencer)
    fireEvent.click(screen.getByRole('button', { name: 'New pattern' }))
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1, kind: 'drum' }]))
  })
})
