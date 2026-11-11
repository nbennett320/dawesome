import type { Mock } from 'vitest'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer, { reduceLoadedProject, reduceProjectView } from './playlistSlice'
import projectReducer from './projectSlice'
import windowReducer from './windowSlice'
import effectsReducer, {
  addEffect,
  dropEffectOnChannel,
  expandSlot,
  fetchPlugins,
  highlightSlot,
  moveEffect,
  openEffectWindow,
  removeEffect,
  scanPlugins,
  selectChannelActive,
  selectChannelCount,
  selectChannelSlots,
  setEffectBypassed,
  setRacks,
  showRack,
  showRackChannel,
} from './effectsSlice'
import { EffectSlot, Racks } from '../../types/effects'
import { LoadedProject, ProjectView } from '../../types/project'
import { View } from '../../types/ui'
import * as paneTree from '../paneTree'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const mockInvoke = invoke as Mock

const makeStore = () => configureStore({
  reducer: {
    playlist: playlistReducer,
    project: projectReducer,
    window: windowReducer,
    effects: effectsReducer,
  },
})

// the store's state is a subset of RootState, which is all the selectors use
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const state = (store: ReturnType<typeof makeStore>) => store.getState() as any

// and the same for thunks that read the state
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const dispatch = (store: ReturnType<typeof makeStore>) => store.dispatch as any

const slot = (id: number, change: Partial<EffectSlot> = {}): EffectSlot => ({
  id,
  pluginId: `vst3:${id}`,
  name: `Effect ${id}`,
  vendor: 'Acme',
  bypassed: false,
  missing: false,
  hasEditor: true,
  ...change,
})

const racks: Racks = [
  { track: 0, slots: [slot(1), slot(2, { bypassed: true })] },
  { track: null, slots: [slot(3, { missing: true })] },
]

const project = (effects?: Racks): LoadedProject => ({
  tempo: 120,
  timeSignature: { numerator: 4, denominator: 4 },
  metronomeEnabled: false,
  loopEnabled: false,
  snapEnabled: true,
  snapNote: 'EighthNote',
  clips: [],
  missing: [],
  mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
  sequencer: { rows: [], patterns: [], folders: [] },
  patternClips: [],
  ...(effects ? { effects } : {}),
})

const view = (effects?: Racks): ProjectView => {
  const { tempo, clips, patternClips, mixer, sequencer } = project()
  return { tempo, clips, patternClips, mixer, sequencer, ...(effects ? { effects } : {}) }
}

beforeEach(() => {
  vi.resetAllMocks()
  mockInvoke.mockResolvedValue(undefined)
})

describe('effectsSlice', () => {
  it('starts with no racks, showing the first track', () => {
    const store = makeStore()
    expect(state(store).effects.racks).toEqual([])
    expect(state(store).effects.rackChannel).toBe(0)
    expect(state(store).effects.expandedSlotId).toBeNull()
  })

  it('selects a channel’s chain, its count and whether any of it is on', () => {
    const store = makeStore()
    store.dispatch(setRacks(racks))

    expect(selectChannelSlots(0)(state(store)).map((s: EffectSlot) => s.id)).toEqual([1, 2])
    expect(selectChannelCount(0)(state(store))).toBe(2)
    expect(selectChannelCount('master')(state(store))).toBe(1)
    expect(selectChannelCount(4)(state(store))).toBe(0)
    expect(selectChannelActive(0)(state(store))).toBe(true)
    // only a missing one
    expect(selectChannelActive('master')(state(store))).toBe(false)
    expect(selectChannelActive(4)(state(store))).toBe(false)

    store.dispatch(setRacks([{ track: 0, slots: [slot(1, { bypassed: true })] }]))
    expect(selectChannelActive(0)(state(store))).toBe(false)
  })

  it('takes a loaded project’s racks, or none from an older engine', () => {
    const store = makeStore()
    store.dispatch(reduceLoadedProject(project(racks)))
    expect(state(store).effects.racks).toEqual(racks)

    store.dispatch(reduceLoadedProject(project()))
    expect(state(store).effects.racks).toEqual([])
  })

  it('follows undo and redo', () => {
    const store = makeStore()
    store.dispatch(setRacks(racks))

    store.dispatch(reduceProjectView({ project: view([{ track: 2, slots: [slot(5)] }]), selected: null }))
    expect(selectChannelCount(2)(state(store))).toBe(1)
    expect(selectChannelCount(0)(state(store))).toBe(0)

    store.dispatch(reduceProjectView({ project: view(), selected: null }))
    expect(state(store).effects.racks).toEqual([])
  })

  it('goes back to the stack when the expanded slot goes, or the channel changes', () => {
    const store = makeStore()
    store.dispatch(setRacks(racks))
    store.dispatch(expandSlot(1))
    expect(state(store).effects.expandedSlotId).toBe(1)

    // undone away
    store.dispatch(reduceProjectView({ project: view([{ track: 0, slots: [slot(2)] }]), selected: null }))
    expect(state(store).effects.expandedSlotId).toBeNull()

    store.dispatch(expandSlot(2))
    store.dispatch(showRackChannel('master'))
    expect(state(store).effects.rackChannel).toBe('master')
    expect(state(store).effects.expandedSlotId).toBeNull()

    // moved to another channel than the one shown
    store.dispatch(showRackChannel(0))
    store.dispatch(expandSlot(2))
    store.dispatch(setRacks([{ track: 1, slots: [slot(2)] }]))
    expect(state(store).effects.expandedSlotId).toBeNull()
  })

  it('only expands a slot with an interface to show', () => {
    const store = makeStore()
    store.dispatch(setRacks([{ track: 0, slots: [slot(1, { hasEditor: false }), slot(2, { missing: true })] }]))
    store.dispatch(expandSlot(1))
    expect(state(store).effects.expandedSlotId).toBeNull()
    store.dispatch(expandSlot(2))
    expect(state(store).effects.expandedSlotId).toBeNull()
  })

  it('drops a highlight whose slot is gone', () => {
    const store = makeStore()
    store.dispatch(setRacks(racks))
    store.dispatch(highlightSlot(2))
    store.dispatch(setRacks(racks))
    expect(state(store).effects.highlightSlotId).toBe(2)
    store.dispatch(setRacks([]))
    expect(state(store).effects.highlightSlotId).toBeNull()
  })
})

describe('commands', () => {
  it('reads the catalog, and rescans everything', async () => {
    const store = makeStore()
    const catalog = { plugins: [], failed: [], scanning: true, done: 1, total: 4 }
    mockInvoke.mockImplementation(async (cmd: string) => (cmd === 'get_plugins' ? catalog : undefined))

    await store.dispatch(fetchPlugins())
    expect(state(store).effects.catalog).toEqual(catalog)

    await store.dispatch(scanPlugins(true))
    expect(mockInvoke).toHaveBeenCalledWith('scan_plugins', { everything: true })
    expect(mockInvoke).toHaveBeenLastCalledWith('get_plugins', {})
  })

  it('adds an effect to a track, or (as a null track) to the master', async () => {
    const store = makeStore()
    mockInvoke.mockResolvedValue({ racks, slotId: 3 })

    expect(await store.dispatch(addEffect('master', 'vst3:a', null))).toBe(3)
    expect(mockInvoke).toHaveBeenCalledWith('add_effect', { track: null, pluginId: 'vst3:a', index: null })
    expect(state(store).effects.racks).toEqual(racks)

    await store.dispatch(addEffect(2, 'vst3:a', 1))
    expect(mockInvoke).toHaveBeenLastCalledWith('add_effect', { track: 2, pluginId: 'vst3:a', index: 1 })
  })

  it('shows why an effect couldn’t be added, and changes nothing', async () => {
    const store = makeStore()
    store.dispatch(setRacks(racks))
    mockInvoke.mockRejectedValue('the plugin crashed while loading')

    expect(await store.dispatch(addEffect(0, 'vst3:a', null))).toBeNull()
    expect(state(store).effects.racks).toEqual(racks)
    expect(state(store).project.notice).toEqual({
      kind: 'error',
      message: 'Couldn’t add the effect: the plugin crashed while loading',
    })
  })

  it('removes, moves and bypasses, taking the racks the engine hands back', async () => {
    const store = makeStore()
    const after: Racks = [{ track: 1, slots: [slot(1)] }]
    mockInvoke.mockResolvedValue(after)

    await store.dispatch(removeEffect(2))
    expect(mockInvoke).toHaveBeenLastCalledWith('remove_effect', { slotId: 2 })
    await store.dispatch(moveEffect(1, 'master', 0))
    expect(mockInvoke).toHaveBeenLastCalledWith('move_effect', { slotId: 1, track: null, index: 0 })
    await store.dispatch(setEffectBypassed(1, true))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_effect_bypassed', { slotId: 1, bypassed: true })
    expect(state(store).effects.racks).toEqual(after)
  })

  it('pops an expanded effect out, back to the stack', async () => {
    const store = makeStore()
    store.dispatch(setRacks(racks))
    store.dispatch(expandSlot(1))

    await dispatch(store)(openEffectWindow(1))
    expect(mockInvoke).toHaveBeenCalledWith('open_effect_window', { slotId: 1 })
    expect(state(store).effects.expandedSlotId).toBeNull()
  })

  it('opens the rack tab on a channel, or selects it where it’s open', () => {
    const store = makeStore()
    store.dispatch(showRack('master'))
    const root = () => state(store).window.windowPane.root

    expect(state(store).effects.rackChannel).toBe('master')
    expect(paneTree.paneWithView(root(), View.EffectsRack)?.activeView).toBe(View.EffectsRack)

    store.dispatch(showRack(1))
    // still the one tab
    const tabs = paneTree.leaves(root()).flatMap(p => p.tabs).filter(t => t.component === View.EffectsRack)
    expect(tabs).toHaveLength(1)
    expect(state(store).effects.rackChannel).toBe(1)
  })

  it('dropped on a channel: added at the end, the rack opens with it expanded', async () => {
    const store = makeStore()
    const after: Racks = [{ track: 1, slots: [slot(1), slot(7)] }]
    mockInvoke.mockResolvedValue({ racks: after, slotId: 7 })

    await dispatch(store)(dropEffectOnChannel(1, 'vst3:x'))
    expect(mockInvoke).toHaveBeenCalledWith('add_effect', { track: 1, pluginId: 'vst3:x', index: null })
    expect(state(store).effects.rackChannel).toBe(1)
    expect(state(store).effects.expandedSlotId).toBe(7)
    expect(state(store).effects.highlightSlotId).toBeNull()
  })

  it('dropped on a channel without an interface of its own: pointed out in the stack', async () => {
    const store = makeStore()
    mockInvoke.mockResolvedValue({ racks: [{ track: null, slots: [slot(8, { hasEditor: false })] }], slotId: 8 })

    await dispatch(store)(dropEffectOnChannel('master', 'vst3:x'))
    expect(state(store).effects.rackChannel).toBe('master')
    expect(state(store).effects.expandedSlotId).toBeNull()
    expect(state(store).effects.highlightSlotId).toBe(8)
  })

  it('dropped on a channel and failed: the rack isn’t opened', async () => {
    const store = makeStore()
    mockInvoke.mockRejectedValue('no')

    expect(await dispatch(store)(dropEffectOnChannel(2, 'vst3:x'))).toBeNull()
    expect(paneTree.paneWithView(state(store).window.windowPane.root, View.EffectsRack)).toBeUndefined()
    expect(state(store).project.notice.kind).toBe('error')
  })
})
