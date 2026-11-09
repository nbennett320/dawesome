import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import playlistReducer, { reduceLoadedProject } from './playlistSlice'
import mixerReducer, {
  clearPendingMixerSends,
  fetchMixer,
  resetMixer,
  selectAnySolo,
  selectTrackMix,
  selectTrackName,
  setMasterVolume,
  setTrackMute,
  setTrackName,
  setTrackSolo,
  setTrackVolume,
} from './mixerSlice'
import { defaultChannelSettings, MixerSettings } from '../../types/mixer'
import { LoadedProject } from '../../types/project'

jest.mock('@tauri-apps/api', () => ({ invoke: jest.fn() }))

const mockInvoke = invoke as jest.Mock

const makeStore = () => configureStore({
  reducer: { playlist: playlistReducer, mixer: mixerReducer },
})

// the store's state is a subset of RootState, which is all the selectors use
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const state = (store: ReturnType<typeof makeStore>) => store.getState() as any

const flat = (trackCount: number): MixerSettings => ({
  tracks: Array.from({ length: trackCount }, (_, trackNumber) => ({ trackNumber, ...defaultChannelSettings })),
  master: defaultChannelSettings,
})

beforeEach(() => {
  jest.resetAllMocks()
  jest.useFakeTimers()
  clearPendingMixerSends()
  mockInvoke.mockResolvedValue(undefined)
})

afterEach(() => {
  jest.useRealTimers()
})

describe('mixerSlice', () => {
  it('reads the engine’s mixer', async () => {
    const store = makeStore()
    const settings = flat(4)
    settings.tracks[2] = { ...settings.tracks[2], volumeDb: -12, mute: true }
    mockInvoke.mockResolvedValue(settings)

    await store.dispatch(fetchMixer())

    expect(selectTrackMix(2)(state(store))).toEqual({ ...defaultChannelSettings, volumeDb: -12, mute: true })
    // tracks it didn't list are at their defaults
    expect(selectTrackMix(9)(state(store))).toEqual(defaultChannelSettings)
  })

  it('moves a fader straight away, and sends only the latest value while dragging', () => {
    const store = makeStore()

    store.dispatch(setTrackVolume(1, -3))
    store.dispatch(setTrackVolume(1, -4))
    store.dispatch(setTrackVolume(1, -5))

    expect(selectTrackMix(1)(state(store)).volumeDb).toBe(-5)
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_volume', { trackNumber: 1, volumeDb: -3 })

    jest.runOnlyPendingTimers()
    expect(mockInvoke).toHaveBeenCalledTimes(2)
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_volume', { trackNumber: 1, volumeDb: -5 })

    // each fader is sent on its own
    store.dispatch(setMasterVolume(-1))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_master_volume', { volumeDb: -1 })
  })

  it('forgets unsent values when told to', () => {
    const store = makeStore()
    store.dispatch(setTrackVolume(0, -3))
    store.dispatch(setTrackVolume(0, -9))

    clearPendingMixerSends()
    jest.runOnlyPendingTimers()

    expect(mockInvoke).toHaveBeenCalledTimes(1)
  })

  it('mutes and solos', () => {
    const store = makeStore()

    store.dispatch(setTrackMute(0, true))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_mute', { trackNumber: 0, mute: true })
    expect(selectAnySolo(state(store))).toBe(false)

    store.dispatch(setTrackSolo(3, true))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_solo', { trackNumber: 3, solo: true })
    expect(selectAnySolo(state(store))).toBe(true)
  })

  it('takes the mix from a loaded project', () => {
    const store = makeStore()
    store.dispatch(setTrackMute(0, true))

    const mixer = flat(2)
    mixer.master = { ...mixer.master, volumeDb: -6 }
    store.dispatch(reduceLoadedProject({
      tempo: 120,
      timeSignature: { numerator: 4, denominator: 4 },
      metronomeEnabled: true,
      loopEnabled: true,
      snapEnabled: true,
      snapNote: 'QuarterNote',
      clips: [],
      missing: [],
      mixer,
      sequencer: { rows: [], patterns: [], folders: [] },
      patternClips: [],
    } as LoadedProject))

    expect(selectTrackMix(0)(state(store)).mute).toBe(false)
    expect(state(store).mixer.master.volumeDb).toBe(-6)
  })

  it('names tracks, with the engine having the final say', async () => {
    const store = makeStore()
    expect(selectTrackName(2)(state(store))).toBe('Track 3')

    mockInvoke.mockResolvedValue('Drums')
    await store.dispatch(setTrackName(2, '  Drums '))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_track_name', { trackNumber: 2, name: 'Drums' })
    expect(selectTrackName(2)(state(store))).toBe('Drums')

    // an empty name goes back to the default
    mockInvoke.mockResolvedValue(null)
    await store.dispatch(setTrackName(2, ''))
    expect(selectTrackName(2)(state(store))).toBe('Track 3')
  })

  it('takes names from the engine’s mixer', async () => {
    const store = makeStore()
    const settings = flat(2)
    settings.tracks[1] = { ...settings.tracks[1], name: 'Bass' }
    mockInvoke.mockResolvedValue(settings)

    await store.dispatch(fetchMixer())

    expect(selectTrackName(1)(state(store))).toBe('Bass')
    // the name isn't mixed into the strip's settings
    expect(selectTrackMix(1)(state(store))).toEqual(defaultChannelSettings)
  })

  it('resets every strip', async () => {
    const store = makeStore()
    store.dispatch(setTrackSolo(1, true))
    mockInvoke.mockResolvedValue(flat(4))

    await store.dispatch(resetMixer())

    expect(mockInvoke).toHaveBeenLastCalledWith('reset_mixer', {})
    expect(selectAnySolo(state(store))).toBe(false)
  })
})
