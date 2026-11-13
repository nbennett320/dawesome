import React from 'react'
import {
  selectGridStep, selectSnapEnabled, selectSnapNote, selectTool, setSnapSubdivision, setTool, toggleSnap,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { Tool } from '../../editing/tools'
import { View } from '../../types/ui'
import ToolTray from '../../components/editing/ToolTray'
import { stepLabel } from '../../editing/beatGrid'
import { theme } from './sketch/theme'

// the playlist's tools
const TOOLS: ReadonlyArray<Tool> = ['select', 'draw', 'cut']

// the strip above the ruler: the mouse tools, snap, then the grid's step
const PlaylistTray = () => {
  const tool = useAppSelector(selectTool)
  const snapEnabled = useAppSelector(selectSnapEnabled)
  const snapNote = useAppSelector(selectSnapNote)
  const gridStep = useAppSelector(selectGridStep)
  const dispatch = useAppDispatch()

  return (
    <div className='shrink-0 px-2 py-1 flex flex-row items-center gap-3'>
      <ToolTray
        view={View.Playlist}
        tools={TOOLS}
        tool={tool}
        onTool={next => dispatch(setTool(next))}
        snap={{
          enabled: snapEnabled,
          note: snapNote,
          onToggle: () => { dispatch(toggleSnap()) },
          onNote: note => { dispatch(setSnapSubdivision(note)) },
          label: 'Playlist snap amount',
        }}
      />
      {/* how far apart the grid lines are at this zoom: they follow it */}
      <div className='flex flex-row items-center gap-1.5' title='Grid: follows the zoom'>
        <span className='ft-label'>Grid</span>
        <span className='ft-readout text-sm tabular-nums' aria-live='polite'>
          {stepLabel(gridStep, theme.beatsPerBar)}
        </span>
      </div>
    </div>
  )
}

export default PlaylistTray
