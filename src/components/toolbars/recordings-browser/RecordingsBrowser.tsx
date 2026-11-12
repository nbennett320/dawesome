import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog'
import { useAppDispatch, useAppSelector } from '../../../hooks/redux'
import { playlistSlice, selectPlaylistItems } from '../../../state/slices/playlistSlice'
import { setProjectNotice } from '../../../state/slices/projectSlice'
import {
  selectRecording, selectTakesRevision, takesChanged,
} from '../../../state/slices/recordingSlice'
import { selectSequencerRows, setSequencer } from '../../../state/slices/sequencerSlice'
import { DragItem, beginSampleDrag } from '../../../state/sampleDrag'
import {
  FolderNode, ROOT, allItems, buildTree, findFolder, freeName,
} from '../../../types/folderTree'
import {
  Library, MovedRecording, RecordingFile, RecordingsListing, TakeUse,
  describeUse, formatTakeDate, formatTakeDuration, movedFolderPath, parentFolder,
  recordingFolders, searchTakes, takeUses,
} from '../../../types/recordings'
import { AnalogButton } from '../../fourtrack'
import ClipMenu, { ClipMenuItem } from '../../../views/playlist/ClipMenu'
import BrowserSearch from '../browser/BrowserSearch'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import FolderTree from '../browser/FolderTree'
import RenameField from '../browser/RenameField'
import { useFolderDrop } from '../browser/useFolderDrop'
import { useSessionSet } from '../browser/useSessionSet'
import { NewFolderIcon, WaveIcon } from '../../icons'
import { plural } from '../../../util/helpers'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

const isUsed = (use: TakeUse | undefined) => !!use && (use.clips > 0 || use.rows > 0)

// what the user calls the files of each library
const words = {
  recordings: {
    noun: 'recording', nouns: 'recordings', title: 'Recordings', made: 'Recorded',
    none: 'Nothing recorded yet. Arm a track and press record.',
  },
  clips: {
    noun: 'clip', nouns: 'clips', title: 'Clips', made: 'Made',
    none: 'No clips yet. Deep copy or join clips to make some.',
  },
}


interface RowProps {
  take: RecordingFile
  library: Library
  use: TakeUse | undefined
  selected: boolean
  depth: number
  // searching: the folder it's in, shown instead of when it was recorded
  showFolder: boolean
  onPreview: (take: RecordingFile) => void
  onMenu: (take: RecordingFile, ev: React.MouseEvent) => void
}

// one take: click to hear it, drag it into the playlist, the Sequencer, or
// a folder, right-click to rename or delete it
const TakeRow = ({ take, library, use, selected, depth, showFolder, onPreview, onMenu }: RowProps) => {
  const recorded = take.modifiedMs === null ? '' : new Date(take.modifiedMs).toLocaleString()

  const onMouseDown = (ev: React.MouseEvent) => {
    if(ev.button !== 0) return
    onPreview(take)
    beginSampleDrag({ kind: 'sample', path: take.path }, ev)
  }

  const onKeyDown = (ev: React.KeyboardEvent) => {
    // (space plays and pauses, see App)
    if(ev.key !== 'Enter') return
    ev.preventDefault()
    onPreview(take)
  }

  return (
    <BrowserRow
      role='treeitem'
      selected={selected}
      used={isUsed(use)}
      title={[take.path, recorded && `${words[library].made} ${recorded}`, describeUse(use)].filter(Boolean).join('\n')}
      style={{ paddingLeft: depth * 12 + 8 }}
      onMouseDown={onMouseDown}
      onKeyDown={onKeyDown}
      onContextMenu={ev => {
        ev.preventDefault()
        ev.stopPropagation()
        onMenu(take, ev)
      }}
    >
      <WaveIcon />
      <span className='truncate'>{take.name}</span>
      {showFolder && take.folder && <RowHint className='ml-auto pl-2 max-w-[35%]'>{take.folder}</RowHint>}
      <RowHint className={`${showFolder && take.folder ? '' : 'ml-auto'} pl-2 tabular-nums`}>
        {formatTakeDuration(take.durationSecs)}
      </RowHint>
      {!showFolder && <RowHint className='w-12 text-right'>{formatTakeDate(take.modifiedMs)}</RowHint>}
    </BrowserRow>
  )
}

type Menu =
  | { kind: 'take', take: RecordingFile, x: number, y: number }
  | { kind: 'folder', node: FolderNode<RecordingFile>, x: number, y: number }
  | { kind: 'top', x: number, y: number }

interface BrowserProps {
  // which of the project's folders of audio to show
  library?: Library
}

// the project's recorded takes (or, for the Clips page, the files the app
// wrote out itself), in the folder's folders: hear them, drag them back in,
// file them into folders, see which ones the project isn't using, and rename
// or bin them
const RecordingsBrowser = ({ library = 'recordings' }: BrowserProps) => {
  const dispatch = useAppDispatch()
  const w = words[library]
  const revision = useAppSelector(selectTakesRevision)
  // only takes are recorded into
  const recording = useAppSelector(selectRecording) && library === 'recordings'
  const items = useAppSelector(selectPlaylistItems)
  const rows = useAppSelector(selectSequencerRows)
  const [listing, setListing] = React.useState<RecordingsListing | null>(null)
  const [expanded, setExpanded] = useSessionSet(library)
  // the take heard last, or the folder clicked last (new folders go in it)
  const [selected, setSelected] = React.useState<string | null>(null)
  const [selectedFolder, setSelectedFolder] = React.useState<string | null>(null)
  const [renamingTake, setRenamingTake] = React.useState<string | null>(null)
  const [renamingFolder, setRenamingFolder] = React.useState<string | null>(null)
  const [menu, setMenu] = React.useState<Menu | null>(null)
  const [query, setQuery] = React.useState('')
  const boxRef = React.useRef<HTMLDivElement>(null)
  const listRef = React.useRef<HTMLDivElement>(null)

  React.useEffect(() => {
    let current = true
    invoke<RecordingsListing>('t_list_recordings', { library })
      .then(next => { if(current) setListing(next) })
      .catch(logError(`list the ${w.nouns}`))
    return () => { current = false }
  }, [revision, library, w.nouns])

  const takes = React.useMemo(() => listing?.takes ?? [], [listing])
  const folders = React.useMemo(() => recordingFolders(listing?.folders ?? []), [listing])
  const tree = React.useMemo(() => buildTree(folders, takes, take => take.folder), [folders, takes])

  const uses = React.useMemo(() => takeUses(
    items.filter(item => item.patternId === undefined).map(item => item.path),
    rows.map(row => row.path),
  ), [items, rows])

  const fail = (what: string) => (e: unknown) => {
    dispatch(setProjectNotice({ kind: 'error', message: `Couldn't ${what}: ${message(e)}` }))
  }

  // the clips and rows that play what moved follow it
  const applyMoved = (moved: MovedRecording) => {
    dispatch(playlistSlice.actions.reduceMovedClips(moved.movedClips))
    if(moved.sequencer) dispatch(setSequencer(moved.sequencer))
    if(moved.problem) dispatch(setProjectNotice({ kind: 'error', message: `Moved, but ${moved.problem}` }))
    dispatch(takesChanged())
  }

  // a folder moved or renamed: what was open, or selected, in it still is
  const followFolder = (from: string, to: string) => {
    setExpanded(prev => new Set(Array.from(prev, key => movedFolderPath(key, from, to))))
    if(selectedFolder !== null) setSelectedFolder(movedFolderPath(selectedFolder, from, to))
  }

  const open = (key: string) => {
    if(key === ROOT) return
    setExpanded(prev => {
      const next = new Set(prev)
      // its parents too, so it shows
      for(let at = key; at !== ROOT; at = parentFolder(at)) next.add(at)
      return next
    })
  }

  const toggle = (key: string) => {
    setSelectedFolder(key)
    setSelected(null)
    setExpanded(prev => {
      const next = new Set(prev)
      if(next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  const preview = (take: RecordingFile) => {
    setSelected(take.path)
    setSelectedFolder(null)
    invoke('t_preview_sample', { path: take.path }).catch(logError(`preview ${take.name}`))
  }

  // takes

  const renameTake = (take: RecordingFile, name: string | null) => {
    setRenamingTake(null)
    if(name === null) return

    invoke<MovedRecording>('t_rename_recording', { path: take.path, name, library })
      .then(moved => {
        if(selected === take.path) setSelected(moved.path)
        applyMoved(moved)
      })
      .catch(fail(`rename ${take.name}`))
  }

  const trashTake = async (take: RecordingFile) => {
    const sure = await ask(`Move "${take.name}" to the Trash?`, `Delete ${w.noun}`)
    if(!sure) return

    invoke('t_delete_recording', { path: take.path, library })
      .then(() => dispatch(takesChanged()))
      .catch(fail(`delete ${take.name}`))
  }

  // folders

  const newFolder = (parent: string) => {
    const siblings = (findFolder(tree, parent) ?? tree).folders.map(f => f.name)
    invoke<string>('t_create_recording_folder', { parent, name: freeName('New folder', siblings), library })
      .then(made => {
        open(parent)
        setSelectedFolder(made)
        setRenamingFolder(made)
        dispatch(takesChanged())
      })
      .catch(fail('make a folder'))
  }

  const renameFolder = (key: string, name: string | null) => {
    setRenamingFolder(null)
    if(name === null) return

    invoke<MovedRecording>('t_rename_recording_folder', { folder: key, name, library })
      .then(moved => {
        followFolder(key, moved.path)
        applyMoved(moved)
      })
      .catch(fail('rename the folder'))
  }

  const trashFolder = async (node: FolderNode<RecordingFile>) => {
    const count = allItems(node).length
    const what = count ? ` and the ${plural(count, w.noun)} in it` : ''
    const sure = await ask(`Move "${node.name}"${what} to the Trash?`, 'Delete folder')
    if(!sure) return

    invoke('t_delete_recording_folder', { folder: node.key, library })
      .then(() => dispatch(takesChanged()))
      .catch(fail(`delete ${node.name}`))
  }

  // dragging into folders

  const takeAt = (path: string) => takes.find(take => take.path === path)

  const canDrop = (item: DragItem, key: string): boolean => {
    if(recording) return false
    if(item.kind === 'sample') {
      const take = takeAt(item.path)
      return !!take && take.folder !== key
    }
    // (a folder of this page's own: not one dragged over from the other)
    if(item.kind === 'folder' && item.scope === library) {
      // not where it is already, and not inside itself
      return parentFolder(item.key) !== key && key !== item.key && !key.startsWith(`${item.key}/`)
    }
    return false
  }

  const onDrop = (item: DragItem, key: string) => {
    if(item.kind === 'sample') {
      invoke<MovedRecording>('t_move_recording', { path: item.path, toFolder: key, library })
        .then(moved => {
          if(selected === item.path) setSelected(moved.path)
          open(key)
          applyMoved(moved)
        })
        .catch(fail(`move the ${w.noun}`))
    } else if(item.kind === 'folder') {
      invoke<MovedRecording>('t_move_recording_folder', { folder: item.key, toFolder: key, library })
        .then(moved => {
          followFolder(item.key, moved.path)
          open(key)
          applyMoved(moved)
        })
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

  const busy = recording ? 'Wait for the recording to finish' : undefined

  const menuItems = (shown: Menu): ClipMenuItem[] => {
    if(shown.kind === 'take') {
      const { take } = shown
      const use = uses.get(take.path)
      const used = isUsed(use)
      return [
        { label: 'Rename', disabled: recording, title: busy, onSelect: () => setRenamingTake(take.path) },
        {
          label: 'Move to Trash',
          danger: true,
          disabled: used || recording,
          title: busy ?? (used ? `${describeUse(use)}. Remove those first` : undefined),
          onSelect: () => { trashTake(take).catch(fail(`delete ${take.name}`)) },
        },
      ]
    }

    if(shown.kind === 'folder') {
      const { node } = shown
      const used = allItems(node).filter(take => isUsed(uses.get(take.path))).length
      return [
        { label: 'New folder', disabled: recording, title: busy, onSelect: () => newFolder(node.key) },
        { label: 'Rename', disabled: recording, title: busy, onSelect: () => setRenamingFolder(node.key) },
        {
          label: 'Move to Trash',
          danger: true,
          disabled: used > 0 || recording,
          title: busy ?? (used ? `The project uses ${plural(used, w.noun)} in it. Remove those first` : undefined),
          onSelect: () => { trashFolder(node).catch(fail(`delete ${node.name}`)) },
        },
      ]
    }

    return [{ label: 'New folder', disabled: recording, title: busy, onSelect: () => newFolder(ROOT) }]
  }

  // what's listed

  const search = query.trim()
  let empty: string | null = null
  if(listing === null) empty = 'Loading…'
  else if(!takes.length && !folders.length) empty = recording ? 'Recording…' : w.none

  const renderTake = (take: RecordingFile, depth: number) => (renamingTake === take.path
    ? (
      <RenameField
        initial={take.name}
        label={`${w.title.slice(0, -1)} name`}
        depth={depth}
        onDone={name => renameTake(take, name)}
      />
    )
    : (
      <TakeRow
        take={take}
        library={library}
        use={uses.get(take.path)}
        selected={selected === take.path}
        depth={depth}
        showFolder={!!search}
        onPreview={preview}
        onMenu={(t, ev) => { setSelected(t.path); setMenu({ kind: 'take', take: t, ...at(ev) }) }}
      />
    ))

  let list: React.ReactNode
  if(search) {
    const found = searchTakes(takes, search)
    list = (
      <div className='pb-2'>
        {found.map(take => <React.Fragment key={take.path}>{renderTake(take, 0)}</React.Fragment>)}
        {!found.length && <div className='px-3 py-2 text-xs text-ink-faint'>No {w.nouns} match.</div>}
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
          { kind: 'folder', scope: library, key: node.key, label: node.name }, ev)}
        folderHint={node => {
          const count = allItems(node).length
          return count ? String(count) : undefined
        }}
        itemKey={take => take.path}
        renderItem={renderTake}
      >
        {empty && <div className='px-3 py-2 text-xs text-ink-faint'>{empty}</div>}
      </FolderTree>
    )
  }

  return (
    <div ref={boxRef} className='relative flex flex-col h-full min-h-0'>
      <BrowserSearch value={query} onChange={setQuery} label={`Search ${w.nouns}`}>
        <AnalogButton
          size='xs'
          aria-label='New folder'
          title={busy ?? (selectedFolder ? 'A new folder, in the folder selected' : 'A new folder')}
          disabled={recording}
          onPress={() => newFolder(selectedFolder ?? ROOT)}
        >
          <NewFolderIcon />
        </AnalogButton>
      </BrowserSearch>

      <div
        ref={listRef}
        className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden'
        role='tree'
        aria-label={w.title}
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

export default RecordingsBrowser
