import { fromSaved, SavedLayout, toSaved } from './savedLayout'
import * as paneTree from './paneTree'
import { EffectSlot, Racks } from '../types/effects'
import { SynthInstrument } from '../types/sequencer'
import { keyOf, PaneTab, tabKey, View, WindowNode } from '../types/ui'

const slot = (id: number, more: Partial<EffectSlot> = {}): EffectSlot => ({
  id, pluginId: `plugin-${id}`, name: `Effect ${id}`, vendor: '', bypassed: false,
  missing: false, hasEditor: true, native: false, ...more,
})

const synth = (slotId: number, name: string, more: Partial<SynthInstrument> = {}): SynthInstrument => ({
  kind: 'synth', slot: slotId, pluginId: name, name, vendor: '', missing: false, hasEditor: true, ...more,
})

const tab = (component: View, slotId?: number): PaneTab =>
  (slotId === undefined ? { component } : { component, slotId })

const ids = () => {
  let n = 0
  return () => `p${n++}`
}

// a round trip through the file, as JSON
const throughFile = (layout: SavedLayout): unknown => JSON.parse(JSON.stringify(layout))

// playlist and an effect's tab on the left; on the right, a synth's tab and
// the mixer (selected, and focused)
const layoutTree = (): WindowNode => ({
  id: 's',
  tabs: [],
  direction: 'vertical',
  ratio: 62.5,
  left: { id: 'a', tabs: [tab(View.Playlist), tab(View.Plugin, 11)], activeTab: tabKey(View.Plugin, 11) },
  right: { id: 'b', tabs: [tab(View.Plugin, 20), tab(View.Mixer)], activeTab: tabKey(View.Mixer) },
})

const racks: Racks = [{ track: 3, slots: [slot(10), slot(11)] }]
const synths = [synth(20, 'DecentSampler')]

// the same project loaded again: every slot has a new id
const reloadedRacks: Racks = [{ track: 3, slots: [slot(110), slot(111)] }]
const reloadedSynths = [synth(120, 'DecentSampler')]

const saved = () => throughFile(toSaved(layoutTree(), 'b', racks, synths))

describe('savedLayout', () => {
  test('views are saved by name and plugins by where they are', () => {
    expect(toSaved(layoutTree(), 'b', racks, synths)).toEqual({
      root: {
        split: 'vertical',
        ratio: 62.5,
        left: { tabs: [{ view: 'playlist' }, { view: 'plugin', effect: { track: 3, index: 1 } }], active: 1 },
        right: { tabs: [{ view: 'plugin', synth: 'DecentSampler' }, { view: 'mixer' }], active: 1 },
      },
      focused: [1],
    })
  })

  test('a layout comes back as it was saved, with the reloaded slots', () => {
    const restored = fromSaved(saved(), reloadedRacks, reloadedSynths, ids())

    expect(restored).toEqual({
      root: {
        id: 'p0',
        tabs: [],
        direction: 'vertical',
        ratio: 62.5,
        left: { id: 'p1', tabs: [tab(View.Playlist), tab(View.Plugin, 111)], activeTab: tabKey(View.Plugin, 111) },
        right: { id: 'p2', tabs: [tab(View.Plugin, 120), tab(View.Mixer)], activeTab: tabKey(View.Mixer) },
      },
      focusedPaneId: 'p2',
    })
  })

  test("a missing plugin's tab is left out and its neighbour selected", () => {
    const missing: Racks = [{ track: 3, slots: [slot(110), slot(111, { missing: true })] }]
    const restored = fromSaved(saved(), missing, reloadedSynths, ids())

    expect(restored?.root.left).toEqual({ id: 'p1', tabs: [tab(View.Playlist)], activeTab: tabKey(View.Playlist) })
  })

  test("a plugin without an interface, or one that's gone, is left out too", () => {
    const noEditor = [synth(120, 'DecentSampler', { hasEditor: false })]
    const gone: Racks = [{ track: 3, slots: [slot(110)] }]
    const restored = fromSaved(saved(), gone, noEditor, ids())

    expect(restored?.root.left?.tabs).toEqual([tab(View.Playlist)])
    expect(restored?.root.right?.tabs).toEqual([tab(View.Mixer)])
  })

  test("a pane whose tabs can't be opened closes, and its split with it", () => {
    const layout = {
      root: {
        split: 'horizontal',
        left: { tabs: [{ view: 'playlist' }], active: 0 },
        right: { tabs: [{ view: 'plugin', synth: 'Gone' }, { view: 'no-such-view' }], active: 1 },
      },
      focused: [1],
    }
    const restored = fromSaved(layout, [], [], ids())

    expect(restored).toEqual({
      root: { id: 'p1', tabs: [tab(View.Playlist)], activeTab: tabKey(View.Playlist) },
      focusedPaneId: 'p1',
    })
  })

  test("with no tab that can be opened, it's an empty window", () => {
    const layout = { root: { tabs: [{ view: 'plugin', synth: 'Gone' }], active: 0 }, focused: [] }
    const restored = fromSaved(layout, [], [], ids())

    expect(restored?.root.tabs).toEqual([])
    expect(restored?.focusedPaneId).toBe(restored?.root.id)
  })

  test('a pane saved empty closes', () => {
    const layout = {
      root: { split: 'vertical', left: { tabs: [], active: 0 }, right: { tabs: [{ view: 'mixer' }], active: 0 } },
      focused: [0],
    }
    const restored = fromSaved(layout, [], [], ids())

    expect(restored?.root).toEqual({ id: 'p2', tabs: [tab(View.Mixer)], activeTab: tabKey(View.Mixer) })
    expect(restored?.focusedPaneId).toBe('p2')
  })

  test('a tab saved twice is opened once', () => {
    const layout = {
      root: {
        split: 'vertical',
        left: { tabs: [{ view: 'mixer' }, { view: 'playlist' }], active: 0 },
        right: { tabs: [{ view: 'mixer' }], active: 0 },
      },
      focused: [],
    }
    const restored = fromSaved(layout, [], [], ids())
    const tabs = paneTree.leaves(restored?.root as WindowNode).flatMap(leaf => leaf.tabs.map(keyOf))

    expect(tabs).toEqual([tabKey(View.Mixer), tabKey(View.Playlist)])
  })

  test('an active index that makes no sense selects the first tab', () => {
    const layout = { root: { tabs: [{ view: 'mixer' }, { view: 'playlist' }], active: 9 }, focused: [] }

    expect(fromSaved(layout, [], [], ids())?.root.activeTab).toBe(tabKey(View.Mixer))
  })

  test('a focused path that leads nowhere focuses the first pane', () => {
    const layout = { root: { tabs: [{ view: 'mixer' }], active: 0 }, focused: [1, 0] }

    expect(fromSaved(layout, [], [], ids())?.focusedPaneId).toBe('p0')
  })

  test('no layout, or a malformed one, is null', () => {
    const pane = { tabs: [{ view: 'mixer' }], active: 0 }
    const bad = [
      undefined,
      null,
      'layout',
      {},
      { root: 'pane' },
      { root: { tabs: ['mixer'], active: 0 } },
      { root: { split: 'diagonal', left: pane, right: pane } },
      { root: { split: 'vertical', ratio: 140, left: pane, right: pane } },
      { root: { split: 'vertical', ratio: '50', left: pane, right: pane } },
      { root: { split: 'vertical', left: pane } },
    ]

    bad.forEach(layout => expect(fromSaved(layout, [], [], ids())).toBeNull())
  })

  test('a layout nested absurdly deep is null', () => {
    let root: unknown = { tabs: [{ view: 'mixer' }], active: 0 }
    for(let i = 0; i < 100; i++) root = { split: 'vertical', left: root, right: { tabs: [], active: 0 } }

    expect(fromSaved({ root, focused: [] }, [], [], ids())).toBeNull()
  })

  test("a plugin tab whose slot is gone isn't saved", () => {
    const tree: WindowNode = {
      id: 'a',
      tabs: [tab(View.Mixer), tab(View.Plugin, 99)],
      activeTab: tabKey(View.Plugin, 99),
    }

    expect(toSaved(tree, 'a', racks, synths)).toEqual({
      root: { tabs: [{ view: 'mixer' }], active: 0 },
      focused: [],
    })
  })

  test("the master's effects are saved as track null", () => {
    const master: Racks = [{ track: null, slots: [slot(5)] }]
    const tree: WindowNode = { id: 'a', tabs: [tab(View.Plugin, 5)], activeTab: tabKey(View.Plugin, 5) }
    const layout = throughFile(toSaved(tree, 'a', master, []))

    expect(layout).toEqual({ root: { tabs: [{ view: 'plugin', effect: { track: null, index: 0 } }], active: 0 }, focused: [] })
    expect(fromSaved(layout, [{ track: null, slots: [slot(50)] }], [], ids())?.root.tabs)
      .toEqual([tab(View.Plugin, 50)])
  })
})
