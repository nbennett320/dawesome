import {
  FolderInfo, ROOT, allItems, buildTree, findFolder, freeName, isWithin, movedFolderPath, visibleRows,
} from './folderTree'

const folders: FolderInfo[] = [
  { key: 'v', name: 'Vocals', parent: ROOT },
  { key: 'v2', name: 'verse 10', parent: 'v' },
  { key: 'v1', name: 'Verse 9', parent: 'v' },
  { key: 'd', name: 'drums', parent: ROOT },
]

interface Item { id: string, folder: string }
const items: Item[] = [
  { id: 'a', folder: ROOT },
  { id: 'b', folder: 'v1' },
  { id: 'c', folder: 'v' },
  { id: 'lost', folder: 'gone' },
]

const tree = () => buildTree(folders, items, item => item.folder)

describe('buildTree', () => {
  it('puts folders in their parents, A to Z with numbers as numbers', () => {
    const root = tree()
    expect(root.folders.map(f => f.name)).toEqual(['drums', 'Vocals'])
    expect(findFolder(root, 'v')?.folders.map(f => f.name)).toEqual(['Verse 9', 'verse 10'])
  })

  it('puts items in their folders, and lost ones at the top', () => {
    const root = tree()
    expect(root.items.map(i => i.id)).toEqual(['a', 'lost'])
    expect(findFolder(root, 'v1')?.items.map(i => i.id)).toEqual(['b'])
    expect(allItems(findFolder(root, 'v') ?? root).map(i => i.id)).toEqual(['c', 'b'])
  })

  it('puts a folder with a missing or looping parent at the top', () => {
    const odd: FolderInfo[] = [
      { key: 'x', name: 'x', parent: 'missing' },
      { key: 'p', name: 'p', parent: 'q' },
      { key: 'q', name: 'q', parent: 'p' },
    ]
    const root = buildTree(odd, [], () => ROOT)
    expect(root.folders.map(f => f.key)).toEqual(['p', 'q', 'x'])
  })
})

describe('visibleRows', () => {
  it('shows what is in open folders, folders before items', () => {
    const rows = visibleRows(tree(), new Set(['v']))
    expect(rows.map(r => (r.kind === 'folder' ? `${r.node.key}@${r.depth}` : `${r.item.id}@${r.depth}`)))
      .toEqual(['d@0', 'v@0', 'v1@1', 'v2@1', 'c@1', 'a@0', 'lost@0'])
  })

  it('gives an item the folder it is shown in', () => {
    const row = visibleRows(tree(), new Set(['v'])).find(r => r.kind === 'item' && r.item.id === 'c')
    expect(row).toMatchObject({ folder: 'v' })
  })
})

describe('isWithin', () => {
  it('follows parents', () => {
    expect(isWithin(folders, 'v1', 'v')).toBe(true)
    expect(isWithin(folders, 'v', 'v')).toBe(true)
    expect(isWithin(folders, 'v', 'v1')).toBe(false)
    expect(isWithin(folders, 'd', 'v')).toBe(false)
    expect(isWithin(folders, 'anything', ROOT)).toBe(true)
  })
})

describe('freeName', () => {
  it('numbers a name that is taken', () => {
    expect(freeName('New folder', ['Drums'])).toBe('New folder')
    expect(freeName('New folder', ['new folder', 'New folder 2'])).toBe('New folder 3')
  })
})

describe('movedFolderPath', () => {
  it('moves the folder and what is in it, nothing else', () => {
    expect(movedFolderPath('Vocals', 'Vocals', 'Old/Vocals')).toBe('Old/Vocals')
    expect(movedFolderPath('Vocals/Verse', 'Vocals', 'Old/Vocals')).toBe('Old/Vocals/Verse')
    expect(movedFolderPath('Vocals 2', 'Vocals', 'Old/Vocals')).toBe('Vocals 2')
  })
})
