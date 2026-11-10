import React from 'react'
import { useStore } from 'react-redux'
import { RootState } from 'state/store'
import { selectTool, setTool } from '../../state/slices/playlistSlice'
import { selectFocusedView } from '../../state/slices/windowSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { View } from '../../types/ui'
import { isTextTarget, toolKey } from '../../util/keys'
import CutToolIcon from '../../components/icons/CutToolIcon'
import SelectToolIcon from '../../components/icons/SelectToolIcon'
import LatchKey from '../../components/topbar/LatchKey'
import SnapButton from '../../components/topbar/SnapButton'
import SnapInput from '../../components/topbar/SnapInput'
import { gestureActive } from '../../state/gesture'

// the strip above the ruler: the mouse tools, then snap
const PlaylistTray = () => {
  const tool = useAppSelector(selectTool)
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()

  // S selects and C cuts, while the playlist is the focused pane's tab and
  // nothing that takes typing has the key. Cut stays until S is pressed
  React.useEffect(() => {
    const onKeyDown = (ev: KeyboardEvent) => {
      const next = toolKey(ev)
      // (a drop-down takes no letters, and the snap one, just here, keeps
      // the keyboard after a pick: the keys still work then)
      if(!next || ev.repeat || isTextTarget(ev.target)) return
      if(selectFocusedView(store.getState()) !== View.Playlist) return
      // not in the middle of a drag, which belongs to the tool it began in
      if(gestureActive()) return

      ev.preventDefault()
      dispatch(setTool(next))
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [store, dispatch])

  return (
    <div className='shrink-0 px-2 py-1'>
      <div className='ft-tray rounded-md flex flex-row items-end gap-1.5 px-2 py-1'>
        <LatchKey label='Select (S)' on={tool === 'select'} onToggle={() => dispatch(setTool('select'))}>
          <SelectToolIcon />
        </LatchKey>
        <LatchKey label='Cut (C)' on={tool === 'cut'} onToggle={() => dispatch(setTool('cut'))}>
          <CutToolIcon />
        </LatchKey>
        <div className='w-3' />
        <SnapButton />
        <div className='self-center'>
          <SnapInput />
        </div>
      </div>
    </div>
  )
}

export default PlaylistTray
