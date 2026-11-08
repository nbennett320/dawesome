import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { ask, open, save } from '@tauri-apps/api/dialog'
import { documentDir, join } from '@tauri-apps/api/path'
import playlistReducer, { playlistSlice } from './playlistSlice'
import projectReducer, {
  openRecentProject, projectName, runProjectCommand, setProjectPath,
} from './projectSlice'
import { LoadedProject } from '../../types/project'

jest.mock('@tauri-apps/api', () => ({ invoke: jest.fn() }))
jest.mock('@tauri-apps/api/dialog', () => ({ ask: jest.fn(), open: jest.fn(), save: jest.fn() }))
jest.mock('@tauri-apps/api/path', () => ({ documentDir: jest.fn(), join: jest.fn() }))

const mockInvoke = invoke as jest.Mock
const mockAsk = ask as jest.Mock
const mockOpen = open as jest.Mock
const mockSave = save as jest.Mock
const mockDocumentDir = documentDir as jest.Mock
const mockJoin = join as jest.Mock

const emptyProject: LoadedProject = {
  tempo: 120,
  timeSignature: { numerator: 4, denominator: 4 },
  metronomeEnabled: true,
  loopEnabled: true,
  snapEnabled: true,
  snapNote: 'QuarterNote',
  clips: [],
  missing: [],
  mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
}

// the engine's answers to the commands these tests use
const engine = (overrides: Record<string, unknown> = {}) => {
  mockInvoke.mockImplementation(async (cmd: string) => {
    if(cmd in overrides) {
      const answer = overrides[cmd]
      if(answer instanceof Error) throw answer
      return answer
    }
    if(cmd === 'get_playlist_data') return [16, 16, 0, 4]
    if(cmd === 'get_snap_setting') return { enabled: true, note: 'QuarterNote', beats: 1 }
    if(cmd === 'new_project' || cmd === 'load_project') return emptyProject
    return undefined
  })
}

const makeStore = () => configureStore({
  reducer: { playlist: playlistReducer, project: projectReducer },
})

const run = (store: ReturnType<typeof makeStore>, command: Parameters<typeof runProjectCommand>[0]) =>
  // the store's state is a subset of RootState, which is all these commands use
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  runProjectCommand(command)(store.dispatch, store.getState as any)

beforeEach(() => {
  jest.resetAllMocks()
  engine()
  mockDocumentDir.mockResolvedValue('/Users/me/Documents/')
  mockJoin.mockImplementation(async (dir: string, file: string) => `${dir.replace(/\/$/, '')}/${file}`)
})

describe('projectName', () => {
  it('is the file name without its extension', () => {
    expect(projectName('/songs/My Song.dawesome')).toBe('My Song')
    expect(projectName('C:\\songs\\beat.DAWESOME')).toBe('beat')
    expect(projectName(null)).toBeNull()
  })
})

describe('runProjectCommand', () => {
  it('only ever starts the save dialog at a full path', async () => {
    const store = makeStore()
    mockSave.mockResolvedValue('/songs/song.dawesome')

    // a bare file name here crashes the app on macOS
    await run(store, 'save-as')
    expect(mockSave).toHaveBeenLastCalledWith(expect.objectContaining({
      defaultPath: '/Users/me/Documents/Untitled.dawesome',
    }))

    await run(store, 'save-as')
    expect(mockSave).toHaveBeenLastCalledWith(expect.objectContaining({
      defaultPath: '/songs/song.dawesome',
    }))

    // without a documents folder, the dialog picks where to start
    store.dispatch(setProjectPath(null))
    mockDocumentDir.mockRejectedValue(new Error('no documents folder'))
    await run(store, 'save-as')
    expect(mockSave).toHaveBeenLastCalledWith(expect.objectContaining({ defaultPath: undefined }))
  })

  it('asks where to save an unsaved project, then saves there from then on', async () => {
    const store = makeStore()
    mockSave.mockResolvedValue('/songs/song')

    await run(store, 'save')
    expect(mockInvoke).toHaveBeenCalledWith('save_project', { path: '/songs/song.dawesome' })
    expect(store.getState().project.path).toBe('/songs/song.dawesome')

    mockInvoke.mockClear()
    await run(store, 'save')
    expect(mockSave).toHaveBeenCalledTimes(1)
    expect(mockInvoke).toHaveBeenCalledWith('save_project', { path: '/songs/song.dawesome' })
  })

  it('does nothing when a dialog is cancelled', async () => {
    const store = makeStore()
    mockSave.mockResolvedValue(null)
    mockOpen.mockResolvedValue(null)

    await run(store, 'save-as')
    await run(store, 'open')

    expect(mockInvoke).not.toHaveBeenCalledWith('save_project', expect.anything())
    expect(mockInvoke).not.toHaveBeenCalledWith('load_project', expect.anything())
    expect(store.getState().project.path).toBeNull()
  })

  it('opens a project and lists the samples it couldn’t find', async () => {
    const store = makeStore()
    const missing = [{ path: '/gone.wav', reason: 'file not found' }]
    engine({ load_project: { ...emptyProject, tempo: 90, missing } })
    mockOpen.mockResolvedValue('/songs/old.dawesome')

    await run(store, 'open')

    expect(store.getState().project.path).toBe('/songs/old.dawesome')
    expect(store.getState().playlist.tempo).toBe(90)
    expect(store.getState().project.notice).toEqual({ kind: 'missing', samples: missing })
  })

  it('keeps the current project when one can’t be opened', async () => {
    const store = makeStore()
    store.dispatch(setProjectPath('/songs/current.dawesome'))
    engine({ load_project: new Error('not a valid project file') })
    mockOpen.mockResolvedValue('/songs/broken.dawesome')

    await run(store, 'open')

    expect(store.getState().project.path).toBe('/songs/current.dawesome')
    expect(store.getState().project.notice).toMatchObject({ kind: 'error' })
  })

  it('asks before replacing a playlist with clips', async () => {
    const store = makeStore()
    store.dispatch(playlistSlice.actions.addPlaylistItem({ id: 1, path: 'a.wav', startBeat: 0, trackNumber: 0 }))

    mockAsk.mockResolvedValue(false)
    await run(store, 'new')
    expect(mockInvoke).not.toHaveBeenCalledWith('new_project', expect.anything())
    expect(store.getState().playlist.playlistItems).toHaveLength(1)

    mockAsk.mockResolvedValue(true)
    await run(store, 'new')
    expect(store.getState().playlist.playlistItems).toHaveLength(0)
  })

  it('adds projects that were opened or saved to the recent list', async () => {
    const store = makeStore()
    mockOpen.mockResolvedValue('/songs/old.dawesome')
    mockSave.mockResolvedValue('/songs/new.dawesome')

    await run(store, 'open')
    expect(mockInvoke).toHaveBeenCalledWith('add_recent_project', { path: '/songs/old.dawesome' })

    await run(store, 'save-as')
    expect(mockInvoke).toHaveBeenCalledWith('add_recent_project', { path: '/songs/new.dawesome' })
  })

  it('doesn’t add a project that failed to save', async () => {
    const store = makeStore()
    engine({ save_project: new Error('disk full') })
    mockSave.mockResolvedValue('/songs/new.dawesome')

    await run(store, 'save-as')

    expect(mockInvoke).not.toHaveBeenCalledWith('add_recent_project', expect.anything())
    expect(store.getState().project.path).toBeNull()
  })
})

describe('openRecentProject', () => {
  const openRecent = (store: ReturnType<typeof makeStore>, path: string) =>
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    openRecentProject(path)(store.dispatch, store.getState as any)

  it('opens the project without asking where', async () => {
    const store = makeStore()

    await openRecent(store, '/songs/old.dawesome')

    expect(mockOpen).not.toHaveBeenCalled()
    expect(mockInvoke).toHaveBeenCalledWith('load_project', { path: '/songs/old.dawesome' })
    expect(store.getState().project.path).toBe('/songs/old.dawesome')
    expect(mockInvoke).toHaveBeenCalledWith('add_recent_project', { path: '/songs/old.dawesome' })
  })

  it('takes a project that can’t be opened off the list', async () => {
    const store = makeStore()
    engine({ load_project: new Error('could not open') })

    await openRecent(store, '/songs/gone.dawesome')

    expect(mockInvoke).toHaveBeenCalledWith('remove_recent_project', { path: '/songs/gone.dawesome' })
    expect(store.getState().project.notice).toMatchObject({ kind: 'error' })
  })

  it('asks before replacing a playlist with clips', async () => {
    const store = makeStore()
    store.dispatch(playlistSlice.actions.addPlaylistItem({ id: 1, path: 'a.wav', startBeat: 0, trackNumber: 0 }))
    mockAsk.mockResolvedValue(false)

    await openRecent(store, '/songs/old.dawesome')

    expect(mockInvoke).not.toHaveBeenCalledWith('load_project', expect.anything())
  })
})
