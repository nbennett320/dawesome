import React from 'react'
import { toggleLoop, selectLoopEnabled } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import LatchKey from './LatchKey'
import { LoopIcon } from '../icons'

const LoopButton = () => {
  const loop = useAppSelector(selectLoopEnabled)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Loop' on={loop} onToggle={() => dispatch(toggleLoop())}>
      <LoopIcon />
    </LatchKey>
  )
}

export default LoopButton
