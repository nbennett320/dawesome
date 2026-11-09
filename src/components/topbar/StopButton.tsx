import React from 'react'
import { stopPlaylist } from '../../state/slices/playlistSlice'
import { useAppDispatch } from '../../hooks/redux'
import { AnalogButton, StopIcon } from '../fourtrack'

// stops, and goes back to the start
const StopButton = () => {
  const dispatch = useAppDispatch()

  return (
    <AnalogButton
      size='sm'
      aria-label='Stop'
      title='Stop, and go back to the start'
      onPress={() => dispatch(stopPlaylist())}
    >
      <StopIcon />
    </AnalogButton>
  )
}

export default StopButton
