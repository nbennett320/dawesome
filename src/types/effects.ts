// effect plugins (VST3): what the engine found, and each channel's chain of
// them. mirrors engine/src/effect_commands.rs (see
// context/vst-project/04-api.md, the contract both sides keep to)
import { trackLabel } from './mixer'

// one plugin the scan found (engine/src/plugin/mod.rs PluginInfo)
export interface PluginInfo {
  // "vst3:<32 hex>", what projects save
  id: string
  // its bundle
  path: string
  name: string
  vendor: string
  version: string
  kind: PluginKind
  // Dynamics, EQ, Reverb, Delay, … or Other
  category: string
  subcategories: string[]
}

export type PluginKind = 'effect' | 'instrument'

// a bundle that crashed, hung or couldn't be read while scanning
export interface FailedPlugin {
  path: string
  reason: string
}

export interface PluginCatalog {
  plugins: PluginInfo[]
  failed: FailedPlugin[]
  // the folders the engine looked in
  searched: string[]
  scanning: boolean
  // bundles scanned so far and in all, while scanning
  done: number
  total: number
}

// one effect in a channel's chain
export interface EffectSlot {
  // unique in the project, never reused
  id: number
  pluginId: string
  name: string
  vendor: string
  bypassed: boolean
  // it couldn't be loaded (not installed, or it failed): sound passes
  // straight through and its saved settings are kept
  missing: boolean
  // has an interface of its own to show
  hasEditor: boolean
}

// a channel's chain, in the order the sound goes through it
export interface ChannelRack {
  // null is the master
  track: number | null
  slots: EffectSlot[]
}

// every channel that has effects (channels without any are left out)
export type Racks = ChannelRack[]

// CSS pixels from the webview's top-left
export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

// a plugin's own interface, as docked in its tab: the size it ended up
export interface EditorInfo {
  width: number
  height: number
  resizable: boolean
}

// what add_effect returns: the racks, and the new slot's id
export interface AddedEffect {
  racks: Racks
  slotId: number
}

// a channel, as the app's own state keeps it: a track's number, or the
// master. the engine is sent `track`, which is null for the master
export type ChannelId = number | 'master'

export const toTrack = (channel: ChannelId): number | null =>
  (channel === 'master' ? null : channel)

export const toChannel = (track: number | null): ChannelId =>
  (track === null ? 'master' : track)

// a channel as text, e.g. for a <select>'s value or a data attribute
export const channelKey = (channel: ChannelId): string => String(channel)

export const parseChannelKey = (key: string | null | undefined): ChannelId | null => {
  if(key === 'master') return 'master'
  if(!key || !/^\d+$/.test(key)) return null
  return Number(key)
}

// a channel's name, as the mixer shows it: the track's name (the mixer's
// `names`), "Track n" without one, or "Master"
export const channelLabel = (names: Record<number, string>, channel: ChannelId): string =>
  (channel === 'master' ? 'Master' : names[channel] ?? trackLabel(channel))

const NO_SLOTS: EffectSlot[] = []

// a channel's chain (empty when it has none). the same empty list every
// time, so selectors don't re-render for nothing
export const slotsOf = (racks: Racks, channel: ChannelId): EffectSlot[] =>
  racks.find(rack => rack.track === toTrack(channel))?.slots ?? NO_SLOTS

// where a slot is: its channel and its place in that chain
export const findSlot = (
  racks: Racks,
  slotId: number,
): { channel: ChannelId, index: number, slot: EffectSlot } | null => {
  for(const rack of racks) {
    const index = rack.slots.findIndex(slot => slot.id === slotId)
    if(index >= 0) return { channel: toChannel(rack.track), index, slot: rack.slots[index] }
  }
  return null
}

// whether an effect is doing anything: switched on, and loaded
export const isActive = (slot: EffectSlot): boolean => !slot.bypassed && !slot.missing

// the mixer's FX jack's light: lit when at least one effect is on
export const fxLit = (slots: EffectSlot[]): boolean => slots.some(isActive)

// whether an effect (or a synth) has an interface to show in a tab (or a
// window)
export const hasTab = (slot: Pick<EffectSlot, 'hasEditor' | 'missing'>): boolean => slot.hasEditor && !slot.missing

// every slot that can have a tab, in the project: the racks' effects, and
// the synths (whose ids are slot ids too)
export const tabSlotIds = (racks: Racks, synths: Array<Pick<EffectSlot, 'id' | 'hasEditor' | 'missing'>> = []): number[] =>
  [...racks.flatMap(rack => rack.slots), ...synths].filter(hasTab).map(slot => slot.id)

// a plugin's tab's name: "Name" when it's the project's only one of that
// plugin, "Name - Channel" when there are more but one to a channel, and
// "Name - Channel (n)" when its channel has more than one, n being its place
// in the chain (from 1). null when the slot's gone
export const pluginTabLabel = (
  racks: Racks,
  names: Record<number, string>,
  slotId: number,
): string | null => {
  const found = findSlot(racks, slotId)
  if(!found) return null

  const { channel, index, slot } = found
  const same = (other: EffectSlot) => other.pluginId === slot.pluginId
  if(racks.flatMap(rack => rack.slots).filter(same).length === 1) return slot.name

  const label = `${slot.name} - ${channelLabel(names, channel)}`
  return slotsOf(racks, channel).filter(same).length > 1 ? `${label} (${index + 1})` : label
}

// moving a slot within its own chain: dropped in `gap` (0 is above the first
// row, slots.length below the last), it goes to the index move_effect wants,
// which is its place after the move. null when it wouldn't move
//
// e.g. in [a, b, c], dragging a below b is gap 2, which is index 1: once a is
// taken out, b is at 0 and a goes after it
export const reorderIndex = (slots: EffectSlot[], slotId: number, gap: number): number | null => {
  const from = slots.findIndex(slot => slot.id === slotId)
  const clamped = Math.max(0, Math.min(gap, slots.length))
  if(from < 0) return clamped

  const to = clamped > from ? clamped - 1 : clamped
  return to === from ? null : to
}

// moving a slot to the end of a chain (dropped on a mixer strip): the index
// move_effect wants. null when it's last in that chain already
export const appendIndex = (slots: EffectSlot[], slotId: number): number | null => {
  const from = slots.findIndex(slot => slot.id === slotId)
  if(from < 0) return slots.length
  return from === slots.length - 1 ? null : slots.length - 1
}

// the browser's folders: plugins of a kind, by category, each sorted by name
export interface PluginGroup {
  category: string
  plugins: PluginInfo[]
}

const byName = (a: string, b: string) =>
  a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' })

// Other is where plugins without a known category go, so it comes last
export const OTHER_CATEGORY = 'Other'

// whether a plugin matches what's typed in the browser's search: every word
// is in its name, vendor or category
export const matchesSearch = (plugin: PluginInfo, query: string): boolean => {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean)
  const text = `${plugin.name} ${plugin.vendor} ${plugin.category}`.toLowerCase()
  return words.every(word => text.includes(word))
}

// a kind's plugins in folders by category, alphabetical (with Other last),
// keeping those that match the search
export const groupPlugins = (plugins: PluginInfo[], kind: PluginKind, query = ''): PluginGroup[] => {
  const groups = new Map<string, PluginInfo[]>()

  plugins
    .filter(plugin => plugin.kind === kind && matchesSearch(plugin, query))
    .forEach(plugin => {
      const category = plugin.category || OTHER_CATEGORY
      groups.set(category, [...(groups.get(category) ?? []), plugin])
    })

  return Array.from(groups, ([category, list]) => ({
    category,
    plugins: [...list].sort((a, b) => byName(a.name, b.name) || byName(a.vendor, b.vendor)),
  })).sort((a, b) => {
    if(a.category === OTHER_CATEGORY) return 1
    if(b.category === OTHER_CATEGORY) return -1
    return byName(a.category, b.category)
  })
}
