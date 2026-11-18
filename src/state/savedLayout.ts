// the window's panes and tabs, as saved in a project file (its `layout`),
// and back again
//
// - views are saved by name (viewNames), not by their number in the enum
// - a plugin's tab can't be saved by its slot, which is new each time the
//   project loads. an effect's is saved by its place (its channel's track,
//   null for the master, and its place in the chain), a synth's by its
//   row's name, which is unique among the synth rows
// - pane ids aren't saved: the focused pane is the way to it from the top
//   (0 is left or top, 1 right or bottom), and panes get new ids on load
//
// on load, a tab that can't be opened (a view this app doesn't know, a
// plugin that's gone, missing, or has no interface) is left out, as if it
// had been closed: its pane's neighbouring tab is selected, and a pane
// left with none closes
import * as paneTree from './paneTree'
import { findSlot, hasTab, Racks, toTrack } from '../types/effects'
import { SynthInstrument } from '../types/sequencer'
import { keyOf, PaneTab, SplitDirection, TabKey, View, viewNames, WindowNode } from '../types/ui'

export type SavedTab = {
  view: string
  // a plugin tab's effect: its channel (null is the master) and place
  effect?: { track: number | null, index: number }
  // a plugin tab's synth: its row's name
  synth?: string
}

export type SavedPane = {
  tabs: SavedTab[]
  // the selected tab, an index into `tabs`
  active: number
}

export type SavedSplit = {
  split: SplitDirection
  // the left (or top) side's share, in percent (left out: half)
  ratio?: number
  left: SavedNode
  right: SavedNode
}

export type SavedNode = SavedPane | SavedSplit

export type SavedLayout = {
  root: SavedNode
  // the way to the focused pane
  focused: number[]
}

type Synths = Array<Pick<SynthInstrument, 'slot' | 'name' | 'hasEditor' | 'missing'>>

// what to save of a tab, or null for a plugin tab whose slot is gone
const saveTab = (tab: PaneTab, racks: Racks, synths: Synths): SavedTab | null => {
  const view = viewNames[tab.component]
  if(tab.component !== View.Plugin) return { view }

  const slotId = tab.slotId as number
  const found = findSlot(racks, slotId)
  if(found) return { view, effect: { track: toTrack(found.channel), index: found.index } }

  const synth = synths.find(s => s.slot === slotId)
  return synth ? { view, synth: synth.name } : null
}

const saveNode = (node: WindowNode, racks: Racks, synths: Synths): SavedNode => {
  if(paneTree.isLeaf(node)) {
    const kept = node.tabs
      .map(tab => ({ tab, saved: saveTab(tab, racks, synths) }))
      .filter((t): t is { tab: PaneTab, saved: SavedTab } => t.saved !== null)
    const active = kept.findIndex(({ tab }) => keyOf(tab) === node.activeTab)

    return { tabs: kept.map(t => t.saved), active: Math.max(active, 0) }
  }

  return {
    split: node.direction ?? 'vertical',
    ...(node.ratio === undefined ? {} : { ratio: node.ratio }),
    left: saveNode(node.left as WindowNode, racks, synths),
    right: saveNode(node.right as WindowNode, racks, synths),
  }
}

// the way from `node` to the pane with this id, or null if it isn't there
const pathTo = (node: WindowNode, id: string): number[] | null => {
  if(node.id === id) return []
  if(paneTree.isLeaf(node)) return null

  const left = pathTo(node.left as WindowNode, id)
  if(left) return [0, ...left]
  const right = pathTo(node.right as WindowNode, id)
  return right ? [1, ...right] : null
}

// the layout to save with the project
export const toSaved = (
  root: WindowNode,
  focusedPaneId: string,
  racks: Racks,
  synths: Synths,
): SavedLayout => ({
  root: saveNode(root, racks, synths),
  focused: pathTo(root, focusedPaneId) ?? [],
})

// a saved layout that isn't one
class Malformed extends Error {}

const DEEPEST = 32

const viewsByName = new Map(
  (Object.entries(viewNames) as Array<[string, string]>).map(([view, name]) => [name, Number(view) as View]))

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value)

// the tab a saved one opens, or null when it can't be opened
const openTab = (saved: unknown, racks: Racks, synths: Synths): PaneTab | null => {
  if(!isObject(saved) || typeof saved.view !== 'string') throw new Malformed()

  const view = viewsByName.get(saved.view)
  if(view === undefined) return null
  if(view !== View.Plugin) return { component: view }

  const { effect, synth } = saved
  if(isObject(effect)) {
    const track = effect.track === null ? null : Number(effect.track)
    const index = Number(effect.index)
    const slot = racks.find(rack => rack.track === track)?.slots[index]
    return slot && hasTab(slot) ? { component: View.Plugin, slotId: slot.id } : null
  }
  if(typeof synth === 'string') {
    const found = synths.find(s => s.name === synth)
    return found && hasTab(found) ? { component: View.Plugin, slotId: found.slot } : null
  }
  return null
}

type Restoring = {
  racks: Racks
  synths: Synths
  newId: () => string
  // keys of tabs already placed: a tab is open in one pane at most
  placed: Set<TabKey>
  // stand-ins for tabs that can't be opened, closed once the tree is built
  unopened: Set<TabKey>
}

// a stand-in for a tab that can't be opened, so closing it later selects
// its neighbour and closes its pane, as closing a tab by hand does
const standIn = (r: Restoring): PaneTab => {
  const tab = { component: View.Plugin, slotId: -1 - r.unopened.size }
  r.unopened.add(keyOf(tab))
  return tab
}

const restoreNode = (saved: unknown, r: Restoring, depth: number): WindowNode => {
  if(!isObject(saved) || depth > DEEPEST) throw new Malformed()

  if(Array.isArray(saved.tabs)) {
    const tabs = saved.tabs.map(s => {
      const tab = openTab(s, r.racks, r.synths)
      if(!tab || r.placed.has(keyOf(tab))) return standIn(r)
      r.placed.add(keyOf(tab))
      return tab
    })
    // a pane saved empty closes too
    if(!tabs.length) tabs.push(standIn(r))

    const active = typeof saved.active === 'number' && tabs[saved.active] ? saved.active : 0
    return { id: r.newId(), tabs, activeTab: keyOf(tabs[active]) }
  }

  const { split, ratio } = saved
  if(split !== 'vertical' && split !== 'horizontal') throw new Malformed()
  if(ratio !== undefined && (typeof ratio !== 'number' || !(ratio >= 0 && ratio <= 100))) throw new Malformed()

  return {
    id: r.newId(),
    tabs: [],
    direction: split,
    ...(ratio === undefined ? {} : { ratio }),
    left: restoreNode(saved.left, r, depth + 1),
    right: restoreNode(saved.right, r, depth + 1),
  }
}

// the pane a path leads to, if it does
const follow = (node: WindowNode, path: unknown): WindowNode | undefined => {
  if(!Array.isArray(path)) return undefined
  let at: WindowNode | undefined = node
  for(const step of path) {
    if(!at || paneTree.isLeaf(at)) return undefined
    at = step === 0 ? at.left : step === 1 ? at.right : undefined
  }
  return at && paneTree.isLeaf(at) ? at : undefined
}

// the window layout a project saved, with the tabs that can't be opened
// left out. null when there's no layout, or it's malformed: the window
// keeps the one it has
// - racks, synths: the project's, as just loaded
// - newId: an id for each pane
export const fromSaved = (
  saved: unknown,
  racks: Racks,
  synths: Synths,
  newId: () => string,
): { root: WindowNode, focusedPaneId: string } | null => {
  if(!isObject(saved)) return null

  const r: Restoring = { racks, synths, newId, placed: new Set(), unopened: new Set() }
  let built: WindowNode
  try {
    built = restoreNode(saved.root, r, 0)
  } catch(e) {
    if(e instanceof Malformed) return null
    throw e
  }

  const focused = follow(built, saved.focused)?.id
  const root = paneTree.closeTabsUnless(built, tab => !r.unopened.has(keyOf(tab)))
  const focusedPaneId = focused && paneTree.findPane(root, focused)
    ? focused
    : paneTree.leaves(root)[0].id

  return { root, focusedPaneId }
}
