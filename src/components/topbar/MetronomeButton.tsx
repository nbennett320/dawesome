import React from 'react'
import { toggleMetronome, selectMetronomeEnabled } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import MetronomeIcon from '../icons/MetronomeIcon'
import LatchKey from './LatchKey'

const MetronomeButton = () => {
  const metronome = useAppSelector(selectMetronomeEnabled)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Metronome' on={metronome} onToggle={() => dispatch(toggleMetronome())}>
      <MetronomeIcon fill='currentColor' />
    </LatchKey>
  )
}

export default MetronomeButton
