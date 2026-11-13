import type { Mock } from 'vitest'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import sequencerReducer, {
  clearPendingNoteSends, sequencerSlice, setPatternNotes, setPatternStep, setSequencer,
} from './slices/sequencerSlice'
import { dropHeldTrim } from './pendingTrim'
import { beginGesture, resetGestures } from './gesture'
import { pattern, sampleRow, sequencerOf, step, steps } from '../types/sequencer.fixture'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock
const { setRun } = sequencerSlice.actions

const BAR = 16
// a step at 120 bpm
const STEP_MS = 125
const START = 1_000_000

// patterns 0 and 1, each grown from a bar (their floor) to two by the
// kick's step 20, and the kick's step 0
const makeStore = () => {
  const store = configureStore({ reducer: { sequencer: sequencerReducer } })
  store.dispatch(setSequencer(sequencerOf({
    rows: [sampleRow(1), sampleRow(2)],
    patterns: [0, 1].map(id => pattern(id, { length: 32, floor: BAR, notes: { 1: steps(0, 20) } })),
  })))
  return store
}
type Store = ReturnType<typeof makeStore>

const lengthShown = (store: Store, id = 0) => store.getState().sequencer.patterns.find(p => p.id === id)?.length
const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command).map(([, args]) => args)

// a pattern playing on its own, its cursor on `step` now
const play = (store: Store, step: number, patternId = 0) => {
  store.dispatch(setRun({ patternId, startTimeMs: Date.now() - step * STEP_MS, beatIntervalMs: STEP_MS * 4 }))
}
// the time `steps` steps take, a frame at a time
const pass = (n: number) => vi.advanceTimersByTime(n * STEP_MS)

// the kick's step 20 cleared, leaving bar 2 empty
const clear = (store: Store, patternId = 0) => store.dispatch(setPatternStep(patternId, 1, 20, false, BAR))

beforeEach(() => {
  vi.useFakeTimers()
  vi.setSystemTime(START)
  resetGestures()
  clearPendingNoteSends()
  dropHeldTrim()
  mockInvoke.mockReset()
  // (the engine's answers don't matter here: what's shown is what was sent)
  mockInvoke.mockImplementation(() => new Promise(() => {}))
})

afterEach(() => {
  vi.useRealTimers()
})

describe('a clear’s trim', () => {
  it('is sent with the clear, and shown straight away, while the pattern isn’t playing', () => {
    const store = makeStore()
    clear(store)
    expect(lengthShown(store)).toBe(BAR)
    expect(calls('set_pattern_step')).toEqual([{ patternId: 0, rowId: 1, step: 20, on: false, gesture: null, trim: true }])
  })

  it('is sent with the clear while another pattern plays, or the playlist (no run)', () => {
    const store = makeStore()
    play(store, 24, 1)
    clear(store)
    expect(lengthShown(store)).toBe(BAR)
    expect(calls('set_pattern_step')[0]).toMatchObject({ trim: true })
  })

  it('is sent with the clear while the cursor is in the bars left', () => {
    const store = makeStore()
    play(store, 4)
    clear(store)
    expect(lengthShown(store)).toBe(BAR)
    expect(calls('set_pattern_step')[0]).toMatchObject({ trim: true })
  })
})

describe('a trim held for the play cursor', () => {
  it('waits while the cursor is in the bars it takes away, and goes with the clear’s gesture once it’s gone round', () => {
    const store = makeStore()
    play(store, 22)
    const gesture = beginGesture()
    clear(store)
    expect(lengthShown(store)).toBe(32)
    expect(calls('set_pattern_step')).toEqual([{ patternId: 0, rowId: 1, step: 20, on: false, gesture, trim: false }])

    // still in bar 2
    pass(9)
    expect(calls('trim_pattern')).toEqual([])
    // round to the start
    pass(2)
    expect(calls('trim_pattern')).toEqual([{ patternId: 0, gesture }])
    pass(40)
    expect(calls('trim_pattern')).toHaveLength(1)
  })

  it('is sent when the pattern stops', () => {
    const store = makeStore()
    play(store, 22)
    clear(store)
    store.dispatch(setRun(null))
    pass(1)
    expect(calls('trim_pattern')).toHaveLength(1)
  })

  it('is called off when notes go back in those bars', () => {
    const store = makeStore()
    play(store, 22)
    clear(store)
    store.dispatch(setPatternStep(0, 2, 28, true, BAR))
    pass(20)
    expect(calls('trim_pattern')).toEqual([])
    expect(lengthShown(store)).toBe(32)
  })

  it('gives a deletion made with a key a gesture, the trim’s', () => {
    const store = makeStore()
    play(store, 22)
    // (the Piano Roll's Delete)
    store.dispatch(setPatternNotes(0, 1, [step(0)], BAR, true))
    const [sent] = calls('set_pattern_notes')
    expect(sent).toMatchObject({ trim: false, gesture: expect.any(Number) })
    pass(11)
    expect(calls('trim_pattern')).toEqual([{ patternId: 0, gesture: sent.gesture }])
  })

  it('is forgotten when dropped (an undo, another project)', () => {
    const store = makeStore()
    play(store, 22)
    clear(store)
    dropHeldTrim()
    pass(20)
    expect(calls('trim_pattern')).toEqual([])
  })

  it('is sent before another pattern’s is held', () => {
    const store = makeStore()
    play(store, 22)
    clear(store)
    play(store, 22, 1)
    clear(store, 1)
    expect(calls('trim_pattern')).toEqual([{ patternId: 0, gesture: expect.any(Number) }])
    pass(11)
    expect(calls('trim_pattern').map(args => args.patternId)).toEqual([0, 1])
  })
})
