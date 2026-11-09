import React from 'react'
import { invoke } from '@tauri-apps/api'
import { ask } from '@tauri-apps/api/dialog'
import { useAppDispatch, useAppSelector } from '../../../hooks/redux'
import { playlistSlice, selectPlaylistItems } from '../../../state/slices/playlistSlice'
import { setProjectNotice } from '../../../state/slices/projectSlice'
import {
  selectRecording, selectTakesRevision, takesChanged,
} from '../../../state/slices/recordingSlice'
import { selectSequencerRows, setSequencer } from '../../../state/slices/sequencerSlice'
import { beginSampleDrag } from '../../../state/sampleDrag'
import {
  MovedRecording, RecordingFile, RecordingsListing, TakeUse, describeUse, formatTakeDate, formatTakeDuration, searchTakes, takeUses,
} from '../../../types/recordings'
import ClipMenu, { ClipMenuItem } from '../../../views/playlist/ClipMenu'
import BrowserSearch from '../browser/BrowserSearch'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import { WaveIcon } from '../sample-browser/SampleItem'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

const isUsed = (use: TakeUse | undefined) => !!use && (use.clips > 0 || use.rows > 0)

const MAX_NAME_CHARS = 120

// a take's name, being changed. enter keeps it, escape (or an unchanged
// name) leaves it as it was
const RenameField = ({ take, onDone }: { take: RecordingFile, onDone: (name: string | null) => void }) => {
  const [draft, setDraft] = React.useState(take.name)
  const done = React.useRef(false)

  const finish = (name: string | null) => {
    if(done.current) return
    done.current = true
    onDone(name !== null && name.trim() && name.trim() !== take.name ? name.trim() : null)
  }

  return (
    <div className='h-6 flex flex-row items-center pl-2 pr-3 border-l-2 border-sky-500 bg-sky-50'>
      <input
        type='text'
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        aria-label='Recording name'
        maxLength={MAX_NAME_CHARS}
        className='ft-scribble w-full h-5 text-xs px-1 outline-none ring-2 ring-sky-400'
        value={draft}
        onFocus={e => e.currentTarget.select()}
        onChange={e => setDraft(e.currentTarget.value)}
        onBlur={() => finish(draft)}
        onKeyDown={e => {
          if(e.key === 'Enter') finish(draft)
          if(e.key === 'Escape') finish(null)
        }}
      />
    </div>
  )
}

interface RowProps {
  take: RecordingFile
  use: TakeUse | undefined
  selected: boolean
  onPreview: (take: RecordingFile) => void
  onMenu: (take: RecordingFile, ev: React.MouseEvent) => void
}

// one take: click to hear it, drag it into the playlist (or the Sequencer),
// right-click to rename or delete it
const TakeRow = ({ take, use, selected, onPreview, onMenu }: RowProps) => {
  const recorded = take.modifiedMs === null ? '' : new Date(take.modifiedMs).toLocaleString()

  const onMouseDown = (ev: React.MouseEvent) => {
    if(ev.button !== 0) return
    onPreview(take)
    beginSampleDrag({ kind: 'sample', path: take.path }, ev)
  }

  const onKeyDown = (ev: React.KeyboardEvent) => {
    if(ev.key !== 'Enter' && ev.key !== ' ') return
    ev.preventDefault()
    onPreview(take)
  }

  return (
    <BrowserRow
      role='option'
      selected={selected}
      used={isUsed(use)}
      title={[take.path, recorded && `Recorded ${recorded}`, describeUse(use)].filter(Boolean).join('\n')}
      onMouseDown={onMouseDown}
      onKeyDown={onKeyDown}
      onContextMenu={ev => onMenu(take, ev)}
    >
      <WaveIcon />
      <span className='truncate'>{take.name}</span>
      <RowHint className='ml-auto pl-2 tabular-nums'>{formatTakeDuration(take.durationSecs)}</RowHint>
      <RowHint className='w-12 text-right'>{formatTakeDate(take.modifiedMs)}</RowHint>
    </BrowserRow>
  )
}

// the project's recorded takes, newest first: hear them, drag them back in,
// see which ones the project isn't using, and rename or bin them
const RecordingsBrowser = () => {
  const dispatch = useAppDispatch()
  const revision = useAppSelector(selectTakesRevision)
  const recording = useAppSelector(selectRecording)
  const items = useAppSelector(selectPlaylistItems)
  const rows = useAppSelector(selectSequencerRows)
  const [takes, setTakes] = React.useState<RecordingFile[] | null>(null)
  const [selected, setSelected] = React.useState<string | null>(null)
  const [renaming, setRenaming] = React.useState<string | null>(null)
  const [menu, setMenu] = React.useState<{ take: RecordingFile, x: number, y: number } | null>(null)
  const [query, setQuery] = React.useState('')
  const boxRef = React.useRef<HTMLDivElement>(null)

  React.useEffect(() => {
    let current = true
    // folders are listed too, for the tree to come. flat for now
    invoke<RecordingsListing>('t_list_recordings', {})
      .then(listing => { if(current) setTakes(listing.takes) })
      .catch(logError('list the recordings'))
    return () => { current = false }
  }, [revision])

  const uses = React.useMemo(() => takeUses(
    items.filter(item => item.patternId === undefined).map(item => item.path),
    rows.map(row => row.path),
  ), [items, rows])

  const preview = (take: RecordingFile) => {
    setSelected(take.path)
    invoke('t_preview_sample', { path: take.path }).catch(logError(`preview ${take.name}`))
  }

  const fail = (what: string) => (e: unknown) => {
    dispatch(setProjectNotice({ kind: 'error', message: `Couldn't ${what}: ${message(e)}` }))
  }

  const rename = (take: RecordingFile, name: string | null) => {
    setRenaming(null)
    if(name === null) return

    invoke<MovedRecording>('t_rename_recording', { path: take.path, name })
      .then(renamed => {
        // the clips and rows that play it follow it to its new name
        dispatch(playlistSlice.actions.reduceMovedClips(renamed.movedClips))
        if(renamed.sequencer) dispatch(setSequencer(renamed.sequencer))
        if(selected === take.path) setSelected(renamed.path)
        dispatch(takesChanged())
      })
      .catch(fail(`rename ${take.name}`))
  }

  const remove = async (take: RecordingFile) => {
    const sure = await ask(`Move "${take.name}" to the Trash?`, 'Delete recording')
    if(!sure) return

    invoke('t_delete_recording', { path: take.path })
      .then(() => dispatch(takesChanged()))
      .catch(fail(`delete ${take.name}`))
  }

  const openMenu = (take: RecordingFile, ev: React.MouseEvent) => {
    ev.preventDefault()
    const box = boxRef.current?.getBoundingClientRect()
    if(!box) return
    setSelected(take.path)
    setMenu({ take, x: ev.clientX - box.left, y: ev.clientY - box.top })
  }

  const closeMenu = React.useCallback(() => setMenu(null), [])

  const menuItems = (take: RecordingFile): ClipMenuItem[] => {
    const used = isUsed(uses.get(take.path))
    const busy = recording ? 'Wait for the recording to finish' : undefined
    return [
      {
        label: 'Rename',
        disabled: recording,
        title: busy,
        onSelect: () => setRenaming(take.path),
      },
      {
        label: 'Move to Trash',
        danger: true,
        disabled: used || recording,
        title: busy ?? (used ? `${describeUse(uses.get(take.path))}. Remove those first` : undefined),
        onSelect: () => { remove(take).catch(fail(`delete ${take.name}`)) },
      },
    ]
  }

  const shown = takes ? searchTakes(takes, query) : []

  let empty: string | null = null
  if(takes === null) empty = 'Loading…'
  else if(!takes.length) empty = recording ? 'Recording…' : 'Nothing recorded yet. Arm a track and press record.'
  else if(!shown.length) empty = 'No recordings match.'

  return (
    <div ref={boxRef} className='relative flex flex-col h-full min-h-0'>
      <BrowserSearch value={query} onChange={setQuery} label='Search recordings' />

      <div className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden pb-2' role='listbox' aria-label='Recordings'>
        {shown.map(take => (renaming === take.path
          ? <RenameField key={take.path} take={take} onDone={name => rename(take, name)} />
          : (
            <TakeRow
              key={take.path}
              take={take}
              use={uses.get(take.path)}
              selected={selected === take.path}
              onPreview={preview}
              onMenu={openMenu}
            />
          )))}
        {empty && <div className='px-3 py-2 text-xs text-slate-400'>{empty}</div>}
      </div>

      {menu && <ClipMenu x={menu.x} y={menu.y} items={menuItems(menu.take)} onClose={closeMenu} />}
    </div>
  )
}

export default RecordingsBrowser
