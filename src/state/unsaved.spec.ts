import type { Mock } from 'vitest'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer, { playlistSlice } from './slices/playlistSlice'
import projectReducer, { settleProject } from './slices/projectSlice'
import mixerReducer from './slices/mixerSlice'
import sequencerReducer from './slices/sequencerSlice'
import effectsReducer from './slices/effectsSlice'
import recordingReducer from './slices/recordingSlice'
import { SETTLE_MS, startUnsavedWatch } from './unsaved'
import type { RootState } from './store'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

const makeStore = () => configureStore({
  reducer: {
    playlist: playlistReducer,
    project: projectReducer,
    mixer: mixerReducer,
    sequencer: sequencerReducer,
    effects: effectsReducer,
    recording: recordingReducer,
  },
})

type TestStore = ReturnType<typeof makeStore>

// the store's state is the subset of RootState the watch reads
const watch = (store: TestStore, target = new EventTarget()) =>
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  startUnsavedWatch(store as any, target as any)

const unsaved = (store: TestStore) => (store.getState() as unknown as RootState).project.unsaved

const addClip = (store: TestStore, id: number) =>
  store.dispatch(playlistSlice.actions.addPlaylistItem({ id, path: 'a.wav', startBeat: 0, trackNumber: 0 }))

// let the timer fire, and the engine's answer come back
const settle = async () => {
  await vi.advanceTimersByTimeAsync(SETTLE_MS)
}

let engineSays = true

beforeEach(() => {
  vi.useFakeTimers()
  engineSays = true
  mockInvoke.mockImplementation(async (cmd: string) => (cmd === 'project_has_unsaved_changes' ? engineSays : undefined))
})

afterEach(() => {
  vi.useRealTimers()
})

describe('startUnsavedWatch', () => {
  it('asks the engine once the project stops changing', async () => {
    const store = makeStore()
    const stop = watch(store)

    addClip(store, 1)
    await vi.advanceTimersByTimeAsync(SETTLE_MS / 2)
    addClip(store, 2)
    expect(mockInvoke).not.toHaveBeenCalled()

    await settle()
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    expect(unsaved(store)).toBe(true)

    // and back, when the engine says it's as saved (an undo)
    engineSays = false
    store.dispatch(playlistSlice.actions.removePlaylistItem({ id: 2 }))
    await settle()
    expect(unsaved(store)).toBe(false)
    stop()
  })

  it("doesn't ask for changes that aren't the project's", async () => {
    const store = makeStore()
    const stop = watch(store)

    store.dispatch(playlistSlice.actions.setPlaying(true))
    await settle()
    expect(mockInvoke).not.toHaveBeenCalled()
    stop()
  })

  it('asks when the window gets the focus back', async () => {
    const store = makeStore()
    const target = new EventTarget()
    const stop = watch(store, target)

    target.dispatchEvent(new Event('focus'))
    await settle()
    expect(unsaved(store)).toBe(true)
    stop()
  })

  it('drops an answer that a save has made out of date', async () => {
    const store = makeStore()
    let answer: (unsaved: boolean) => void = () => {}
    mockInvoke.mockImplementation(() => new Promise(resolve => { answer = resolve }))
    const stop = watch(store)

    addClip(store, 1)
    await settle()
    // saved while the engine was answering (from before the save)
    store.dispatch(settleProject())
    answer(true)
    await vi.advanceTimersByTimeAsync(0)
    expect(unsaved(store)).toBe(false)
    stop()
  })

  it('stops', async () => {
    const store = makeStore()
    const stop = watch(store)
    stop()

    addClip(store, 1)
    await settle()
    expect(mockInvoke).not.toHaveBeenCalled()
  })
})
