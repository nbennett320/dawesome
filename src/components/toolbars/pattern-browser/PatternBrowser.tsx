import React from 'react'
import { useAppDispatch, useAppSelector } from '../../../hooks/redux'
import {
  loadSequencer,
  selectPattern,
  selectPatternRun,
  selectPatterns,
  selectSelectedPatternId,
} from '../../../state/slices/sequencerSlice'
import { selectPlaylistItems } from '../../../state/slices/playlistSlice'
import { openTab } from '../../../state/slices/windowSlice'
import { beginSampleDrag } from '../../../state/sampleDrag'
import { Pattern, patternHint, patternLabel, searchPatterns } from '../../../types/sequencer'
import { View } from '../../../types/ui'
import { useTransport } from '../../../views/transport/useTransport'
import { Led } from '../../fourtrack'
import BrowserSearch from '../browser/BrowserSearch'
import BrowserRow, { RowHint } from '../browser/BrowserRow'

// pixels the mouse can move between press and release and still count as a
// click (the same as a drag's threshold in state/sampleDrag.ts)
const CLICK_SLOP = 5

interface RowProps {
  pattern: Pattern
  hint: string
  // placed in the playlist at least once
  used: boolean
  selected: boolean
  playing: boolean
  onSelect: (pattern: Pattern) => void
}

// one pattern: click to show it in the Sequencer, drag it into the playlist
const PatternRow = ({ pattern, hint, used, selected, playing, onSelect }: RowProps) => {
  const label = patternLabel(pattern)

  // where the press started, so letting go after a drag isn't a click
  const down = React.useRef<{ x: number, y: number } | null>(null)

  // only a drag starts on mousedown: showing the Sequencer straight away could
  // hide the playlist (when they share a pane) before the pattern's dropped
  const onMouseDown = (ev: React.MouseEvent) => {
    if(ev.button !== 0) return
    down.current = { x: ev.clientX, y: ev.clientY }
    beginSampleDrag({ kind: 'pattern', patternId: pattern.id, label }, ev)
  }

  // also enter and space, which buttons turn into clicks
  const onClick = (ev: React.MouseEvent) => {
    const from = down.current
    down.current = null
    // detail is 0 for a click from the keyboard
    if(ev.detail && from && Math.hypot(ev.clientX - from.x, ev.clientY - from.y) >= CLICK_SLOP) return
    onSelect(pattern)
  }

  return (
    <BrowserRow
      role='option'
      selected={selected}
      used={used}
      title={`${label}. Click to show it in the Sequencer, drag into the playlist to place it`}
      onMouseDown={onMouseDown}
      onClick={onClick}
    >
      <Led color='lime' state={playing ? 'on' : 'off'} className='shrink-0 scale-75' />
      <span className='truncate'>{label}</span>
      <RowHint className='ml-auto pl-2'>{hint}</RowHint>
    </BrowserRow>
  )
}

// the Sequencer's patterns, to find by name and drag into the playlist
const PatternBrowser = () => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatterns)
  const selectedId = useAppSelector(selectSelectedPatternId)
  const run = useAppSelector(selectPatternRun)
  const items = useAppSelector(selectPlaylistItems)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4
  const [query, setQuery] = React.useState('')

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

  const show = (pattern: Pattern) => {
    dispatch(selectPattern(pattern.id))
    dispatch(openTab(View.Sequencer))
  }

  const shown = searchPatterns(patterns, query)

  return (
    <div className='flex flex-col h-full min-h-0'>
      <BrowserSearch value={query} onChange={setQuery} label='Search patterns' />

      <div className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden pb-2' role='listbox' aria-label='Patterns'>
        {shown.map(pattern => (
          <PatternRow
            key={pattern.id}
            pattern={pattern}
            hint={patternHint(pattern, beatsPerBar, clipCounts.get(pattern.id) ?? 0)}
            used={(clipCounts.get(pattern.id) ?? 0) > 0}
            selected={pattern.id === selectedId}
            playing={pattern.id === run?.patternId}
            onSelect={show}
          />
        ))}
        {!patterns.length && (
          <div className='px-3 py-2 text-xs text-slate-400'>
            No patterns yet. Make one in the Sequencer.
          </div>
        )}
        {!!patterns.length && !shown.length && (
          <div className='px-3 py-2 text-xs text-slate-400'>No patterns match.</div>
        )}
      </div>
    </div>
  )
}

export default PatternBrowser
