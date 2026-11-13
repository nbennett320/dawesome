import React from 'react'
import { toggleMetronome, selectMetronomeEnabled } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import LatchKey from './LatchKey'
import { MetronomeIcon } from '../icons'

const MetronomeButton = () => {
  const metronome = useAppSelector(selectMetronomeEnabled)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Metronome'
      hint={metronome ? 'Metronome is on: clicks on every beat. Click to turn it off' : 'Metronome is off. Click to hear a click on every beat'}
      on={metronome} onToggle={() => dispatch(toggleMetronome())}>
      <MetronomeIcon />
    </LatchKey>
  )
}

export default MetronomeButton
