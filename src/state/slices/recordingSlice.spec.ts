import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import playlistReducer, { reduceLoadedProject } from './playlistSlice'
import projectReducer from './projectSlice'
import recordingReducer, { collectRecording, fetchRecordSetup, setTrackArmed, toggleRecording } from './recordingSlice'
import { LoadedClip, LoadedProject, Recorded } from '../../types/project'

jest.mock('@tauri-apps/api', () => ({ invoke: jest.fn() }))
// the notices go through the project slice, which uses these
jest.mock('@tauri-apps/api/dialog', () => ({ ask: jest.fn(), open: jest.fn(), save: jest.fn() }))
jest.mock('@tauri-apps/api/path', () => ({ dirname: jest.fn(), documentDir: jest.fn(), join: jest.fn() }))
jest.mock('@tauri-apps/api/fs', () => ({ readDir: jest.fn() }))

const mockInvoke = invoke as jest.Mock

const makeStore = () => configureStore({
  reducer: { playlist: playlistReducer, project: projectReducer, recording: recordingReducer },
})
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const getState = (store: ReturnType<typeof makeStore>) => store.getState as any

const clip: LoadedClip = {
  id: 7,
  path: '/songs/song/recordings/song_11092026_0.wav',
  trackNumber: 1,
  startBeat: 2,
  settings: { gainDb: 0, pan: 0, pitch: 0, attackMs: 0, decayMs: 0 },
  sampleDuration: 4,
  playedDuration: 4,
}
const recorded = (overrides: Partial<Recorded> = {}): Recorded =>
  ({ recording: false, clips: [], problems: [], ...overrides })

// the engine's answers
const engine = (answers: Record<string, unknown>) => {
  mockInvoke.mockImplementation(async (cmd: string) => {
    if(cmd in answers) {
      const answer = answers[cmd]
      if(answer instanceof Error) throw answer
      return answer
    }
    return undefined
  })
}

beforeEach(() => jest.resetAllMocks())

test('recording starts, then stops with its takes on their tracks', async () => {
  const store = makeStore()

  engine({ t_start_recording: recorded({ recording: true }), get_playlist_playing: true })
  await toggleRecording()(store.dispatch, getState(store))
  expect(mockInvoke).toHaveBeenCalledWith('t_start_recording', {})
  expect(store.getState().recording.recording).toBe(true)
  expect(store.getState().playlist.playing).toBe(true)

  engine({ t_stop_recording: recorded({ clips: [clip] }), get_playlist_playing: false })
  await toggleRecording()(store.dispatch, getState(store))
  expect(mockInvoke).toHaveBeenCalledWith('t_stop_recording', {})
  expect(store.getState().recording.recording).toBe(false)
  expect(store.getState().playlist.playing).toBe(false)

  const items = store.getState().playlist.playlistItems
  expect(items).toEqual([expect.objectContaining({ id: 7, path: clip.path, trackNumber: 1, startBeat: 2 })])
  expect(store.getState().playlist.clipSampleDurations[7]).toBe(4)
  expect(store.getState().project.notice).toBeNull()
})

test('a recording that ended some other way is caught', async () => {
  const store = makeStore()
  engine({ t_start_recording: recorded({ recording: true }) })
  await toggleRecording()(store.dispatch, getState(store))

  engine({ collect_recording: recorded({ recording: true }) })
  await collectRecording()(store.dispatch)
  expect(store.getState().recording.recording).toBe(true)

  // playback stopped, or the loop came round
  engine({ collect_recording: recorded({ clips: [clip] }) })
  await collectRecording()(store.dispatch)
  expect(store.getState().recording.recording).toBe(false)
  expect(store.getState().playlist.playlistItems).toHaveLength(1)
})

test('what went wrong is said', async () => {
  const store = makeStore()

  engine({ t_start_recording: new Error('Arm a track first') })
  await toggleRecording()(store.dispatch, getState(store))
  expect(store.getState().recording.recording).toBe(false)
  expect(store.getState().project.notice).toEqual({ kind: 'error', message: "Couldn't record: Arm a track first" })

  engine({ collect_recording: recorded({ clips: [clip], problems: ['some audio was lost'] }) })
  await collectRecording()(store.dispatch)
  expect(store.getState().project.notice).toEqual({ kind: 'error', message: 'Recording: some audio was lost' })
  // the take is still kept
  expect(store.getState().playlist.playlistItems).toHaveLength(1)
})

test('arming tracks', async () => {
  const store = makeStore()
  const input = { first: 2, stereo: true }

  engine({ get_record_setup: { armed: [], inputChannels: 4, inputError: null } })
  await fetchRecordSetup()(store.dispatch)
  expect(store.getState().recording.inputChannels).toBe(4)

  engine({ set_track_armed: { armed: [{ trackNumber: 3, input }], inputChannels: 4, inputError: null } })
  await setTrackArmed(3, input)(store.dispatch)
  expect(mockInvoke).toHaveBeenCalledWith('set_track_armed', { trackNumber: 3, input })
  expect(store.getState().recording.armed).toEqual({ 3: input })
})

test('a project brings its armed tracks', () => {
  const store = makeStore()
  const project = {
    tempo: 120,
    timeSignature: { numerator: 4, denominator: 4 },
    metronomeEnabled: true,
    loopEnabled: true,
    snapEnabled: true,
    snapNote: 'QuarterNote',
    clips: [],
    missing: [],
    mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
    sequencer: { rows: [], patterns: [] },
    patternClips: [],
    armed: [{ trackNumber: 0, input: { first: 0, stereo: false } }],
  } as LoadedProject

  store.dispatch(reduceLoadedProject(project))
  expect(store.getState().recording.armed).toEqual({ 0: { first: 0, stereo: false } })
})
