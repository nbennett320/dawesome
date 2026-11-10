import { invoke } from '@tauri-apps/api/core'
import reducer, { reduceLoadedProject, playlistSlice, setPlaylistTempo, validTempo } from './playlistSlice'
import { LoadedProject } from '../../types/project'
import { defaultClipSettings } from '../../types/clip'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))

const loaded: LoadedProject = {
  tempo: 90,
  timeSignature: { numerator: 3, denominator: 4 },
  metronomeEnabled: false,
  loopEnabled: false,
  snapEnabled: true,
  snapNote: 'EighthNote',
  clips: [
    {
      id: 7,
      path: '/samples/kick.wav',
      trackNumber: 2,
      startBeat: 4.5,
      settings: { ...defaultClipSettings, pitch: 12 },
      sampleDuration: 2,
      playedDuration: 1,
    },
  ],
  missing: [],
  mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
  sequencer: { rows: [], patterns: [], folders: [] },
  patternClips: [{ id: 9, patternId: 2, trackNumber: 1, startBeat: 8 }],
}

describe('movePlaylistItem', () => {
  it('keeps what a clip plays: its sample, or its pattern', () => {
    const { actions } = playlistSlice
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, actions.addPlaylistItem({ id: 3, path: '', patternId: 5, startBeat: 0, trackNumber: 0 }))
    state = reducer(state, actions.movePlaylistItem({ id: 3, path: '', startBeat: 4, trackNumber: 2 }))

    expect(state.playlistItems[0]).toMatchObject({ id: 3, patternId: 5, startBeat: 4, trackNumber: 2 })
  })
})

describe('reduceExploded', () => {
  it('swaps pattern clips for a pattern clip per sample', () => {
    const { actions } = playlistSlice
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, actions.addPlaylistItem({ id: 3, path: '', patternId: 5, startBeat: 4, trackNumber: 0 }))
    state = reducer(state, actions.addPlaylistItem({ id: 4, path: 'keep.wav', startBeat: 0, trackNumber: 1 }))

    state = reducer(state, actions.reduceExploded({
      removed: [3],
      clips: [
        { id: 10, patternId: 6, trackNumber: 0, startBeat: 4 },
        { id: 11, patternId: 7, trackNumber: 2, startBeat: 4 },
      ],
    }))

    expect(state.playlistItems.map(i => i.id)).toEqual([4, 10, 11])
    expect(state.playlistItems[2]).toMatchObject({ path: '', patternId: 7, trackNumber: 2, startBeat: 4 })
  })
})

describe('setPlaylistTempo', () => {
  const mockInvoke = invoke as jest.Mock

  it('takes the tempos the engine does', () => {
    expect([120, 1, 1000].every(validTempo)).toBe(true)
    expect([0, 0.5, -90, NaN, Infinity, 1000.5, 1e-20].some(validTempo)).toBe(false)
  })

  it('sends a tempo, then asks where the playhead is', async () => {
    mockInvoke.mockImplementation(async (command: string, args?: { val: number }) => {
      if(command === 'set_playlist_tempo') return args?.val
      if(command === 'get_playlist_playing') return true
      return undefined
    })
    const dispatch = jest.fn()

    expect(await setPlaylistTempo(90)(dispatch)).toBe(true)

    expect(mockInvoke).toHaveBeenCalledWith('set_playlist_tempo', { val: 90 })
    const state = dispatch.mock.calls.reduce((s, [action]) => reducer(s, action), reducer(undefined, { type: 'init' }))
    expect(state.tempo).toBe(90)
    // views that follow the playhead ask the engine again
    expect(state.transportRevision).toBe(1)
  })

  it('does not send half a number', async () => {
    const dispatch = jest.fn()
    expect(await setPlaylistTempo(NaN)(dispatch)).toBe(false)
    expect(await setPlaylistTempo(0)(dispatch)).toBe(false)

    expect(mockInvoke).not.toHaveBeenCalled()
    expect(dispatch).not.toHaveBeenCalled()
  })

  it('keeps the tempo when the engine refuses (while recording)', async () => {
    mockInvoke.mockRejectedValue('the tempo can\'t be changed while recording')
    const error = jest.spyOn(console, 'error').mockImplementation(() => {})
    const dispatch = jest.fn()

    expect(await setPlaylistTempo(90)(dispatch)).toBe(false)

    expect(dispatch).not.toHaveBeenCalled()
    expect(error).toHaveBeenCalled()
    error.mockRestore()
  })
})

describe('the playlist view', () => {
  it('is 64 bars long', () => {
    const state = reducer(undefined, { type: 'init' })
    expect(state.ui.maxBeatsDisplayed).toBe(256)
  })

  it('goes back to its first bars when a project is opened', () => {
    let state = reducer(undefined, { type: 'init' })
    const before = state.viewRevision
    state = reducer(state, reduceLoadedProject(loaded))
    expect(state.viewRevision).toBe(before + 1)

    // moving a clip doesn't
    state = reducer(state, playlistSlice.actions.movePlaylistItems([{ id: 7, startBeat: 1, trackNumber: 0 }]))
    expect(state.viewRevision).toBe(before + 1)
  })
})

describe('reduceLoadedProject', () => {
  it('replaces the playlist with the loaded project', () => {
    const { actions } = playlistSlice
    let state = reducer(undefined, { type: 'init' })

    // a clip from the old project, being inspected, while playing
    state = reducer(state, actions.addPlaylistItem({ id: 1, path: 'old.wav', startBeat: 0, trackNumber: 0 }))
    state = reducer(state, actions.setClipInfo({
      id: 1, settings: defaultClipSettings, sampleDuration: 3, playedDuration: 3,
    }))
    state = reducer(state, actions.setInspectedClip(1))
    state = reducer(state, actions.setPlaying(true))

    state = reducer(state, reduceLoadedProject(loaded))

    expect(state.playing).toBe(false)
    expect(state.inspectedClipId).toBeNull()
    expect(state.tempo).toBe(90)
    expect(state.metronomeEnabled).toBe(false)
    expect(state.loopEnabled).toBe(false)
    expect(state.snapNote).toBe('EighthNote')

    expect(state.playlistItems).toHaveLength(2)
    expect(state.playlistItems[0]).toMatchObject({
      id: 7, path: '/samples/kick.wav', trackNumber: 2, startBeat: 4.5,
    })
    // pattern clips come after, with no sample
    expect(state.playlistItems[1]).toMatchObject({
      id: 9, path: '', patternId: 2, trackNumber: 1, startBeat: 8,
    })
    expect(state.clipSettings).toEqual({ 7: { ...defaultClipSettings, pitch: 12 } })
    expect(state.clipSampleDurations).toEqual({ 7: 2 })
  })

  it('empties the playlist for a new project', () => {
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, playlistSlice.actions.addPlaylistItem({
      id: 1, path: 'old.wav', startBeat: 0, trackNumber: 0,
    }))

    state = reducer(state, reduceLoadedProject({ ...loaded, clips: [], patternClips: [] }))

    expect(state.playlistItems).toEqual([])
    expect(state.clipSettings).toEqual({})
  })
})
