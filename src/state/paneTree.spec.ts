import * as paneTree from './paneTree'
import { PaneTab, tabKey, View, WindowNode } from '../types/ui'

const playlist: PaneTab = { component: View.Playlist }
const sample: PaneTab = { component: View.SamplePreview }
const PLAYLIST = tabKey(View.Playlist)
const SAMPLE = tabKey(View.SamplePreview)

const twoTabs = (): WindowNode => ({
  id: 'root',
  tabs: [playlist, sample],
  activeTab: PLAYLIST,
})

// root split side by side: playlist on the left, sample on the right
const splitRoot = (): WindowNode =>
  paneTree.splitPane(twoTabs(), 'root', SAMPLE, 'vertical', 'b', 's')

describe('paneTree', () => {
  test('splitting moves the tab into a new pane beside the old one', () => {
    const root = splitRoot()

    expect(root.id).toBe('s')
    expect(root.direction).toBe('vertical')
    expect(root.left).toEqual({ id: 'root', tabs: [playlist], activeTab: PLAYLIST })
    expect(root.right).toEqual({ id: 'b', tabs: [sample], activeTab: SAMPLE })
    expect(paneTree.leaves(root).map(l => l.id)).toEqual(['root', 'b'])
  })

  test('splitting before puts the new pane first', () => {
    const root = paneTree.splitPane(
      twoTabs(), 'root', SAMPLE, 'horizontal', 'b', 's', true)

    expect(paneTree.leaves(root).map(l => l.id)).toEqual(['b', 'root'])
  })

  test("a pane's only tab can't be split away from it", () => {
    const root = splitRoot()

    expect(paneTree.splitPane(root, 'b', SAMPLE, 'vertical', 'c', 't'))
      .toBe(root)
  })

  test('closing the last tab in a pane collapses the split', () => {
    const root = paneTree.closeTab(splitRoot(), SAMPLE)

    expect(root).toEqual({ id: 'root', tabs: [playlist], activeTab: PLAYLIST })
  })

  test('closing the selected tab selects its neighbour', () => {
    const root = paneTree.closeTab(twoTabs(), PLAYLIST)

    expect(root.tabs).toEqual([sample])
    expect(root.activeTab).toBe(SAMPLE)
  })

  test('closing every tab leaves an empty root', () => {
    const root = paneTree.closeTab(paneTree.closeTab(twoTabs(), PLAYLIST), SAMPLE)

    expect(root).toEqual({ id: 'root', tabs: [] })
  })

  test('moving a pane\'s only tab to another pane collapses the split', () => {
    const root = paneTree.moveTab(splitRoot(), SAMPLE, 'root')

    expect(root).toEqual({
      id: 'root',
      tabs: [playlist, sample],
      activeTab: SAMPLE,
    })
  })

  test('moving a tab within its pane reorders it', () => {
    // drop the playlist tab after the sample tab
    const root = paneTree.moveTab(twoTabs(), PLAYLIST, 'root', 2)
    expect(root.tabs).toEqual([sample, playlist])

    // and back before it
    expect(paneTree.moveTab(root, PLAYLIST, 'root', 0).tabs).toEqual([playlist, sample])
  })

  test('the tree it was given is never changed', () => {
    const before = splitRoot()
    const snapshot = JSON.parse(JSON.stringify(before))

    paneTree.closeTab(before, SAMPLE)
    paneTree.moveTab(before, SAMPLE, 'root')
    paneTree.selectTab(before, 'b', SAMPLE)

    expect(before).toEqual(snapshot)
  })

  test('finds the pane a tab is open in', () => {
    expect(paneTree.paneWithTab(splitRoot(), SAMPLE)?.id).toBe('b')
    expect(paneTree.paneWithTab(paneTree.closeTab(twoTabs(), PLAYLIST), PLAYLIST))
      .toBeUndefined()
  })

  test('a plugin tab per slot, told apart by the slot', () => {
    const plugin = (slotId: number): PaneTab => ({ component: View.Plugin, slotId })
    let root = paneTree.addTab(twoTabs(), 'root', plugin(4))
    root = paneTree.addTab(root, 'root', plugin(7))

    expect(root.tabs).toEqual([playlist, sample, plugin(4), plugin(7)])
    expect(root.activeTab).toBe(tabKey(View.Plugin, 7))
    expect(paneTree.activeTabOf(root)).toEqual(plugin(7))

    root = paneTree.splitPane(root, 'root', tabKey(View.Plugin, 4), 'vertical', 'b', 's')
    expect(paneTree.paneWithTab(root, tabKey(View.Plugin, 4))?.id).toBe('b')
    expect(paneTree.paneWithTab(root, tabKey(View.Plugin, 7))?.id).toBe('root')

    // closing one leaves the other
    root = paneTree.closeTab(root, tabKey(View.Plugin, 7))
    expect(paneTree.leaves(root).flatMap(l => l.tabs)).toEqual([playlist, sample, plugin(4)])
    expect(paneTree.findPane(root, 'root')?.activeTab).toBe(SAMPLE)
  })

  test('closes the tabs it\'s told not to keep', () => {
    const plugin = (slotId: number): PaneTab => ({ component: View.Plugin, slotId })
    const root = paneTree.splitPane(
      paneTree.addTab(paneTree.addTab(twoTabs(), 'root', plugin(4)), 'root', plugin(7)),
      'root', tabKey(View.Plugin, 4), 'vertical', 'b', 's')

    const kept = paneTree.closeTabsUnless(root, tab => tab.slotId !== 4)
    expect(kept).toEqual({ id: 'root', tabs: [playlist, sample, plugin(7)], activeTab: tabKey(View.Plugin, 7) })
    // nothing to close: the same tree
    expect(paneTree.closeTabsUnless(kept, () => true)).toBe(kept)
  })
})
