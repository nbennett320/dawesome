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
import sequencerReducer, { sequencerSlice } from '../../state/slices/sequencerSlice'
import { DragItem, beginSampleDrag, onSampleDrag } from '../../state/sampleDrag'
import { Note, Pattern, Row, SequencerState, withStep, withTrim } from '../../types/sequencer'
import {
  freshRev, pattern, sampleRow, sequencerOf, steps, synthRow,
} from '../../types/sequencer.fixture'
import { View } from '../../types/ui'
import PatternSequencer from './PatternSequencer'
import { GRID_LEFT, barX, stepX } from './stepGrid'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))

// the rows' renders, counted: the row itself, memoised as it is
let rowRenders = 0
vi.mock('./PatternRow', async importOriginal => {
  const actual = await importOriginal<typeof import('./PatternRow')>()
  type Props = React.ComponentProps<typeof actual.default>
  const Row = (actual.default as unknown as { type: React.ComponentType<Props> }).type
  const Counted = (props: Props) => {
    rowRenders += 1
    return React.createElement(Row, props)
  }
  return { ...actual, default: React.memo(Counted) }
})

const mockInvoke = invoke as Mock

const note = (pitch: number, start: number): Note => ({ pitch, start, length: 2, velocity: 0.8 })

// the kick (a sample row, 0), then Pad and Gone (synth rows 3 and 1, in
// slots 103 and 101), then the snare (sample row 2). patterns Beat (the
// kick's first step), Verse (Pad's notes), Chorus (empty, 4 bars) and Keys
// (Pad's steps)
const makeSequencer = (change: Partial<SequencerState> = {}): SequencerState => sequencerOf({
  rows: [
    sampleRow(0, { path: '/drums/kick.wav' }),
    synthRow(3, { name: 'Pad' }),
    synthRow(1, { name: 'Gone', missing: true }),
    sampleRow(2, { path: '/drums/snare.wav' }),
  ],
  patterns: [
    pattern(0, { name: 'Beat', notes: { 0: steps(0) } }),
    pattern(1, { name: 'Verse', notes: { 3: [note(60, 0), note(64, 4)] } }),
    pattern(2, { name: 'Chorus', length: 64 }),
    pattern(3, { name: 'Keys', notes: { 3: steps(0, 4) } }),
  ],
  ...change,
})

const stopped = { playing: false, patternId: null, startTimeMs: null, beatIntervalMs: 500, beatsPerBar: 4 }

let sequencer: SequencerState
// what get_playlist_transport answers
let transport: object

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

const setup = async () => {
  const store = makeStore()
  store.dispatch(openTab(View.PatternSequencer))
  const { container } = render(<Provider store={store}><PatternSequencer /></Provider>)
  // the sequencer is asked for when the tab opens
  await act(async () => {})
  return { store, container }
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command).map(([, args]) => args)
const keysOf = (name: string) => screen.queryAllByRole('button', { name: new RegExp(`^${name}, step`) })
const shownRows = (container: HTMLElement) =>
  Array.from(container.querySelectorAll<HTMLElement>('[data-row-id]')).map(row => Number(row.dataset.rowId))
const pick = (name: string) => fireEvent.click(screen.getByRole('button', { name }))

// a drag from the sidebar, let go over `target`
const dragOnto = async (item: DragItem, target: Element) => {
  document.elementFromPoint = () => target
  beginSampleDrag(item, { button: 0, clientX: 0, clientY: 0 })
  window.dispatchEvent(new MouseEvent('mousemove', { clientX: 40, clientY: 40 }))
  window.dispatchEvent(new MouseEvent('mouseup', { clientX: 40, clientY: 40 }))
  await act(async () => {})
}

// the scroller the rows are in is this wide (jsdom lays nothing out)
const VIEW_WIDTH = 1200
let widthWas: PropertyDescriptor | undefined
beforeAll(() => {
  widthWas = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth')
  Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => VIEW_WIDTH })
})
afterAll(() => {
  if(widthWas) Object.defineProperty(HTMLElement.prototype, 'clientWidth', widthWas)
})

beforeEach(() => {
  mockInvoke.mockReset()
  rowRenders = 0
  sequencer = makeSequencer()
  transport = stopped
  const row = (id: unknown) => sequencer.rows.find(r => r.id === id) as Row
  mockInvoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
    if(command === 'get_sequencer') return sequencer
    if(command === 'get_playlist_transport') return transport
    if(command === 'add_synth_row') {
      sequencer = { ...sequencer, rows: [...sequencer.rows, synthRow(9, { name: 'Keys' })] }
      return { rowId: 9, sequencer }
    }
    if(command === 'add_sample_row') {
      sequencer = { ...sequencer, rows: [...sequencer.rows, sampleRow(9, { path: args.path as string })] }
      return { rowId: 9, sequencer }
    }
    if(command === 'remove_row') {
      sequencer = { ...sequencer, rows: sequencer.rows.filter(r => r.id !== args.rowId) }
      return sequencer
    }
    if(command === 'move_row') {
      const moved = sequencer.rows.filter(r => r.id !== args.rowId)
      moved.splice(args.index as number, 0, row(args.rowId))
      sequencer = { ...sequencer, rows: moved }
      return sequencer
    }
    // the step as the engine sets it (grown, or trimmed), at a rev of its own
    if(command === 'set_pattern_step') {
      const { patternId, rowId, step, on, trim } = args as {
        patternId: number, rowId: number, step: number, on: boolean, trim: boolean,
      }
      const set = (p: Pattern) => withStep(p, row(rowId), step, on, 16)
      const patterns = sequencer.patterns.map(p => (p.id === patternId
        ? { ...(trim ? withTrim(set(p), 16) : set(p)), rev: freshRev() }
        : p))
      sequencer = { ...sequencer, patterns }
      return { pattern: patterns.find(p => p.id === patternId) }
    }
    // (changing nothing: each answers with what it would have changed)
    if(command === 'set_row_step_pitch') return sequencer
    if(['set_row_muted', 'set_row_sample_pitch', 'set_row_gated'].includes(command)) return { row: row(args.rowId) }
    if(command === 'set_pattern_bars') return { pattern: sequencer.patterns.find(p => p.id === args.patternId) }
    if(command === 'add_pattern') {
      sequencer = { ...sequencer, patterns: [...sequencer.patterns, pattern(5)] }
      return { id: 5, sequencer }
    }
    return undefined
  })
})

describe('the Pattern Sequencer', () => {
  it('has every row in one list, samples and synths in their order, and every pattern along the top', async () => {
    const { container } = await setup()
    expect(shownRows(container)).toEqual([0, 3, 1, 2])
    expect(screen.getByRole('button', { name: 'Beat' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Chorus' })).toBeTruthy()
    // the same controls on both kinds: names (a sample's without its extension)
    expect(screen.getByRole('button', { name: 'Mute kick' })).toBeTruthy()
    expect(screen.getByRole('slider', { name: 'Pad pan' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Remove Gone' })).toBeTruthy()
    expect(screen.getByText('Not installed')).toBeTruthy()
    expect((screen.getByText('Gone').closest('button') as HTMLButtonElement).disabled).toBe(true)
  })

  it('shows steps for plain steps and a picture for other notes, a synth row’s as well as a sample row’s', async () => {
    await setup()
    // Beat: the kick's first step lit, the synth rows with no notes as steps
    expect(keysOf('kick')[0].getAttribute('aria-pressed')).toBe('true')
    expect(keysOf('kick')[1].getAttribute('aria-pressed')).toBe('false')
    expect(keysOf('Pad').length).toBeGreaterThan(0)

    pick('Verse')
    expect(screen.getByRole('button', { name: 'Pad notes' }).getAttribute('title')).toMatch(/^2 notes\./)
    expect(keysOf('Pad').every(key => key.getAttribute('aria-label')?.endsWith('past the end'))).toBe(true)

    // only 1-step notes at its step pitch: steps, lit
    pick('Keys')
    expect(screen.queryByRole('button', { name: 'Pad notes' })).toBeNull()
    expect(keysOf('Pad').filter(key => key.getAttribute('aria-pressed') === 'true').map(key => key.title)).toEqual(['Step 1', 'Step 5'])
  })

  it('opens the Piano Roll on a synth row’s notes in the pattern', async () => {
    const { store } = await setup()
    pick('Verse')
    pick('Pad notes')

    const state = store.getState()
    expect(selectFocusedView(state as never)).toBe(View.PianoRoll)
    expect(state.sequencer.selected).toBe(1)
    expect(state.sequencer.pianoRollRowId).toBe(3)
  })

  it('shows a sample row’s notes that aren’t plain steps as a picture, which opens the Piano Roll on them', async () => {
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Beat', notes: { 0: [...steps(0), note(60, 2.5)] } })] })
    const { store } = await setup()
    expect(keysOf('kick').every(key => key.getAttribute('aria-label')?.endsWith('past the end'))).toBe(true)
    pick('kick notes')

    const state = store.getState()
    expect(selectFocusedView(state as never)).toBe(View.PianoRoll)
    expect(state.sequencer.pianoRollRowId).toBe(0)
    expect(mockInvoke.mock.calls.map(([command]) => command)).not.toContain('set_pattern_notes')
  })

  it('opens the Piano Roll on a row showing plain steps from its right-click menu', async () => {
    const { store } = await setup()
    fireEvent.contextMenu(screen.getByText('kick'))
    fireEvent.click(screen.getAllByRole('menuitem').find(item => item.textContent === 'Edit in Piano Roll') as HTMLElement)

    const state = store.getState()
    expect(selectFocusedView(state as never)).toBe(View.PianoRoll)
    expect(state.sequencer.pianoRollRowId).toBe(0)
    expect(mockInvoke.mock.calls.map(([command]) => command)).not.toContain('set_pattern_notes')
  })

  it('opens a synth’s own interface from its name, by its slot, and plays a sample’s', async () => {
    await setup()
    pick('Pad')
    await waitFor(() => expect(calls('open_effect_window')).toEqual([{ slotId: 103 }]))
    fireEvent.click(screen.getByText('kick'))
    expect(calls('preview_row')).toEqual([{ rowId: 0 }])
  })

  it('mutes and removes rows by their ids', async () => {
    await setup()
    pick('Mute Pad')
    await waitFor(() => expect(calls('set_row_muted')).toEqual([{ rowId: 3, muted: true, gesture: null }]))

    pick('Remove Pad')
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Mute Pad' })).toBeNull())
    expect(calls('remove_row')).toEqual([{ rowId: 3 }])
  })

  it('lights a step straight away, as a note at the row’s step pitch, and paints a stroke', async () => {
    const { store } = await setup()
    const key = (n: number) => keysOf('snare')[n]
    fireEvent.mouseDown(key(4), { button: 0 })
    expect(key(4).getAttribute('aria-pressed')).toBe('true')
    expect(store.getState().sequencer.patterns[0].notes[2]).toEqual(steps(4))
    fireEvent.mouseEnter(key(5))
    fireEvent.mouseEnter(key(6))
    fireEvent.mouseUp(window)
    fireEvent.mouseEnter(key(7))
    expect(calls('set_pattern_step').map(args => (args as { step: number }).step)).toEqual([4, 5, 6])
    expect(calls('set_pattern_step')[0]).toEqual({ patternId: 0, rowId: 2, step: 4, on: true, gesture: null, trim: false })
  })

  it('shows a bar of greyed keys past the end, and grows the pattern to the one clicked, straight away', async () => {
    const { store } = await setup()
    const greyed = () => keysOf('kick').filter(key => key.getAttribute('aria-label')?.endsWith('past the end'))
    expect(greyed().map(key => key.title.split('.')[0])).toEqual(Array.from({ length: 16 }, (_, i) => `Step ${17 + i}`))
    expect(greyed().every(key => key.classList.contains('is-greyed') && key.getAttribute('aria-pressed') === 'false')).toBe(true)

    fireEvent.mouseDown(greyed()[4], { button: 0 })
    // in the same frame: the pattern 2 bars long, the step lit, a greyed bar after it
    expect(store.getState().sequencer.patterns[0].length).toBe(32)
    expect(keysOf('kick')[20].getAttribute('aria-pressed')).toBe('true')
    expect(greyed()[0].title).toMatch(/^Step 33\./)
    expect(calls('set_pattern_step')).toEqual([{ patternId: 0, rowId: 0, step: 20, on: true, gesture: null, trim: false }])
    await act(async () => {})
    expect(store.getState().sequencer.patterns[0].length).toBe(32)

    // cleared: the bar it leaves empty goes, in the same frame too
    fireEvent.mouseUp(window)
    fireEvent.mouseDown(keysOf('kick')[20], { button: 0 })
    expect(store.getState().sequencer.patterns[0].length).toBe(16)
    expect(greyed()[0].title).toMatch(/^Step 17\./)
    expect(calls('set_pattern_step')[1]).toEqual({ patternId: 0, rowId: 0, step: 20, on: false, gesture: null, trim: true })
    await act(async () => {})
    expect(store.getState().sequencer.patterns[0].length).toBe(16)
  })

  it('shows no greyed bar on a pattern as long as a pattern can be', async () => {
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Long', length: 2048 })] })
    await setup()
    expect(screen.queryAllByRole('button', { name: /past the end/ })).toEqual([])
  })

  it('makes the keys of the bars in view only, and more as it scrolls', async () => {
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Long', length: 2048, notes: { 0: steps(0, 820) } })] })
    await setup()
    const steps_ = () => keysOf('kick').map(key => Number(/step (\d+)/.exec(key.getAttribute('aria-label') ?? '')?.[1]))
    // 1200px from the start: bars 1 to 4 of 128
    expect(steps_()).toEqual(Array.from({ length: 64 }, (_, i) => i + 1))

    const scroller = screen.getByTestId('pattern-sequencer-rows')
    act(() => {
      scroller.scrollLeft = barX(50, 4) + 8 + GRID_LEFT
      fireEvent.scroll(scroller)
    })
    expect(steps_()[0]).toBe(49 * 16 + 1)
    expect(steps_()).toHaveLength(16 * 5)
    expect(keysOf('kick').find(key => key.title === 'Step 821')?.getAttribute('aria-pressed')).toBe('true')
    // each beat where the grid has it, so the bars line up row to row
    const beat = keysOf('kick')[0].closest('.seq-beat') as HTMLElement
    expect(beat.style.left).toBe(`${barX(49, 4)}px`)
  })

  it('moves the playing step along without rendering a row', async () => {
    const frames: FrameRequestCallback[] = []
    const raf = vi.spyOn(window, 'requestAnimationFrame').mockImplementation(f => frames.push(f))
    const now = vi.spyOn(Date, 'now').mockReturnValue(10_000)
    const { store, container } = await setup()
    const column = container.querySelector('.seq-now') as HTMLElement
    expect(column.hidden).toBe(true)

    // Beat plays from 10s, a step every 125ms
    act(() => { store.dispatch(sequencerSlice.actions.setRun({ patternId: 0, startTimeMs: 10_000, beatIntervalMs: 500, length: 16 })) })
    expect(column.hidden).toBe(false)
    expect(column.style.left).toBe(`${GRID_LEFT}px`)
    rowRenders = 0
    for(const step of [1, 2, 5, 15, 0]) {
      now.mockReturnValue(10_000 + (step === 0 ? 16 : step) * 125 + 1)
      act(() => { frames.splice(0).forEach(f => f(0)) })
      expect(column.style.transform).toBe(`translateX(${stepX(step, 4)}px)`)
    }
    expect(rowRenders).toBe(0)

    act(() => { store.dispatch(sequencerSlice.actions.setRun(null)) })
    expect(column.hidden).toBe(true)
    raf.mockRestore()
    now.mockRestore()
  })

  it('keeps the playing step on the step heard when the pattern grows while it plays', async () => {
    const frames: FrameRequestCallback[] = []
    const raf = vi.spyOn(window, 'requestAnimationFrame').mockImplementation(f => frames.push(f))
    const now = vi.spyOn(Date, 'now').mockReturnValue(10_000)
    const { store, container } = await setup()
    const column = container.querySelector('.seq-now') as HTMLElement
    const show = () => act(() => { frames.splice(0).forEach(f => f(0)) })

    // Beat (a bar) has played 53 steps, a step every 125ms: it's on step 5
    act(() => {
      store.dispatch(sequencerSlice.actions.setRun({ patternId: 0, startTimeMs: 10_000 - 53 * 125, beatIntervalMs: 500, length: 16 }))
    })
    show()
    expect(column.style.transform).toBe(`translateX(${stepX(5, 4)}px)`)

    // a greyed step grows it to two bars. the engine carries on from step 5
    // and counts from there: the column stays on it (not on step 21, the
    // 53rd step of 32)
    transport = { ...stopped, playing: true, patternId: 0, startTimeMs: 10_000 - 5 * 125, beatIntervalMs: 500, patternLength: 32 }
    fireEvent.mouseDown(keysOf('kick')[20], { button: 0 })
    fireEvent.mouseUp(window)
    await act(async () => {})
    expect(store.getState().sequencer.patterns[0].length).toBe(32)
    show()
    expect(column.style.transform).toBe(`translateX(${stepX(5, 4)}px)`)
    // and on into bar 2
    now.mockReturnValue(10_000 + 12 * 125 + 1)
    show()
    expect(column.style.transform).toBe(`translateX(${stepX(17, 4)}px)`)
    raf.mockRestore()
    now.mockRestore()
  })

  it('renders only the row whose step was clicked', async () => {
    await setup()
    rowRenders = 0
    fireEvent.mouseDown(keysOf('snare')[3], { button: 0 })
    expect(rowRenders).toBe(1)
  })

  it('counts the pattern’s bars on wheels, and sets them with ▲, ▼ and by typing, 1 to 128', async () => {
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Beat', length: 32 })] })
    await setup()
    expect(screen.getByRole('img', { name: 'Bars 002' })).toBeTruthy()

    pick('A bar longer')
    pick('A bar shorter')
    fireEvent.click(screen.getByRole('button', { name: 'Bars 002' }))
    const typed = screen.getByRole('spinbutton', { name: 'Bars' }) as HTMLInputElement
    expect(typed.value).toBe('2')
    fireEvent.change(typed, { target: { value: '129' } })
    fireEvent.keyDown(typed, { key: 'Enter' })
    fireEvent.click(screen.getByRole('button', { name: 'Bars 002' }))
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Bars' }), { target: { value: '99' } })
    fireEvent.keyDown(screen.getByRole('spinbutton', { name: 'Bars' }), { key: 'Enter' })
    // Escape: nothing
    fireEvent.click(screen.getByRole('button', { name: 'Bars 002' }))
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Bars' }), { target: { value: '7' } })
    fireEvent.keyDown(screen.getByRole('spinbutton', { name: 'Bars' }), { key: 'Escape' })

    await waitFor(() => expect(calls('set_pattern_bars')).toEqual([
      { patternId: 0, bars: 3 }, { patternId: 0, bars: 1 }, { patternId: 0, bars: 99 },
    ]))
  })

  it('has no ▼ below one bar, nor ▲ past 128', async () => {
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Beat' }), pattern(1, { name: 'Long', length: 2048 })] })
    await setup()
    expect((screen.getByRole('button', { name: 'A bar shorter' }) as HTMLButtonElement).disabled).toBe(true)
    pick('Long')
    expect(screen.getByRole('img', { name: 'Bars 128' })).toBeTruthy()
    expect((screen.getByRole('button', { name: 'A bar longer' }) as HTMLButtonElement).disabled).toBe(true)
  })

  it('shows only the rows with notes in the pattern while Used only is on', async () => {
    const { container, store } = await setup()
    pick('Used only')
    expect(shownRows(container)).toEqual([0])
    pick('Verse')
    expect(shownRows(container)).toEqual([3])
    pick('Used only')
    expect(shownRows(container)).toEqual([0, 3, 1, 2])
    expect(store.getState().sequencer.usedOnly).toBe(false)
  })

  it('keeps a row whose last step a stroke clears while Used only is on, until the stroke ends', async () => {
    // Beat: the kick's step 0, and the snare's 0 and 1
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Beat', notes: { 0: steps(0), 2: steps(0, 1) } })] })
    const { container } = await setup()
    pick('Used only')
    expect(shownRows(container)).toEqual([0, 2])

    // the kick's only step cleared: its row stays under the mouse, so the
    // stroke can't carry on into the snare's row as it slides up
    fireEvent.mouseDown(keysOf('kick')[0], { button: 0 })
    await act(async () => {})
    expect(shownRows(container)).toEqual([0, 2])
    fireEvent.mouseEnter(keysOf('kick')[1])
    expect(calls('set_pattern_step')).toEqual([{ patternId: 0, rowId: 0, step: 0, on: false, gesture: null, trim: true }])

    // and it goes once the stroke ends
    act(() => { fireEvent.mouseUp(window) })
    expect(shownRows(container)).toEqual([2])
    expect(keysOf('snare')[0].getAttribute('aria-pressed')).toBe('true')
  })

  it('drags a row of either kind to any place among them', async () => {
    const { container, store } = await setup()
    // (jsdom has no layout: every row's middle is at 0, so above them all is
    // the first gap)
    fireEvent.mouseDown(container.querySelector('[data-row-id="1"] [title="Drag to move the row"]') as Element, { button: 0 })
    act(() => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: 0, clientY: -5 })) })
    act(() => { window.dispatchEvent(new MouseEvent('mouseup', { clientX: 0, clientY: -5 })) })
    await act(async () => {})
    expect(calls('move_row')).toEqual([{ rowId: 1, index: 0 }])
    expect(store.getState().sequencer.rows.map(r => r.id)).toEqual([1, 0, 3, 2])
    expect(shownRows(container)).toEqual([1, 0, 3, 2])
  })

  it('replaces a row’s sample, or its plugin, dropped on a row of its kind', async () => {
    await setup()
    await dragOnto({ kind: 'sample', path: '/drums/rim.wav' }, keysOf('snare')[3])
    await dragOnto({ kind: 'instrument', pluginId: 'vst3:keys', label: 'Keys' }, screen.getByText('Pad'))
    expect(calls('replace_row_sample')).toEqual([{ rowId: 2, path: '/drums/rim.wav' }])
    expect(calls('replace_row_synth')).toEqual([{ rowId: 3, pluginId: 'vst3:keys' }])
    expect(calls('add_sample_row')).toEqual([])
    expect(calls('add_synth_row')).toEqual([])
  })

  it('adds a row for one dropped on a row of the other kind, or anywhere else, and nothing else', async () => {
    const { container } = await setup()
    await dragOnto({ kind: 'sample', path: '/drums/rim.wav' }, keysOf('Pad')[2])
    await dragOnto({ kind: 'instrument', pluginId: 'vst3:keys', label: 'Keys' }, screen.getByRole('button', { name: 'Verse' }))
    await dragOnto({ kind: 'effect', pluginId: 'vst3:verb', label: 'Verb' }, container.firstElementChild as Element)
    expect(calls('add_sample_row')).toEqual([{ path: '/drums/rim.wav' }])
    expect(calls('add_synth_row')).toEqual([{ pluginId: 'vst3:keys' }])
    expect(calls('replace_row_sample')).toEqual([])
    expect(await screen.findByRole('button', { name: 'Mute Keys' })).toBeTruthy()
  })

  it('says what a drop would do: replace on a row of its kind, add on another', async () => {
    await setup()
    const over = (target: Element) => {
      document.elementFromPoint = () => target
      act(() => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: 40, clientY: 40 })) })
    }
    beginSampleDrag({ kind: 'sample', path: '/drums/rim.wav' }, { button: 0, clientX: 0, clientY: 0 })
    over(keysOf('snare')[3])
    expect(screen.getByText('Replace sample with rim.wav')).toBeTruthy()
    expect(screen.getByText('Let go to replace snare’s sample with rim.wav')).toBeTruthy()
    over(keysOf('Pad')[3])
    expect(screen.queryByText('Replace sample with rim.wav')).toBeNull()
    expect(screen.getByText('Let go to add rim.wav')).toBeTruthy()

    act(() => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })) })
    act(() => { window.dispatchEvent(new MouseEvent('mouseup', { clientX: 40, clientY: 40 })) })
    expect(screen.getByText('Drop more samples or instruments here to add rows')).toBeTruthy()
  })

  it('asks for samples or instruments while it has no rows', async () => {
    sequencer = makeSequencer({ rows: [] })
    await setup()
    expect(screen.getByText('Drag samples or instruments here from the sidebar. Each one becomes a row')).toBeTruthy()
  })

  it('sets a row’s step pitch, sample pitch and gate from its right-click menu', async () => {
    await setup()
    const menu = (name: string) => {
      fireEvent.contextMenu(screen.getByText(name))
      return screen.getAllByRole('menuitem').map(item => item.textContent)
    }
    // (by what it says: jsdom names an item with a tooltip by the tooltip)
    const choose = (name: string, item: string) => {
      menu(name)
      fireEvent.click(screen.getAllByRole('menuitem').find(each => each.textContent === item) as HTMLElement)
    }
    const type = (label: string, text: string) => {
      const field = screen.getByLabelText(label) as HTMLInputElement
      fireEvent.change(field, { target: { value: text } })
      fireEvent.keyDown(field, { key: 'Enter' })
    }

    expect(menu('Pad')).toEqual(['Edit in Piano Roll', 'Rename…', 'Step pitch…', 'Remove'])
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(menu('kick')).toEqual(['Edit in Piano Roll', 'Rename…', 'Step pitch…', 'Sample pitch…', 'Gated', 'Remove'])
    fireEvent.keyDown(window, { key: 'Escape' })
    choose('kick', 'Step pitch…')
    expect((screen.getByLabelText('Step pitch') as HTMLInputElement).value).toBe('C4')
    type('Step pitch', 'D4')
    choose('kick', 'Sample pitch…')
    type('Sample pitch', '72')
    choose('kick', 'Gated')
    // what isn't a pitch, or the pitch it is: nothing
    choose('snare', 'Step pitch…')
    type('Step pitch', 'H2')
    choose('snare', 'Step pitch…')
    type('Step pitch', 'C4')

    expect(calls('set_row_step_pitch')).toEqual([{ rowId: 0, pitch: 62 }])
    expect(calls('set_row_sample_pitch')).toEqual([{ rowId: 0, pitch: 72 }])
    expect(calls('set_row_gated')).toEqual([{ rowId: 0, gated: true }])
  })

  it('draws its step keys in the style the preferences pick', async () => {
    const { store, container } = await setup()
    const list = () => container.querySelector('[data-keys]') as HTMLElement
    // a TR-909's by default, for every row at once
    expect(list().dataset.keys).toBe('tr909')
    // in beats of four, coloured by their place in the bar
    const beats = Array.from(container.querySelectorAll('[data-row-id="0"] .seq-beat')).slice(0, 4)
    expect(beats.map(b => b.classList.contains('is-beat-0'))).toEqual([true, false, false, false])

    act(() => { store.dispatch(setPreferences({ ...store.getState().appearance.preferences, stepKeys: 'tr808' })) })
    expect(list().dataset.keys).toBe('tr808')
  })
})

describe('the Pattern Sequencer’s strip', () => {
  it('offers to make a pattern when there’s none', async () => {
    sequencer = makeSequencer({ patterns: [] })
    await setup()
    expect(keysOf('kick')).toEqual([])
    pick('Make one')
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1 }]))
    expect((await screen.findAllByRole('button', { name: /^kick, step/ })).length).toBeGreaterThan(0)
  })

  it('makes new patterns a bar long', async () => {
    sequencer = makeSequencer({ patterns: [pattern(0, { name: 'Long', length: 64 })] })
    await setup()
    pick('New pattern')
    await waitFor(() => expect(calls('add_pattern')).toEqual([{ bars: 1 }]))
  })

  it('has no Play key of its own: the top bar plays its pattern', async () => {
    await setup()
    expect(screen.queryByRole('button', { name: 'Play the pattern' })).toBeNull()
  })

  it('lists every pattern under ▾, to pick one', async () => {
    const { store } = await setup()
    pick('All patterns')
    const items = screen.getAllByRole('menuitemradio')
    expect(items.map(i => i.textContent)).toEqual(['✓Beat', 'Verse', 'Chorus', 'Keys'])
    expect(items[0].getAttribute('aria-checked')).toBe('true')

    fireEvent.click(items[2])
    expect(store.getState().sequencer.selected).toBe(2)
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('searches the list, and Enter picks the first found', async () => {
    const { store } = await setup()
    pick('All patterns')
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
    pick('All patterns')
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
    pick('All patterns')
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('menu')).toBeNull()
  })
})
