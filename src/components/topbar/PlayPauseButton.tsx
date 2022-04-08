import React from 'react'
import {
  togglePlay,
  selectPlaylistPlaying,
} from '../../state/slices/playlistSlice'
import Button from '../common/Button'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

const PlayPauseButton = () => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()

  return (
    <Button
      onClick={() => dispatch(togglePlay())}
      aria-label="Play/Pause Button"
    >
      {playing ? <>Pause</> : <>Play</>}
    </Button>
  )
}

export default PlayPauseButton
