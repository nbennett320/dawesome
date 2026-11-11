import React from 'react'
import { useStore } from 'react-redux'
import type { RootState } from '../../state/store'
import { selectFocusedView } from '../../state/slices/windowSlice'
import { gestureActive } from '../../state/gesture'
import { SnapNote } from '../../editing/snap'
import { TOOL_LABELS, Tool, toolKey } from '../../editing/tools'
import { View } from '../../types/ui'
import { isTextTarget } from '../../util/keys'
import CutToolIcon from '../icons/CutToolIcon'
import DrawToolIcon from '../icons/DrawToolIcon'
import SelectToolIcon from '../icons/SelectToolIcon'
import LatchKey from '../topbar/LatchKey'
import SnapButton from './SnapButton'
import SnapInput from './SnapInput'

const ICONS: Record<Tool, React.ReactNode> = {
  select: <SelectToolIcon />,
  draw: <DrawToolIcon />,
  cut: <CutToolIcon />,
}

export interface TraySnap {
  enabled: boolean
  note: SnapNote
  onToggle: () => void
  onNote: (note: SnapNote) => void
  // what a screen reader calls the amount: "Playlist snap amount"
  label: string
}

interface Props {
  // the view this tray belongs to: its keys work while that view is the
  // focused pane's tab
  view: View
  // the tools the view has, in the order their keys are shown
  tools: ReadonlyArray<Tool>
  tool: Tool
  onTool: (tool: Tool) => void
  snap: TraySnap
}

// the mouse tools, then snap: the same strip in the playlist and the Piano
// Roll, each with its own tool and snap setting
const ToolTray = ({ view, tools, tool, onTool, snap }: Props) => {
  const store = useStore<RootState>()
  // the latest of these, for the listener on the window
  const latest = React.useRef({ tools, onTool })
  latest.current = { tools, onTool }

  // S selects, D draws and C cuts, while the view is the focused pane's tab
  // and nothing that takes typing has the key. a tool stays until another
  // is picked
  React.useEffect(() => {
    const onKeyDown = (ev: KeyboardEvent) => {
      const next = toolKey(ev)
      // (a drop-down takes no letters, and the snap one, just here, keeps
      // the keyboard after a pick: the keys still work then)
      if(!next || ev.repeat || isTextTarget(ev.target)) return
      if(!latest.current.tools.includes(next)) return
      if(selectFocusedView(store.getState()) !== view) return
      // not in the middle of a drag, which belongs to the tool it began in
      if(gestureActive()) return

      ev.preventDefault()
      latest.current.onTool(next)
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [store, view])

  return (
    <div className='ft-tray rounded-md flex flex-row items-end gap-1.5 px-2 py-1'>
      {tools.map(each => (
        <LatchKey key={each} label={TOOL_LABELS[each]} on={tool === each} onToggle={() => onTool(each)}>
          {ICONS[each]}
        </LatchKey>
      ))}
      <div className='w-3' />
      <SnapButton enabled={snap.enabled} onToggle={snap.onToggle} />
      <div className='self-center'>
        <SnapInput note={snap.note} enabled={snap.enabled} onNote={snap.onNote} label={snap.label} />
      </div>
    </div>
  )
}

export default ToolTray
