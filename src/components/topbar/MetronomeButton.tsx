import React from 'react'
import { toggleMetronome, selectMetronomeEnabled } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import LatchKey from './LatchKey'
import { MetronomeIcon } from '../icons'

const MetronomeButton = () => {
  const metronome = useAppSelector(selectMetronomeEnabled)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Metronome' on={metronome} onToggle={() => dispatch(toggleMetronome())}>
      <MetronomeIcon />
    </LatchKey>
  )
}

export default MetronomeButton
