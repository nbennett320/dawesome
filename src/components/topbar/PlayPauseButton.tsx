import React from 'react'
import { 
  togglePlay, 
  selectPlaylistPlaying,
  PlayPausePlaylistState
} from '../../state/slices/playPausePlaylistSlice'
import Button from '../common/Button'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

const PlayPauseButton = () => {
  const playing = useAppSelector<PlayPausePlaylistState>(selectPlaylistPlaying) 
  const dispatch = useAppDispatch()

  return (
    <Button 
      onClick={() => dispatch(togglePlay())}
      aria-label="Play/Pause Button"
    > 
      {playing.value ? <>Pause</> : <>Play</>}
    </Button>
  )
}

export default PlayPauseButton
