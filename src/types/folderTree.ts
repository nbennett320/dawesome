// folders in the sidebar's browsers. recordings' folders are on disk and
// patterns' are in the project, so a folder is just a key, a name and its
// parent's key here. the top level's key is ROOT

export const ROOT = ''

export interface FolderInfo {
  key: string
  name: string
  // ROOT for a folder at the top
  parent: string
}

export interface FolderNode<T> {
  key: string
  name: string
  // A to Z
  folders: FolderNode<T>[]
  // in the order they were given
  items: T[]
}

// names compared as people read them: case aside, numbers as numbers
export const compareNames = (a: string, b: string): number =>
  a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' })

// whether `key` is `ancestor`, or inside it, following parents. a folder
// whose parents loop is inside nothing but itself
export const isWithin = (folders: FolderInfo[], key: string, ancestor: string): boolean => {
  if(ancestor === ROOT || key === ancestor) return true
  const parents = new Map(folders.map(f => [f.key, f.parent]))
  const seen = new Set<string>()
  let at = parents.get(key)
  while(at !== undefined && at !== ROOT && !seen.has(at)) {
    if(at === ancestor) return true
    seen.add(at)
    at = parents.get(at)
  }
  return false
}

// the folders as a tree, with each item in its folder. a folder whose parent
// is missing (or whose parents loop), and an item whose folder is missing,
// go at the top
export const buildTree = <T, >(
  folders: FolderInfo[],
  items: T[],
  folderOf: (item: T) => string,
): FolderNode<T> => {
  const root: FolderNode<T> = { key: ROOT, name: '', folders: [], items: [] }
  const nodes = new Map<string, FolderNode<T>>([[ROOT, root]])
  folders.forEach(f => {
    if(!nodes.has(f.key)) nodes.set(f.key, { key: f.key, name: f.name, folders: [], items: [] })
  })

  const placed = new Set<string>()
  folders.forEach(f => {
    const node = nodes.get(f.key)
    if(!node || node === root || placed.has(f.key)) return
    placed.add(f.key)
    const loops = f.parent !== ROOT && isWithin(folders, f.parent, f.key)
    const parent = (!loops && nodes.get(f.parent)) || root
    parent.folders.push(node)
  })

  items.forEach(item => { (nodes.get(folderOf(item)) ?? root).items.push(item) })

  const sort = (node: FolderNode<T>) => {
    node.folders.sort((a, b) => compareNames(a.name, b.name))
    node.folders.forEach(sort)
  }
  sort(root)
  return root
}

export type TreeRow<T> =
  | { kind: 'folder', node: FolderNode<T>, depth: number }
  | { kind: 'item', item: T, folder: string, depth: number }

// the rows showing, top to bottom: each folder, then (when it's open) what's
// in it, folders before items
export const visibleRows = <T, >(root: FolderNode<T>, expanded: Set<string>): TreeRow<T>[] => {
  const rows: TreeRow<T>[] = []
  const add = (node: FolderNode<T>, depth: number) => {
    node.folders.forEach(folder => {
      rows.push({ kind: 'folder', node: folder, depth })
      if(expanded.has(folder.key)) add(folder, depth + 1)
    })
    node.items.forEach(item => rows.push({ kind: 'item', item, folder: node.key, depth }))
  }
  add(root, 0)
  return rows
}

export const findFolder = <T, >(root: FolderNode<T>, key: string): FolderNode<T> | null => {
  if(root.key === key) return root
  return root.folders.reduce<FolderNode<T> | null>((found, folder) => found ?? findFolder(folder, key), null)
}

// everything in a folder, and in its folders
export const allItems = <T, >(node: FolderNode<T>): T[] =>
  [...node.items, ...node.folders.flatMap(allItems)]

// a name not used by any of `taken`: the name, or the name and a number
export const freeName = (name: string, taken: string[]): string => {
  const used = new Set(taken.map(t => t.toLowerCase()))
  if(!used.has(name.toLowerCase())) return name
  let n = 2
  while(used.has(`${name} ${n}`.toLowerCase())) n += 1
  return `${name} ${n}`
}
