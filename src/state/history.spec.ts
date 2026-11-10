import { invoke } from '@tauri-apps/api/core'
import { store } from './store'
import { resetTravel, samePress, travel } from './history'
import { beginGesture, resetGestures } from './gesture'
import { Travelled } from '../types/project'
import { defaultClipSettings } from '../types/clip'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
// the store's project slice opens dialogs and lists folders
jest.mock('@tauri-apps/plugin-dialog', () => ({ ask: jest.fn(), open: jest.fn(), save: jest.fn() }))
jest.mock('@tauri-apps/plugin-fs', () => ({ readDir: jest.fn() }))
jest.mock('@tauri-apps/api/path', () => ({ dirname: jest.fn(), documentDir: jest.fn(), join: jest.fn() }))
const mockInvoke = invoke as jest.Mock

// the project after undoing a clip's deletion: the clip is back
const travelled: Travelled = {
  changed: true,
  selected: [7],
  canUndo: true,
  canRedo: true,
  project: {
    tempo: 93,
    clips: [{
      id: 7,
      path: '/samples/kick.wav',
      trackNumber: 2,
      startBeat: 4.5,
      settings: { ...defaultClipSettings, pitch: 12 },
      sampleDuration: 2,
      playedDuration: 1,
    }],
    patternClips: [{ id: 9, patternId: 2, trackNumber: 1, startBeat: 8 }],
    mixer: {
      tracks: [{ trackNumber: 0, name: 'drums', volumeDb: -6, pan: 0.25, mute: true, solo: false }],
      master: { volumeDb: -3, pan: 0, mute: false, solo: false },
    },
    sequencer: {
      rows: [{ id: 0, path: '/drums/kick.wav', volume: 1, muted: false }],
      patterns: [{ id: 2, name: 'verse', length: 16, steps: { 0: [0, 8] } }],
      folders: [],
    },
  },
}

// the engine, as far as undoing goes
const engine = (result: Travelled) => {
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 'undo' || command === 'redo') return result
    if(command === 'get_playlist_data') return [16, 256, 8, 4]
    if(command === 'get_playlist_playing') return false
    if(command === 'get_playlist_transport') return { playing: false, patternId: null, startTimeMs: null, beatIntervalMs: 645 }
    return undefined
  })
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command).length

describe('undo and redo', () => {
  beforeEach(() => {
    resetTravel()
    resetGestures()
  })

  it('shows the project the engine went back to, and selects what changed', async () => {
    engine(travelled)
    const before = store.getState().playlist.travelled.count

    expect(await store.dispatch(travel('undo'))).toBe(true)

    const { playlist, mixer, sequencer } = store.getState()
    expect(playlist.tempo).toBe(93)
    expect(playlist.playlistItems.map(item => [item.id, item.startBeat, item.trackNumber, item.patternId]))
      .toEqual([[7, 4.5, 2, undefined], [9, 8, 1, 2]])
    expect(playlist.clipSettings[7].pitch).toBe(12)
    expect(playlist.clipSampleDurations[7]).toBe(2)
    expect(playlist.travelled).toEqual({ count: before + 1, ids: [7] })
    expect(mixer.tracks[0]).toMatchObject({ volumeDb: -6, pan: 0.25, mute: true })
    expect(mixer.names[0]).toBe('drums')
    expect(mixer.master.volumeDb).toBe(-3)
    expect(sequencer.patterns.map(p => p.name)).toEqual(['verse'])
    expect(sequencer.selectedPatternId).toBe(2)
  })

  it('changes nothing when there is nothing to undo', async () => {
    engine({ ...travelled, changed: false, selected: [] })
    const before = store.getState()

    expect(await store.dispatch(travel('redo'))).toBe(false)
    expect(store.getState()).toBe(before)
  })

  it('waits for the mouse to be let go', async () => {
    engine(travelled)
    // a fader is being dragged
    beginGesture()

    expect(await store.dispatch(travel('undo'))).toBe(false)
    expect(calls('undo')).toBe(0)
  })

  it('acts once on a shortcut that arrives by the menu and by the key', async () => {
    engine(travelled)
    const first = store.dispatch(travel('undo', 'key'))
    const second = store.dispatch(travel('undo', 'menu'))

    expect(await first).toBe(true)
    expect(await second).toBe(false)
    expect(calls('undo')).toBe(1)
  })

  it('tells one press arriving twice from two presses', () => {
    const last = { way: 'undo' as const, source: 'key' as const, at: 1_000 }
    // the menu's copy of the same press
    expect(samePress(last, 'undo', 'menu', 1_020)).toBe(true)
    // the key held down, repeating
    expect(samePress(last, 'undo', 'key', 1_030)).toBe(false)
    // pressed again later, or a different command
    expect(samePress(last, 'undo', 'menu', 1_400)).toBe(false)
    expect(samePress(last, 'redo', 'menu', 1_020)).toBe(false)
    expect(samePress(null, 'undo', 'menu', 1_020)).toBe(false)
  })

  it('undoes in the order asked', async () => {
    engine(travelled)
    const results = await Promise.all([
      store.dispatch(travel('undo', 'key')),
      store.dispatch(travel('undo', 'key')),
      store.dispatch(travel('redo', 'key')),
    ])

    expect(results).toEqual([true, true, true])
    const order = mockInvoke.mock.calls.map(([name]) => name).filter(name => name === 'undo' || name === 'redo')
    expect(order).toEqual(['undo', 'undo', 'redo'])
  })

  it('carries on after the engine fails', async () => {
    mockInvoke.mockRejectedValue('no engine')
    const error = jest.spyOn(console, 'error').mockImplementation(() => {})
    expect(await store.dispatch(travel('undo'))).toBe(false)
    error.mockRestore()

    resetTravel()
    engine(travelled)
    expect(await store.dispatch(travel('undo'))).toBe(true)
  })
})
