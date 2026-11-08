import reducer, { reduceLoadedProject, playlistSlice } from './playlistSlice'
import { LoadedProject } from '../../types/project'
import { defaultClipSettings } from '../../types/clip'

jest.mock('@tauri-apps/api', () => ({ invoke: jest.fn() }))

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
}

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

    expect(state.playlistItems).toHaveLength(1)
    expect(state.playlistItems[0]).toMatchObject({
      id: 7, path: '/samples/kick.wav', trackNumber: 2, startBeat: 4.5,
    })
    expect(state.clipSettings).toEqual({ 7: { ...defaultClipSettings, pitch: 12 } })
    expect(state.clipSampleDurations).toEqual({ 7: 2 })
  })

  it('empties the playlist for a new project', () => {
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, playlistSlice.actions.addPlaylistItem({
      id: 1, path: 'old.wav', startBeat: 0, trackNumber: 0,
    }))

    state = reducer(state, reduceLoadedProject({ ...loaded, clips: [] }))

    expect(state.playlistItems).toEqual([])
    expect(state.clipSettings).toEqual({})
  })
})
