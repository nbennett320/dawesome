import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useAppSelector } from '../../../hooks/redux'
import { selectPlaylistItems } from '../../../state/slices/playlistSlice'
import { selectRecording, selectTakesRevision } from '../../../state/slices/recordingSlice'
import { selectSequencerRows } from '../../../state/slices/sequencerSlice'
import { beginSampleDrag } from '../../../state/sampleDrag'
import {
  RecordingFile, TakeUse, describeUse, formatTakeDate, formatTakeDuration, searchTakes, takeUses,
} from '../../../types/recordings'
import BrowserSearch from '../browser/BrowserSearch'
import { WaveIcon } from '../sample-browser/SampleItem'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

interface RowProps {
  take: RecordingFile
  use: TakeUse | undefined
  selected: boolean
  onPreview: (take: RecordingFile) => void
}

// one take: click to hear it, drag it into the playlist (or the Sequencer)
const TakeRow = ({ take, use, selected, onPreview }: RowProps) => {
  const used = !!use && (use.clips > 0 || use.rows > 0)
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
    <button
      type='button'
      role='option'
      aria-selected={selected}
      title={[take.path, recorded && `Recorded ${recorded}`, describeUse(use)].filter(Boolean).join('\n')}
      className={`w-full h-6 flex flex-row items-center gap-1.5 pl-2 pr-3 text-left text-xs outline-none
        border-l-2 focus-visible:bg-sky-50 cursor-grab active:cursor-grabbing ${
          selected
            ? 'border-sky-500 bg-sky-50 text-slate-800'
            : `border-transparent hover:bg-slate-100 hover:text-slate-800 ${used ? 'text-slate-600' : 'text-slate-400'}`
        }`}
      onMouseDown={onMouseDown}
      onKeyDown={onKeyDown}
    >
      {/* a dot for takes the project uses, so the unused ones stand apart */}
      <span
        className={`w-1.5 h-1.5 shrink-0 rounded-full ${used ? 'bg-sky-500' : 'bg-transparent'}`}
        aria-label={used ? 'In use' : 'Not used'}
      />
      <WaveIcon />
      <span className='truncate'>{take.name}</span>
      <span className='ml-auto pl-2 shrink-0 text-[10px] text-slate-400 tabular-nums'>
        {formatTakeDuration(take.durationSecs)}
      </span>
      <span className='w-12 shrink-0 text-right text-[10px] text-slate-400 truncate'>
        {formatTakeDate(take.modifiedMs)}
      </span>
    </button>
  )
}

// the project's recorded takes, newest first: hear them, drag them back in,
// and see which ones the project isn't using
const RecordingsBrowser = () => {
  const revision = useAppSelector(selectTakesRevision)
  const recording = useAppSelector(selectRecording)
  const items = useAppSelector(selectPlaylistItems)
  const rows = useAppSelector(selectSequencerRows)
  const [takes, setTakes] = React.useState<RecordingFile[] | null>(null)
  const [selected, setSelected] = React.useState<string | null>(null)
  const [query, setQuery] = React.useState('')

  React.useEffect(() => {
    let current = true
    invoke<RecordingFile[]>('t_list_recordings', {})
      .then(list => { if(current) setTakes(list) })
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

  const shown = takes ? searchTakes(takes, query) : []

  let empty: string | null = null
  if(takes === null) empty = 'Loading…'
  else if(!takes.length) empty = recording ? 'Recording…' : 'Nothing recorded yet. Arm a track and press record.'
  else if(!shown.length) empty = 'No recordings match.'

  return (
    <div className='flex flex-col h-full min-h-0'>
      <BrowserSearch value={query} onChange={setQuery} label='Search recordings' />

      <div className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden pb-2' role='listbox' aria-label='Recordings'>
        {shown.map(take => (
          <TakeRow
            key={take.path}
            take={take}
            use={uses.get(take.path)}
            selected={selected === take.path}
            onPreview={preview}
          />
        ))}
        {empty && <div className='px-3 py-2 text-xs text-slate-400'>{empty}</div>}
      </div>
    </div>
  )
}

export default RecordingsBrowser
