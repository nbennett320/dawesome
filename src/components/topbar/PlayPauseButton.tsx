import React from 'react'
import { selectPlaylistPlaying } from '../../state/slices/playlistSlice'
import { togglePlayHere } from '../../state/slices/sequencerSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { AnalogButton } from '../fourtrack'
import { PauseIcon, PlayIcon } from '../icons'

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
      data-hint={playing
        ? 'Pause: stop where the playhead is (Space)'
        : 'Play from the playhead, or loop the open pattern in the sequencer or piano roll (Space)'}
      // in the Sequencer, loops its pattern instead (see togglePlayHere)
      onPress={() => dispatch(togglePlayHere())}
    >
      {playing ? <PauseIcon /> : <PlayIcon />}
    </AnalogButton>
  )
}

export default PlayPauseButton
