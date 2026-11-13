import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog'
import { useAppDispatch, useAppSelector } from '../../../hooks/redux'
import { playlistSlice, selectPlaylistItems } from '../../../state/slices/playlistSlice'
import { setProjectNotice } from '../../../state/slices/projectSlice'
import {
  selectRecording, selectTakesRevision, takesChanged,
} from '../../../state/slices/recordingSlice'
import { selectSampleRows, setSequencer } from '../../../state/slices/sequencerSlice'
import { beginSampleDrag } from '../../../state/sampleDrag'
import { allItems, buildTree } from '../../../types/folderTree'
import {
  Library, MovedRecording, RecordingFile, RecordingsListing, TakeUse,
  describeUse, formatTakeDate, formatTakeDuration, recordingFolders, searchTakes, takeUses,
} from '../../../types/recordings'
import { plural } from '../../../util/helpers'
import { WaveIcon } from '../../icons'
import { ClipMenuItem } from '../../../views/playlist/ClipMenu'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import FolderBrowser, { ItemContext } from '../browser/FolderBrowser'
import RenameField from '../browser/RenameField'
import { useFail } from '../browser/useFail'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

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
  const fail = useFail()
  const w = words[library]
  const revision = useAppSelector(selectTakesRevision)
  // only takes are recorded into
  const recording = useAppSelector(selectRecording) && library === 'recordings'
  const items = useAppSelector(selectPlaylistItems)
  const sampleRows = useAppSelector(selectSampleRows)
  const [listing, setListing] = React.useState<RecordingsListing | null>(null)
  // the take heard last
  const [selected, setSelected] = React.useState<string | null>(null)
  const [renamingTake, setRenamingTake] = React.useState<string | null>(null)

  React.useEffect(() => {
    let current = true
    invoke<RecordingsListing>('list_recordings', { library })
      .then(next => { if(current) setListing(next) })
      .catch(logError(`list the ${w.nouns}`))
    return () => { current = false }
  }, [revision, library, w.nouns])

  const takes = React.useMemo(() => listing?.takes ?? [], [listing])
  const folders = React.useMemo(() => recordingFolders(listing?.folders ?? []), [listing])
  const tree = React.useMemo(() => buildTree(folders, takes, take => take.folder), [folders, takes])

  const uses = React.useMemo(() => takeUses(
    items.filter(item => item.patternId === undefined).map(item => item.path),
    sampleRows.map(row => row.instrument.path),
  ), [items, sampleRows])

  // the clips and rows that play what moved follow it
  const applyMoved = (moved: MovedRecording) => {
    dispatch(playlistSlice.actions.reduceMovedClips(moved.movedClips))
    if(moved.sequencer) dispatch(setSequencer(moved.sequencer))
    if(moved.problem) dispatch(setProjectNotice({ kind: 'error', message: `Moved, but ${moved.problem}` }))
    dispatch(takesChanged())
  }

  // the engine moves a folder or take, and says where it went
  const moved = async (command: string, args: Record<string, unknown>) => {
    const result = await invoke<MovedRecording>(command, { ...args, library })
    applyMoved(result)
    return result
  }

  const busy = recording ? 'Wait for the recording to finish' : undefined

  // takes

  const preview = (take: RecordingFile, { deselectFolder }: ItemContext) => {
    setSelected(take.path)
    deselectFolder()
    invoke('preview_sample', { path: take.path }).catch(logError(`preview ${take.name}`))
  }

  const renameTake = (take: RecordingFile, name: string | null) => {
    setRenamingTake(null)
    if(name === null) return

    moved('rename_recording', { path: take.path, name })
      .then(result => { if(selected === take.path) setSelected(result.path) })
      .catch(fail(`rename ${take.name}`))
  }

  const trashTake = async (take: RecordingFile) => {
    const sure = await ask(`Move "${take.name}" to the Trash?`, `Delete ${w.noun}`)
    if(!sure) return

    invoke('delete_recording', { path: take.path, library })
      .then(() => dispatch(takesChanged()))
      .catch(fail(`delete ${take.name}`))
  }

  const takeMenu = (take: RecordingFile): ClipMenuItem[] => {
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

  const renderTake = (take: RecordingFile, depth: number, context: ItemContext) => (renamingTake === take.path
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
        showFolder={context.searching}
        onPreview={t => preview(t, context)}
        onMenu={(t, ev) => {
          setSelected(t.path)
          context.openMenu(ev, () => takeMenu(t))
        }}
      />
    ))

  let empty: string | null = null
  if(listing === null) empty = 'Loading…'
  else if(!takes.length && !folders.length) empty = recording ? 'Recording…' : w.none

  return (
    <FolderBrowser
      scope={library}
      label={w.title}
      searchLabel={`Search ${w.nouns}`}
      noun={w.noun}
      folders={folders}
      tree={tree}
      itemKey={take => take.path}
      renderItem={renderTake}
      search={query => searchTakes(takes, query).map(take => ({ item: take }))}
      noMatch={`No ${w.nouns} match.`}
      emptyMessage={empty}
      busy={busy}
      createFolder={async (parent, name) => {
        const made = await invoke<string>('create_recording_folder', { parent, name, library })
        dispatch(takesChanged())
        return made
      }}
      renameFolder={async (key, name) => (await moved('rename_recording_folder', { folder: key, name })).path}
      moveFolder={async (key, into) => (await moved('move_recording_folder', { folder: key, toFolder: into })).path}
      deleteFolder={async node => {
        await invoke('delete_recording_folder', { folder: node.key, library })
        dispatch(takesChanged())
      }}
      deleteLabel='Move to Trash'
      deletePrompt={(name, what) => `Move "${name}"${what} to the Trash?`}
      blockDelete={node => {
        const used = allItems(node).filter(take => isUsed(uses.get(take.path))).length
        return used ? `The project uses ${plural(used, w.noun)} in it. Remove those first` : undefined
      }}
      canDropItem={(item, key) => {
        if(item.kind !== 'sample') return false
        const take = takes.find(t => t.path === item.path)
        return !!take && take.folder !== key
      }}
      dropItem={async (item, key) => {
        if(item.kind !== 'sample') return
        const result = await moved('move_recording', { path: item.path, toFolder: key })
        if(selected === item.path) setSelected(result.path)
      }}
      dropFailure={`move the ${w.noun}`}
      onSelectFolder={() => setSelected(null)}
    />
  )
}

export default RecordingsBrowser
