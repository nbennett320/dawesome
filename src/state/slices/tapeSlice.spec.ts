import type { Mock } from 'vitest'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer, { reduceLoadedProject, reduceProjectView } from './playlistSlice'
import tapeReducer, {
  changeTapeMachine,
  changeTrackTape,
  fetchTape,
  selectTapeEnabled,
  selectTapeMachine,
  selectTrackTape,
  setTape,
} from './tapeSlice'
import { clearPendingMixerSends, flushMixerSends } from './mixerSlice'
import { defaultTapeMachine, TapeSettings } from '../../types/tape'
import { LoadedProject, ProjectView } from '../../types/project'
import { MixerSettings } from '../../types/mixer'
import { beginGesture, resetGestures } from '../gesture'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const mockInvoke = invoke as Mock

const makeStore = () => configureStore({
  reducer: { playlist: playlistReducer, tape: tapeReducer },
})

// the store's state is a subset of RootState, which is all the selectors use
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const state = (store: ReturnType<typeof makeStore>) => store.getState() as any

// a thunk reading the state, run on the smaller store
const run = (store: ReturnType<typeof makeStore>, thunk: ReturnType<typeof changeTapeMachine>) =>
  thunk(store.dispatch, () => state(store))

const flatMixer: MixerSettings = { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } }

const loaded = (tape?: TapeSettings): LoadedProject => ({
  tempo: 120,
  timeSignature: { numerator: 4, denominator: 4 },
  metronomeEnabled: true,
  loopEnabled: true,
  snapEnabled: true,
  snapNote: 'QuarterNote',
  clips: [],
  missing: [],
  mixer: flatMixer,
  sequencer: { rows: [], patterns: [], folders: [] },
  patternClips: [],
  ...(tape ? { tape } : {}),
} as LoadedProject)

const view = (tape?: TapeSettings): ProjectView => ({
  tempo: 120,
  clips: [],
  patternClips: [],
  mixer: flatMixer,
  sequencer: { rows: [], patterns: [], folders: [] },
  ...(tape ? { tape } : {}),
} as ProjectView)

const used: TapeSettings = {
  enabled: true,
  machine: 1,
  age: 35,
  wobble: 150,
  hiss: 0,
  dbx: false,
  outputDb: -6,
  pitch: 2,
  tracks: [{ trackNumber: 2, onTape: true, driveDb: 4.5 }],
}

beforeEach(() => {
  vi.resetAllMocks()
  vi.useFakeTimers()
  clearPendingMixerSends()
  resetGestures()
  mockInvoke.mockResolvedValue(undefined)
})

afterEach(() => {
  vi.useRealTimers()
})

describe('tapeSlice', () => {
  it('starts with the tape off and no track on it', () => {
    const store = makeStore()
    expect(selectTapeMachine(state(store))).toEqual({
      enabled: false, machine: 0, age: 0, wobble: 100, hiss: 100, dbx: true, outputDb: 0, pitch: 0,
    })
    expect(selectTapeEnabled(state(store))).toBe(false)
    expect(selectTrackTape(3)(state(store))).toEqual({ onTape: false, driveDb: 0 })
  })

  it('reads the engine’s tape', async () => {
    const store = makeStore()
    mockInvoke.mockResolvedValue(used)
    await store.dispatch(fetchTape())

    expect(mockInvoke).toHaveBeenCalledWith('get_tape', {})
    expect(selectTapeMachine(state(store))).toEqual({
      enabled: true, machine: 1, age: 35, wobble: 150, hiss: 0, dbx: false, outputDb: -6, pitch: 2,
    })
    expect(selectTrackTape(2)(state(store))).toEqual({ onTape: true, driveDb: 4.5 })
    expect(selectTrackTape(1)(state(store))).toEqual({ onTape: false, driveDb: 0 })
  })

  it('takes a loaded project’s tape', () => {
    const store = makeStore()
    store.dispatch(reduceLoadedProject(loaded(used)))
    expect(selectTapeEnabled(state(store))).toBe(true)
    expect(selectTrackTape(2)(state(store)).onTape).toBe(true)
  })

  it('loads an older project (no tape) with the tape off and no track on it', () => {
    const store = makeStore()
    store.dispatch(setTape(used))
    store.dispatch(reduceLoadedProject(loaded()))
    expect(selectTapeMachine(state(store))).toEqual(defaultTapeMachine)
    expect(state(store).tape.tracks).toEqual({})
  })

  it('fills in what an older engine leaves out', () => {
    const store = makeStore()
    store.dispatch(setTape({ enabled: true } as TapeSettings))
    expect(selectTapeMachine(state(store))).toEqual({ ...defaultTapeMachine, enabled: true })
  })

  it('follows an undo or redo', () => {
    const store = makeStore()
    store.dispatch(reduceProjectView({ project: view(used), selected: null }))
    expect(selectTapeEnabled(state(store))).toBe(true)
    store.dispatch(reduceProjectView({ project: view({ ...defaultTapeMachine }), selected: null }))
    expect(selectTapeEnabled(state(store))).toBe(false)
    expect(state(store).tape.tracks).toEqual({})
    // an older engine's view, without the tape, leaves it alone
    store.dispatch(setTape(used))
    store.dispatch(reduceProjectView({ project: view(), selected: null }))
    expect(selectTapeEnabled(state(store))).toBe(true)
  })

  it('sends the whole machine with each change, a drag as one gesture', async () => {
    const store = makeStore()
    run(store, changeTapeMachine({ enabled: true }))
    expect(selectTapeEnabled(state(store))).toBe(true)
    expect(mockInvoke).toHaveBeenLastCalledWith('set_tape_machine', {
      machine: { ...defaultTapeMachine, enabled: true }, gesture: null,
    })

    vi.advanceTimersByTime(100)
    mockInvoke.mockClear()
    const gesture = beginGesture()
    run(store, changeTapeMachine({ age: 10 }))
    run(store, changeTapeMachine({ age: 20 }))
    run(store, changeTapeMachine({ age: 30 }))
    // the first goes at once, the latest after a moment
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    vi.advanceTimersByTime(100)
    expect(mockInvoke).toHaveBeenCalledTimes(2)
    expect(mockInvoke).toHaveBeenLastCalledWith('set_tape_machine', {
      machine: { ...defaultTapeMachine, enabled: true, age: 30 }, gesture,
    })
  })

  it('puts tracks on tape and drives them', () => {
    const store = makeStore()
    run(store, changeTrackTape(1, { onTape: true }))
    expect(selectTrackTape(1)(state(store))).toEqual({ onTape: true, driveDb: 0 })
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_tape', {
      track: { trackNumber: 1, onTape: true, driveDb: 0 }, gesture: null,
    })

    vi.advanceTimersByTime(100)
    run(store, changeTrackTape(1, { driveDb: 6 }))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_tape', {
      track: { trackNumber: 1, onTape: true, driveDb: 6 }, gesture: null,
    })

    // back at its defaults, it's no longer listed, as the engine keeps it
    run(store, changeTrackTape(1, { onTape: false, driveDb: 0 }))
    expect(state(store).tape.tracks).toEqual({})
  })

  it('has sends waiting flushed before an undo', async () => {
    const store = makeStore()
    run(store, changeTrackTape(0, { driveDb: 1 }))
    run(store, changeTrackTape(0, { driveDb: 2 }))
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    await flushMixerSends()
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_tape', {
      track: { trackNumber: 0, onTape: false, driveDb: 2 }, gesture: null,
    })
  })

  it('round-trips: what’s sent, loaded back, is what was set', () => {
    const store = makeStore()
    run(store, changeTapeMachine({ enabled: true, machine: 1, age: 35, wobble: 150, hiss: 0, dbx: false, outputDb: -6, pitch: 2 }))
    run(store, changeTrackTape(2, { onTape: true, driveDb: 4.5 }))
    const machine = mockInvoke.mock.calls.filter(([command]) => command === 'set_tape_machine').at(-1)?.[1].machine
    const track = mockInvoke.mock.calls.filter(([command]) => command === 'set_track_tape').at(-1)?.[1].track
    const before = state(store).tape

    // as the engine would save it and give it back
    const other = makeStore()
    other.dispatch(reduceLoadedProject(loaded({ ...machine, tracks: [track] })))
    expect(state(other).tape).toEqual(before)
  })
})
