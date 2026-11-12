import React from 'react'
import { FolderNode, TreeRow, visibleRows } from '../../../types/folderTree'
import BrowserRow, { RowHint } from './BrowserRow'
import RenameField from './RenameField'
import { DROP_FOLDER } from './useFolderDrop'
import { useClickWithoutDrag } from './useClickWithoutDrag'
import { ChevronIcon, FolderIcon } from '../../icons'

interface FolderRowProps<T> {
  node: FolderNode<T>
  depth: number
  open: boolean
  selected: boolean
  // what's dragged would go in here
  dropping: boolean
  hint?: string
  onToggle: (key: string) => void
  onMenu: (node: FolderNode<T>, ev: React.MouseEvent) => void
  onDragStart: (node: FolderNode<T>, ev: React.MouseEvent) => void
}

// a folder: click to open or close it, drag it into another, right-click for
// more
const FolderRow = <T, >({
  node, depth, open, selected, dropping, hint, onToggle, onMenu, onDragStart,
}: FolderRowProps<T>) => {
  const { pressed, isClick } = useClickWithoutDrag()

  return (
    <BrowserRow
      role='treeitem'
      aria-expanded={open}
      selected={selected}
      title={node.name}
      className={dropping ? '!bg-accent-tint-strong ring-1 ring-inset ring-accent-focus' : ''}
      style={{ paddingLeft: depth * 12 + 8 }}
      {...{ [DROP_FOLDER]: node.key }}
      onMouseDown={ev => {
        if(ev.button !== 0) return
        pressed(ev)
        onDragStart(node, ev)
      }}
      onClick={ev => { if(isClick(ev)) onToggle(node.key) }}
      onContextMenu={ev => {
        ev.preventDefault()
        ev.stopPropagation()
        onMenu(node, ev)
      }}
    >
      <ChevronIcon open={open} />
      <FolderIcon />
      <span className='truncate font-medium'>{node.name}</span>
      {hint && <RowHint className='ml-auto pl-2'>{hint}</RowHint>}
    </BrowserRow>
  )
}

interface Props<T> {
  root: FolderNode<T>
  expanded: Set<string>
  onToggle: (key: string) => void
  selectedFolder: string | null
  // the folder whose name is being changed
  renaming: string | null
  onRenamed: (key: string, name: string | null) => void
  // the folder what's dragged would go into ('' for the top)
  dropTarget: string | null
  onFolderMenu: (node: FolderNode<T>, ev: React.MouseEvent) => void
  onFolderDragStart: (node: FolderNode<T>, ev: React.MouseEvent) => void
  // shown faintly after a folder's name, e.g. how much is in it
  folderHint?: (node: FolderNode<T>) => string | undefined
  itemKey: (item: T) => string
  renderItem: (item: T, depth: number) => React.ReactNode
  // shown under the rows, e.g. when there's nothing
  children?: React.ReactNode
}

// a browser's folders and what's in them, as a tree. what's dropped on a
// folder (or on a row in it) goes into that folder, and on the space around
// them into the top
const FolderTree = <T, >({
  root, expanded, onToggle, selectedFolder, renaming, onRenamed, dropTarget,
  onFolderMenu, onFolderDragStart, folderHint, itemKey, renderItem, children,
}: Props<T>) => {
  const rows: TreeRow<T>[] = visibleRows(root, expanded)

  return (
    <div
      className={`min-h-full pb-2 ${dropTarget === '' ? 'bg-accent-tint/60 ring-1 ring-inset ring-accent-soft' : ''}`}
      {...{ [DROP_FOLDER]: '' }}
    >
      {rows.map(row => {
        if(row.kind === 'folder') {
          const { node, depth } = row
          if(renaming === node.key) {
            return (
              <RenameField
                key={`folder:${node.key}`}
                initial={node.name}
                label='Folder name'
                depth={depth}
                onDone={name => onRenamed(node.key, name)}
              />
            )
          }
          return (
            <FolderRow
              key={`folder:${node.key}`}
              node={node}
              depth={depth}
              open={expanded.has(node.key)}
              selected={selectedFolder === node.key}
              dropping={dropTarget === node.key}
              hint={folderHint?.(node)}
              onToggle={onToggle}
              onMenu={onFolderMenu}
              onDragStart={onFolderDragStart}
            />
          )
        }
        return (
          <div key={`item:${itemKey(row.item)}`} {...{ [DROP_FOLDER]: row.folder }}>
            {renderItem(row.item, row.depth)}
          </div>
        )
      })}
      {children}
    </div>
  )
}

export default FolderTree
