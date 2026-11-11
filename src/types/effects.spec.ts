import {
  appendIndex, channelKey, dockable, EffectSlot, findSlot, fxLabel, fxLit, groupPlugins, matchesSearch,
  parseChannelKey, PluginInfo, Racks, reorderIndex, slotsOf, toChannel, toTrack,
} from './effects'
import { dragItemKey, isEffectItem, isPlaceable } from '../state/sampleDrag'

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

const plugin = (name: string, category: string, change: Partial<PluginInfo> = {}): PluginInfo => ({
  id: `vst3:${name}`,
  path: `/Library/Audio/Plug-Ins/VST3/${name}.vst3`,
  name,
  vendor: 'Acme',
  version: '1.0',
  kind: 'effect',
  category,
  subcategories: [category],
  ...change,
})

describe('channels', () => {
  it('sends the master as a null track', () => {
    expect(toTrack('master')).toBeNull()
    expect(toTrack(3)).toBe(3)
    expect(toChannel(null)).toBe('master')
    expect(toChannel(0)).toBe(0)
  })

  it('reads back its key', () => {
    expect(parseChannelKey(channelKey('master'))).toBe('master')
    expect(parseChannelKey(channelKey(12))).toBe(12)
    expect(parseChannelKey('nope')).toBeNull()
    expect(parseChannelKey(null)).toBeNull()
  })

  it('finds a channel’s chain and a slot', () => {
    const racks: Racks = [{ track: 1, slots: [slot(1), slot(2)] }, { track: null, slots: [slot(3)] }]
    expect(slotsOf(racks, 1).map(s => s.id)).toEqual([1, 2])
    expect(slotsOf(racks, 'master').map(s => s.id)).toEqual([3])
    expect(slotsOf(racks, 0)).toEqual([])
    // the same empty list each time
    expect(slotsOf(racks, 0)).toBe(slotsOf([], 5))
    expect(findSlot(racks, 2)).toEqual({ channel: 1, index: 1, slot: racks[0].slots[1] })
    expect(findSlot(racks, 3)?.channel).toBe('master')
    expect(findSlot(racks, 9)).toBeNull()
  })
})

describe('the FX button', () => {
  it('shows how many effects there are', () => {
    expect(fxLabel(0)).toBe('FX')
    expect(fxLabel(3)).toBe('FX 3')
  })

  it('is lit while any effect is on and loaded', () => {
    expect(fxLit([])).toBe(false)
    expect(fxLit([slot(1, { bypassed: true }), slot(2, { missing: true })])).toBe(false)
    expect(fxLit([slot(1, { bypassed: true }), slot(2)])).toBe(true)
  })
})

describe('reorderIndex', () => {
  const chain = [slot(1), slot(2), slot(3)]

  it('moving up: the gap is the index', () => {
    expect(reorderIndex(chain, 3, 0)).toBe(0)
    expect(reorderIndex(chain, 3, 1)).toBe(1)
  })

  it('moving down: one less, since it leaves a gap above', () => {
    // 1 dropped between 2 and 3 ends up second
    expect(reorderIndex(chain, 1, 2)).toBe(1)
    // and below the last, last
    expect(reorderIndex(chain, 1, 3)).toBe(2)
    expect(reorderIndex(chain, 2, 3)).toBe(2)
  })

  it('dropped on either side of itself, it doesn’t move', () => {
    expect(reorderIndex(chain, 2, 1)).toBeNull()
    expect(reorderIndex(chain, 2, 2)).toBeNull()
  })

  it('from another chain: the gap, kept inside the chain', () => {
    expect(reorderIndex(chain, 9, 1)).toBe(1)
    expect(reorderIndex(chain, 9, 7)).toBe(3)
  })
})

describe('appendIndex', () => {
  const chain = [slot(1), slot(2), slot(3)]

  it('from another channel: after the last', () => {
    expect(appendIndex(chain, 9)).toBe(3)
    expect(appendIndex([], 9)).toBe(0)
  })

  it('within its own channel: last, or nothing when it’s last already', () => {
    expect(appendIndex(chain, 1)).toBe(2)
    expect(appendIndex(chain, 3)).toBeNull()
  })
})

describe('dockable', () => {
  it('leaves out effects without an interface, and missing ones', () => {
    const chain = [slot(1), slot(2, { hasEditor: false }), slot(3, { missing: true }), slot(4, { bypassed: true })]
    expect(dockable(chain).map(s => s.id)).toEqual([1, 4])
  })
})

describe('the browser’s folders', () => {
  const plugins = [
    plugin('Verb', 'Reverb', { vendor: 'Tidewater' }),
    plugin('Comp', 'Dynamics'),
    plugin('Thing', 'Other'),
    plugin('Analyzer', 'Analyzer'),
    plugin('Gate', 'Dynamics', { vendor: 'Tidewater' }),
    plugin('Piano', 'Instrument', { kind: 'instrument' }),
    plugin('Blank', ''),
  ]

  it('groups a kind by category, alphabetically, with Other last', () => {
    const groups = groupPlugins(plugins, 'effect')
    expect(groups.map(g => g.category)).toEqual(['Analyzer', 'Dynamics', 'Reverb', 'Other'])
    expect(groups[1].plugins.map(p => p.name)).toEqual(['Comp', 'Gate'])
    // no category is Other
    expect(groups[3].plugins.map(p => p.name)).toEqual(['Blank', 'Thing'])
    expect(groupPlugins(plugins, 'instrument').map(g => g.category)).toEqual(['Instrument'])
  })

  it('searches names, vendors and categories', () => {
    expect(groupPlugins(plugins, 'effect', 'tide').flatMap(g => g.plugins.map(p => p.name))).toEqual(['Gate', 'Verb'])
    expect(groupPlugins(plugins, 'effect', 'dynamics').flatMap(g => g.plugins.map(p => p.name))).toEqual(['Comp', 'Gate'])
    expect(groupPlugins(plugins, 'effect', 'GATE tidewater')).toHaveLength(1)
    expect(groupPlugins(plugins, 'effect', 'nothing like it')).toEqual([])
    expect(matchesSearch(plugins[0], '  ')).toBe(true)
  })
})

describe('dragging effects', () => {
  it('can’t place them in the playlist', () => {
    expect(isPlaceable({ kind: 'effect', pluginId: 'vst3:a', label: 'A' })).toBe(false)
    expect(isPlaceable({ kind: 'slot', slotId: 1, label: 'A' })).toBe(false)
    expect(isPlaceable({ kind: 'sample', path: '/a.wav' })).toBe(true)
    expect(isEffectItem({ kind: 'slot', slotId: 1, label: 'A' })).toBe(true)
    expect(isEffectItem({ kind: 'pattern', patternId: 1, label: 'A' })).toBe(false)
    expect(dragItemKey({ kind: 'effect', pluginId: 'vst3:a', label: 'A' })).toBe('effect:vst3:a')
    expect(dragItemKey({ kind: 'slot', slotId: 4, label: 'A' })).toBe('slot:4')
  })
})
