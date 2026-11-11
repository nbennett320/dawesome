import React from 'react'
import {
  selectSnapEnabled, selectSnapNote, selectTool, setSnapSubdivision, setTool, toggleSnap,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { Tool } from '../../editing/tools'
import { View } from '../../types/ui'
import ToolTray from '../../components/editing/ToolTray'

// the playlist's tools
const TOOLS: ReadonlyArray<Tool> = ['select', 'draw', 'cut']

// the strip above the ruler: the mouse tools, then snap
const PlaylistTray = () => {
  const tool = useAppSelector(selectTool)
  const snapEnabled = useAppSelector(selectSnapEnabled)
  const snapNote = useAppSelector(selectSnapNote)
  const dispatch = useAppDispatch()

  return (
    <div className='shrink-0 px-2 py-1'>
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
    </div>
  )
}

export default PlaylistTray
