// the window layout: a tree of panes, like vs code's editor groups
//
// - a leaf pane has tabs, and shows the selected one
// - a split pane shows its two children side by side or stacked
//
// every tab is open in at most one pane, and a tab is told apart by its key
// (see tabKey): the view, so some views (like the playlist), which share one
// renderer, can't be on screen twice. a plugin's tab adds its slot, so any
// number of plugins can have tabs
//
// these functions never mutate the tree they're given, they return a new one
import { keyOf, PaneTab, SplitDirection, TabKey, WindowNode } from '../types/ui'

export const isLeaf = (node: WindowNode): boolean => !node.left || !node.right

// every leaf pane, left to right (or top to bottom)
export const leaves = (node: WindowNode): WindowNode[] => (
  isLeaf(node)
    ? [node]
    : [...leaves(node.left as WindowNode), ...leaves(node.right as WindowNode)]
)

export const findPane = (node: WindowNode, id: string): WindowNode | undefined =>
  leaves(node).find(leaf => leaf.id === id)

// the leaf pane a tab is open in
export const paneWithTab = (node: WindowNode, key: TabKey): WindowNode | undefined =>
  leaves(node).find(leaf => leaf.tabs.some(t => keyOf(t) === key))

// a pane's selected tab
export const activeTabOf = (pane: WindowNode): PaneTab | undefined =>
  pane.tabs.find(t => keyOf(t) === pane.activeTab)

// replace the node with this id by what `fn` returns. returning null removes
// the node, and its parent split collapses into the other child
const update = (
  node: WindowNode,
  id: string,
  fn: (node: WindowNode) => WindowNode | null,
): WindowNode | null => {
  if(node.id === id) return fn(node)
  if(isLeaf(node)) return node

  const left = update(node.left as WindowNode, id, fn)
  const right = update(node.right as WindowNode, id, fn)

  if(!left) return right
  if(!right) return left
  if(left === node.left && right === node.right) return node

  return { ...node, left, right }
}

// the root always exists, even with every tab closed
const orEmptyRoot = (root: WindowNode, updated: WindowNode | null): WindowNode =>
  updated ?? { id: root.id, tabs: [] }

export const selectTab = (root: WindowNode, paneId: string, key: TabKey): WindowNode =>
  orEmptyRoot(root, update(root, paneId, pane => ({ ...pane, activeTab: key })))

// close a tab. a pane left with no tabs is removed
export const closeTab = (root: WindowNode, key: TabKey): WindowNode => {
  const pane = paneWithTab(root, key)
  if(!pane) return root

  return orEmptyRoot(root, update(root, pane.id, leaf => {
    const index = leaf.tabs.findIndex(t => keyOf(t) === key)
    const tabs = leaf.tabs.filter(t => keyOf(t) !== key)

    if(!tabs.length) return null

    // closing the selected tab selects its neighbour, like vs code
    const activeTab = leaf.activeTab === key
      ? keyOf(tabs[Math.min(index, tabs.length - 1)])
      : leaf.activeTab

    return { ...leaf, tabs, activeTab }
  }))
}

// close every tab that `keep` turns down
export const closeTabsUnless = (root: WindowNode, keep: (tab: PaneTab) => boolean): WindowNode =>
  leaves(root)
    .flatMap(leaf => leaf.tabs)
    .filter(tab => !keep(tab))
    .reduce((tree, tab) => closeTab(tree, keyOf(tab)), root)

// add a tab to a pane and select it. `index` defaults to the end
export const addTab = (
  root: WindowNode,
  paneId: string,
  tab: PaneTab,
  index?: number,
): WindowNode => orEmptyRoot(root, update(root, paneId, leaf => {
  const tabs = [...leaf.tabs]
  tabs.splice(index ?? tabs.length, 0, tab)

  return { ...leaf, tabs, activeTab: keyOf(tab) }
}))

// move a tab to a pane (the same pane reorders it) and select it
// - index: where among the pane's current tabs to insert it, defaults to the end
export const moveTab = (
  root: WindowNode,
  key: TabKey,
  toPaneId: string,
  index?: number,
): WindowNode => {
  const from = paneWithTab(root, key)
  const tab = from?.tabs.find(t => keyOf(t) === key)
  if(!from || !tab || !findPane(root, toPaneId)) return root

  // closing the tab first removes its pane if it was the only tab, which
  // would be the target pane when moving a tab within itself
  if(from.id === toPaneId && from.tabs.length === 1) return root

  // reordering within a pane: the index counts the tab itself, which
  // is removed before inserting it again
  const fromIndex = from.tabs.indexOf(tab)
  const insertAt = from.id === toPaneId && index !== undefined && fromIndex < index
    ? index - 1
    : index

  return addTab(closeTab(root, key), toPaneId, tab, insertAt)
}

// move a tab into a new pane beside `paneId`
// - direction: 'vertical' puts the panes side by side, 'horizontal' stacks them
// - newPaneId, splitId: ids for the new pane and the split holding both
// - before: put the new pane left of (or above) `paneId` instead of after it
export const splitPane = (
  root: WindowNode,
  paneId: string,
  key: TabKey,
  direction: SplitDirection,
  newPaneId: string,
  splitId: string,
  before = false,
): WindowNode => {
  const from = paneWithTab(root, key)
  const tab = from?.tabs.find(t => keyOf(t) === key)
  if(!from || !tab || !findPane(root, paneId)) return root

  // splitting a pane's only tab away would leave it empty
  if(from.id === paneId && from.tabs.length === 1) return root

  const closed = closeTab(root, key)
  const newPane: WindowNode = { id: newPaneId, tabs: [tab], activeTab: key }

  return orEmptyRoot(closed, update(closed, paneId, pane => ({
    id: splitId,
    direction,
    left: before ? newPane : pane,
    right: before ? pane : newPane,
    tabs: [],
  })))
}
