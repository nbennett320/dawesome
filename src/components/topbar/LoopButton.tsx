import React from 'react'
import { toggleLoop, selectLoopEnabled } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import LatchKey from './LatchKey'
import { LoopIcon } from '../icons'

const LoopButton = () => {
  const loop = useAppSelector(selectLoopEnabled)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Loop'
      hint={loop ? 'Loop is on: playback jumps back to the start after the last clip. Click to turn it off' : 'Loop is off. Click to repeat the song from the start after the last clip'}
      on={loop} onToggle={() => dispatch(toggleLoop())}>
      <LoopIcon />
    </LatchKey>
  )
}

export default LoopButton
