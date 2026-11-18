import type { Mock } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { message } from '@tauri-apps/plugin-dialog'
import { ONLY_COPY, PLACE_ALL, dropFilesOnPlaylist, importFiles, leftOutSentence } from './fileDrop'
import { setProjectNotice } from './slices/projectSlice'
import { defaultClipSettings } from '../types/clip'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), message: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))

const mockInvoke = invoke as Mock
const mockMessage = message as unknown as Mock

const files = (n: number) => Array.from({ length: n }, (_, i) => ({ path: `/finder/f${i}.wav`, folder: '' }))
const copies = (n: number) => files(n).map(({ path }, i) => ({ source: path, path: `/song/clips/f${i}.wav`, reused: false }))
const offset = { x: 10, y: 20, left: 0, top: 0, right: 100, bottom: 100 }

// what the engine answers, by command
const engine = (over: Record<string, unknown>) => mockInvoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
  if(command in over) {
    const answer = over[command]
    return typeof answer === 'function' ? answer(args) : answer
  }
  if(command === 'get_clip') return { settings: defaultClipSettings, sampleDuration: 1, window: null }
  if(command === 'get_playlist_data') return { trackCount: 8 }
  throw new Error(`unexpected ${command}`)
})
const actions = (dispatch: Mock) => dispatch.mock.calls.map(([a]) => a).filter(a => typeof a !== 'function')
const types = (dispatch: Mock) => actions(dispatch).map(a => a.type)
const notices = (dispatch: Mock) => actions(dispatch).filter(a => a.type === setProjectNotice.type).map(a => a.payload)
const placed = (n: number) => Array.from({ length: n }, (_, i) => ({ id: 100 + i, startBeat: 4 }))

beforeEach(() => {
  mockInvoke.mockReset()
  mockMessage.mockReset()
})

describe('leftOutSentence', () => {
  it('names a few, and counts the rest', () => {
    expect(leftOutSentence([{ path: '/a/notes.txt', reason: 'not audio' }])).toBe('Left out: notes.txt (not audio).')
    const many = Array.from({ length: 8 }, (_, i) => ({ path: `/a/${i}.txt`, reason: 'not audio' }))
    expect(leftOutSentence(many, 2)).toBe('Left out: 0.txt (not audio), 1.txt (not audio), and 6 more.')
  })
})

describe('dropFilesOnPlaylist', () => {
  it('copies, then places one per track as the engine put them, selected', async () => {
    engine({ import_files: { files: copies(3), skipped: [] }, place_files: placed(3) })
    const dispatch = vi.fn()
    const ids = await dropFilesOnPlaylist(files(3), [], 2, 4.3, offset)(dispatch)

    expect(mockInvoke).toHaveBeenCalledWith('import_files', { files: files(3), into: '' })
    expect(mockInvoke).toHaveBeenCalledWith('place_files', {
      paths: ['/song/clips/f0.wav', '/song/clips/f1.wav', '/song/clips/f2.wav'], trackNumber: 2, dropBeat: 4.3,
    })
    expect(ids).toEqual([100, 101, 102])
    const added = actions(dispatch).filter(a => a.type === 'playlistSlice/addPlaylistItem').map(a => a.payload)
    expect(added.map((a: { trackNumber: number }) => a.trackNumber)).toEqual([2, 3, 4])
    expect(added[1]).toMatchObject({ id: 101, path: '/song/clips/f1.wav', startBeat: 4 })
    expect(actions(dispatch)).toContainEqual({ type: 'playlistSlice/selectClips', payload: [100, 101, 102] })
    expect(types(dispatch)).toContain('recordingSlice/takesChanged')
    expect(types(dispatch)).toContain('playlistSlice/setTrackCount')
    expect(actions(dispatch).filter(a => a.type === 'playlistSlice/setClipInfo')).toHaveLength(3)
    expect(mockMessage).not.toHaveBeenCalled()
    expect(notices(dispatch)).toEqual([])
  })

  it('says what was left out, by the scan or the copy', async () => {
    engine({ import_files: { files: copies(1), skipped: [{ path: '/finder/full.wav', reason: 'could not be copied (disk full)' }] }, place_files: placed(1) })
    const dispatch = vi.fn()
    await dropFilesOnPlaylist(files(1), [{ path: '/finder/notes.txt', reason: 'not audio' }], 0, 0, offset)(dispatch)
    expect(notices(dispatch)).toEqual([{
      kind: 'error', message: 'Left out: notes.txt (not audio), full.wav (could not be copied (disk full)).',
    }])
  })

  it('asks about more than 16 files: place all, only copy, or cancel', async () => {
    engine({ import_files: { files: copies(17), skipped: [] }, place_files: placed(17) })

    mockMessage.mockResolvedValueOnce(PLACE_ALL)
    expect(await dropFilesOnPlaylist(files(17), [], 0, 0, offset)(vi.fn())).toHaveLength(17)
    expect(mockMessage.mock.calls[0][1].buttons).toEqual({ yes: PLACE_ALL, no: ONLY_COPY, cancel: 'Cancel' })

    mockInvoke.mockClear()
    mockMessage.mockResolvedValueOnce(ONLY_COPY)
    const copyOnly = vi.fn()
    expect(await dropFilesOnPlaylist(files(17), [], 0, 0, offset)(copyOnly)).toEqual([])
    expect(mockInvoke.mock.calls.map(([c]) => c)).toEqual(['import_files'])
    expect(types(copyOnly)).toContain('recordingSlice/takesChanged')

    mockInvoke.mockClear()
    mockMessage.mockResolvedValueOnce('Cancel')
    expect(await dropFilesOnPlaylist(files(17), [], 0, 0, offset)(vi.fn())).toEqual([])
    expect(mockInvoke).not.toHaveBeenCalled()

    // 16 just goes ahead
    mockMessage.mockClear()
    engine({ import_files: { files: copies(16), skipped: [] }, place_files: placed(16) })
    await dropFilesOnPlaylist(files(16), [], 0, 0, offset)(vi.fn())
    expect(mockMessage).not.toHaveBeenCalled()
  })

  it('places nothing when nothing could be brought in', async () => {
    engine({})
    const dispatch = vi.fn()
    expect(await dropFilesOnPlaylist([], [{ path: '/a/notes.txt', reason: 'not audio' }], 0, 0, offset)(dispatch)).toEqual([])
    expect(mockInvoke).not.toHaveBeenCalled()
    expect(notices(dispatch)).toEqual([{ kind: 'error', message: 'Left out: notes.txt (not audio).' }])
  })

  it('shows a failure to place, keeping the copies', async () => {
    engine({ import_files: { files: copies(2), skipped: [] }, place_files: () => Promise.reject('could not decode sample') })
    const dispatch = vi.fn()
    expect(await dropFilesOnPlaylist(files(2), [], 0, 0, offset)(dispatch)).toEqual([])
    expect(notices(dispatch)).toEqual([{ kind: 'error', message: 'could not decode sample' }])
    expect(types(dispatch)).toContain('recordingSlice/takesChanged')
    expect(types(dispatch)).not.toContain('playlistSlice/addPlaylistItem')
  })

  // the files drawn where they're landing, as loading, until they're in
  describe('while placing', () => {
    const holding = () => {
      const events: string[] = []
      const hold = vi.fn(() => {
        events.push('held')
        return () => { events.push('let go') }
      })
      return { events, hold }
    }

    it('shows the files until the clips and their lengths are in', async () => {
      const { events, hold } = holding()
      engine({
        import_files: { files: copies(2), skipped: [] },
        place_files: () => { events.push('placing'); return placed(2) },
      })
      const dispatch = vi.fn()
      dispatch.mockImplementation((action: { type?: string }) => {
        if(action.type === 'playlistSlice/setClipInfo') events.push('length')
      })
      await dropFilesOnPlaylist(files(2), [], 0, 0, offset, undefined, hold)(dispatch)
      expect(events).toEqual(['held', 'placing', 'length', 'length', 'let go'])
    })

    it('stops showing them when placing fails', async () => {
      const { events, hold } = holding()
      engine({ import_files: { files: copies(1), skipped: [] }, place_files: () => Promise.reject('could not decode sample') })
      await dropFilesOnPlaylist(files(1), [], 0, 0, offset, undefined, hold)(vi.fn())
      expect(events).toEqual(['held', 'let go'])
    })

    it('doesn’t show them for only copying, or while asking', async () => {
      const { hold } = holding()
      engine({ import_files: { files: copies(17), skipped: [] } })
      mockMessage.mockImplementationOnce(async () => {
        expect(hold).not.toHaveBeenCalled()
        return ONLY_COPY
      })
      await dropFilesOnPlaylist(files(17), [], 0, 0, offset, undefined, hold)(vi.fn())
      mockMessage.mockResolvedValueOnce('Cancel')
      await dropFilesOnPlaylist(files(17), [], 0, 0, offset, undefined, hold)(vi.fn())
      expect(hold).not.toHaveBeenCalled()
    })
  })
})

describe('importFiles', () => {
  it('copies into a folder of the clips, and only lists again for new files', async () => {
    engine({ import_files: { files: [{ source: '/f/a.wav', path: '/song/clips/Drums/a.wav', reused: true }], skipped: [] } })
    const dispatch = vi.fn()
    const imported = await importFiles(dispatch, files(1), [], 'Drums')
    expect(mockInvoke).toHaveBeenCalledWith('import_files', { files: files(1), into: 'Drums' })
    expect(imported?.files).toHaveLength(1)
    expect(types(dispatch)).not.toContain('recordingSlice/takesChanged')
  })

  it('shows the engine refusing', async () => {
    engine({ import_files: () => Promise.reject("'..' isn't a folder in the clips") })
    const dispatch = vi.fn()
    expect(await importFiles(dispatch, files(1), [], '..')).toBeNull()
    expect(notices(dispatch)).toEqual([{ kind: 'error', message: "'..' isn't a folder in the clips" }])
  })
})
