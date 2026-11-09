import React from 'react'
import { togglePlay, selectPlaylistPlaying } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { AnalogButton, PauseIcon, PlayIcon } from '../fourtrack'

// latches down while playing, like a tape deck's play key
const PlayPauseButton = () => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()

  return (
    <AnalogButton
      size='sm'
      latched={playing}
      aria-label={playing ? 'Pause' : 'Play'}
      title={playing ? 'Pause' : 'Play'}
      onPress={() => dispatch(togglePlay())}
    >
      {playing ? <PauseIcon /> : <PlayIcon />}
    </AnalogButton>
  )
}

export default PlayPauseButton
