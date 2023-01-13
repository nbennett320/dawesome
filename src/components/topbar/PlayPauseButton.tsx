import React from 'react'
import {
  togglePlay,
  selectPlaylistPlaying,
} from '../../state/slices/playlistSlice'
import Button from '../common/Button'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import PlayIcon from '../icons/PlayIcon'
import PauseIcon from '../icons/PauseIcon'

interface Props {
  className?: string
}

const PlayPauseButton = (props: Props) => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()
  
  return (
    <Button
      onClick={() => dispatch(togglePlay())}
      className={`bg-gray-400 hover:bg-gray-500 py-1 px-3 rounded inline-flex items-center ${props.className}`}
      aria-label="Play/Pause Button"
    >
      {playing ? <PauseIcon fill="white" /> : <PlayIcon fill="white" />}
    </Button>
  )
}

export default PlayPauseButton
