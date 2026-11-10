import React from 'react'
import { ask } from '@tauri-apps/plugin-dialog'
import { useAppDispatch, useAppSelector } from '../../../hooks/redux'
import {
  addPatternFolder,
  loadSequencer,
  movePatternFolder,
  movePatternToFolder,
  removePatternFolder,
  renamePatternFolder,
  selectPattern,
  selectPatternFolders,
  selectPatternRun,
  selectPatterns,
  selectSelectedPatternId,
} from '../../../state/slices/sequencerSlice'
import { selectPlaylistItems } from '../../../state/slices/playlistSlice'
import { setProjectNotice } from '../../../state/slices/projectSlice'
import { openTab } from '../../../state/slices/windowSlice'
import { DragItem, beginSampleDrag } from '../../../state/sampleDrag'
import {
  FolderNode, ROOT, allItems, buildTree, findFolder, freeName, isWithin,
} from '../../../types/folderTree'
import {
  Pattern, folderIdOf, folderKey, folderPathLabel, patternFolderInfos, patternHint, patternLabel, searchPatterns,
} from '../../../types/sequencer'
import { View } from '../../../types/ui'
import { useTransport } from '../../../views/transport/useTransport'
import ClipMenu, { ClipMenuItem } from '../../../views/playlist/ClipMenu'
import { AnalogButton, Led } from '../../fourtrack'
import BrowserSearch from '../browser/BrowserSearch'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import FolderTree from '../browser/FolderTree'
import { NewFolderIcon } from '../browser/icons'
import { useClickWithoutDrag } from '../browser/useClickWithoutDrag'
import { useFolderDrop } from '../browser/useFolderDrop'
import { useSessionSet } from '../browser/useSessionSet'

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`

interface RowProps {
  pattern: Pattern
  hint: string
  // placed in the playlist at least once
  used: boolean
  selected: boolean
  playing: boolean
  depth: number
  onSelect: (pattern: Pattern) => void
}

// one pattern: click to show it in the Sequencer, drag it into the playlist
// or a folder
const PatternRow = ({ pattern, hint, used, selected, playing, depth, onSelect }: RowProps) => {
  const label = patternLabel(pattern)
  const { pressed, isClick } = useClickWithoutDrag()

  return (
    <BrowserRow
      role='treeitem'
      selected={selected}
      used={used}
      title={`${label}. Click to show it in the Sequencer, drag into the playlist to place it`}
      style={{ paddingLeft: depth * 12 + 8 }}
      // only a drag starts on mousedown: showing the Sequencer straight away
      // could hide the playlist (when they share a pane) before it's dropped
      onMouseDown={ev => {
        if(ev.button !== 0) return
        pressed(ev)
        beginSampleDrag({ kind: 'pattern', patternId: pattern.id, label }, ev)
      }}
      // also enter and space, which buttons turn into clicks
      onClick={ev => { if(isClick(ev)) onSelect(pattern) }}
    >
      <Led color='lime' state={playing ? 'on' : 'off'} className='shrink-0 scale-75' />
      <span className='truncate'>{label}</span>
      <RowHint className='ml-auto pl-2'>{hint}</RowHint>
    </BrowserRow>
  )
}

type Menu =
  | { kind: 'folder', node: FolderNode<Pattern>, x: number, y: number }
  | { kind: 'top', x: number, y: number }

// the Sequencer's patterns, in folders: find them by name, drag them into
// the playlist, and file them away
const PatternBrowser = () => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatterns)
  const folders = useAppSelector(selectPatternFolders)
  const selectedId = useAppSelector(selectSelectedPatternId)
  const run = useAppSelector(selectPatternRun)
  const items = useAppSelector(selectPlaylistItems)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4
  const [expanded, setExpanded] = useSessionSet('patterns')
  // the folder clicked last: new folders go in it
  const [selectedFolder, setSelectedFolder] = React.useState<string | null>(null)
  const [renamingFolder, setRenamingFolder] = React.useState<string | null>(null)
  const [menu, setMenu] = React.useState<Menu | null>(null)
  const [query, setQuery] = React.useState('')
  const boxRef = React.useRef<HTMLDivElement>(null)
  const listRef = React.useRef<HTMLDivElement>(null)

  // the engine owns the sequencer: catch up with it, in case the Sequencer
  // tab hasn't been opened yet
  React.useEffect(() => { dispatch(loadSequencer()) }, [dispatch])

  const clipCounts = React.useMemo(() => {
    const counts = new Map<number, number>()
    items.forEach(item => {
      if(item.patternId !== undefined) counts.set(item.patternId, (counts.get(item.patternId) ?? 0) + 1)
    })
    return counts
  }, [items])

  const infos = React.useMemo(() => patternFolderInfos(folders), [folders])
  const tree = React.useMemo(() => buildTree(infos, patterns, p => folderKey(p.folder)), [infos, patterns])

  const fail = (what: string) => (e: unknown) => {
    dispatch(setProjectNotice({ kind: 'error', message: `Couldn't ${what}: ${message(e)}` }))
  }

  const open = (key: string) => {
    if(key === ROOT) return
    setExpanded(prev => {
      const next = new Set(prev)
      // its parents too, so it shows
      infos.forEach(f => { if(isWithin(infos, key, f.key)) next.add(f.key) })
      return next
    })
  }

  const toggle = (key: string) => {
    setSelectedFolder(key)
    setExpanded(prev => {
      const next = new Set(prev)
      if(next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  const show = (pattern: Pattern) => {
    setSelectedFolder(null)
    dispatch(selectPattern(pattern.id))
    dispatch(openTab(View.Sequencer))
  }

  // folders

  const newFolder = (parent: string) => {
    const siblings = (findFolder(tree, parent) ?? tree).folders.map(f => f.name)
    dispatch(addPatternFolder(freeName('New folder', siblings), folderIdOf(parent)))
      .then(id => {
        open(parent)
        setSelectedFolder(folderKey(id))
        setRenamingFolder(folderKey(id))
      })
      .catch(fail('make a folder'))
  }

  const renameFolder = (key: string, name: string | null) => {
    setRenamingFolder(null)
    const id = folderIdOf(key)
    if(name === null || id === null) return
    dispatch(renamePatternFolder(id, name)).catch(fail('rename the folder'))
  }

  const placedIn = (node: FolderNode<Pattern>) =>
    allItems(node).filter(p => (clipCounts.get(p.id) ?? 0) > 0).length

  const deleteFolder = async (node: FolderNode<Pattern>) => {
    const id = folderIdOf(node.key)
    if(id === null) return
    const count = allItems(node).length
    const what = count ? ` and the ${plural(count, 'pattern')} in it` : ''
    const sure = await ask(`Delete "${node.name}"${what}? This can't be undone.`, 'Delete folder')
    if(!sure) return
    await dispatch(removePatternFolder(id))
  }

  // dragging into folders

  const canDrop = (item: DragItem, key: string): boolean => {
    if(item.kind === 'pattern') {
      const pattern = patterns.find(p => p.id === item.patternId)
      return !!pattern && folderKey(pattern.folder) !== key
    }
    if(item.kind === 'folder' && item.scope === 'patterns') {
      const parent = infos.find(f => f.key === item.key)?.parent ?? ROOT
      // not where it is already, and not inside itself
      return parent !== key && !isWithin(infos, key, item.key)
    }
    return false
  }

  const onDrop = (item: DragItem, key: string) => {
    const to = folderIdOf(key)
    if(item.kind === 'pattern') {
      dispatch(movePatternToFolder(item.patternId, to))
        .then(() => open(key))
        .catch(fail('move the pattern'))
    } else if(item.kind === 'folder') {
      const id = folderIdOf(item.key)
      if(id === null) return
      dispatch(movePatternFolder(id, to))
        .then(() => open(key))
        .catch(fail('move the folder'))
    }
  }

  const dropTarget = useFolderDrop(listRef, { canDrop, onDrop, onHold: open })

  // menus

  const at = (ev: React.MouseEvent) => {
    const box = boxRef.current?.getBoundingClientRect()
    return box ? { x: ev.clientX - box.left, y: ev.clientY - box.top } : { x: 0, y: 0 }
  }

  const closeMenu = React.useCallback(() => setMenu(null), [])

  const menuItems = (shown: Menu): ClipMenuItem[] => {
    if(shown.kind === 'top') return [{ label: 'New folder', onSelect: () => newFolder(ROOT) }]

    const { node } = shown
    const placed = placedIn(node)
    return [
      { label: 'New folder', onSelect: () => newFolder(node.key) },
      { label: 'Rename', onSelect: () => setRenamingFolder(node.key) },
      {
        label: 'Delete',
        danger: true,
        disabled: placed > 0,
        title: placed
          ? `${plural(placed, 'pattern')} in it ${placed === 1 ? 'is' : 'are'} in the playlist. Remove those clips first`
          : undefined,
        onSelect: () => { deleteFolder(node).catch(fail(`delete ${node.name}`)) },
      },
    ]
  }

  // what's listed

  const renderPattern = (pattern: Pattern, depth: number, where?: string) => {
    const clips = clipCounts.get(pattern.id) ?? 0
    const hint = patternHint(pattern, beatsPerBar, clips)
    return (
      <PatternRow
        pattern={pattern}
        hint={where ? `${where} · ${hint}` : hint}
        used={clips > 0}
        selected={pattern.id === selectedId}
        playing={pattern.id === run?.patternId}
        depth={depth}
        onSelect={show}
      />
    )
  }

  const search = query.trim()
  let list: React.ReactNode
  if(search) {
    const found = searchPatterns(patterns, search)
    list = (
      <div className='pb-2'>
        {found.map(pattern => (
          <React.Fragment key={pattern.id}>
            {renderPattern(pattern, 0, folderPathLabel(folders, pattern.folder))}
          </React.Fragment>
        ))}
        {!found.length && <div className='px-3 py-2 text-xs text-slate-400'>No patterns match.</div>}
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
        onRenamed={renameFolder}
        dropTarget={dropTarget}
        onFolderMenu={(node, ev) => setMenu({ kind: 'folder', node, ...at(ev) })}
        onFolderDragStart={(node, ev) => beginSampleDrag(
          { kind: 'folder', scope: 'patterns', key: node.key, label: node.name }, ev)}
        folderHint={node => {
          const count = allItems(node).length
          return count ? String(count) : undefined
        }}
        itemKey={pattern => String(pattern.id)}
        renderItem={(pattern, depth) => renderPattern(pattern, depth)}
      >
        {!patterns.length && !folders.length && (
          <div className='px-3 py-2 text-xs text-slate-400'>No patterns yet. Make one in the Sequencer.</div>
        )}
      </FolderTree>
    )
  }

  return (
    <div ref={boxRef} className='relative flex flex-col h-full min-h-0'>
      <BrowserSearch value={query} onChange={setQuery} label='Search patterns'>
        <AnalogButton
          size='xs'
          aria-label='New folder'
          title={selectedFolder ? 'A new folder, in the folder selected' : 'A new folder'}
          onPress={() => newFolder(selectedFolder ?? ROOT)}
        >
          <NewFolderIcon />
        </AnalogButton>
      </BrowserSearch>

      <div
        ref={listRef}
        className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden'
        role='tree'
        aria-label='Patterns'
        tabIndex={-1}
        onContextMenu={ev => {
          ev.preventDefault()
          if(!search) setMenu({ kind: 'top', ...at(ev) })
        }}
      >
        {list}
      </div>

      {menu && <ClipMenu x={menu.x} y={menu.y} items={menuItems(menu)} onClose={closeMenu} />}
    </div>
  )
}

export default PatternBrowser
