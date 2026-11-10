import { invoke } from '@tauri-apps/api/core'
import { clipCommand, ClipTarget, sameCommand } from './clipCommands'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
jest.mock('@tauri-apps/plugin-dialog', () => ({ ask: jest.fn(), open: jest.fn(), save: jest.fn() }))
jest.mock('@tauri-apps/plugin-fs', () => ({ readDir: jest.fn() }))
jest.mock('@tauri-apps/api/path', () => ({ dirname: jest.fn(), documentDir: jest.fn(), join: jest.fn() }))

const mockInvoke = invoke as jest.Mock
const target = (over: Partial<ClipTarget> = {}): ClipTarget => ({
  selected: [3, 4], all: [3, 4, 5], pasteBeat: 8, focused: true, ...over,
})

describe('clipCommand', () => {
  beforeEach(() => {
    mockInvoke.mockReset()
    const placed = { clips: [], patternClips: [], leftOut: 0 }
    const answers: Record<string, unknown> = {
      get_playlist_data: [16, 256, 240, 4],
      paste_clips: placed,
      repeat_clips: placed,
    }
    mockInvoke.mockImplementation(async (command: string) => answers[command] ?? 2)
  })

  const run = (command: Parameters<typeof clipCommand>[0], t: ClipTarget, select = jest.fn()) =>
    clipCommand(command, t, select)(jest.fn(), jest.fn()).then(() => select)

  it('copies the selection', async () => {
    await run('copy', target())
    expect(mockInvoke).toHaveBeenCalledWith('copy_clips', { ids: [3, 4] })
  })

  it('cuts the selection', async () => {
    await run('cut', target())
    expect(mockInvoke).toHaveBeenCalledWith('cut_clips', { ids: [3, 4] })
  })

  it('pastes at the playhead, on the original tracks', async () => {
    await run('paste', target())
    expect(mockInvoke).toHaveBeenCalledWith('paste_clips', { beat: 8, track: null })
  })

  it('pastes at beat 0', async () => {
    await run('paste', target({ pasteBeat: 0 }))
    expect(mockInvoke).toHaveBeenCalledWith('paste_clips', { beat: 0, track: null })
  })

  it('repeats the selection', async () => {
    await run('repeat', target())
    expect(mockInvoke).toHaveBeenCalledWith('repeat_clips', { ids: [3, 4] })
  })

  it('joins two or more selected clips', async () => {
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'join_clips' ? { clips: [], patternClips: [], removed: [], sequencer: null } : [16, 256, 240, 4]))
    await run('join', target())
    expect(mockInvoke).toHaveBeenCalledWith('join_clips', { ids: [3, 4] })
  })

  it('does not join fewer than two clips', async () => {
    await run('join', target({ selected: [3] }))
    await run('join', target({ selected: [] }))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('deep copies the selection, and passes on the wait cursor', async () => {
    const setBusy = jest.fn()
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'deep_copy_clips' ? { clips: [], patternClips: [] } : [16, 256, 240, 4]))
    await clipCommand('deep-copy', target(), jest.fn(), setBusy)(jest.fn(), jest.fn())
    expect(mockInvoke).toHaveBeenCalledWith('deep_copy_clips', { ids: [3, 4] })
    // (quick, so the cursor never showed)
    expect(setBusy).not.toHaveBeenCalled()
  })

  it('deep copies nothing with nothing selected', async () => {
    await run('deep-copy', target({ selected: [] }))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('selects every clip, without asking the engine', async () => {
    const select = await run('select-all', target())
    expect(select).toHaveBeenCalledWith([3, 4, 5])
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('does nothing with nothing selected, bar paste and select all', async () => {
    await run('copy', target({ selected: [] }))
    await run('cut', target({ selected: [] }))
    await run('repeat', target({ selected: [] }))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('does not paste when there is no place for it', async () => {
    await run('paste', target({ pasteBeat: null }))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('does nothing unless the Playlist is focused', async () => {
    const select = jest.fn()
    await Promise.all((['copy', 'cut', 'paste', 'repeat', 'select-all', 'join', 'deep-copy'] as const)
      .map(command => run(command, target({ focused: false }), select)))
    expect(mockInvoke).not.toHaveBeenCalled()
    expect(select).not.toHaveBeenCalled()
  })
})

describe('sameCommand', () => {
  const last = { command: 'paste' as const, source: 'key' as const, at: 1000 }

  it('is the same command arriving by another route a moment later', () => {
    expect(sameCommand(last, 'paste', 'menu', 1050)).toBe(true)
    expect(sameCommand(last, 'paste', 'clipboard-event', 1100)).toBe(true)
  })

  it('is not one arriving by the same route, as a held key repeats', () => {
    expect(sameCommand(last, 'paste', 'key', 1050)).toBe(false)
  })

  it('is not a different command, a later press, or the first', () => {
    expect(sameCommand(last, 'copy', 'menu', 1050)).toBe(false)
    expect(sameCommand(last, 'paste', 'menu', 1150)).toBe(false)
    expect(sameCommand(null, 'paste', 'menu', 1050)).toBe(false)
  })
})
