import React from 'react'
import { ask } from '@tauri-apps/plugin-dialog'
import { DragItem, FolderScope, beginSampleDrag } from '../../../state/sampleDrag'
import {
  FolderInfo, FolderNode, ROOT, allItems, findFolder, freeName, isWithin, movedFolderPath,
} from '../../../types/folderTree'
import { plural } from '../../../util/helpers'
import ClipMenu, { ClipMenuItem } from '../../../views/playlist/ClipMenu'
import { AnalogButton } from '../../fourtrack'
import { NewFolderIcon } from '../../icons'
import BrowserSearch from './BrowserSearch'
import FolderTree from './FolderTree'
import { useFail } from './useFail'
import { useFolderDrop } from './useFolderDrop'
import { useSessionSet } from './useSessionSet'

// what an item's row is told about where it's shown, and what it can ask of
// the browser around it
export interface ItemContext {
  // listed as a search result, not in its folder
  searching: boolean
  // searching: the folder it's in
  where?: string
  // right-click on it: show a menu (made when it's shown, so it's up to date)
  openMenu: (ev: React.MouseEvent, items: () => ClipMenuItem[]) => void
  // something other than a folder was chosen: no folder is selected any more
  deselectFolder: () => void
}

interface Props<T> {
  // which browser this is, for what's dragged and the folders kept open
  scope: FolderScope
  // the tree's name, and the search box's, as read out
  label: string
  searchLabel: string
  // what's in the folders, in the singular: "2 recordings"
  noun: string
  folders: FolderInfo[]
  tree: FolderNode<T>
  itemKey: (item: T) => string
  renderItem: (item: T, depth: number, context: ItemContext) => React.ReactNode
  // the items that match, each with the folder it's in
  search: (query: string) => Array<{ item: T, where?: string }>
  noMatch: string
  // shown under the tree, e.g. when there's nothing yet
  emptyMessage: string | null
  // why folders can't be changed right now
  busy?: string

  // the folder operations. the new key of a folder that moved or was renamed is
  // returned when a folder's key is its path, so what was open follows it
  createFolder: (parent: string, name: string) => Promise<string>
  renameFolder: (key: string, name: string) => Promise<string | void>
  moveFolder: (key: string, into: string) => Promise<string | void>
  deleteFolder: (node: FolderNode<T>) => Promise<void>
  deleteLabel: string
  // what to ask before deleting a folder, given `what` is in it
  deletePrompt: (name: string, what: string) => string
  // why a folder can't be deleted, if it can't
  blockDelete: (node: FolderNode<T>) => string | undefined

  // things other than folders dropped into one
  canDropItem: (item: DragItem, key: string) => boolean
  dropItem: (item: DragItem, key: string) => Promise<void>
  // what failing to do that is called: "move the pattern"
  dropFailure: string
  // a folder was clicked
  onSelectFolder?: (key: string) => void
}

type Menu = { x: number, y: number, items: () => ClipMenuItem[] }

// a sidebar page of folders and what's in them: find things by name, drag them
// to a folder, and make, rename, move and delete the folders. what's in the
// folders (and what can be done with it) is up to the page
const FolderBrowser = <T, >({
  scope, label, searchLabel, noun, folders, tree, itemKey, renderItem, search, noMatch, emptyMessage, busy,
  createFolder, renameFolder, moveFolder, deleteFolder, deleteLabel, deletePrompt, blockDelete,
  canDropItem, dropItem, dropFailure, onSelectFolder,
}: Props<T>) => {
  const fail = useFail()
  const [expanded, setExpanded] = useSessionSet(scope)
  // the folder clicked last: new folders go in it
  const [selectedFolder, setSelectedFolder] = React.useState<string | null>(null)
  const [renamingFolder, setRenamingFolder] = React.useState<string | null>(null)
  const [menu, setMenu] = React.useState<Menu | null>(null)
  const [query, setQuery] = React.useState('')
  const boxRef = React.useRef<HTMLDivElement>(null)
  const listRef = React.useRef<HTMLDivElement>(null)

  const open = (key: string) => {
    if(key === ROOT) return
    setExpanded(prev => {
      const next = new Set(prev)
      // its parents too, so it shows
      next.add(key)
      folders.forEach(f => { if(isWithin(folders, key, f.key)) next.add(f.key) })
      return next
    })
  }

  const toggle = (key: string) => {
    setSelectedFolder(key)
    onSelectFolder?.(key)
    setExpanded(prev => {
      const next = new Set(prev)
      if(next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  // a folder moved or renamed: what was open, or selected, in it still is
  const follow = (from: string, to: string | void) => {
    if(to === undefined || to === from) return
    setExpanded(prev => new Set(Array.from(prev, key => movedFolderPath(key, from, to))))
    setSelectedFolder(prev => (prev === null ? prev : movedFolderPath(prev, from, to)))
  }

  // folders

  const newFolder = (parent: string) => {
    const siblings = (findFolder(tree, parent) ?? tree).folders.map(f => f.name)
    createFolder(parent, freeName('New folder', siblings))
      .then(made => {
        open(parent)
        setSelectedFolder(made)
        setRenamingFolder(made)
      })
      .catch(fail('make a folder'))
  }

  const renamed = (key: string, name: string | null) => {
    setRenamingFolder(null)
    if(name === null) return
    renameFolder(key, name).then(to => follow(key, to)).catch(fail('rename the folder'))
  }

  const confirmDelete = async (node: FolderNode<T>) => {
    const count = allItems(node).length
    const what = count ? ` and the ${plural(count, noun)} in it` : ''
    const sure = await ask(deletePrompt(node.name, what), 'Delete folder')
    if(!sure) return
    await deleteFolder(node)
  }

  // dragging into folders

  const canDrop = (item: DragItem, key: string): boolean => {
    if(busy) return false
    // (a folder of this page's own: not one dragged over from another)
    if(item.kind === 'folder' && item.scope === scope) {
      const parent = folders.find(f => f.key === item.key)?.parent ?? ROOT
      // not where it is already, and not inside itself
      return parent !== key && !isWithin(folders, key, item.key)
    }
    return canDropItem(item, key)
  }

  const onDrop = (item: DragItem, key: string) => {
    if(item.kind === 'folder') {
      moveFolder(item.key, key)
        .then(to => {
          follow(item.key, to)
          open(key)
        })
        .catch(fail('move the folder'))
      return
    }
    dropItem(item, key).then(() => open(key)).catch(fail(dropFailure))
  }

  const dropTarget = useFolderDrop(listRef, { canDrop, onDrop, onHold: open })

  // menus

  const openMenu = (ev: React.MouseEvent, items: () => ClipMenuItem[]) => {
    const box = boxRef.current?.getBoundingClientRect()
    const at = box ? { x: ev.clientX - box.left, y: ev.clientY - box.top } : { x: 0, y: 0 }
    setMenu({ ...at, items })
  }

  const closeMenu = React.useCallback(() => setMenu(null), [])

  const folderMenu = (node: FolderNode<T>): ClipMenuItem[] => {
    const blocked = blockDelete(node)
    return [
      { label: 'New folder', disabled: !!busy, title: busy, onSelect: () => newFolder(node.key) },
      { label: 'Rename', disabled: !!busy, title: busy, onSelect: () => setRenamingFolder(node.key) },
      {
        label: deleteLabel,
        danger: true,
        disabled: !!blocked || !!busy,
        title: busy ?? blocked,
        onSelect: () => { confirmDelete(node).catch(fail(`delete ${node.name}`)) },
      },
    ]
  }

  // what's listed

  const searched = query.trim()
  const context: ItemContext = { searching: !!searched, openMenu, deselectFolder: () => setSelectedFolder(null) }

  let list: React.ReactNode
  if(searched) {
    const found = search(searched)
    list = (
      <div className='pb-2'>
        {found.map(({ item, where }) => (
          <React.Fragment key={itemKey(item)}>
            {renderItem(item, 0, { ...context, where })}
          </React.Fragment>
        ))}
        {!found.length && <div className='px-3 py-2 text-xs text-ink-faint'>{noMatch}</div>}
      </div>
    )
  } else {
    list = (
      <FolderTree
        root={tree}
        expanded={expanded}
        onToggle={toggle}
        selectedFolder={selectedFolder}
        renaming={renamingFolder}
        onRenamed={renamed}
        dropTarget={dropTarget}
        onFolderMenu={(node, ev) => openMenu(ev, () => folderMenu(node))}
        onFolderDragStart={(node, ev) => beginSampleDrag({ kind: 'folder', scope, key: node.key, label: node.name }, ev)}
        folderHint={node => {
          const count = allItems(node).length
          return count ? String(count) : undefined
        }}
        itemKey={itemKey}
        renderItem={(item, depth) => renderItem(item, depth, context)}
      >
        {emptyMessage && <div className='px-3 py-2 text-xs text-ink-faint'>{emptyMessage}</div>}
      </FolderTree>
    )
  }

  return (
    <div ref={boxRef} className='relative flex flex-col h-full min-h-0'>
      <BrowserSearch value={query} onChange={setQuery} label={searchLabel}>
        <AnalogButton
          size='xs'
          aria-label='New folder'
          title={busy ?? (selectedFolder ? 'A new folder, in the folder selected' : 'A new folder')}
          disabled={!!busy}
          onPress={() => newFolder(selectedFolder ?? ROOT)}
        >
          <NewFolderIcon />
        </AnalogButton>
      </BrowserSearch>

      <div
        ref={listRef}
        className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden'
        role='tree'
        aria-label={label}
        tabIndex={-1}
        onContextMenu={ev => {
          ev.preventDefault()
          if(!searched) {
            openMenu(ev, () => [{ label: 'New folder', disabled: !!busy, title: busy, onSelect: () => newFolder(ROOT) }])
          }
        }}
      >
        {list}
      </div>

      {menu && <ClipMenu x={menu.x} y={menu.y} items={menu.items()} onClose={closeMenu} />}
    </div>
  )
}

export default FolderBrowser
