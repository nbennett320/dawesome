import type { Mock } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { store } from './store'
import { selectPattern, setSequencer, togglePlayHere } from './slices/sequencerSlice'
import { openTab } from './slices/windowSlice'
import { listenForTransportKey } from '../util/keys'
import { View } from '../types/ui'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
// the store's project slice opens dialogs and lists folders
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

const stopped = {
  playing: false, patternId: null, startTimeMs: null, beatIntervalMs: 500, startBeat: 0, loopBeats: 16,
}

// the commands the engine was sent, in order, without the ones that only ask
const sent = () => mockInvoke.mock.calls
  .map(([command, args]) => [command, args])
  .filter(([command]) => (command as string).startsWith('t_'))

const patterns = (ids: number[]) => ({
  rows: [],
  patterns: ids.map(id => ({ id, name: null, length: 16, steps: {} })),
  folders: [],
})

// what the topbar's Play key and Space both dispatch
const pressPlay = () => store.dispatch(togglePlayHere())

describe('the Play key and Space', () => {
  beforeEach(() => {
    mockInvoke.mockReset()
    mockInvoke.mockImplementation(async (command: string) =>
      (command === 'get_playlist_transport' ? stopped : undefined))
    store.dispatch(setSequencer(patterns([3, 5])))
    store.dispatch(selectPattern(5))
  })

  it('plays the playlist when the playlist has the focus', async () => {
    store.dispatch(openTab(View.Playlist))
    await pressPlay()
    expect(sent()).toEqual([['t_toggle_playlist', {}]])
  })

  it('plays the playlist from any other view', async () => {
    store.dispatch(openTab(View.Mixer))
    await pressPlay()
    expect(sent()).toEqual([['t_toggle_playlist', {}]])
  })

  it('loops the selected pattern when the Sequencer has the focus', async () => {
    store.dispatch(openTab(View.Sequencer))
    await pressPlay()
    expect(sent()).toEqual([['t_toggle_pattern', { patternId: 5 }]])
  })

  it('plays the playlist from the Sequencer when there is no pattern', async () => {
    store.dispatch(openTab(View.Sequencer))
    store.dispatch(setSequencer(patterns([])))
    await pressPlay()
    expect(sent()).toEqual([['t_toggle_playlist', {}]])
  })
})

describe('Space, listened for on the window', () => {
  const key = (type: 'keydown' | 'keyup', init: KeyboardEventInit = {}) =>
    new KeyboardEvent(type, { key: ' ', bubbles: true, cancelable: true, ...init })

  let stop: () => void
  let toggle: Mock
  let reached: Mock
  let button: HTMLButtonElement

  beforeEach(() => {
    toggle = vi.fn()
    reached = vi.fn()
    stop = listenForTransportKey(window, toggle)
    button = document.createElement('button')
    document.body.appendChild(button)
    // what a focused control, React's root and p5 would hear
    button.addEventListener('keydown', reached)
    button.addEventListener('keyup', reached)
    document.addEventListener('keydown', reached)
    document.addEventListener('keyup', reached)
  })

  afterEach(() => {
    stop()
    document.removeEventListener('keydown', reached)
    document.removeEventListener('keyup', reached)
    button.remove()
  })

  it('toggles once, before a focused button hears the key go down or up', () => {
    const down = key('keydown')
    const up = key('keyup')
    button.dispatchEvent(down)
    button.dispatchEvent(up)

    expect(toggle).toHaveBeenCalledTimes(1)
    expect(down.defaultPrevented).toBe(true)
    expect(up.defaultPrevented).toBe(true)
    expect(reached).not.toHaveBeenCalled()
  })

  it('toggles from a slider, and from the page itself', () => {
    const slider = document.createElement('input')
    slider.type = 'range'
    document.body.appendChild(slider)
    slider.dispatchEvent(key('keydown'))
    document.dispatchEvent(key('keydown'))
    slider.remove()

    expect(toggle).toHaveBeenCalledTimes(2)
  })

  it('leaves a text field its space, down and up', () => {
    const field = document.createElement('input')
    document.body.appendChild(field)
    const down = key('keydown')
    const up = key('keyup')
    field.dispatchEvent(down)
    field.dispatchEvent(up)
    field.remove()

    expect(toggle).not.toHaveBeenCalled()
    expect(down.defaultPrevented).toBe(false)
    expect(up.defaultPrevented).toBe(false)
    // and the page still hears both
    expect(reached).toHaveBeenCalledTimes(2)
  })

  it('keeps Shift+Space and Option+Space from a button without toggling', () => {
    const shifted = key('keydown', { shiftKey: true })
    const opted = key('keyup', { altKey: true })
    button.dispatchEvent(shifted)
    button.dispatchEvent(opted)

    expect(toggle).not.toHaveBeenCalled()
    expect(shifted.defaultPrevented).toBe(true)
    expect(opted.defaultPrevented).toBe(true)
    expect(reached).not.toHaveBeenCalled()
  })

  it('leaves the system its ⌘ Space and ⌃ Space', () => {
    const cmd = key('keydown', { metaKey: true })
    button.dispatchEvent(cmd)
    button.dispatchEvent(key('keydown', { ctrlKey: true }))

    expect(toggle).not.toHaveBeenCalled()
    expect(cmd.defaultPrevented).toBe(false)
  })

  it('stops listening when asked', () => {
    stop()
    button.dispatchEvent(key('keydown'))
    expect(toggle).not.toHaveBeenCalled()
    expect(reached).toHaveBeenCalled()
  })
})
