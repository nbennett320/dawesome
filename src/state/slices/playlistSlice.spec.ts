import type { Mock } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import reducer, {
  addToPlaylist, copyClips, cutClips, deepCopyClips, joinClips, NEEDS_SAVING, pasteClips, repeatClips, reduceLoadedProject, playlistSlice, removeManyFromPlaylist, selectClipOverrides, selectClipWindow, setClipWindows,
  setPlaylistTempo,
  splitClips, stretchClips, updateClipSettings, validTempo, withWaitCursor,
} from './playlistSlice'
import { beginGesture, resetGestures } from '../gesture'
import { runProjectCommand, setProjectNotice } from './projectSlice'
import { ChangedClips, LoadedProject, Placed, ProjectView } from '../../types/project'
import { defaultClipSettings } from '../../types/clip'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('./projectSlice', async () => {
  const actual = await vi.importActual('./projectSlice')
  return { ...actual, runProjectCommand: vi.fn() }
})
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))

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
      window: { start: 0.25, end: 1.5 },
    },
  ],
  missing: [],
  mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
  sequencer: { rows: [], synths: [], patterns: [], folders: [] },
  patternClips: [{ id: 9, patternId: 2, trackNumber: 1, startBeat: 8, window: { start: 4, end: null } }],
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

describe('addToPlaylist', () => {
  const mockInvoke = invoke as Mock

  it('asks for a dropped clip\'s length, which trimming it needs', async () => {
    mockInvoke.mockReset()
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'add_audiograph_node') return { id: 31, startBeat: 2 }
      if(command === 'get_clip') {
        return { settings: defaultClipSettings, sampleDuration: 1.5, playedDuration: 1.5, window: null }
      }
      if(command === 'get_playlist_data') return [16, 256, 240, 4]
      return undefined
    })
    const dispatch = vi.fn()
    const box = { x: 0, y: 0, left: 0, top: 0, right: 0, bottom: 0 }
    await addToPlaylist('/samples/kick.wav', 1, 2.2, box)(dispatch)

    const state = dispatch.mock.calls.reduce(
      (s, [action]) => (typeof action === 'function' ? s : reducer(s, action)),
      reducer(undefined, { type: 'init' }),
    )
    expect(state.playlistItems.map((item: { id: number }) => item.id)).toEqual([31])
    expect(state.clipSampleDurations[31]).toBe(1.5)
    // so the canvas knows how far its edges can go
    const overrides = selectClipOverrides({ playlist: state } as unknown as Parameters<typeof selectClipOverrides>[0])
    expect(overrides[31]).toMatchObject({ sampleDuration: 1.5, rate: 1, playedDuration: 1.5 })
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
        { id: 10, patternId: 6, trackNumber: 0, startBeat: 4, window: null },
        { id: 11, patternId: 7, trackNumber: 2, startBeat: 4, window: null },
      ],
    }))

    expect(state.playlistItems.map(i => i.id)).toEqual([4, 10, 11])
    expect(state.playlistItems[2]).toMatchObject({ path: '', patternId: 7, trackNumber: 2, startBeat: 4 })
  })

  it('gives each new clip the part of its pattern the trimmed clip played', () => {
    const { actions } = playlistSlice
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, actions.reduceChangedClips({
      clips: [],
      patternClips: [{ id: 3, patternId: 5, trackNumber: 0, startBeat: 5, window: { start: 4, end: 12 } }],
    }))
    expect(state.clipWindows[3]).toEqual({ start: 4, end: 12 })

    state = reducer(state, actions.reduceExploded({
      removed: [3],
      clips: [
        { id: 10, patternId: 6, trackNumber: 0, startBeat: 5, window: { start: 4, end: 12 } },
        { id: 11, patternId: 7, trackNumber: 2, startBeat: 5, window: { start: 4, end: 12 } },
      ],
    }))

    expect(state.clipWindows).toEqual({ 10: { start: 4, end: 12 }, 11: { start: 4, end: 12 } })
  })
})

describe('setPlaylistTempo', () => {
  const mockInvoke = invoke as Mock

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
    const dispatch = vi.fn()

    expect(await setPlaylistTempo(90)(dispatch)).toBe(true)

    expect(mockInvoke).toHaveBeenCalledWith('set_playlist_tempo', { val: 90, gesture: null })
    const state = dispatch.mock.calls.reduce((s, [action]) => reducer(s, action), reducer(undefined, { type: 'init' }))
    expect(state.tempo).toBe(90)
    // views that follow the playhead ask the engine again
    expect(state.transportRevision).toBe(1)

    // typed in the tempo field: the visit it was typed in goes with it
    await setPlaylistTempo(100, 12)(dispatch)
    expect(mockInvoke).toHaveBeenCalledWith('set_playlist_tempo', { val: 100, gesture: 12 })
  })

  it('does not send half a number', async () => {
    const dispatch = vi.fn()
    expect(await setPlaylistTempo(NaN)(dispatch)).toBe(false)
    expect(await setPlaylistTempo(0)(dispatch)).toBe(false)

    expect(mockInvoke).not.toHaveBeenCalled()
    expect(dispatch).not.toHaveBeenCalled()
  })

  it('keeps the tempo when the engine refuses (while recording)', async () => {
    mockInvoke.mockRejectedValue('the tempo can\'t be changed while recording')
    const error = vi.spyOn(console, 'error').mockImplementation(() => {})
    const dispatch = vi.fn()

    expect(await setPlaylistTempo(90)(dispatch)).toBe(false)

    expect(dispatch).not.toHaveBeenCalled()
    expect(error).toHaveBeenCalled()
    error.mockRestore()
  })
})

describe('removeManyFromPlaylist', () => {
  const mockInvoke = invoke as Mock

  it('removes a selection with one command, so it is undone as one step', async () => {
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'get_playlist_data' ? [16, 256, 8, 4] : undefined))
    const dispatch = vi.fn()

    await removeManyFromPlaylist([3, 4, 5])(dispatch)

    expect(mockInvoke).toHaveBeenCalledWith('remove_audiograph_nodes', { ids: [3, 4, 5] })
    expect(mockInvoke.mock.calls.filter(([name]) => name === 'remove_audiograph_node')).toHaveLength(0)
    const removed = dispatch.mock.calls
      .map(([action]) => action)
      .filter(action => action.type === playlistSlice.actions.removePlaylistItem.type)
      .map(action => action.payload.id)
    expect(removed).toEqual([3, 4, 5])
  })

  it('removes none of them when the engine refuses', async () => {
    mockInvoke.mockRejectedValue('no node with id 4')
    const dispatch = vi.fn()

    await expect(removeManyFromPlaylist([3, 4])(dispatch)).rejects.toBe('no node with id 4')
    expect(dispatch).not.toHaveBeenCalled()
  })
})

describe('updateClipSettings', () => {
  const mockInvoke = invoke as Mock

  it('sends the gesture a setting was changed in', async () => {
    mockInvoke.mockResolvedValue(undefined)
    let state = reducer(undefined, { type: 'init' })
    const getState = () => ({ playlist: state }) as never
    const dispatch = ((action: { type: string }) => { state = reducer(state, action); return action }) as never

    // a knob dragged with the mouse held
    const gesture = beginGesture()
    updateClipSettings(11, { gainDb: -6 })(dispatch, getState)
    expect(mockInvoke).toHaveBeenCalledWith('set_clip_settings', {
      id: 11, settings: expect.objectContaining({ gainDb: -6 }), gesture,
    })

    // a key press belongs to no gesture
    resetGestures()
    updateClipSettings(12, { pan: 0.5 })(dispatch, getState)
    expect(mockInvoke).toHaveBeenCalledWith('set_clip_settings', {
      id: 12, settings: expect.objectContaining({ pan: 0.5 }), gesture: null,
    })
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
    expect(state.clipWindows).toEqual({ 7: { start: 0.25, end: 1.5 }, 9: { start: 4, end: null } })
  })

  it('clears the windows of the old project', () => {
    const { actions } = playlistSlice
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, actions.reduceChangedClips({
      clips: [], patternClips: [{ id: 1, patternId: 2, trackNumber: 0, startBeat: 0, window: { start: 4, end: 8 } }],
    }))
    expect(state.clipWindows[1]).toBeDefined()

    state = reducer(state, reduceLoadedProject(loaded))
    expect(state.clipWindows[1]).toBeUndefined()

    state = reducer(state, reduceLoadedProject({ ...loaded, clips: [], patternClips: [] }))
    expect(state.clipWindows).toEqual({})
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

describe('reduceProjectView', () => {
  it('fills the windows of both kinds of clip, and clears stale ones', () => {
    const { actions } = playlistSlice
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, actions.reduceChangedClips({
      clips: [], patternClips: [{ id: 1, patternId: 2, trackNumber: 0, startBeat: 0, window: { start: 4, end: 8 } }],
    }))

    const project: ProjectView = {
      tempo: 90,
      clips: loaded.clips,
      patternClips: loaded.patternClips,
      mixer: loaded.mixer,
      sequencer: loaded.sequencer,
    }
    state = reducer(state, actions.reduceProjectView({ project, selected: null }))

    expect(state.clipWindows).toEqual({ 7: { start: 0.25, end: 1.5 }, 9: { start: 4, end: null } })
  })
})

describe('trimmed and cut clips', () => {
  const { actions } = playlistSlice
  const clip = (overrides = {}) => ({
    ...loaded.clips[0], id: 7, startBeat: 4.5, window: null, ...overrides,
  })
  const patternClip = (overrides = {}) => ({
    id: 9, patternId: 2, trackNumber: 1, startBeat: 8, window: null, ...overrides,
  })
  const start = () => {
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, reduceLoadedProject({
      ...loaded,
      clips: [clip({ window: { start: 0.25, end: 1.5 } })],
      patternClips: [patternClip({ window: { start: 4, end: null } })],
    }))
    return state
  }

  it('is as long as the part of its sample it plays, and keeps its fades within that', () => {
    // two seconds of sample an octave up, playing 0.25s to 1.5s of it
    let state = start()
    const overrides = (playlist: typeof state) =>
      selectClipOverrides({ playlist } as unknown as Parameters<typeof selectClipOverrides>[0])
    expect(overrides(state)[7].playedDuration).toBeCloseTo(0.625, 10)

    // a fade can't be longer than that
    state = reducer(state, actions.reduceClipSettings({ id: 7, settings: { attackMs: 5_000 } }))
    expect(state.clipSettings[7].attackMs).toBeCloseTo(625, 6)

    // untrimmed again, it's the whole sample's length
    state = reducer(state, actions.reduceChangedClips({ clips: [clip({ window: null })], patternClips: [] }))
    expect(overrides(state)[7].playedDuration).toBe(1)
  })

  it('updates a clip where it is, and adds new ones', () => {
    let state = start()
    state = reducer(state, actions.reduceChangedClips({
      clips: [
        clip({ startBeat: 5, window: { start: 0.5, end: null } }),
        clip({ id: 20, startBeat: 6, trackNumber: 3, window: { start: 0, end: 0.5 } }),
      ],
      patternClips: [
        patternClip({ startBeat: 9, window: { start: 4, end: 12 } }),
        patternClip({ id: 21, startBeat: 12, window: { start: 12, end: null } }),
      ],
    }))

    expect(state.playlistItems.map(i => i.id)).toEqual([7, 9, 20, 21])
    expect(state.playlistItems[0]).toMatchObject({ id: 7, startBeat: 5, trackNumber: 2, path: '/samples/kick.wav' })
    expect(state.playlistItems[1]).toMatchObject({ id: 9, startBeat: 9, patternId: 2 })
    expect(state.playlistItems[2]).toMatchObject({ id: 20, startBeat: 6, trackNumber: 3, path: '/samples/kick.wav' })
    expect(state.playlistItems[3]).toMatchObject({ id: 21, startBeat: 12, patternId: 2, path: '' })
    expect(state.clipWindows).toEqual({
      7: { start: 0.5, end: null },
      9: { start: 4, end: 12 },
      20: { start: 0, end: 0.5 },
      21: { start: 12, end: null },
    })
    expect(state.clipSampleDurations[20]).toBe(2)
    expect(selectClipWindow(20)({ playlist: state } as never)).toEqual({ start: 0, end: 0.5 })
    expect(selectClipWindow(99)({ playlist: state } as never)).toBeNull()
    expect(selectClipWindow()({ playlist: state } as never)).toBeNull()
  })

  it('drops the window of a clip that comes back whole', () => {
    let state = start()
    expect(state.clipWindows[7]).toBeDefined()
    state = reducer(state, actions.reduceChangedClips({
      clips: [clip()], patternClips: [patternClip()],
    }))
    expect(state.clipWindows).toEqual({})
  })

  it('forgets the window of a removed clip', () => {
    let state = start()
    state = reducer(state, actions.removePlaylistItem({ id: 7 }))
    state = reducer(state, actions.removePlaylistItem({ id: 9 }))
    expect(state.clipWindows).toEqual({})
  })

  describe('the commands', () => {
    const mockInvoke = invoke as Mock
    const changed: ChangedClips = {
      clips: [clip({ id: 7, window: { start: 0.5, end: null } })],
      patternClips: [],
    }
    const run = async (thunk: (dispatch: Mock) => Promise<ChangedClips | null>) => {
      const dispatch = vi.fn()
      const result = await thunk(dispatch)
      const state = dispatch.mock.calls.reduce(
        (s, [action]) => typeof action === 'function' ? s : reducer(s, action),
        start(),
      )
      return { result, state }
    }

    beforeEach(() => {
      mockInvoke.mockReset()
      mockInvoke.mockImplementation(async (command: string) =>
        command === 'get_playlist_data' ? [16, 256, 240, 4] : changed)
    })

    it('trims clips', async () => {
      const edits = [{ id: 7, start: 0.5, end: null }]
      const { result, state } = await run(dispatch => setClipWindows(edits)(dispatch))

      expect(mockInvoke).toHaveBeenCalledWith('set_clip_windows', { edits })
      expect(result).toEqual(changed)
      expect(state.clipWindows[7]).toEqual({ start: 0.5, end: null })
    })

    it('cuts clips', async () => {
      const { result, state } = await run(dispatch => splitClips([7], 5)(dispatch))

      expect(mockInvoke).toHaveBeenCalledWith('split_clips', { ids: [7], beat: 5 })
      expect(result).toEqual(changed)
      expect(state.clipWindows[7]).toEqual({ start: 0.5, end: null })
    })

    it('stretches clips, and shows the pitch and place the engine gave them', async () => {
      const stretchedClip = clip({ id: 7, startBeat: 3, settings: { ...defaultClipSettings, pitch: -12 } })
      mockInvoke.mockImplementation(async (command: string) =>
        (command === 'get_playlist_data' ? [16, 256, 240, 4] : { clips: [stretchedClip], patternClips: [] }))
      const edits = [{ id: 7, pitch: -12, keep: 'end' as const }]
      const { result, state } = await run(dispatch => stretchClips(edits)(dispatch))

      expect(mockInvoke).toHaveBeenCalledWith('stretch_clips', { edits })
      expect(result?.clips[0].settings.pitch).toBe(-12)
      expect(state.clipSettings[7].pitch).toBe(-12)
      expect(state.playlistItems.find((item: { id: number }) => item.id === 7)?.startBeat).toBe(3)
    })

    it('gives null when the engine refuses', async () => {
      const error = vi.spyOn(console, 'error').mockImplementation(() => undefined)
      mockInvoke.mockRejectedValue('no node with id 7')

      const { result, state } = await run(dispatch => splitClips([7], 5)(dispatch))
      expect(result).toBeNull()
      expect(state.clipWindows[7]).toEqual({ start: 0.25, end: 1.5 })
      error.mockRestore()
    })
  })
})

describe('the playlist tool', () => {
  const { actions } = playlistSlice

  it('is select by default', () => {
    expect(reducer(undefined, { type: 'init' }).tool).toBe('select')
  })

  it('goes back to select when a project is loaded', () => {
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, actions.setTool('cut'))
    expect(state.tool).toBe('cut')
    state = reducer(state, reduceLoadedProject(loaded))
    expect(state.tool).toBe('select')
  })

  it('is left alone by undo and redo', () => {
    let state = reducer(undefined, { type: 'init' })
    state = reducer(state, actions.setTool('cut'))
    const project: ProjectView = {
      tempo: 90,
      clips: loaded.clips,
      patternClips: loaded.patternClips,
      mixer: loaded.mixer,
      sequencer: loaded.sequencer,
    }
    state = reducer(state, actions.reduceProjectView({ project, selected: null }))
    expect(state.tool).toBe('cut')
  })
})

describe('the clip clipboard', () => {
  const mockInvoke = invoke as Mock
  const clip = (overrides = {}) => ({
    ...loaded.clips[0], id: 20, startBeat: 12, window: null, ...overrides,
  })
  const start = () => reducer(undefined, reduceLoadedProject(loaded))
  const placed = (overrides = {}): Placed => ({
    clips: [clip({ id: 20 })],
    patternClips: [{ id: 21, patternId: 2, trackNumber: 1, startBeat: 13, window: null }],
    leftOut: 0,
    ...overrides,
  })
  const run = async <T,>(thunk: (dispatch: Mock) => Promise<T>) => {
    const dispatch = vi.fn()
    const result = await thunk(dispatch)
    const state = dispatch.mock.calls.reduce(
      (s, [action]) => typeof action === 'function' ? s : reducer(s, action),
      start(),
    )
    return { result, state, dispatch }
  }
  const noticeFrom = (dispatch: Mock) =>
    dispatch.mock.calls.map(([a]) => a).filter(a => a.type === setProjectNotice.type)

  beforeEach(() => {
    mockInvoke.mockReset()
    mockInvoke.mockImplementation(async (command: string) =>
      command === 'get_playlist_data' ? [16, 256, 240, 4] : 0)
  })

  it('copies clips, and keeps the count the engine gives', async () => {
    mockInvoke.mockImplementation(async (command: string) => (command === 'copy_clips' ? 2 : [16, 256, 240, 4]))
    const { state } = await run(dispatch => copyClips([7, 9])(dispatch))

    expect(mockInvoke).toHaveBeenCalledWith('copy_clips', { ids: [7, 9] })
    expect(state.clipboardCount).toBe(2)
    expect(state.playlistItems).toHaveLength(2)
  })

  it('cuts clips out of the playlist', async () => {
    mockInvoke.mockImplementation(async (command: string) => (command === 'cut_clips' ? 1 : [16, 256, 240, 4]))
    const { state } = await run(dispatch => cutClips([7])(dispatch))

    expect(mockInvoke).toHaveBeenCalledWith('cut_clips', { ids: [7] })
    expect(state.playlistItems.map((i: { id: number }) => i.id)).toEqual([9])
    expect(state.clipboardCount).toBe(1)
  })

  it('changes nothing when a cut is refused', async () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    mockInvoke.mockRejectedValue('no node with id 7')
    const { state } = await run(dispatch => cutClips([7])(dispatch))

    expect(state.playlistItems).toHaveLength(2)
    expect(state.clipboardCount).toBe(0)
    error.mockRestore()
  })

  it('pastes, and selects what it placed', async () => {
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'paste_clips' ? placed() : [16, 256, 240, 4]))
    const { result, state } = await run(dispatch => pasteClips(12, 3)(dispatch))

    expect(mockInvoke).toHaveBeenCalledWith('paste_clips', { beat: 12, track: 3 })
    expect(result).toEqual(placed())
    expect(state.playlistItems.map((i: { id: number }) => i.id)).toEqual([7, 9, 20, 21])
    expect(state.travelled.ids).toEqual([20, 21])
    expect(state.travelled.count).toBe(1)
  })

  it('pastes on the original tracks by default', async () => {
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'paste_clips' ? placed() : [16, 256, 240, 4]))
    await run(dispatch => pasteClips(12)(dispatch))
    expect(mockInvoke).toHaveBeenCalledWith('paste_clips', { beat: 12, track: null })
  })

  it('says why when a paste is refused, and adds nothing', async () => {
    mockInvoke.mockRejectedValue('That would go past the end of the playlist.')
    const { result, state, dispatch } = await run(d => pasteClips(250)(d))

    expect(result).toBeNull()
    expect(state.playlistItems).toHaveLength(2)
    expect(state.travelled.count).toBe(0)
    expect(noticeFrom(dispatch)).toEqual([
      setProjectNotice({ kind: 'info', message: 'That would go past the end of the playlist.' }),
    ])
  })

  it('says how many clips were left out', async () => {
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'paste_clips' ? placed({ leftOut: 1 }) : [16, 256, 240, 4]))
    const one = await run(dispatch => pasteClips(12)(dispatch))
    expect(noticeFrom(one.dispatch)).toEqual([
      setProjectNotice({ kind: 'info', message: '1 clip was left out: its pattern no longer exists.' }),
    ])

    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'paste_clips' ? placed({ leftOut: 2 }) : [16, 256, 240, 4]))
    const two = await run(dispatch => pasteClips(12)(dispatch))
    expect(noticeFrom(two.dispatch)).toEqual([
      setProjectNotice({ kind: 'info', message: '2 clips were left out: their patterns no longer exist.' }),
    ])
  })

  it('selects nothing when nothing was placed', async () => {
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'paste_clips' ? placed({ clips: [], patternClips: [], leftOut: 1 }) : [16, 256, 240, 4]))
    const { state } = await run(dispatch => pasteClips(12)(dispatch))
    expect(state.travelled.count).toBe(0)
  })

  it('repeats clips, and selects the duplicates', async () => {
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'repeat_clips' ? placed() : [16, 256, 240, 4]))
    const { result, state } = await run(dispatch => repeatClips([7, 9])(dispatch))

    expect(mockInvoke).toHaveBeenCalledWith('repeat_clips', { ids: [7, 9] })
    expect(result).toEqual(placed())
    expect(state.travelled.ids).toEqual([20, 21])
    expect(state.playlistItems).toHaveLength(4)
  })

  it('says why when a repeat is refused', async () => {
    mockInvoke.mockRejectedValue('Nothing to repeat there.')
    const { result, state, dispatch } = await run(d => repeatClips([7])(d))

    expect(result).toBeNull()
    expect(state.playlistItems).toHaveLength(2)
    expect(noticeFrom(dispatch)).toEqual([setProjectNotice({ kind: 'info', message: 'Nothing to repeat there.' })])
  })

  it('does nothing for no clips', async () => {
    await run(dispatch => copyClips([])(dispatch))
    await run(dispatch => cutClips([])(dispatch))
    await run(dispatch => repeatClips([])(dispatch))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('is empty again once a project is loaded', () => {
    const { setClipboardCount } = playlistSlice.actions
    const state = reducer(start(), setClipboardCount(3))
    expect(state.clipboardCount).toBe(3)
    expect(reducer(state, reduceLoadedProject(loaded)).clipboardCount).toBe(0)
  })
})

describe('withWaitCursor', () => {
  // (written for jest, which had no async timer advance): let the promises settle after each
  const advance = async (ms: number) => {
    vi.advanceTimersByTime(ms)
    await Array.from({ length: 10 }).reduce<Promise<unknown>>(p => p.then(() => undefined), Promise.resolve())
  }
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('shows nothing for quick work', async () => {
    const setBusy = vi.fn()
    expect(await withWaitCursor(Promise.resolve(5), setBusy)).toBe(5)
    await advance(600)
    expect(setBusy).not.toHaveBeenCalled()
  })

  it('shows the wait once, and clears it, for slow work', async () => {
    const setBusy = vi.fn()
    const work = new Promise<number>(resolve => { setTimeout(() => resolve(7), 2000) })
    const result = withWaitCursor(work, setBusy)
    await advance(499)
    expect(setBusy).not.toHaveBeenCalled()
    await advance(2)
    expect(setBusy.mock.calls).toEqual([[true]])
    await advance(2000)
    expect(await result).toBe(7)
    expect(setBusy.mock.calls).toEqual([[true], [false]])
  })

  it('clears the wait when slow work fails, and still rejects', async () => {
    const setBusy = vi.fn()
    const work = new Promise<number>((_, reject) => { setTimeout(() => reject(new Error('no')), 1000) })
    const result = withWaitCursor(work, setBusy)
    const caught = expect(result).rejects.toThrow('no')
    await advance(1000)
    await caught
    expect(setBusy.mock.calls).toEqual([[true], [false]])
  })

  it('never shows the wait for quick work that fails', async () => {
    const setBusy = vi.fn()
    await expect(withWaitCursor(Promise.reject(new Error('no')), setBusy)).rejects.toThrow('no')
    await advance(600)
    expect(setBusy).not.toHaveBeenCalled()
  })
})

describe('reduceChangedClips with a deep copy', () => {
  it('turns a pattern clip into a sample clip of the same id', () => {
    const base = reducer(undefined, reduceLoadedProject(loaded))
    const withPattern = reducer(base, playlistSlice.actions.reduceChangedClips({
      clips: [],
      patternClips: [{ id: 30, patternId: 2, trackNumber: 1, startBeat: 3, window: null }],
    }))
    expect(withPattern.playlistItems.find(i => i.id === 30)?.patternId).toBe(2)

    const state = reducer(withPattern, playlistSlice.actions.reduceChangedClips({
      clips: [{
        ...loaded.clips[0], id: 30, path: '/project/recordings/30.wav', trackNumber: 1, startBeat: 3, window: null,
      }],
      patternClips: [],
    }))
    const item = state.playlistItems.find(i => i.id === 30)
    expect(state.playlistItems.filter(i => i.id === 30)).toHaveLength(1)
    expect(item?.patternId).toBeUndefined()
    expect(item?.path).toBe('/project/recordings/30.wav')
    expect(state.clipSettings[30]).toEqual(loaded.clips[0].settings)
    expect(state.clipSampleDurations[30]).toBe(loaded.clips[0].sampleDuration)
  })
})

describe('deep copy and join', () => {
  const mockInvoke = invoke as Mock
  const mockSave = runProjectCommand as unknown as Mock
  const clip = (overrides = {}) => ({
    ...loaded.clips[0], id: 7, path: '/project/7.wav', window: null, ...overrides,
  })
  const start = () => reducer(undefined, reduceLoadedProject(loaded))
  const path = (value: string | null) => ({ project: { path: value } })
  // runs a thunk against a state whose project is (or isn't) saved
  const run = async <T,>(thunk: (dispatch: Mock, getState: () => unknown) => Promise<T>, saved = true) => {
    const dispatch = vi.fn()
    const result = await thunk(dispatch, () => path(saved ? '/p.dawesome' : null))
    const state = dispatch.mock.calls.reduce(
      (s, [action]) => typeof action === 'function' ? s : reducer(s, action),
      start(),
    )
    return { result, state, dispatch }
  }
  const actions = (dispatch: Mock) => dispatch.mock.calls.map(([a]) => a).filter(a => typeof a !== 'function')
  const noticeFrom = (dispatch: Mock) => actions(dispatch).filter(a => a.type === setProjectNotice.type)
  // the engine's refusals are plain sentences, not Errors
  const refuse = (message: string) => vi.fn().mockRejectedValue(message)()
  const answers = (over: Record<string, unknown>) => async (command: string) =>
    command in over ? over[command] : [16, 256, 240, 4]
  const joined = (over = {}) => ({
    clips: [clip({ id: 7, path: '/project/joined.wav' })], patternClips: [], removed: [9], sequencer: null, ...over,
  })

  beforeEach(() => {
    mockInvoke.mockReset()
    mockSave.mockReset()
    mockSave.mockReturnValue(() => Promise.resolve())
  })

  it('deep copies clips and shows the new files', async () => {
    const changed = { clips: [clip()], patternClips: [] }
    mockInvoke.mockImplementation(answers({ deep_copy_clips: changed }))
    const { result, state } = await run((d, g) => deepCopyClips([7])(d, g as never))

    expect(mockInvoke).toHaveBeenCalledWith('deep_copy_clips', { ids: [7] })
    expect(result).toEqual(changed)
    expect(state.playlistItems.find((i: { id: number }) => i.id === 7)?.path).toBe('/project/7.wav')
  })

  it('tells the Clips page to list again after a deep copy, not after a failed one', async () => {
    mockInvoke.mockImplementation(answers({ deep_copy_clips: { clips: [clip()], patternClips: [] } }))
    const done = await run((d, g) => deepCopyClips([7])(d, g as never))
    expect(actions(done.dispatch)).toContainEqual({ type: 'recordingSlice/takesChanged', payload: undefined })

    mockInvoke.mockReset()
    mockInvoke.mockRejectedValue('disk full')
    const failed = await run((d, g) => deepCopyClips([7])(d, g as never))
    expect(actions(failed.dispatch).some(a => a.type === 'recordingSlice/takesChanged')).toBe(false)
  })

  it('tells the Clips page to list again after a join, not after a failed one', async () => {
    mockInvoke.mockImplementation(answers({ join_clips: joined() }))
    const done = await run((d, g) => joinClips([7, 9])(d, g as never))
    expect(actions(done.dispatch)).toContainEqual({ type: 'recordingSlice/takesChanged', payload: undefined })

    mockInvoke.mockReset()
    mockInvoke.mockRejectedValue('disk full')
    const failed = await run((d, g) => joinClips([7, 9])(d, g as never))
    expect(actions(failed.dispatch).some(a => a.type === 'recordingSlice/takesChanged')).toBe(false)
  })

  it('deep copies nothing for no clips', async () => {
    await run((d, g) => deepCopyClips([])(d, g as never))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('saves first when the engine asks, then tries again once', async () => {
    const changed = { clips: [clip()], patternClips: [] }
    let calls = 0
    mockInvoke.mockImplementation(async (command: string) => {
      if(command !== 'deep_copy_clips') return [16, 256, 240, 4]
      if(++calls === 1) return refuse(NEEDS_SAVING)
      return changed
    })
    const dispatch = vi.fn()
    let saved = false
    mockSave.mockReturnValue(async () => { saved = true })
    const result = await deepCopyClips([7])(dispatch, () => path(saved ? '/p.dawesome' : null) as never)

    expect(mockSave).toHaveBeenCalledWith('save')
    expect(calls).toBe(2)
    expect(result).toEqual(changed)
    expect(noticeFrom(dispatch)).toEqual([])
  })

  it('does nothing more when the save is cancelled', async () => {
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'deep_copy_clips') return refuse(NEEDS_SAVING)
      return [16, 256, 240, 4]
    })
    const { result, state, dispatch } = await run((d, g) => deepCopyClips([7])(d, g as never), false)

    expect(result).toBeNull()
    expect(mockInvoke.mock.calls.filter(([c]) => c === 'deep_copy_clips')).toHaveLength(1)
    expect(noticeFrom(dispatch)).toEqual([])
    expect(actions(dispatch)).toEqual([])
    expect(state.playlistItems).toHaveLength(2)
  })

  it('treats a second failure after saving like any other', async () => {
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'deep_copy_clips') return refuse(NEEDS_SAVING)
      return [16, 256, 240, 4]
    })
    const { result, dispatch } = await run((d, g) => deepCopyClips([7])(d, g as never))

    expect(result).toBeNull()
    expect(mockInvoke.mock.calls.filter(([c]) => c === 'deep_copy_clips')).toHaveLength(2)
    expect(noticeFrom(dispatch)).toEqual([setProjectNotice({ kind: 'error', message: NEEDS_SAVING })])
  })

  it('shows another failure in the error style, and changes nothing', async () => {
    mockInvoke.mockRejectedValue('disk full')
    const { result, state, dispatch } = await run((d, g) => deepCopyClips([7])(d, g as never))

    expect(result).toBeNull()
    expect(noticeFrom(dispatch)).toEqual([setProjectNotice({ kind: 'error', message: 'disk full' })])
    expect(state.playlistItems).toHaveLength(2)
  })

  it('joins clips: the others go, the joined clip is selected', async () => {
    mockInvoke.mockImplementation(answers({ join_clips: joined() }))
    const { result, state } = await run((d, g) => joinClips([7, 9])(d, g as never))

    expect(mockInvoke).toHaveBeenCalledWith('join_clips', { ids: [7, 9] })
    expect(result).toEqual(joined())
    expect(state.playlistItems.map((i: { id: number }) => i.id)).toEqual([7])
    expect(state.playlistItems[0].path).toBe('/project/joined.wav')
    expect(state.travelled.ids).toEqual([7])
  })

  it('sets the sequencer when a join made a pattern', async () => {
    const sequencer = { patterns: [] }
    mockInvoke.mockImplementation(answers({
      join_clips: joined({
        clips: [], removed: [9], sequencer,
        patternClips: [{ id: 7, patternId: 3, trackNumber: 2, startBeat: 4.5, window: null }],
      }),
    }))
    const { state, dispatch } = await run((d, g) => joinClips([7, 9])(d, g as never))

    expect(actions(dispatch)).toContainEqual({ type: 'sequencerSlice/setSequencer', payload: sequencer })
    expect(state.playlistItems.find((i: { id: number }) => i.id === 7)?.patternId).toBe(3)
    expect(state.travelled.ids).toEqual([7])
  })

  it('does not set the sequencer when there is no new pattern', async () => {
    mockInvoke.mockImplementation(answers({ join_clips: joined() }))
    const { dispatch } = await run((d, g) => joinClips([7, 9])(d, g as never))
    expect(actions(dispatch).some(a => a.type === 'sequencerSlice/setSequencer')).toBe(false)
  })

  it('invokes nothing for fewer than two clips', async () => {
    const { dispatch } = await run((d, g) => joinClips([7])(d, g as never))
    await run((d, g) => joinClips([])(d, g as never))
    expect(mockInvoke).not.toHaveBeenCalled()
    expect(dispatch).not.toHaveBeenCalled()
  })

  it('saves first, then joins', async () => {
    let calls = 0
    mockInvoke.mockImplementation(async (command: string) => {
      if(command !== 'join_clips') return [16, 256, 240, 4]
      if(++calls === 1) return refuse(NEEDS_SAVING)
      return joined()
    })
    const { result, state } = await run((d, g) => joinClips([7, 9])(d, g as never))

    expect(mockSave).toHaveBeenCalledWith('save')
    expect(calls).toBe(2)
    expect(result).toEqual(joined())
    expect(state.playlistItems.map((i: { id: number }) => i.id)).toEqual([7])
  })

  it('leaves the clips alone when the save is cancelled', async () => {
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'join_clips') return refuse(NEEDS_SAVING)
      return [16, 256, 240, 4]
    })
    const { result, state, dispatch } = await run((d, g) => joinClips([7, 9])(d, g as never), false)

    expect(result).toBeNull()
    expect(noticeFrom(dispatch)).toEqual([])
    expect(state.playlistItems).toHaveLength(2)
  })

  it('says in the info style when a join does not apply', async () => {
    mockInvoke.mockRejectedValue('Join needs clips on one track.')
    const { result, state, dispatch } = await run((d, g) => joinClips([7, 9])(d, g as never))

    expect(result).toBeNull()
    expect(noticeFrom(dispatch)).toEqual([setProjectNotice({ kind: 'info', message: 'Join needs clips on one track.' })])
    expect(state.playlistItems).toHaveLength(2)
  })

  it('shows any other join failure in the error style', async () => {
    mockInvoke.mockRejectedValue('disk full')
    const { dispatch } = await run((d, g) => joinClips([7, 9])(d, g as never))
    expect(noticeFrom(dispatch)).toEqual([setProjectNotice({ kind: 'error', message: 'disk full' })])
  })
})
