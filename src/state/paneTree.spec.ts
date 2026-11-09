import * as paneTree from './paneTree'
import { PaneTab, View, WindowNode } from '../types/ui'

const tab = (view: View): PaneTab => ({ label: String(view), component: view })

const playlist = tab(View.Playlist)
const sample = tab(View.SamplePreview)

const twoTabs = (): WindowNode => ({
  id: 'root',
  tabs: [playlist, sample],
  activeView: View.Playlist,
})

// root split side by side: playlist on the left, sample on the right
const splitRoot = (): WindowNode =>
  paneTree.splitPane(twoTabs(), 'root', View.SamplePreview, 'vertical', 'b', 's')

describe('paneTree', () => {
  test('splitting moves the tab into a new pane beside the old one', () => {
    const root = splitRoot()

    expect(root.id).toBe('s')
    expect(root.direction).toBe('vertical')
    expect(root.left).toEqual({ id: 'root', tabs: [playlist], activeView: View.Playlist })
    expect(root.right).toEqual({ id: 'b', tabs: [sample], activeView: View.SamplePreview })
    expect(paneTree.leaves(root).map(l => l.id)).toEqual(['root', 'b'])
  })

  test('splitting before puts the new pane first', () => {
    const root = paneTree.splitPane(
      twoTabs(), 'root', View.SamplePreview, 'horizontal', 'b', 's', true)

    expect(paneTree.leaves(root).map(l => l.id)).toEqual(['b', 'root'])
  })

  test("a pane's only tab can't be split away from it", () => {
    const root = splitRoot()

    expect(paneTree.splitPane(root, 'b', View.SamplePreview, 'vertical', 'c', 't'))
      .toBe(root)
  })

  test('closing the last tab in a pane collapses the split', () => {
    const root = paneTree.closeTab(splitRoot(), View.SamplePreview)

    expect(root).toEqual({ id: 'root', tabs: [playlist], activeView: View.Playlist })
  })

  test('closing the selected tab selects its neighbour', () => {
    const root = paneTree.closeTab(twoTabs(), View.Playlist)

    expect(root.tabs).toEqual([sample])
    expect(root.activeView).toBe(View.SamplePreview)
  })

  test('closing every tab leaves an empty root', () => {
    const root = paneTree.closeTab(paneTree.closeTab(twoTabs(), View.Playlist), View.SamplePreview)

    expect(root).toEqual({ id: 'root', tabs: [] })
  })

  test('moving a pane\'s only tab to another pane collapses the split', () => {
    const root = paneTree.moveTab(splitRoot(), View.SamplePreview, 'root')

    expect(root).toEqual({
      id: 'root',
      tabs: [playlist, sample],
      activeView: View.SamplePreview,
    })
  })

  test('moving a tab within its pane reorders it', () => {
    // drop the playlist tab after the sample tab
    const root = paneTree.moveTab(twoTabs(), View.Playlist, 'root', 2)
    expect(root.tabs).toEqual([sample, playlist])

    // and back before it
    expect(paneTree.moveTab(root, View.Playlist, 'root', 0).tabs).toEqual([playlist, sample])
  })

  test('the tree it was given is never changed', () => {
    const before = splitRoot()
    const snapshot = JSON.parse(JSON.stringify(before))

    paneTree.closeTab(before, View.SamplePreview)
    paneTree.moveTab(before, View.SamplePreview, 'root')
    paneTree.selectTab(before, 'b', View.SamplePreview)

    expect(before).toEqual(snapshot)
  })

  test('finds the pane a view is open in', () => {
    expect(paneTree.paneWithView(splitRoot(), View.SamplePreview)?.id).toBe('b')
    expect(paneTree.paneWithView(paneTree.closeTab(twoTabs(), View.Playlist), View.Playlist))
      .toBeUndefined()
  })
})
