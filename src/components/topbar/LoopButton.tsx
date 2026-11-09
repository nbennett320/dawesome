import React from 'react'
import { toggleLoop, selectLoopEnabled } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import LoopIcon from '../icons/LoopIcon'
import LatchKey from './LatchKey'

const LoopButton = () => {
  const loop = useAppSelector(selectLoopEnabled)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Loop' on={loop} onToggle={() => dispatch(toggleLoop())}>
      <LoopIcon fill='currentColor' />
    </LatchKey>
  )
}

export default LoopButton
