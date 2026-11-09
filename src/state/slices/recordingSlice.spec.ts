import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import projectReducer from './projectSlice'
import recordingReducer, { toggleRecording } from './recordingSlice'

jest.mock('@tauri-apps/api', () => ({ invoke: jest.fn() }))
// the notices go through the project slice, which uses these
jest.mock('@tauri-apps/api/dialog', () => ({ ask: jest.fn(), open: jest.fn(), save: jest.fn() }))
jest.mock('@tauri-apps/api/path', () => ({ dirname: jest.fn(), documentDir: jest.fn(), join: jest.fn() }))
jest.mock('@tauri-apps/api/fs', () => ({ readDir: jest.fn() }))

const mockInvoke = invoke as jest.Mock

const makeStore = () => configureStore({
  reducer: { project: projectReducer, recording: recordingReducer },
})

beforeEach(() => jest.resetAllMocks())

test('recording starts, then stops with where the take went', async () => {
  const store = makeStore()

  mockInvoke.mockResolvedValue({ recording: true, take: null })
  await toggleRecording()(store.dispatch)
  expect(mockInvoke).toHaveBeenCalledWith('t_toggle_recording', {})
  expect(store.getState().recording.recording).toBe(true)
  expect(store.getState().project.notice).toBeNull()

  mockInvoke.mockResolvedValue({
    recording: false,
    take: { path: '/songs/song/recordings/song_11092026_0.wav', seconds: 2, channels: 2, dropped: 0 },
  })
  await toggleRecording()(store.dispatch)
  expect(store.getState().recording.recording).toBe(false)
  expect(store.getState().project.notice).toEqual({
    kind: 'info', message: 'Recorded song_11092026_0.wav (2.0s, stereo)',
  })
})

test('a take that can’t start says why, and isn’t shown as recording', async () => {
  const store = makeStore()
  mockInvoke.mockRejectedValue("there's no audio input device")

  await toggleRecording()(store.dispatch)

  expect(store.getState().recording.recording).toBe(false)
  expect(store.getState().project.notice).toEqual({
    kind: 'error', message: "Couldn't record: there's no audio input device",
  })
})
