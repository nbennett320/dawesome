import type { Mock } from 'vitest'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import sequencerReducer, {
  clearPendingNoteSends, flushNoteSends, setPatternNotes, setSequencer,
} from './sequencerSlice'
import { beginGesture, endGesture, resetGestures } from '../gesture'
import { Note, SequencerState } from '../../types/sequencer'
import { pattern, sequencerOf } from '../../types/sequencer.fixture'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

const note = (pitch: number, start = 0): Note => ({ pitch, start, length: 1, velocity: 0.8 })

// row 7's notes in pattern 2
const sequencer = (notes: Note[]): SequencerState =>
  sequencerOf({ patterns: [pattern(2, { notes: notes.length ? { 7: notes } : {} })] })

const makeStore = () => {
  const store = configureStore({ reducer: { sequencer: sequencerReducer } })
  store.dispatch(setSequencer(sequencer([])))
  return store
}

const notesShown = (store: ReturnType<typeof makeStore>) => store.getState().sequencer.patterns[0].notes[7] ?? []

const sends = () => mockInvoke.mock.calls.filter(([command]) => command === 'set_pattern_notes').map(([, args]) => args)

// answers to set_pattern_notes, let go one at a time
let answers: Array<() => void>

beforeEach(() => {
  vi.useFakeTimers()
  resetGestures()
  clearPendingNoteSends()
  answers = []
  mockInvoke.mockReset()
  mockInvoke.mockImplementation((command: string, args: { notes: Note[] }) => {
    if(command !== 'set_pattern_notes') return Promise.resolve(sequencer([]))
    return new Promise(resolve => { answers.push(() => resolve(sequencer(args.notes))) })
  })
})

afterEach(() => {
  vi.runOnlyPendingTimers()
  vi.useRealTimers()
})

describe('setPatternNotes', () => {
  it('keeps each row’s notes apart, sending only the one changed', () => {
    const store = makeStore()
    store.dispatch(setPatternNotes(2, 8, [note(50)]))
    expect(store.getState().sequencer.patterns[0].notes).toEqual({ 8: [note(50)] })
    expect(sends()).toEqual([{ patternId: 2, rowId: 8, notes: [note(50)], gesture: null }])
  })

  it('puts the notes in order, by start then pitch, before showing and sending them', () => {
    const store = makeStore()
    store.dispatch(setPatternNotes(2, 7, [note(64, 4), note(60, 4), note(72, 0)]))
    const inOrder = [note(72, 0), note(60, 4), note(64, 4)]
    expect(notesShown(store)).toEqual(inOrder)
    expect(sends()).toEqual([{ patternId: 2, rowId: 7, notes: inOrder, gesture: null }])
  })

  it('shows the notes straight away, and sends the whole list with the drag’s gesture', () => {
    const store = makeStore()
    const gesture = beginGesture()
    store.dispatch(setPatternNotes(2, 7, [note(60)]))

    expect(notesShown(store)).toEqual([note(60)])
    expect(sends()).toEqual([{ patternId: 2, rowId: 7, notes: [note(60)], gesture }])
  })

  it('a change made without the mouse is a step of its own', () => {
    const store = makeStore()
    store.dispatch(setPatternNotes(2, 7, []))
    expect(sends()).toEqual([{ patternId: 2, rowId: 7, notes: [], gesture: null }])
  })

  it('sends at most every so often while dragging, the latest last, all in one gesture', () => {
    const store = makeStore()
    const gesture = beginGesture()
    store.dispatch(setPatternNotes(2, 7, [note(60)]))
    store.dispatch(setPatternNotes(2, 7, [note(61)]))
    store.dispatch(setPatternNotes(2, 7, [note(62)]))

    expect(notesShown(store)).toEqual([note(62)])
    expect(sends()).toHaveLength(1)

    // the mouse is let go: the gesture ends a moment later, but what was
    // changed during it is sent with it
    endGesture()
    flushNoteSends()
    expect(sends()).toEqual([
      { patternId: 2, rowId: 7, notes: [note(60)], gesture },
      { patternId: 2, rowId: 7, notes: [note(62)], gesture },
    ])
  })

  it('sends what’s waiting by itself, after a moment', () => {
    const store = makeStore()
    store.dispatch(setPatternNotes(2, 7, [note(60)]))
    store.dispatch(setPatternNotes(2, 7, [note(64)]))
    vi.advanceTimersByTime(40)
    expect(sends().map(args => args.notes)).toEqual([[note(60)], [note(64)]])
  })

  it('keeps only the latest send’s answer, so an earlier one can’t put back old notes', async () => {
    const store = makeStore()
    store.dispatch(setPatternNotes(2, 7, [note(60)]))
    vi.advanceTimersByTime(40)
    store.dispatch(setPatternNotes(2, 7, [note(70)]))
    expect(sends()).toHaveLength(2)

    // the second is answered first, then the first arrives late
    answers[1]()
    await vi.runAllTimersAsync()
    answers[0]()
    await vi.runAllTimersAsync()
    expect(notesShown(store)).toEqual([note(70)])
  })
})
